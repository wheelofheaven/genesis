//! Repeatable resident-settlement render audit. Explicit GPU waits isolate work;
//! these timings are diagnostics, not interactive frame pacing or simulation FPS.
use crate::{dwelling, landscape, scene, terrain};
use genesis_renderer::{
    atmosphere::Atmosphere,
    camera::Camera,
    gpu::{Gpu, WorldTarget},
    tonemap::ToneMap,
    visibility::Frustum,
};
use genesis_sim_core::{units::metres, worldgen};
use glam::Vec3;
use serde_json::{Value, json};
use std::{
    error::Error,
    path::Path,
    time::{Duration, Instant},
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
const SIZE: [u32; 2] = [1920, 1080];
const WARMUP: usize = 12;
const SAMPLES: usize = 60;

fn distribution(values: &[f64]) -> Value {
    if values.is_empty() {
        return Value::Null;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let percentile = |p: f64| {
        sorted[((sorted.len() as f64 * p).ceil() as usize)
            .saturating_sub(1)
            .min(sorted.len() - 1)]
    };
    json!({"samples":sorted.len(),"median":percentile(0.5),"p95":percentile(0.95),"p99":percentile(0.99),"max":sorted[sorted.len()-1]})
}

fn camera_at(center: Vec3, rotation: f32, t: f32) -> Camera {
    // Exponential descent preserves a useful near-house part of the route.
    let distance = 8.4_f32 * (100_f32 / 8.4).powf(1. - t);
    let local = Vec3::new(2. + (1. - t) * 28., 1.7 + (1. - t).powi(2) * 50., distance) / 20.;
    let (s, c) = rotation.sin_cos();
    let eye = center
        + Vec3::new(
            c * local.x - s * local.z,
            local.y,
            s * local.x + c * local.z,
        );
    let mut camera = Camera::default();
    camera.enter_ground(eye - Vec3::Y * metres(1.7), metres(1.7));
    camera.look_at_ground(center + Vec3::Y * metres(1.45));
    camera
}

pub async fn run(path: &Path) -> Result<()> {
    let start = Instant::now();
    let sim = worldgen::generate(worldgen::DEFAULT_SEED);
    let placements = dwelling::placements(&sim);
    let first = placements.first().ok_or("benchmark needs a dwelling")?;
    let center = Vec3::from_slice(&first.position_scale[..3]);
    let rotation = first.tint_rotation[3];
    let near = camera_at(center, rotation, 1.);
    let cache = terrain::Cache::at(&sim, terrain::lod::Key::at(near.eye()));
    let mesh = scene::mesh(&sim, Default::default(), &cache, &cache.rivers);
    let scenery = landscape::Scenery::new(&sim);
    let preparation_ms = start.elapsed().as_secs_f64() * 1000.;
    let instance = wgpu::Instance::default();
    let adapter = instance.request_adapter(&Default::default()).await?;
    let info = adapter.get_info();
    let mut gpu = Gpu::new(&adapter).await?;
    dwelling::load(&mut gpu, None)?;
    gpu.upload_scene(&mesh);
    gpu.contacts.upload(&gpu.queue, &cache.contacts);
    let foliage: Vec<_> = landscape::meshes()
        .iter()
        .map(|mesh| gpu.instances.add_scenery(&gpu.device, mesh))
        .collect();
    for (mesh, placements) in terrain::ground_cover::meshes()
        .iter()
        .zip(&cache.ground_cover)
    {
        let batch = gpu.instances.add_scenery(&gpu.device, mesh);
        gpu.instances
            .upload(&gpu.device, &gpu.queue, batch, placements);
    }
    let [width, height] = SIZE;
    let aspect = width as f32 / height as f32;
    let tone = ToneMap::new(&gpu, width, height, wgpu::TextureFormat::Rgba8UnormSrgb);
    let depth = gpu
        .multisample_texture(width, height, wgpu::TextureFormat::Depth32Float)
        .create_view(&Default::default());
    let output = gpu.texture(
        width,
        height,
        wgpu::TextureFormat::Rgba8UnormSrgb,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    );
    let output_view = output.create_view(&Default::default());
    let mut phases = Vec::new();
    for (name, fixed) in [
        ("settlement", Some(0.)),
        ("approach", Some(0.55)),
        ("eye", Some(1.)),
        ("descent", None),
    ] {
        eprintln!("Benchmark: {name}, {WARMUP} warm-up + {SAMPLES} samples at {width}x{height}");
        let mut cpu = Vec::new();
        let mut waited = Vec::new();
        let mut passes: [Vec<f64>; 4] = Default::default();
        let mut frames = Vec::new();
        for frame in 0..WARMUP + SAMPLES {
            let t = fixed.unwrap_or_else(|| {
                if frame < WARMUP {
                    0.
                } else {
                    (frame - WARMUP) as f32 / (SAMPLES - 1) as f32
                }
            });
            let camera = camera_at(center, rotation, t);
            let begin = Instant::now();
            let visible = scenery.visible(&camera, aspect);
            let mut upload = 0;
            for (i, &batch) in foliage.iter().enumerate() {
                gpu.instances
                    .upload(&gpu.device, &gpu.queue, batch, &visible.main[i]);
                upload +=
                    visible.main[i].len() * size_of::<genesis_renderer::instances::Instance>();
                for cascade in 0..2 {
                    gpu.instances.upload_shadows(
                        &gpu.device,
                        &gpu.queue,
                        batch,
                        cascade,
                        &visible.shadows[cascade][i],
                    );
                    upload += visible.shadows[cascade][i].len()
                        * size_of::<genesis_renderer::instances::Instance>();
                }
            }
            gpu.assets.prepare(
                &gpu.device,
                &gpu.queue,
                &placements,
                &camera,
                aspect,
                height,
            );
            upload += gpu.assets.stats.upload_bytes;
            let mut encoder = gpu.encode(
                WorldTarget {
                    color: &tone.multisample,
                    resolve: &tone.resolve,
                    depth: &depth,
                    size: SIZE,
                },
                &camera,
                aspect,
                frame as f64 / 60.,
                Atmosphere::default(),
                true,
            );
            tone.composite(&gpu, &mut encoder, &output_view, false);
            let commands = encoder.finish();
            let cpu_ms = begin.elapsed().as_secs_f64() * 1000.;
            gpu.queue.submit([commands]);
            gpu.device.poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(Duration::from_secs(30)),
            })?;
            let waited_ms = begin.elapsed().as_secs_f64() * 1000.;
            let timings = gpu.capture_timings()?.unwrap_or([None; 4]);
            if frame >= WARMUP {
                cpu.push(cpu_ms);
                waited.push(waited_ms);
                for (list, value) in passes.iter_mut().zip(timings) {
                    if let Some(value) = value {
                        list.push(value);
                    }
                }
                let world = gpu
                    .world
                    .stats(&Frustum::new(camera.view_projection(aspect)), false);
                let instances = gpu.instances.main_stats();
                let assets = gpu.assets.stats;
                frames.push(json!({"route_t":t,"cpu_prepare_encode_ms":cpu_ms,"submit_and_wait_ms":waited_ms,"gpu_pass_ms":timings,
                    "main_draws":world.chunks+instances.batches+assets.draws as usize,"main_triangles":world.vertices/3+instances.vertices/3+assets.triangles,
                    "dwelling_instances":assets.instances,"dwelling_triangles":assets.triangles,"placement_upload_bytes":upload,"dwelling_upload_bytes":assets.upload_bytes}));
            }
        }
        let capture = path.with_extension(format!("{name}.png"));
        gpu.capture(&output, &capture)?;
        phases.push(json!({"name":name,"capture":capture,"cpu_prepare_encode_ms":distribution(&cpu),"submit_and_wait_ms":distribution(&waited),"gpu_pass_ms":passes.each_ref().map(|v|distribution(v)),"frames":frames}));
    }
    let os = std::process::Command::new("uname")
        .arg("-sr")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned());
    let report = json!({"schema":1,"scene":"resident Levant Coast starter settlement","seed":worldgen::DEFAULT_SEED,"build":if cfg!(debug_assertions){"dev"}else{"release"},"resolution":SIZE,"msaa":4,"weather":"fair","gpu":info.name,"backend":format!("{:?}",info.backend),"os":os,"warmup_per_phase":WARMUP,"samples_per_phase":SAMPLES,
        "buildings":sim.buildings.len(),"active_animated_agents":0,"preparation_ms":preparation_ms,"initial_world_upload_bytes":gpu.world.last_upload.bytes,
        "asset_texture_payload_bytes":gpu.assets.texture_bytes,"peak_resident_memory_bytes":Value::Null,"gpu_pass_order":["near_shadow","wide_shadow","world","sky"],
        "limitations":["Serialized submit/wait diagnostic; not interactive FPS.","Terrain held at near-house detail; no streaming or simulation/UI work.","Scenery culling/uploads run each sample, including stationary views; app caches unchanged camera streams.","GPU counters cover four passes, not total GPU frame; null means unavailable.","Small starter settlement; does not certify dense-city performance."],"phases":phases});
    std::fs::write(path, serde_json::to_vec_pretty(&report)?)?;
    eprintln!(
        "Wrote benchmark report and four captures beside {}",
        path.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_samples_are_not_zero_and_percentiles_are_nearest_rank() {
        assert!(distribution(&[]).is_null());
        let d = distribution(&[10., 2., 4., 1., 3.]);
        assert_eq!(d["median"], 3.);
        assert_eq!(d["p95"], 10.);
    }
    #[test]
    fn route_finishes_at_human_eye_height_without_jumps() {
        let center = Vec3::new(8., 4., -7.);
        let end = camera_at(center, 0.3, 1.);
        assert!((end.eye().y - center.y - metres(1.7)).abs() < 1e-5);
        let mut previous = camera_at(center, 0.3, 0.).eye();
        for i in 1..=60 {
            let next = camera_at(center, 0.3, i as f32 / 60.).eye();
            assert!(next.distance(previous) < 0.3);
            previous = next;
        }
    }
}
