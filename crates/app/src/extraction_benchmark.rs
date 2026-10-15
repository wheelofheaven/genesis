//! CPU preparation diagnostic. Uses the worker's actual stages without a GPU,
//! worker scheduling, uploads or simulation advancement in the timed workload.
pub(crate) mod edits;
use crate::{scene, survey::Survey, terrain};
use genesis_renderer::{camera::Camera, geometry::Vertex, world_mesh::PreparedWorld};
use genesis_sim_core::worldgen;
use glam::Vec3;
use serde_json::{Value, json};
use std::{collections::BTreeMap, hint::black_box, path::Path, time::Instant};

const WARMUP: usize = 2;
const SAMPLES: usize = 20;

fn measure<T>(run: impl FnOnce() -> T) -> (T, f64) {
    let start = Instant::now();
    let result = black_box(run());
    (result, start.elapsed().as_secs_f64() * 1000.)
}

fn distribution(values: &[f64]) -> Value {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    json!({"median": sorted[(sorted.len() - 1) / 2],
        "p95": sorted[(sorted.len() as f64 * 0.95).ceil() as usize - 1],
        "max": sorted[sorted.len() - 1], "samples_ms": values})
}

// FNV-1a over explicit little-endian float bits. Presentation comparison only;
// this is neither a cryptographic hash nor a canonical domain fingerprint.
fn fingerprint(vertices: &[Vertex]) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for vertex in vertices {
        for value in vertex.position.iter().chain(&vertex.color) {
            for byte in value.to_bits().to_le_bytes() {
                hash = (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3);
            }
        }
    }
    format!("{hash:016x}")
}

pub fn run(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut sim = worldgen::generate(worldgen::DEFAULT_SEED);
    // Preserve the original unpopulated rendering benchmark workload.
    sim.people = Default::default();
    sim.food = Default::default();
    let mut camera = Camera::default();
    terrain::home_camera(&sim, &mut camera);
    let building = sim.buildings.first().ok_or("scenario needs a dwelling")?;
    let near = terrain::point(&sim, building.position) + Vec3::new(0.4, 0.085, 0.4);
    let mut reports = Vec::new();
    for (view, eye) in [("region", camera.eye()), ("settlement", near)] {
        let key = terrain::lod::Key::at(eye);
        let mut snapshots = Vec::new();
        let mut caches = Vec::new();
        let mut pickers = Vec::new();
        let mut stages: BTreeMap<&str, Vec<f64>> = BTreeMap::new();
        for i in 0..WARMUP + SAMPLES {
            let (snapshot, snapshot_ms) = measure(|| sim.clone());
            let (cache, cache_ms) = measure(|| {
                let mut start = Instant::now();
                terrain::Cache::at_profiled(&snapshot, key, |stage| {
                    let elapsed = start.elapsed().as_secs_f64() * 1000.;
                    if i >= WARMUP {
                        stages.entry(stage).or_default().push(elapsed);
                    }
                    start = Instant::now();
                })
            });
            let (_picker, picker_ms) = measure(|| terrain::Picker::from_cache(&snapshot, &cache));
            if i >= WARMUP {
                snapshots.push(snapshot_ms);
                caches.push(cache_ms);
                pickers.push(picker_ms);
            }
        }
        let cache = terrain::Cache::at(&sim, key);
        let mut layers = Vec::new();
        for (name, survey) in [
            ("surface", Survey::default()),
            (
                "water",
                Survey {
                    water: true,
                    ..Survey::default()
                },
            ),
        ] {
            let mut extraction = Vec::new();
            let mut partition = Vec::new();
            let mut expected = None;
            let mut count = 0;
            for i in 0..WARMUP + SAMPLES {
                let (vertices, extraction_ms) =
                    measure(|| scene::mesh(&sim, survey, &cache, &cache.rivers));
                let (_prepared, partition_ms) = measure(|| PreparedWorld::new(&vertices));
                let hash = fingerprint(&vertices);
                if expected.as_ref().is_some_and(|previous| *previous != hash) {
                    return Err("identical extraction inputs produced different vertices".into());
                }
                expected = Some(hash);
                count = vertices.len();
                if i >= WARMUP {
                    extraction.push(extraction_ms);
                    partition.push(partition_ms);
                }
            }
            layers.push(json!({"survey": name, "vertices": count,
                "vertex_bytes": count * std::mem::size_of::<Vertex>(),
                "vertex_fnv1a": expected, "scene_ms": distribution(&extraction),
                "partition_ms": distribution(&partition)}));
        }
        let stages: BTreeMap<_, _> = stages
            .iter()
            .map(|(name, samples)| (*name, distribution(samples)))
            .collect();
        reports.push(
            json!({"view": view, "eye": eye.to_array(), "lod_key": key.0,
            "snapshot_ms": distribution(&snapshots), "cache_ms": distribution(&caches),
            "picker_ms": distribution(&pickers), "cache_stages_ms": stages, "layers": layers}),
        );
        eprintln!("Measured {view} CPU preparation");
    }
    let report = json!({"schema": 1, "seed": worldgen::DEFAULT_SEED,
        "region": "LevantCoast", "grid": [sim.grid.width, sim.grid.height],
        "generator_version": sim.geography.version, "content": genesis_sim_core::save::CONTENT,
        "build_debug_assertions": cfg!(debug_assertions), "arch": std::env::consts::ARCH,
        "os": std::env::consts::OS, "warmup": WARMUP, "samples": SAMPLES,
        "buildings": sim.buildings.len(), "paths": sim.paths.len(), "wells": sim.subsurface.wells.len(),
        "logical_ticks": 0, "commands": [], "views": reports});
    std::fs::write(path, serde_json::to_vec_pretty(&report)?)?;
    Ok(())
}
