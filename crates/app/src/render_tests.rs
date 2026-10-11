//! Opt-in native GPU regression and representative CPU preparation audit.
mod cloud_shadows;
mod sky;
use genesis_renderer::{
    geometry::Vertex,
    world_mesh::{WorldMesh, partition},
};

#[test]
#[ignore = "requires native GPU access"]
fn resident_world_buffers_reuse_update_and_remove() {
    pollster::block_on(async {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&Default::default())
            .await
            .expect("native adapter");
        // Also compile all world/atmosphere/material pipelines on the native backend.
        let gpu = genesis_renderer::gpu::Gpu::new(&adapter)
            .await
            .expect("native renderer");
        let (device, queue) = (&gpu.device, &gpu.queue);
        let mut mesh = WorldMesh::default();
        let mut vertices: Vec<_> = [
            [0., 0., 0.],
            [1., 0., 0.],
            [0., 0., 1.],
            [32., 0., 0.],
            [33., 0., 0.],
            [32., 0., 1.],
        ]
        .map(|position| Vertex {
            position,
            color: [0.3, 0.4, 0.2, 2.],
        })
        .to_vec();
        let copied = vertices.clone();
        let prepared =
            std::thread::spawn(move || genesis_renderer::world_mesh::PreparedWorld::new(&copied))
                .join()
                .expect("background scene preparation");
        mesh.upload_prepared(device, queue, &prepared);
        assert_eq!(mesh.last_upload.chunks, 2);
        assert_eq!(mesh.last_upload.changed, 2);
        mesh.upload_prepared(device, queue, &prepared);
        assert_eq!(mesh.last_upload.bytes, 0);
        vertices[0].position[1] = 0.5;
        mesh.upload(device, queue, &vertices);
        assert_eq!(mesh.last_upload.changed, 1);
        assert_eq!(mesh.last_upload.bytes, 3 * size_of::<Vertex>());
        // A removed chunk must not survive a region replacement.
        mesh.upload(device, queue, &vertices[..3]);
        assert_eq!(mesh.last_upload.chunks, 1);
        assert_eq!(mesh.last_upload.changed, 0);
        mesh.upload(device, queue, &[]);
        assert_eq!(mesh.last_upload.chunks, 0);
        let all = genesis_renderer::visibility::Frustum::new(glam::Mat4::orthographic_rh(
            -100., 100., -100., 100., -100., 100.,
        ));
        assert_eq!(mesh.stats(&all, false).vertices, 0);
        queue.submit([]);
        device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(10)),
            })
            .expect("GPU upload completion");
    });
}

#[test]
#[ignore = "manual terrain batching audit; no performance thresholds"]
fn world_chunk_timing() {
    use crate::{scene, survey::Survey, terrain};
    use genesis_renderer::{camera::Camera, shadows::Shadows, visibility::Frustum};
    use genesis_sim_core::{units::metres, worldgen};
    use std::time::Instant;
    let sim = worldgen::generate(worldgen::DEFAULT_SEED);
    let mut camera = Camera::default();
    camera.frame_region(192.);
    camera.enter_ground(terrain::point(&sim, sim.buildings[0].position), metres(1.7));
    let start = Instant::now();
    let cache = terrain::Cache::at(&sim, terrain::lod::Key::at(camera.eye()));
    println!(
        "terrain cache: {:.2} ms",
        start.elapsed().as_secs_f64() * 1000.
    );
    let start = Instant::now();
    let _picker = terrain::Picker::from_cache(&sim, &cache);
    println!("picker: {:.2} ms", start.elapsed().as_secs_f64() * 1000.);
    let start = Instant::now();
    let vertices = scene::mesh(&sim, Survey::default(), &cache, &cache.rivers);
    println!(
        "scene colours/geometry: {:.2} ms",
        start.elapsed().as_secs_f64() * 1000.
    );
    let start = Instant::now();
    let chunks = partition(&vertices);
    println!(
        "partition: {:.2} ms; {} chunks, {} vertices, {} bytes",
        start.elapsed().as_secs_f64() * 1000.,
        chunks.len(),
        vertices.len(),
        vertices.len() * size_of::<Vertex>()
    );
    for (name, volume, shadow) in [
        ("camera", Frustum::new(camera.view_projection(1.6)), false),
        ("sun", Frustum::new(Shadows::matrix(camera.focus)), true),
    ] {
        let count: usize = chunks
            .values()
            .map(|c| c.visible_vertices(&volume, shadow) as usize)
            .sum();
        println!(
            "{name}: {count} vertices ({:.1}% of original submission)",
            count as f64 * 100. / vertices.len() as f64
        );
    }
    camera.focus.x += metres(20.);
    let next = terrain::Cache::at(&sim, terrain::lod::Key::at(camera.eye()));
    let moved = scene::mesh(&sim, Survey::default(), &next, &next.rivers);
    let next_chunks = partition(&moved);
    let changed: Vec<_> = next_chunks
        .iter()
        .filter(|(key, c)| {
            chunks
                .get(key)
                .is_none_or(|old| old.fingerprint() != c.fingerprint())
        })
        .collect();
    let upload: usize = changed
        .iter()
        .map(|(_, c)| c.vertices.len() * size_of::<Vertex>())
        .sum();
    println!(
        "20 m camera move: {}/{} chunks changed, {upload}/{} bytes ({:.1}%)",
        changed.len(),
        next_chunks.len(),
        moved.len() * size_of::<Vertex>(),
        upload as f64 * 100. / (moved.len() * size_of::<Vertex>()) as f64
    );
}

#[test]
#[ignore = "requires native GPU access; manual POV refresh timing"]
fn pov_refresh_upload_timing() {
    use genesis_renderer::{camera::Camera, world_mesh::PreparedWorld};
    use std::time::Instant;
    let mut sim = genesis_sim_core::worldgen::generate(genesis_sim_core::worldgen::DEFAULT_SEED);
    let mut camera = Camera::default();
    crate::ground::toggle(&sim, sim.buildings[0].position, &mut camera);
    let cache = crate::terrain::Cache::at(&sim, crate::terrain::lod::Key::at(camera.eye()));
    pollster::block_on(async {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&Default::default())
            .await
            .expect("native adapter");
        let gpu = genesis_renderer::gpu::Gpu::new(&adapter)
            .await
            .expect("native renderer");
        let mut world = WorldMesh::default();
        for refresh in 0..2 {
            let start = Instant::now();
            let vertices = crate::scene::mesh(&sim, Default::default(), &cache, &cache.rivers);
            let prepared = PreparedWorld::new(&vertices);
            let preparation = start.elapsed();
            let start = Instant::now();
            world.upload_prepared(&gpu.device, &gpu.queue, &prepared);
            eprintln!(
                "POV refresh {refresh}: {:.2} ms CPU preparation; {:.2} ms foreground upload; {:?}",
                preparation.as_secs_f64() * 1000.,
                start.elapsed().as_secs_f64() * 1000.,
                world.last_upload
            );
            gpu.queue.submit([]);
            gpu.device
                .poll(wgpu::PollType::Wait {
                    submission_index: None,
                    timeout: Some(std::time::Duration::from_secs(10)),
                })
                .expect("upload completion");
            for _ in 0..10 {
                sim.step(0.1, &[]);
            }
        }
    });
}

#[test]
#[ignore = "requires native GPU access"]
fn hdr_display_preserves_highlights_survey_colours_and_resize() {
    use genesis_renderer::{gpu::Gpu, tonemap::ToneMap};
    pollster::block_on(async {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&Default::default())
            .await
            .expect("adapter");
        let gpu = Gpu::new(&adapter).await.expect("renderer");
        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let mut tone = ToneMap::new(&gpu, 8, 1, format);
        // Exactly representable half floats: 0, 1/8, 1/2, 1, 2, 4, 16, -1.
        // Values above 1 must survive HDR storage and remain distinguishable.
        let swatches: Vec<[u16; 4]> = [0, 0x3000, 0x3800, 0x3c00, 0x4000, 0x4400, 0x4c00, 0xbc00]
            .map(|v| [v, v, v, 0x3c00])
            .to_vec();
        for (height, survey) in [(1, false), (1, true), (2, false)] {
            if height == 2 {
                tone.resize(&gpu, 8, height);
            }
            let values = swatches.repeat(height as usize);
            let bytes: Vec<_> = values
                .iter()
                .flatten()
                .flat_map(|v| v.to_le_bytes())
                .collect();
            gpu.queue.write_texture(
                tone.scene.as_image_copy(),
                &bytes,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(64),
                    rows_per_image: Some(height),
                },
                tone.scene.size(),
            );
            let output = gpu.texture(
                8,
                height,
                format,
                wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            );
            let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("tone mapping regression readback"),
                size: 256 * height as u64,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            let mut encoder = gpu.device.create_command_encoder(&Default::default());
            tone.composite(
                &gpu,
                &mut encoder,
                &output.create_view(&Default::default()),
                survey,
            );
            encoder.copy_texture_to_buffer(
                output.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &readback,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(256),
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
                    timeout: Some(std::time::Duration::from_secs(10)),
                })
                .expect("readback");
            rx.recv().expect("callback").expect("mapped");
            let data = readback.slice(..).get_mapped_range();
            for row in data.chunks_exact(256) {
                let pixels: Vec<_> = row[..32].chunks_exact(4).collect();
                assert_eq!(pixels[0], [0, 0, 0, 255]);
                assert_eq!(pixels[7], [0, 0, 0, 255]);
                for p in &pixels {
                    assert_eq!(p[0], p[1]);
                    assert_eq!(p[1], p[2]);
                    assert_eq!(p[3], 255);
                }
                if survey {
                    // One sRGB encoding, with no tone curve or exposure adjustment.
                    for (i, expected) in [(1, 99_i16), (2, 188), (3, 255)] {
                        assert!((pixels[i][0] as i16 - expected).abs() <= 1);
                    }
                } else {
                    assert!((pixels[2][0] as i16 - 181).abs() <= 1);
                    for i in 1..7 {
                        assert!(
                            pixels[i][0] > pixels[i - 1][0],
                            "HDR levels collapsed: {pixels:?}"
                        );
                    }
                }
            }
            drop(data);
            readback.unmap();
        }
    });
}
