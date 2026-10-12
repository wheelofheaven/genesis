//! Read actual terrain receivers through the production cloud shadow pipeline.
use genesis_renderer::{atmosphere::Atmosphere, camera::Camera, geometry::Vertex, gpu::Gpu};
use glam::{Vec2, Vec3};

#[test]
#[ignore = "requires native GPU access"]
fn cloud_shadows_follow_wind_stop_reverse_and_clear_on_native_gpu() {
    pollster::block_on(async {
        let (mut gpu, camera) = receiver_fixture().await;
        let mut atmosphere = Atmosphere {
            density: 0.,
            ..Default::default()
        };
        atmosphere.clouds.shadow_strength = 0.;
        let mut render =
            |atmosphere, time| super::sky::render_hdr(&mut gpu, &camera, atmosphere, time, 128, 64);
        let clear = render(atmosphere, 0.);
        atmosphere.clouds.shadow_strength = 1.;
        let initial = render(atmosphere, 0.);
        let darkened = clear
            .chunks_exact(4)
            .zip(initial.chunks_exact(4))
            .filter(|(a, b)| a[0] > b[0] + 100)
            .count();
        let lit = clear
            .chunks_exact(4)
            .zip(initial.chunks_exact(4))
            .filter(|(a, b)| a[0].abs_diff(b[0]) < 10)
            .count();
        assert!(darkened > 100, "visible cloud shadows: {darkened}");
        assert!(lit > 100, "clear gaps between clouds: {lit}");
        atmosphere.clouds.wind_metres_per_second = Vec2::new(20., 10.);
        assert_eq!(
            initial,
            render(atmosphere, 0.),
            "changing wind cannot teleport clouds"
        );
        let moved = render(atmosphere, 60.);
        assert_ne!(initial, moved, "wind must move the ground shadows");
        atmosphere.clouds.wind_metres_per_second = Vec2::ZERO;
        assert_eq!(
            moved,
            render(atmosphere, 120.),
            "calm must freeze the cloud field"
        );
        atmosphere.clouds.wind_metres_per_second = Vec2::new(-20., -10.);
        assert_eq!(moved, render(atmosphere, 120.));
        assert_eq!(
            initial,
            render(atmosphere, 180.),
            "reversing the same distance restores the field"
        );
        atmosphere.clouds.cover = 0.;
        assert_eq!(
            clear,
            render(atmosphere, 180.),
            "clear weather removes cached shadows"
        );
        atmosphere.clouds.cover = 0.46;
        assert_eq!(
            initial,
            render(atmosphere, 180.),
            "returning to fair restores the cached shadow field"
        );
        atmosphere.clouds.extinction_per_metre = 0.;
        assert_eq!(
            clear,
            render(atmosphere, 180.),
            "transparent clouds cannot cast a shadow"
        );
    });
}

async fn receiver_fixture() -> (Gpu, Camera) {
    let instance = wgpu::Instance::default();
    let adapter = instance
        .request_adapter(&Default::default())
        .await
        .expect("adapter");
    let mut gpu = Gpu::new(&adapter).await.expect("renderer");
    let corners = [
        [-1000., 0., -1000.],
        [-1000., 0., 1000.],
        [1000., 0., 1000.],
        [1000., 0., -1000.],
    ];
    gpu.upload_scene(&[0, 1, 2, 0, 2, 3].map(|i| Vertex {
        position: corners[i],
        color: [0.6, 0.6, 0.6, 3.],
    }));
    let mut camera = Camera::default();
    camera.enter_ground(Vec3::new(0., 300., 0.), 0.085);
    camera.look_at_ground(Vec3::ZERO);
    (gpu, camera)
}

#[test]
#[ignore = "requires native GPU access"]
fn cloud_shade_is_subtle_and_caches_only_rebuild_when_needed_on_native_gpu() {
    pollster::block_on(async {
        let (mut gpu, mut camera) = receiver_fixture().await;
        let mut atmosphere = Atmosphere {
            density: 0.,
            ..Default::default()
        };
        atmosphere.clouds.shadow_strength = 0.;
        let clear = super::sky::render_hdr(&mut gpu, &camera, atmosphere, 0., 128, 64);
        assert!(gpu.atmosphere_updates.sky_lookup);
        assert!(
            !gpu.atmosphere_updates.cloud_shadows,
            "disabled shadows skip generation"
        );
        atmosphere.clouds.shadow_strength = Atmosphere::default().clouds.shadow_strength;
        let subtle = super::sky::render_hdr(&mut gpu, &camera, atmosphere, 0., 128, 64);
        assert!(!gpu.atmosphere_updates.sky_lookup);
        assert!(gpu.atmosphere_updates.cloud_shadows);
        // Compare linear HDR red values, rather than display tone-map brightness.
        let half = |v: u16| {
            let exponent = i32::from((v >> 10) & 31);
            let mantissa = f32::from(v & 1023);
            if exponent == 0 {
                mantissa * 2_f32.powi(-24)
            } else {
                (1. + mantissa / 1024.) * 2_f32.powi(exponent - 15)
            }
        };
        let ratios: Vec<_> = clear
            .chunks_exact(4)
            .zip(subtle.chunks_exact(4))
            .map(|(a, b)| half(b[0]) / half(a[0]).max(0.0001))
            .collect();
        assert!(
            ratios.iter().all(|v| *v >= 0.80 && *v <= 1.002),
            "shade must stay gentle"
        );
        assert!(
            ratios.iter().filter(|v| **v < 0.95).count() > 100,
            "shade must remain visible"
        );
        assert!(
            ratios.iter().filter(|v| **v > 0.995).count() > 100,
            "clear gaps remain lit"
        );
        super::sky::render_hdr(&mut gpu, &camera, atmosphere, 60., 128, 64);
        assert!(
            !gpu.atmosphere_updates.sky_lookup && !gpu.atmosphere_updates.cloud_shadows,
            "wind reuses both maps"
        );
        atmosphere.clouds.shadow_strength = 0.15;
        super::sky::render_hdr(&mut gpu, &camera, atmosphere, 60., 128, 64);
        assert!(
            !gpu.atmosphere_updates.cloud_shadows,
            "strength is a sampling parameter"
        );
        camera.focus.y += 10.;
        super::sky::render_hdr(&mut gpu, &camera, atmosphere, 60., 128, 64);
        assert!(gpu.atmosphere_updates.sky_lookup && !gpu.atmosphere_updates.cloud_shadows);
        atmosphere.clouds.cover = 0.65;
        super::sky::render_hdr(&mut gpu, &camera, atmosphere, 60., 128, 64);
        assert!(!gpu.atmosphere_updates.sky_lookup && gpu.atmosphere_updates.cloud_shadows);
        atmosphere.survey = true;
        super::sky::render_hdr(&mut gpu, &camera, atmosphere, 60., 128, 64);
        assert!(!gpu.atmosphere_updates.sky_lookup && !gpu.atmosphere_updates.cloud_shadows);
        atmosphere.survey = false;
        super::sky::render_hdr(&mut gpu, &camera, atmosphere, 60., 128, 64);
        assert!(
            !gpu.atmosphere_updates.cloud_shadows,
            "survey must preserve the cloud cache"
        );
        atmosphere.aerosol += 0.5;
        super::sky::render_hdr(&mut gpu, &camera, atmosphere, 60., 128, 64);
        assert!(gpu.atmosphere_updates.sky_lookup && !gpu.atmosphere_updates.cloud_shadows);
    });
}
