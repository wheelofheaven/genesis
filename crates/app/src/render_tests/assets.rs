//! Exercise actual imported geometry/materials, culling, reload and both shadow paths.
use genesis_renderer::{atmosphere::Atmosphere, camera::Camera, gpu::Gpu, instances::Instance};
use glam::Vec3;

#[test]
#[ignore = "requires native GPU access"]
fn imported_dwelling_renders_reloads_and_culls_on_native_gpu() {
    pollster::block_on(async {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&Default::default())
            .await
            .expect("adapter");
        let mut gpu = Gpu::new(&adapter).await.expect("renderer");
        crate::dwelling::load(&mut gpu, None).expect("dwelling");
        assert!(
            gpu.assets.texture_bytes < 6 * 1024 * 1024,
            "shared R8 atlas budget"
        );
        let placement = Instance {
            position_scale: [0., 0., 0., 1.],
            tint_rotation: [1., 1., 1., 0.],
            coverage: [0., 1.],
        };
        let mut camera = Camera::default();
        camera.enter_ground(Vec3::new(0.1, 0., 0.4), 0.085);
        camera.look_at_ground(Vec3::Y * 0.075);
        gpu.assets
            .prepare(&gpu.device, &gpu.queue, &[placement], &camera, 2., 128);
        assert_eq!(gpu.assets.stats.instances, 1);
        let near = gpu.assets.stats.triangles;
        assert!(gpu.assets.stats.upload_bytes > 0);
        gpu.assets
            .prepare(&gpu.device, &gpu.queue, &[placement], &camera, 2., 128);
        assert_eq!(
            gpu.assets.stats.upload_bytes, 0,
            "stationary stream should stay resident"
        );
        let clear = super::sky::render_hdr(
            &mut gpu,
            &camera,
            Atmosphere::preview("clear").unwrap(),
            0.,
            256,
            128,
        );
        let mut unoccluded = crate::dwelling::models(None).unwrap();
        for model in &mut unoccluded {
            for material in &mut model.materials {
                material.surface[3] = 0.;
            }
        }
        gpu.assets
            .replace_lods(&gpu.device, &gpu.queue, &unoccluded);
        gpu.assets
            .prepare(&gpu.device, &gpu.queue, &[placement], &camera, 2., 128);
        let open = super::sky::render_hdr(
            &mut gpu,
            &camera,
            Atmosphere::preview("clear").unwrap(),
            0.,
            256,
            128,
        );
        assert!(
            open.iter().zip(&clear).all(|(a, b)| a >= b),
            "AO must only reduce illumination"
        );
        assert!(
            open.iter().zip(&clear).any(|(a, b)| a > b),
            "baked occlusion must affect the visible dwelling"
        );
        crate::dwelling::load(&mut gpu, None).unwrap();
        let tinted = Instance {
            tint_rotation: [0.65, 0.65, 0.65, 0.],
            ..placement
        };
        gpu.assets
            .prepare(&gpu.device, &gpu.queue, &[tinted], &camera, 2., 128);
        let darker = super::sky::render_hdr(
            &mut gpu,
            &camera,
            Atmosphere::preview("clear").unwrap(),
            0.,
            256,
            128,
        );
        let pixel = (64 * 256 + 128) * 4;
        assert!(
            darker[pixel..pixel + 3]
                .iter()
                .zip(&clear[pixel..pixel + 3])
                .all(|(a, b)| a <= b)
        );
        assert!(
            darker[pixel..pixel + 3]
                .iter()
                .zip(&clear[pixel..pixel + 3])
                .any(|(a, b)| a < b),
            "instance tint must reach the material without recolouring the sky"
        );
        gpu.assets
            .prepare(&gpu.device, &gpu.queue, &[placement], &camera, 2., 128);
        let cloudy = super::sky::render_hdr(
            &mut gpu,
            &camera,
            Atmosphere::preview("overcast").unwrap(),
            0.,
            256,
            128,
        );
        assert_ne!(
            clear, cloudy,
            "weather should reach imported surfaces and sky"
        );
        let models = crate::dwelling::models(None).unwrap();
        gpu.assets.replace_lods(&gpu.device, &gpu.queue, &models);
        gpu.assets
            .prepare(&gpu.device, &gpu.queue, &[placement], &camera, 2., 128);
        assert_eq!(
            clear,
            super::sky::render_hdr(
                &mut gpu,
                &camera,
                Atmosphere::preview("clear").unwrap(),
                0.,
                256,
                128
            ),
            "reload must preserve material appearance"
        );
        gpu.assets
            .prepare(&gpu.device, &gpu.queue, &[], &camera, 2., 128);
        let empty = super::sky::render_hdr(
            &mut gpu,
            &camera,
            Atmosphere::preview("clear").unwrap(),
            0.,
            256,
            128,
        );
        assert_ne!(clear, empty, "asset must actually rasterize");
        // Camera-away culling leaves independent shadow casters available.
        camera.look_at_ground(camera.eye() + Vec3::Z);
        gpu.assets
            .prepare(&gpu.device, &gpu.queue, &[placement], &camera, 2., 128);
        assert_eq!(gpu.assets.stats.instances, 0);
        super::sky::render_hdr(&mut gpu, &camera, Atmosphere::default(), 0., 65, 33);
        camera.focus = Vec3::new(0., 2., 10.);
        camera.look_at_ground(Vec3::ZERO);
        gpu.assets
            .prepare(&gpu.device, &gpu.queue, &[placement], &camera, 2., 128);
        assert!(
            gpu.assets.stats.triangles < near,
            "distant geometry must fall with projected size"
        );
        assert!(gpu.assets.stats.triangles > 0);
    });
}
