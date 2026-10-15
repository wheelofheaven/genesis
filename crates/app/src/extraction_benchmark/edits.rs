//! Fixed committed edits, excluding windowing, GPU submission and display latency.
use super::{SAMPLES, WARMUP, distribution, fingerprint, measure};
use crate::{scene, session::Session, survey::Survey, terrain};
use genesis_renderer::{camera::Camera, world_mesh::PreparedWorld};
use genesis_sim_core::{actions::Action, events::SimEvent, worldgen};
use glam::Vec3;
use serde_json::json;
use std::{collections::BTreeMap, path::Path, time::Instant};

pub fn run(path: &Path, full_rebuild: bool) -> Result<(), Box<dyn std::error::Error>> {
    let mut initial = worldgen::generate(worldgen::DEFAULT_SEED);
    // Preserve the original unpopulated rendering benchmark workload.
    initial.people = Default::default();
    let site = initial
        .buildings
        .first()
        .ok_or("scenario needs a dwelling")?
        .position;
    let action = Action::Terraform {
        x: site.x.floor() as i32,
        y: site.z.floor() as i32,
        radius: 2,
        strength: 0.4,
    };
    let mut camera = Camera::default();
    terrain::home_camera(&initial, &mut camera);
    let center = terrain::point(&initial, site);
    let mut control = Session::from_sim(initial.clone());
    control.clock.paused = true;
    control.step_once();
    let mut views = Vec::new();
    for (name, eye) in [
        ("region", camera.eye()),
        ("management", center + Vec3::new(4., 5., 6.)),
        ("inspection", center + Vec3::new(0.4, 0.085, 0.4)),
    ] {
        let key = terrain::lod::Key::at(eye);
        // Establish a fully resident baseline, then rebuild at the same tick
        // without an edit. Rebuilding both sides of the control comparison also
        // removes unrelated differences from cached horizon/environment colors.
        let old_cache = terrain::Cache::at(&initial, key);
        let old_vertices = scene::mesh(&initial, Survey::default(), &old_cache, &old_cache.rivers);
        let previous = PreparedWorld::new(&old_vertices);
        let old_hash = fingerprint(&old_vertices);
        drop(old_vertices);
        let old_picker = terrain::Picker::from_cache(&initial, &old_cache);
        let control_cache = terrain::Cache::at(&control.sim, key);
        let control_vertices = scene::mesh(
            &control.sim,
            Survey::default(),
            &control_cache,
            &control_cache.rivers,
        );
        let control_hash = fingerprint(&control_vertices);
        let control_world = PreparedWorld::new(&control_vertices);
        drop(control_vertices);
        drop(control_cache);
        let mut timings: BTreeMap<&str, Vec<f64>> = BTreeMap::new();
        let mut expected = None;
        let mut geometry = json!(null);
        for i in 0..WARMUP + SAMPLES {
            // Fresh state each trial: preparation timings do not include setup
            // cloning and no cumulative edits can alter the measured workload.
            let mut session = Session::from_sim(initial.clone());
            session.clock.paused = true;
            session.queue(action.clone());
            let total_start = Instant::now();
            let (_, command_ms) = measure(|| session.step_once());
            let mut cache_stages = BTreeMap::new();
            let ((cache, affected), cache_ms) = measure(|| {
                let mut start = Instant::now();
                if !full_rebuild {
                    return old_cache.refresh_profiled(
                        &session.sim,
                        key,
                        session
                            .terrain_changes_since(0)
                            .expect("committed edit bounds"),
                        |stage| {
                            cache_stages.insert(stage, start.elapsed().as_secs_f64() * 1000.);
                            start = Instant::now();
                        },
                    );
                }
                let cache = terrain::Cache::at_profiled(&session.sim, key, |stage| {
                    cache_stages.insert(stage, start.elapsed().as_secs_f64() * 1000.);
                    start = Instant::now();
                });
                (cache, None)
            });
            let (picker, picker_ms) = measure(|| match affected {
                Some(affected) => old_picker.refresh(&session.sim, &cache, affected),
                None => terrain::Picker::from_cache(&session.sim, &cache),
            });
            let (vertices, scene_ms) =
                measure(|| scene::mesh(&session.sim, Survey::default(), &cache, &cache.rivers));
            let (prepared, partition_ms) = measure(|| PreparedWorld::new(&vertices));
            let total_ms = total_start.elapsed().as_secs_f64() * 1000.;

            // Validate outside timings; a fast no-op must never pass as an edit.
            let changed = session
                .sim
                .grid
                .cells()
                .iter()
                .zip(initial.grid.cells())
                .filter(|(a, b)| a.elevation.to_bits() != b.elevation.to_bits())
                .count();
            let reported: usize = session
                .events
                .iter()
                .filter_map(|event| match event {
                    SimEvent::TerrainChanged { changed_cells, .. } => Some(*changed_cells),
                    _ => None,
                })
                .sum();
            if changed != 9
                || reported != changed
                || session.terrain_revision != 1
                || session.sim.completed_ticks() != 1
                || session.pending_count() != 0
            {
                return Err("edit fixture did not commit exactly nine samples at tick one".into());
            }
            let p = terrain::point(&session.sim, site);
            let hit = picker
                .pick(p + Vec3::Y * 100., -Vec3::Y, false)
                .ok_or("edited terrain picker missed the fixture site")?;
            if (hit.x - site.x).abs() > 0.001 || (hit.z - site.z).abs() > 0.001 {
                return Err("edited terrain picking changed world coordinates".into());
            }
            let hash = fingerprint(&vertices);
            if i == 0 && !full_rebuild {
                // One full reference per view outside timings; subsequent trials
                // must reproduce the same complete vertex fingerprint.
                let reference = terrain::Cache::at(&session.sim, key);
                let reference_vertices = scene::mesh(
                    &session.sim,
                    Survey::default(),
                    &reference,
                    &reference.rivers,
                );
                if fingerprint(&reference_vertices) != hash {
                    return Err(format!("local refresh differs from full rebuild in {name}").into());
                }
                let reference_picker = terrain::Picker::from_cache(&session.sim, &reference);
                for j in 0..32 {
                    let origin = p + Vec3::new(j as f32 * 0.13 - 2., 8., 2.);
                    let dir = Vec3::new(0.1, -1., -0.2).normalize();
                    for water in [false, true] {
                        if picker.pick(origin, dir, water)
                            != reference_picker.pick(origin, dir, water)
                        {
                            return Err(format!(
                                "local picker differs from full rebuild in {name}"
                            )
                            .into());
                        }
                    }
                }
            }
            if hash == old_hash
                || hash == control_hash
                || expected.as_ref().is_some_and(|previous| *previous != hash)
            {
                return Err("edit presentation is unchanged or not repeatable".into());
            }
            expected = Some(hash.clone());
            let from_previous = prepared.upload_estimate(&previous);
            let from_control = prepared.upload_estimate(&control_world);
            geometry = json!({
                "appearance":cache.appearance_metrics(), "local_refresh": affected.is_some(), "retained_lod_bytes": cache.lod_storage_bytes(), "changed_cells": changed, "vertices": vertices.len(), "vertex_fnv1a": hash,
                "prepared_chunks": from_previous.chunks,
                "estimated_upload_from_previous": {"chunks": from_previous.changed, "bytes": from_previous.bytes},
                "estimated_upload_from_same_tick_rebuilt_control": {"chunks": from_control.changed, "bytes": from_control.bytes},
            });
            if i >= WARMUP {
                for (stage, elapsed) in [
                    ("command_tick", command_ms),
                    ("cache", cache_ms),
                    ("picker", picker_ms),
                    ("scene", scene_ms),
                    ("partition", partition_ms),
                    ("total", total_ms),
                ] {
                    timings.entry(stage).or_default().push(elapsed);
                }
                for (stage, elapsed) in cache_stages {
                    timings.entry(stage).or_default().push(elapsed);
                }
            }
        }
        let timings: BTreeMap<_, _> = timings
            .iter()
            .map(|(name, samples)| (*name, distribution(samples)))
            .collect();
        views.push(
            json!({"view": name, "eye": eye.to_array(), "lod_key": key.0,
            "timings_ms": timings, "geometry": geometry}),
        );
        eprintln!("Measured {name} committed terrain edit");
    }
    let report = json!({
        "schema": 2, "full_rebuild": full_rebuild, "workload": "committed terrain edit CPU preparation",
        "seed": worldgen::DEFAULT_SEED, "region": "LevantCoast",
        "grid": [initial.grid.width, initial.grid.height],
        "generator_version": initial.geography.version,
        "content": genesis_sim_core::save::CONTENT,
        "build_debug_assertions": cfg!(debug_assertions),
        "os": std::env::consts::OS, "arch": std::env::consts::ARCH,
        "warmup": WARMUP, "samples": SAMPLES, "survey": "surface",
        "command_tick": 1, "action": action,
        "buildings": initial.buildings.len(), "paths": initial.paths.len(), "people": 0,
        "excludes": ["setup", "validation", "fingerprints outside partition", "GPU uploads",
            "instance and contact publication", "window/UI", "worker scheduling", "display latency"],
        "views": views,
    });
    std::fs::write(path, serde_json::to_vec_pretty(&report)?)?;
    Ok(())
}
