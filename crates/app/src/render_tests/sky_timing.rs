//! Repeatable sky-only GPU diagnostic; does not measure gameplay frame rate.
use genesis_renderer::{
    atmosphere::Atmosphere,
    camera::Camera,
    gpu::{Gpu, WorldTarget},
    tonemap::WORLD_FORMAT,
};

#[test]
#[ignore = "requires native GPU access; manual sky timing, no performance thresholds"]
fn sky_render_timing() {
    pollster::block_on(async {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&Default::default())
            .await
            .expect("adapter");
        let mut gpu = Gpu::new(&adapter).await.expect("renderer");
        let (width, height) = (1920, 1080);
        let color = gpu
            .multisample_texture(width, height, WORLD_FORMAT)
            .create_view(&Default::default());
        let depth = gpu
            .multisample_texture(width, height, wgpu::TextureFormat::Depth32Float)
            .create_view(&Default::default());
        let resolve = gpu
            .texture(
                width,
                height,
                WORLD_FORMAT,
                wgpu::TextureUsages::RENDER_ATTACHMENT,
            )
            .create_view(&Default::default());
        let mut camera = Camera::default();
        camera.enter_ground(glam::Vec3::ZERO, 0.085);
        camera.look_at_ground(camera.eye() + glam::Vec3::new(-5., 2.4, -7.));
        for name in ["clear", "fair", "overcast"] {
            let atmosphere = Atmosphere::preview(name).expect("profile");
            let mut samples = Vec::new();
            for frame in 0..24 {
                let encoder = gpu.encode(
                    WorldTarget {
                        color: &color,
                        resolve: &resolve,
                        depth: &depth,
                        size: [width, height],
                    },
                    &camera,
                    width as f32 / height as f32,
                    1.,
                    atmosphere,
                    true,
                );
                gpu.queue.submit([encoder.finish()]);
                if let Some(timings) = gpu.capture_timings().expect("GPU timestamps")
                    && let Some(ms) = timings[3]
                    && frame >= 4
                {
                    samples.push(ms);
                }
            }
            samples.sort_by(f64::total_cmp);
            if samples.is_empty() {
                println!("{name}: sky GPU timestamps unavailable");
            } else {
                println!(
                    "{name}: 960x540 sky environment, {} valid samples; median {:.3} ms, range {:.3}..{:.3} ms; excludes sky lookup, cloud-shadow rebuild, composition and world",
                    samples.len(),
                    samples[samples.len() / 2],
                    samples[0],
                    samples[samples.len() - 1]
                );
            }
        }
    });
}
