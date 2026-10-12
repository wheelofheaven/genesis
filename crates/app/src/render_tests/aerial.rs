//! Recover transmission from black/white receivers through the actual world
//! pipeline, and compare it with independent numerical integration in metres.
use genesis_renderer::{atmosphere::Atmosphere, camera::Camera, geometry::Vertex, gpu::Gpu};
use glam::Vec3;

#[test]
#[ignore = "requires native GPU access"]
fn aerial_depth_height_and_survey_are_stable_on_native_gpu() {
    pollster::block_on(async {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&Default::default())
            .await
            .expect("adapter");
        let mut gpu = Gpu::new(&adapter).await.expect("renderer");
        let atmosphere = Atmosphere::preview("clear").expect("profile");
        for (distance, start, end) in [
            (20., 100., 100.),
            (25.01, 100., 100.),
            (100., 100., 100.),
            (1000., 100., 100.),
            (5000., 100., 100.),
            (5000., 3100., 3100.),
            (1000., 100., 100.01),
            (2000., -200., 800.),
            (2000., 800., -200.),
            (1000., -300., -300.),
        ] {
            let (clear, hazy) = contrast(&mut gpu, atmosphere, distance, start, end);
            let measured = hazy / clear;
            // Midpoint quadrature of the clamped exponential density along the
            // actual finite ray. Independent of the shader's closed-form columns.
            let steps = 8192;
            let length = (distance - 25_f32).max(0.);
            let mut depth = Vec3::ZERO;
            for i in 0..steps {
                let t = (25. + (i as f32 + 0.5) * length / steps as f32) / distance;
                let height = (start + (end - start) * t).max(0.);
                depth += atmosphere.rayleigh / atmosphere.molecular_scale_height_metres
                    * (-height / atmosphere.molecular_scale_height_metres).exp()
                    + Vec3::splat(
                        0.015 * atmosphere.aerosol / atmosphere.aerosol_scale_height_metres
                            * (-height / atmosphere.aerosol_scale_height_metres).exp()
                            + atmosphere.density * (-height / atmosphere.scale_height_metres).exp(),
                    );
            }
            let expected = (-depth * (length / steps as f32)).exp();
            assert!(
                (measured - expected).abs().max_element() < 0.006,
                "distance {distance}, heights {start}/{end}: {measured:?} vs {expected:?}"
            );
            if distance == 5000. && start == 100. {
                assert!(
                    measured.z < measured.x - 0.05,
                    "wavelength-dependent extinction"
                );
            }
        }
        let mut survey = atmosphere;
        survey.survey = true;
        let (clear, hazy) = contrast(&mut gpu, survey, 5000., 100., 100.);
        assert_eq!(
            clear, hazy,
            "scientific colours bypass atmospheric scattering"
        );
    });
}

fn contrast(
    gpu: &mut Gpu,
    atmosphere: Atmosphere,
    distance: f32,
    start: f32,
    end: f32,
) -> (Vec3, Vec3) {
    let mut camera = Camera::default();
    camera.enter_ground(Vec3::new(0., start / 20. - 0.085, 0.), 0.085);
    let center = Vec3::new(
        0.,
        end / 20.,
        -(distance * distance - (end - start).powi(2)).sqrt() / 20.,
    );
    camera.look_at_ground(center);
    let forward = (center - camera.eye()).normalize();
    let up = Vec3::X.cross(forward);
    let radius = distance / 20.;
    let corners = [
        center - Vec3::X * radius - up * radius,
        center + Vec3::X * radius - up * radius,
        center + Vec3::X * radius + up * radius,
        center - Vec3::X * radius + up * radius,
    ];
    let mut samples = Vec::new();
    for density in [0., atmosphere.density] {
        for value in [0., 0.6] {
            gpu.upload_scene(&[0, 1, 2, 0, 2, 3].map(|i| Vertex {
                position: corners[i].to_array(),
                color: [value, value, value, 3.],
            }));
            let pixels = super::sky::render_hdr(
                gpu,
                &camera,
                Atmosphere {
                    density,
                    ..atmosphere
                },
                0.,
                33,
                17,
            );
            let index = (8 * 33 + 16) * 4;
            let rgb = std::array::from_fn(|i| half(pixels[index + i]));
            samples.push(Vec3::from_array(rgb));
        }
    }
    (samples[1] - samples[0], samples[3] - samples[2])
}

fn half(v: u16) -> f32 {
    let exponent = i32::from((v >> 10) & 31);
    let mantissa = f32::from(v & 1023);
    if exponent == 0 {
        mantissa * 2_f32.powi(-24)
    } else {
        (1. + mantissa / 1024.) * 2_f32.powi(exponent - 15)
    }
}
