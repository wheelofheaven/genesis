//! Flat regional receiver isolates cloud-shadow sampling from scenery cost.
use genesis_renderer::{
    atmosphere::Atmosphere,
    camera::Camera,
    geometry::Vertex,
    gpu::{Gpu, WorldTarget},
    tonemap::WORLD_FORMAT,
};
use glam::{Vec2, Vec3};

#[test]
#[ignore = "requires native GPU access; manual cloud-shadow timing, no performance thresholds"]
fn cloud_shadow_receiver_timing() {
    pollster::block_on(async {
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
        let mut atmosphere = Atmosphere {
            density: 0.,
            ..Default::default()
        };
        atmosphere.clouds.wind_metres_per_second = Vec2::ZERO;
        for strength in [0., 0.28] {
            atmosphere.clouds.shadow_strength = strength;
            let mut samples = Vec::new();
            let mut completed = Vec::new();
            for frame in 0..24 {
                let start = std::time::Instant::now();
                let encoder = gpu.encode(
                    WorldTarget {
                        color: &color,
                        resolve: &resolve,
                        depth: &depth,
                        size: [width, height],
                    },
                    &camera,
                    width as f32 / height as f32,
                    0.,
                    atmosphere,
                    true,
                );
                gpu.queue.submit([encoder.finish()]);
                if let Some(timings) = gpu.capture_timings().expect("GPU timestamps")
                    && let Some(ms) = timings[2]
                    && frame >= 4
                {
                    samples.push(ms);
                }
                if frame >= 4 {
                    completed.push(start.elapsed().as_secs_f64() * 1000.);
                }
            }
            completed.sort_by(f64::total_cmp);
            println!(
                "cloud shade {strength}: isolated encode/submit/readback wall median {:.3} ms; includes all passes and synchronization, not gameplay FPS",
                completed[completed.len() / 2]
            );
            samples.sort_by(f64::total_cmp);
            if samples.is_empty() {
                println!("cloud shade {strength}: world GPU timestamps unavailable");
            } else {
                println!(
                    "cloud shade {strength}: 1920x1080 receiver, {} valid samples; median {:.3} ms, range {:.3}..{:.3} ms; world pass only, excludes cache generation and sky",
                    samples.len(),
                    samples[samples.len() / 2],
                    samples[0],
                    samples[samples.len() - 1]
                );
            }
        }
    });
}
