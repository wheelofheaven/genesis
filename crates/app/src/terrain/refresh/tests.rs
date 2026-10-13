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
    assert_eq!(cache.samples, full.samples);
    assert_eq!(cache.indices, full.indices);
    assert_eq!(cache.patches, full.patches);
    vertices(&cache.rivers, &full.rivers);
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
fn camera_and_changed_lod_splits_fall_back_to_full_build() {
    let mut sim = fixture(Boundary::Clamp);
    let key = lod::Key::at(Vec3::new(0., 2.1, 0.));
    let cache = Cache::at(&sim, key);
    let bounds = edit(&mut sim, 32, 24, 50.);
    let (next, affected) = cache.refresh(&sim, key, bounds);
    assert!(
        affected.is_none(),
        "large elevation change changes root LOD"
    );
    equivalent(&sim, &next, &Picker::from_cache(&sim, &next));
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
