//! Isolate sunlight reflections with black diffuse receivers in the production
//! instanced stone pipeline. Tests roughness, view response and cast shadows.
use genesis_renderer::{
    atmosphere::Atmosphere, camera::Camera, geometry::Vertex, gpu::Gpu, instances::Instance,
};
use glam::Vec3;

#[test]
#[ignore = "requires native GPU access"]
fn stone_reflections_follow_view_roughness_and_occlusion_on_native_gpu() {
    pollster::block_on(async {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&Default::default())
            .await
            .expect("adapter");
        let mut gpu = Gpu::new(&adapter).await.expect("renderer");
        let light = Vec3::new(-0.5, 0.75, 0.3).normalize();
        let reflected = Vec3::new(-light.x, light.y, -light.z);
        let mut atmosphere = Atmosphere::preview("clear").expect("weather");
        atmosphere.density = 0.;
        let placement = Instance {
            position_scale: [0., 0., 0., 1.],
            tint_rotation: [1., 1., 1., 0.],
            coverage: [0., 1.],
        };
        let mut peaks = Vec::new();
        for tag in [7., 8., 9.] {
            let vertices = quad(Vec3::ZERO, 1., [0., 0., 0., tag]);
            let mesh = gpu.instances.add_mesh(&gpu.device, &vertices);
            gpu.instances
                .upload(&gpu.device, &gpu.queue, mesh, &[placement]);
            let peak = sample(&mut gpu, reflected * 0.3, atmosphere);
            let away = sample(&mut gpu, light * 0.3, atmosphere);
            assert!(
                peak > away * 1.2,
                "tag {tag}: view-dependent lobe {peak} vs {away}"
            );
            assert!(peak > 0.001 && peak < 0.4, "bounded dry reflection: {peak}");
            peaks.push(peak);
            atmosphere.survey = true;
            assert_eq!(
                sample(&mut gpu, reflected * 0.3, atmosphere),
                0.,
                "survey bypass"
            );
            atmosphere.survey = false;
            // A small elevated caster lies along the receiver-to-sun ray, away
            // from the opposite camera ray. Reflections must not shine through it.
            gpu.upload_scene(&quad(light * (0.12 / light.y), 0.09, [0.3, 0.3, 0.3, 3.]));
            let shadow = sample(&mut gpu, reflected * 0.3, atmosphere);
            assert!(
                shadow < peak * 0.02,
                "sun reflection leaks through shadow: {shadow}/{peak}"
            );
            gpu.upload_scene(&[]);
            gpu.instances.upload(&gpu.device, &gpu.queue, mesh, &[]);
        }
        assert!(
            peaks[2] > peaks[0] && peaks[0] > peaks[1],
            "basalt/limestone/sandstone roughness: {peaks:?}"
        );
    });
}

fn quad(center: Vec3, radius: f32, color: [f32; 4]) -> [Vertex; 6] {
    let corners = [
        Vec3::new(-radius, 0., -radius),
        Vec3::new(-radius, 0., radius),
        Vec3::new(radius, 0., radius),
        Vec3::new(radius, 0., -radius),
    ];
    [0, 1, 2, 0, 2, 3].map(|i| Vertex {
        position: (center + corners[i]).to_array(),
        color,
    })
}

fn sample(gpu: &mut Gpu, eye: Vec3, atmosphere: Atmosphere) -> f32 {
    let mut camera = Camera::default();
    camera.enter_ground(eye - Vec3::Y * 0.085, 0.085);
    camera.look_at_ground(Vec3::ZERO);
    let pixels = super::sky::render_hdr(gpu, &camera, atmosphere, 0., 33, 17);
    let v = pixels[(8 * 33 + 16) * 4];
    let exponent = i32::from((v >> 10) & 31);
    let mantissa = f32::from(v & 1023);
    if exponent == 0 {
        mantissa * 2_f32.powi(-24)
    } else {
        (1. + mantissa / 1024.) * 2_f32.powi(exponent - 15)
    }
}
