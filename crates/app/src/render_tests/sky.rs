//! Actual HDR readback checks for the sky passes, profile changes and resize.
use genesis_renderer::{
    atmosphere::Atmosphere,
    camera::Camera,
    gpu::{Gpu, WorldTarget},
    tonemap::WORLD_FORMAT,
};

#[test]
#[ignore = "requires native GPU access"]
fn sky_profiles_resize_and_stay_finite_on_native_gpu() {
    pollster::block_on(async {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&Default::default())
            .await
            .expect("adapter");
        let mut gpu = Gpu::new(&adapter).await.expect("renderer");
        let mut camera = Camera::default();
        camera.enter_ground(glam::Vec3::ZERO, 0.085);
        camera.look_at_ground(camera.eye() + glam::Vec3::new(-5., 2.4, -7.));
        let mut profiles = Vec::new();
        for (name, width, height, eye) in [
            ("clear", 64_u32, 32_u32, 0.),
            ("fair", 64, 32, 0.),
            ("overcast", 64, 32, 0.),
            ("clear", 64, 32, 0.),
            ("fair", 65, 33, 100.),
            ("fair", 128, 64, 200.),
        ] {
            // Heights exercise inside/above the slab as well as ground viewing.
            camera.focus.y = eye;
            let multisample = gpu
                .multisample_texture(width, height, WORLD_FORMAT)
                .create_view(&Default::default());
            let depth = gpu
                .multisample_texture(width, height, wgpu::TextureFormat::Depth32Float)
                .create_view(&Default::default());
            let output = gpu.texture(
                width,
                height,
                WORLD_FORMAT,
                wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            );
            let resolve = output.create_view(&Default::default());
            let stride = (width * 8).div_ceil(256) * 256;
            let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("HDR sky readback"),
                size: u64::from(stride * height),
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            let mut encoder = gpu.encode(
                WorldTarget {
                    color: &multisample,
                    resolve: &resolve,
                    depth: &depth,
                    size: [width, height],
                },
                &camera,
                width as f32 / height as f32,
                1.,
                Atmosphere::preview(name).expect("profile"),
                false,
            );
            encoder.copy_texture_to_buffer(
                output.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &readback,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(stride),
                        rows_per_image: Some(height),
                    },
                },
                output.size(),
            );
            gpu.queue.submit([encoder.finish()]);
            let (tx, rx) = std::sync::mpsc::channel();
            readback.slice(..).map_async(wgpu::MapMode::Read, move |r| {
                let _ = tx.send(r);
            });
            gpu.device
                .poll(wgpu::PollType::Wait {
                    submission_index: None,
                    timeout: Some(std::time::Duration::from_secs(15)),
                })
                .expect("readback");
            rx.recv().expect("callback").expect("mapping");
            let mapped = readback.slice(..).get_mapped_range();
            let pixels: Vec<_> = mapped
                .chunks_exact(stride as usize)
                .flat_map(|row| {
                    row[..(width * 8) as usize]
                        .chunks_exact(2)
                        .map(|v| u16::from_le_bytes([v[0], v[1]]))
                })
                .collect();
            for pixel in pixels.chunks_exact(4) {
                // Inspect IEEE half bits directly: every channel finite/nonnegative,
                // and the world composition opaque even through clear atmosphere.
                assert!(
                    pixel[..3]
                        .iter()
                        .all(|v| v & 0x7c00 != 0x7c00 && v & 0x8000 == 0)
                );
                assert_eq!(pixel[3], 0x3c00);
            }
            assert!(pixels.chunks_exact(4).any(|v| v[0] > 0 && v[2] > 0));
            profiles.push(pixels);
            drop(mapped);
            readback.unmap();
        }
        assert_ne!(profiles[0], profiles[1]);
        assert_ne!(profiles[1], profiles[2]);
        assert_eq!(
            profiles[0], profiles[3],
            "switching weather must not leave stale cloud history"
        );
    });
}
