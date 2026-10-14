use super::*;
use genesis_sim_core::{
    actions::Action, config::SimConfig, events::SimEvent, surface::channels, worldgen::River,
};

fn fixture(boundary: Boundary) -> SimState {
    let mut sim = SimState::new(SimConfig {
        width: 64,
        height: 48,
        ..Default::default()
    });
    sim.grid.boundary = boundary;
    sim.geography.version = 9;
    for z in 0..48 {
        for x in 0..64 {
            let c = sim.grid.get_mut(x, z);
            c.elevation = 2. + x as f32 * 0.003 + z as f32 * 0.005;
            c.plants = 0.5;
        }
    }
    sim.geography.rivers = vec![
        River {
            points: vec![
                Position::new(4., 8.),
                Position::new(24., 8.),
                Position::new(40., 8.),
            ],
            width: 0.4,
            widths: vec![],
            bounds: None,
        },
        River {
            points: vec![Position::new(24., 4.), Position::new(24., 8.)],
            width: 0.2,
            widths: vec![],
            bounds: None,
        },
    ];
    sim.geography.channels = channels::Index::new(64, 48, &sim.geography.rivers);
    sim
}

fn edit(sim: &mut SimState, x: i32, z: i32, strength: f32) -> SampleBounds {
    sim.step(
        0.1,
        &[Action::Terraform {
            x,
            y: z,
            radius: 2,
            strength,
        }],
    )
    .into_iter()
    .find_map(|e| match e {
        SimEvent::TerrainChanged { bounds, .. } => Some(bounds),
        _ => None,
    })
    .unwrap()
}

fn vertices(a: &[Vertex], b: &[Vertex]) {
    assert_eq!(a.len(), b.len());
    for (i, (a, b)) in a.iter().zip(b).enumerate() {
        assert_eq!(
            a.position.map(f32::to_bits),
            b.position.map(f32::to_bits),
            "position {i}"
        );
        assert_eq!(
            a.color.map(f32::to_bits),
            b.color.map(f32::to_bits),
            "color {i}"
        );
    }
}

fn equivalent(sim: &SimState, cache: &Cache, picker: &Picker) {
    let full = Cache::at(sim, cache.key);
    // Vertex storage can be compacted differently; every canonical sample and
    // ordered receiver triangle must still be exactly the full-build result.
    let samples = |c: &Cache| {
        c.samples
            .iter()
            .map(|&s| ((s.0.x.to_bits(), s.0.z.to_bits()), s))
            .collect::<std::collections::BTreeMap<_, _>>()
    };
    assert_eq!(cache.samples.len(), full.samples.len(), "no orphan samples");
    assert_eq!(samples(cache), samples(&full));
    assert_eq!(cache.indices.len(), full.indices.len());
    for (a, b) in cache.indices.iter().zip(full.indices.iter()) {
        assert_eq!(a.map(|i| cache.samples[i]), b.map(|i| full.samples[i]));
    }
    assert_eq!(cache.patches, full.patches);
    vertices(&cache.rivers, &full.rivers);
    vertices(&cache.rivers, &crate::water::reference_rivers(sim, cache));
    for survey in [
        crate::survey::Survey::default(),
        crate::survey::Survey {
            water: true,
            minerals: true,
            seafloor: true,
            ..Default::default()
        },
    ] {
        vertices(
            &crate::scene::mesh(sim, survey, cache, &cache.rivers),
            &crate::scene::mesh(sim, survey, &full, &full.rivers),
        );
    }
    for (a, b) in cache.ground_cover.iter().zip(&full.ground_cover) {
        assert_eq!(a.len(), b.len());
        for (a, b) in a.iter().zip(b) {
            assert_eq!(a.position_scale, b.position_scale);
            assert_eq!(a.tint_rotation, b.tint_rotation);
            assert_eq!(a.coverage, b.coverage);
        }
    }
    let reference = Picker::from_cache(sim, &full);
    for (a, b) in picker.chunks.iter().zip(&reference.chunks) {
        assert_eq!(a.triangles, b.triangles);
        assert_eq!(a.min, b.min);
        assert_eq!(a.max, b.max);
    }
    for i in 0..64 {
        let origin = Vec3::new(i as f32 - 32., 15., 12.);
        let dir = Vec3::new(0.15, -1., -0.6).normalize();
        for water in [false, true] {
            assert_eq!(
                picker.pick(origin, dir, water),
                reference.pick(origin, dir, water)
            );
        }
    }
}

#[test]
fn local_edits_match_full_geometry_water_colors_and_picking() {
    for boundary in [Boundary::Clamp, Boundary::Wrap] {
        let mut sim = fixture(boundary);
        let key = lod::Key::at(Vec3::new(0., 100., 0.));
        let mut cache = Cache::at(&sim, key);
        let mut picker = Picker::from_cache(&sim, &cache);
        // Picker bucket seam, river slope endpoint, confluence and world edge.
        for (x, z) in [(32, 32), (4, 8), (24, 8), (2, 2), (0, 0)] {
            let bounds = edit(&mut sim, x, z, 0.4);
            let (next, affected) = cache.refresh(&sim, key, bounds);
            let affected = affected.expect("far camera keeps topology stable");
            let next_picker = picker.refresh(&sim, &next, affected);
            assert!(Arc::ptr_eq(&cache.indices, &next.indices));
            if x == 32 {
                assert!(Arc::ptr_eq(&cache.rivers, &next.rivers));
                assert!(
                    picker
                        .chunks
                        .iter()
                        .zip(&next_picker.chunks)
                        .any(|(a, b)| Arc::ptr_eq(a, b))
                );
            } else if z == 8 {
                assert!(!Arc::ptr_eq(&cache.rivers, &next.rivers));
            }
            equivalent(&sim, &next, &next_picker);
            cache = next;
            picker = next_picker;
        }
    }
}

#[test]
fn changed_lod_splits_refresh_locally_and_camera_changes_rebuild() {
    let mut sim = fixture(Boundary::Clamp);
    let key = lod::Key::at(Vec3::new(0., 2.1, 0.));
    let cache = Cache::at(&sim, key);
    let bounds = edit(&mut sim, 32, 24, 50.);
    let (next, affected) = cache.refresh(&sim, key, bounds);
    assert!(affected.is_some());
    assert!(
        !Arc::ptr_eq(&cache.indices, &next.indices),
        "changed topology rebuilt"
    );
    let picker = Picker::from_cache(&fixture(Boundary::Clamp), &cache);
    equivalent(&sim, &next, &picker.refresh(&sim, &next, affected.unwrap()));
    let (moved, affected) = next.refresh(&sim, lod::Key::at(Vec3::new(0., 100., 0.)), bounds);
    assert!(affected.is_none());
    equivalent(&sim, &moved, &Picker::from_cache(&sim, &moved));
}

#[test]
fn close_detail_and_legacy_geometry_keep_exact_surface_samples() {
    for version in [0, 9] {
        let mut sim = fixture(Boundary::Clamp);
        sim.geography.version = version;
        let key = lod::Key::at(Vec3::new(-8., 2.1, -16.));
        let cache = Cache::at(&sim, key);
        let picker = Picker::from_cache(&sim, &cache);
        let bounds = edit(&mut sim, 24, 8, 0.00001);
        let (next, affected) = cache.refresh(&sim, key, bounds);
        let affected = affected.expect("small change retains nearby topology");
        equivalent(&sim, &next, &picker.refresh(&sim, &next, affected));
    }
}

#[test]
fn repeated_refinement_and_coarsening_preserve_seams_water_and_bounded_storage() {
    for (boundary, x, z) in [
        (Boundary::Clamp, 24, 8),
        (Boundary::Wrap, 1, 1),
        (Boundary::Clamp, 32, 24),
    ] {
        let mut sim = fixture(boundary);
        // Start coarse and lower a local patch toward the camera so new edge
        // vertices force unchanged neighboring roots to retriangulate too.
        for cell in sim.grid.cells_mut() {
            cell.elevation += 50.;
        }
        let key = lod::Key::at(Vec3::new(x as f32 - 32., 2.1, z as f32 - 24.));
        let mut cache = Cache::at(&sim, key);
        let mut picker = Picker::from_cache(&sim, &cache);
        let initial_samples = cache.samples.len();
        let mut saw_seam_change = false;
        for strength in [-50., 50., -50., 50.] {
            let bounds = edit(&mut sim, x, z, strength);
            let changed = lod::changed_splits(&sim, key, &cache.elevations, bounds);
            assert!(!changed.is_empty());
            let (next, affected) = cache.refresh(&sim, key, bounds);
            for (leaf, range) in next.patches.iter() {
                if !changed.contains_key(&(leaf.0 / 32, leaf.1 / 32))
                    && cache
                        .patches
                        .get(leaf)
                        .is_some_and(|old| old.len() != range.len())
                {
                    saw_seam_change = true;
                }
            }
            let next_picker =
                picker.refresh(&sim, &next, affected.expect("same camera remains local"));
            equivalent(&sim, &next, &next_picker);
            cache = next;
            picker = next_picker;
        }
        assert!(
            saw_seam_change,
            "exercise a shared edge outside changed roots"
        );
        assert_eq!(
            cache.samples.len(),
            initial_samples,
            "coarsening cannot retain abandoned vertices"
        );
    }
}

#[test]
fn geometry_only_edit_reuses_colors_and_later_ecology_stays_current() {
    let mut sim = fixture(Boundary::Clamp);
    let key = lod::Key::at(Vec3::new(0., 100., 0.));
    let cache = Cache::at(&sim, key);
    sim.grid.get_mut(32, 24).elevation += 0.4;
    let (next, affected) = cache.refresh(
        &sim,
        key,
        SampleBounds {
            min: [32, 24],
            max: [32, 24],
        },
    );
    assert!(next.paint.refreshed > 0 && next.paint.refreshed < next.samples.len());
    let picker = Picker::from_cache(&fixture(Boundary::Clamp), &cache);
    equivalent(&sim, &next, &picker.refresh(&sim, &next, affected.unwrap()));
    // Ordinary ticks do not reconstruct geometry; extraction must still see
    // environmental changes remote from the last brush/cache publication.
    sim.grid.get_mut(4, 40).plants = 0.;
    sim.grid.get_mut(60, 40).moisture = 1.;
    for (s, v) in next
        .samples
        .iter()
        .zip(next.paint.vertices(&sim, &next.samples).iter())
    {
        let expected = colored_vertex(&sim, s.0, s.1, s.2, s.3, Default::default());
        assert_eq!(v.color.map(f32::to_bits), expected.color.map(f32::to_bits));
    }
}
