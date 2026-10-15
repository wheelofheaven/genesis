use super::*;
use genesis_sim_core::{
    actions::Action, runtime::Simulation, save::Snapshot, scenarios, spatial::Position,
};

fn snapshot() -> Snapshot {
    Snapshot::capture(
        &Simulation::new(scenarios::coastal_outpost()),
        "test-world",
        &[],
    )
    .unwrap()
}
struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("genesis-save-test-{}", new_world_id()));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn save_round_trip_preserves_pending_intent_water_and_continuation() {
    let mut run = Simulation::new(scenarios::coastal_outpost());
    run.advance(&[Action::DrillWell {
        position: Position::new(18.25, 21.5),
    }]);
    for _ in 1..43 {
        run.advance(&[]);
    }
    let build = Action::PlaceBuilding {
        position: Position::new(25.31, 20.27),
        rotation: 0.47,
    };
    let pending = [
        build.clone(),
        build,
        Action::SetWellActive {
            id: 1,
            active: false,
        },
    ];
    let saved = Snapshot::capture(&run, "continuation-world", &pending).unwrap();
    let bytes = encode(&saved).unwrap();
    let mut loaded = decode(&bytes).unwrap();
    assert_eq!(run.completed_ticks(), 43);
    assert_eq!(loaded.simulation.completed_ticks(), 43);
    assert_eq!(loaded.world_id, "continuation-world");
    assert_eq!(
        loaded.simulation.buildings.len(),
        6,
        "saving must not execute placement"
    );
    assert!(loaded.simulation.subsurface.wells[0].active);
    assert_eq!(run.time.to_bits(), loaded.simulation.time.to_bits());
    assert_eq!(
        bytes,
        encode(&Snapshot::capture(&loaded.simulation, &loaded.world_id, &loaded.pending).unwrap())
            .unwrap()
    );
    for tick in 44..=100 {
        let expected = run.advance(if tick == 44 { &pending } else { &[] });
        let actions = std::mem::take(&mut loaded.pending);
        let actual = loaded.simulation.advance(&actions);
        assert_eq!(
            serde_json::to_vec(&expected).unwrap(),
            serde_json::to_vec(&actual).unwrap()
        );
    }
    assert_eq!(run.buildings.len(), 7);
    assert_eq!(
        serde_json::to_vec(&*run).unwrap(),
        serde_json::to_vec(&*loaded.simulation).unwrap()
    );
}

#[test]
fn generated_region_round_trip_rebuilds_matching_surface_queries() {
    for region in genesis_sim_core::worldgen::Region::ALL {
        check_region(region);
    }
}

fn check_region(region: genesis_sim_core::worldgen::Region) {
    let run = Simulation::new(genesis_sim_core::worldgen::generate_region(7319, region));
    let bytes = encode(&Snapshot::capture(&run, "generated-world", &[]).unwrap()).unwrap();
    let loaded = decode(&bytes).unwrap().simulation;
    assert_eq!(
        serde_json::to_vec(&*run).unwrap(),
        serde_json::to_vec(&*loaded).unwrap()
    );
    for river in &run.geography.rivers {
        for &p in river.points.iter().step_by(7) {
            assert_eq!(
                genesis_sim_core::surface::elevation(&run, p),
                genesis_sim_core::surface::elevation(&loaded, p)
            );
            assert_eq!(
                genesis_sim_core::worldgen::river_distance(&run, p),
                genesis_sim_core::worldgen::river_distance(&loaded, p)
            );
        }
    }
}

#[test]
fn malformed_or_incompatible_records_fail_before_world_use() {
    let good: serde_json::Value =
        serde_json::from_slice(include_bytes!("../fixtures/v1.json")).unwrap();
    for (pointer, value) in [
        ("/schema", serde_json::json!(2)),
        ("/rules", serde_json::json!("genesis:rules/unknown")),
        ("/content", serde_json::json!("missing-pack")),
        ("/completed_ticks", serde_json::json!(u64::MAX)),
        ("/state/grid/width", serde_json::json!(0)),
        ("/state/grid/width", serde_json::json!(u32::MAX)),
        ("/state/subsurface/width", serde_json::json!(2)),
        ("/state/grid/cells/0/fungus", serde_json::json!(2.)),
        ("/state/subsurface/columns/0/stored", serde_json::json!(1.)),
        ("/state/geography/version", serde_json::json!(999)),
    ] {
        let mut bad = good.clone();
        *bad.pointer_mut(pointer).unwrap() = value;
        assert!(
            decode(&serde_json::to_vec(&bad).unwrap()).is_err(),
            "{pointer}"
        );
    }
    let mut invalid = snapshot();
    invalid.state.buildings[1].id = invalid.state.buildings[0].id;
    assert!(invalid.restore().is_err());
    let mut invalid = snapshot();
    invalid.state.grid.get_mut(0, 0).elevation = f32::NAN;
    assert!(encode(&invalid).is_err());
    assert!(decode(b"{truncated").is_err());
    assert!(decode(&vec![b' '; MAX_SAVE_BYTES as usize + 1]).is_err());
}

#[test]
fn checked_in_v1_fixture_loads_and_continues() {
    let mut saved = decode(include_bytes!("../fixtures/v1.json")).unwrap();
    assert_eq!(saved.world_id, "fixture-v1");
    assert_eq!(saved.simulation.completed_ticks(), 0);
    let events = saved.simulation.advance(&saved.pending);
    assert!(matches!(
        events[0],
        genesis_sim_core::events::SimEvent::OutpostBuilt { x: 0, y: 0 }
    ));
    assert_eq!(saved.simulation.completed_ticks(), 1);
}

#[test]
fn file_replacement_keeps_prior_save_and_failures_leave_it_intact() {
    let dir = Directory::new();
    let path = dir.0.join("world.json");
    let old = snapshot();
    store(&path, &old).unwrap();
    let old_bytes = fs::read(&path).unwrap();
    let mut next = old.clone();
    next.world_id = "second-world".into();
    let backup = backup_path(&path).unwrap();
    fs::create_dir(&backup).unwrap(); // deterministic I/O failure before main commit
    assert!(store(&path, &next).is_err());
    assert_eq!(fs::read(&path).unwrap(), old_bytes);
    assert_eq!(
        fs::read_dir(&dir.0).unwrap().count(),
        2,
        "temporary file was cleaned"
    );
    fs::remove_dir(&backup).unwrap();
    store(&path, &next).unwrap();
    assert_eq!(fs::read(&backup).unwrap(), old_bytes);
    assert_eq!(load(&path).unwrap().world_id, "second-world");
    assert_eq!(load(&backup).unwrap().world_id, "test-world");
    next.state.time = f64::INFINITY;
    assert!(store(&path, &next).is_err());
    assert_eq!(load(&path).unwrap().world_id, "second-world");
}

#[test]
fn v1_object_migration_preserves_references_and_allocation_after_removal() {
    use genesis_sim_core::{events::SimEvent, settlement::Building, subsurface::WellError};
    let mut old = decode(include_bytes!("../fixtures/v1-objects.json")).unwrap();
    assert_eq!(old.simulation.paths[0].id, 1);
    let events = old.simulation.advance(&old.pending);
    assert!(matches!(
        events[0],
        SimEvent::WellOperationChanged {
            id: 2,
            active: false
        }
    ));
    assert!(old.simulation.subsurface.wells[0].active);
    assert!(!old.simulation.subsurface.wells[1].active);
    let mut saved = Snapshot::capture(&old.simulation, &old.world_id, &[]).unwrap();
    assert_eq!(saved.schema, genesis_sim_core::save::SCHEMA);
    saved.state.buildings.reverse();
    saved.state.subsurface.wells.reverse();
    let reordered = decode(&encode(&saved).unwrap()).unwrap();
    assert_eq!(reordered.simulation.buildings[0].id, 2);
    assert_eq!(reordered.simulation.subsurface.wells[0].id, 2);
    saved.state.buildings.retain(|b| b.id != 2);
    saved.state.subsurface.wells.retain(|w| w.id != 2);
    saved.state.paths.clear();
    saved.pending = vec![
        Action::SetWellActive {
            id: 2,
            active: false,
        },
        Action::PlaceBuilding {
            position: Position::new(5., 2.),
            rotation: 0.,
        },
        Action::DrillWell {
            position: Position::new(3., 5.),
        },
        Action::BuildPath {
            waypoints: vec![Position::new(5., 5.), Position::new(6., 6.)],
            width: Building::PATH_WIDTH,
        },
    ];
    let mut restored = decode(&encode(&saved).unwrap()).unwrap();
    let events = restored.simulation.advance(&restored.pending);
    assert!(matches!(
        events[0],
        SimEvent::WellRejected {
            reason: WellError::UnknownWell
        }
    ));
    assert!(matches!(events[1], SimEvent::BuildingPlaced { id: 3 }));
    assert!(matches!(events[2], SimEvent::WellDrilled { id: 3 }));
    assert!(matches!(events[3], SimEvent::PathBuilt { id: 2 }));
    let again = decode(
        &encode(&Snapshot::capture(&restored.simulation, &restored.world_id, &[]).unwrap())
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        serde_json::to_vec(&*again.simulation).unwrap(),
        serde_json::to_vec(&*restored.simulation).unwrap()
    );
}

#[test]
fn corrupt_identity_metadata_is_rejected_without_guessing() {
    let good = serde_json::to_value(snapshot()).unwrap();
    for (pointer, value) in [
        ("/schema", serde_json::json!(999)),
        ("/state/building_ids", serde_json::json!(1)),
        ("/state/path_ids", serde_json::json!(0)),
        ("/state/subsurface/well_ids", serde_json::json!(0)),
        ("/state/paths/0/id", serde_json::json!(0)),
        ("/state/buildings/0/id", serde_json::json!(u64::MAX)),
    ] {
        let mut bad = good.clone();
        *bad.pointer_mut(pointer).unwrap() = value;
        assert!(
            decode(&serde_json::to_vec(&bad).unwrap()).is_err(),
            "{pointer}"
        );
    }
    let mut bad = good.clone();
    bad["state"].as_object_mut().unwrap().remove("building_ids");
    assert!(decode(&serde_json::to_vec(&bad).unwrap()).is_err());
    let mut bad = snapshot();
    bad.state.paths.push(bad.state.paths[0].clone());
    assert!(encode(&bad).is_err());
    let mut legacy: serde_json::Value =
        serde_json::from_slice(include_bytes!("../fixtures/v1-objects.json")).unwrap();
    legacy["state"]["buildings"][1]["id"] = serde_json::json!(1);
    assert!(decode(&serde_json::to_vec(&legacy).unwrap()).is_err());
}

#[test]
fn pending_terrain_requests_round_trip_without_applying_early() {
    use genesis_sim_core::{events::SimEvent, grid::Boundary};
    let mut state = scenarios::coastal_outpost();
    state.grid.boundary = Boundary::Clamp;
    let mut run = Simulation::new(state);
    let pending = [
        Action::Terraform {
            x: 15,
            y: 12,
            radius: 2,
            strength: 0.4,
        },
        Action::Terraform {
            x: 100_000,
            y: 12,
            radius: 2,
            strength: 0.4,
        },
        Action::Terraform {
            x: 15,
            y: 12,
            radius: 2,
            strength: 0.,
        },
    ];
    let before = run.grid.get(15, 12).elevation;
    let saved = Snapshot::capture(&run, "terrain-continuation", &pending).unwrap();
    let mut loaded = decode(&encode(&saved).unwrap()).unwrap();
    assert_eq!(loaded.simulation.grid.get(15, 12).elevation, before);
    let expected = run.advance(&pending);
    let actual = loaded
        .simulation
        .advance(&std::mem::take(&mut loaded.pending));
    assert!(matches!(actual[0], SimEvent::TerrainChanged { .. }));
    assert!(matches!(actual[1], SimEvent::TerraformRejected { .. }));
    assert!(matches!(actual[2], SimEvent::TerrainUnchanged { .. }));
    assert_eq!(
        serde_json::to_vec(&expected).unwrap(),
        serde_json::to_vec(&actual).unwrap()
    );
    assert_eq!(
        serde_json::to_vec(&*run).unwrap(),
        serde_json::to_vec(&*loaded.simulation).unwrap()
    );
}

#[test]
fn pending_seed_brushes_preserve_saved_coverage_and_exact_continuation() {
    for boundary in [
        genesis_sim_core::grid::Boundary::Clamp,
        genesis_sim_core::grid::Boundary::Wrap,
    ] {
        let mut state = scenarios::coastal_outpost();
        state.grid.boundary = boundary;
        let mut run = Simulation::new(state);
        let pending = [
            Action::SeedFungus {
                x: -1,
                y: 12,
                radius: 2,
            },
            Action::SeedPlants {
                x: 15,
                y: 12,
                radius: 2,
            },
            Action::SeedPlants {
                x: 15,
                y: 12,
                radius: 2,
            },
        ];
        let saved = Snapshot::capture(&run, "seed-continuation", &pending).unwrap();
        let mut loaded = decode(&encode(&saved).unwrap()).unwrap();
        assert_eq!(
            serde_json::to_vec(&*run).unwrap(),
            serde_json::to_vec(&*loaded.simulation).unwrap()
        );
        for tick in 0..10 {
            let expected = run.advance(if tick == 0 { &pending } else { &[] });
            let actual = loaded
                .simulation
                .advance(&std::mem::take(&mut loaded.pending));
            assert_eq!(
                serde_json::to_vec(&expected).unwrap(),
                serde_json::to_vec(&actual).unwrap()
            );
        }
        assert_eq!(
            serde_json::to_vec(&*run).unwrap(),
            serde_json::to_vec(&*loaded.simulation).unwrap()
        );
        assert_eq!(run.completed_ticks(), loaded.simulation.completed_ticks());
    }
}

#[test]
fn resident_save_continuation_and_v2_compatibility() {
    let old = decode(include_bytes!("../fixtures/v2-objects.json")).unwrap();
    assert!(old.simulation.people.all().is_empty());
    assert_eq!(old.simulation.buildings[0].id, 1);
    let mut a = Simulation::new(genesis_sim_core::scenarios::living_settlement());
    for _ in 0..35 {
        a.advance(&[]);
    }
    let saved = Snapshot::capture(&a, "living", &[]).unwrap();
    let bytes = encode(&saved).unwrap();
    let mut b = decode(&bytes).unwrap().simulation;
    for _ in 0..600 {
        a.advance(&[]);
        b.advance(&[]);
    }
    assert_eq!(
        serde_json::to_vec(&*a).unwrap(),
        serde_json::to_vec(&*b).unwrap()
    );
    let good = serde_json::to_value(saved).unwrap();
    for (pointer, value) in [
        ("/state/people/entries/0/home", serde_json::json!(999)),
        ("/state/people/entries/0/id", serde_json::json!(2)),
        ("/state/people/entries/0/next", serde_json::json!(99999)),
        (
            "/state/people/entries/0/remaining_ms",
            serde_json::json!(99999),
        ),
        ("/state/people/ids", serde_json::json!(1)),
        ("/schema", serde_json::json!(2)),
    ] {
        let mut bad = good.clone();
        *bad.pointer_mut(pointer).unwrap() = value;
        assert!(
            decode(&serde_json::to_vec(&bad).unwrap()).is_err(),
            "{pointer}"
        );
    }
    let mut missing = good;
    missing["state"].as_object_mut().unwrap().remove("people");
    assert!(decode(&serde_json::to_vec(&missing).unwrap()).is_err());
}
