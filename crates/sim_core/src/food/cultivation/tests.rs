use super::*;
use crate::{actions::Action, paths, scenarios, settlement::Building};

#[test]
fn repeated_harvests_feed_six_homes_alongside_water_and_conserve_food() {
    let mut sim = scenarios::cultivated_settlement();
    assert_eq!(sim.food.sources().len(), 1);
    let mut dry = sim.clone();
    for c in dry.grid.cells_mut() {
        c.rainfall = 0.01;
    }
    let mut maturities = 0;
    for _ in 0..6000 {
        for event in sim.step(0.1, &[]) {
            if matches!(event, SimEvent::CropMatured { .. }) {
                maturities += 1;
            }
        }
        dry.step(0.1, &[]);
        assert!(sim.food.valid(&sim));
        assert!(sim.people.valid(&sim));
    }
    assert!(maturities >= 3, "only {maturities} harvests");
    assert!(sim.food.homes().values().all(|h| h.consumed >= 8));
    assert!(sim.households.all().values().all(|h| h.consumed > 0.));
    assert_eq!(dry.food.sources()[0].produced(), 0);
    assert_eq!(status(&dry, &dry.food.sources()[0]), Status::Dry);
    assert!(dry.households.all().values().all(|h| h.consumed > 0.));
}

#[test]
fn placement_revalidates_and_preserves_existing_footprints() {
    let mut sim = scenarios::water_settlement();
    let p = Position::new(7.5, 8.);
    assert!(validate(&sim, p).is_ok());
    let events = sim.step(
        0.,
        &[
            Action::PlantCrop { position: p },
            Action::PlantCrop { position: p },
        ],
    );
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, SimEvent::CropPlanted { .. }))
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, SimEvent::CropPlantRejected { .. }))
            .count(),
        1
    );
    assert_eq!(sim.food.sources()[0].id, SourceId(1));
    let before = sim.food.clone();
    for p in [
        p,
        Position::new(f32::NAN, 8.),
        Position::new(0., 8.),
        sim.buildings[0].position,
        sim.subsurface.wells[0].position,
    ] {
        assert!(
            sim.step(0., &[Action::PlantCrop { position: p }])
                .iter()
                .any(|e| matches!(e, SimEvent::CropPlantRejected { .. }))
        );
        assert_eq!(sim.food, before);
    }
    assert!(
        sim.validate_building(&Building {
            id: 0,
            position: p,
            rotation: 0.7
        })
        .is_err()
    );
    assert!(sim.validate_well(p).is_err());
    assert!(
        paths::validate(
            &sim,
            &[
                Position::new(p.x - metres(5.), p.z),
                Position::new(p.x + metres(5.), p.z)
            ],
            metres(1.2)
        )
        .is_err()
    );
    let mut other = scenarios::water_settlement();
    let route = [
        Position::new(p.x - metres(5.), p.z),
        Position::new(p.x + metres(5.), p.z),
    ];
    other.step(
        0.,
        &[Action::BuildPath {
            waypoints: route.to_vec(),
            width: metres(1.2),
        }],
    );
    assert_eq!(other.paths.len(), 1);
    assert!(validate(&other, p).is_err());
}

#[test]
fn growth_uses_logical_time_and_moisture_and_blocks_without_erasing_stock() {
    let mut wet = scenarios::cultivated_settlement();
    for c in wet.grid.cells_mut() {
        c.moisture = 0.6;
    }
    let mut slow = wet.clone();
    for c in slow.grid.cells_mut() {
        c.moisture = 0.4;
    }
    let mut fine = wet.clone();
    for _ in 0..300 {
        advance(&mut wet, 0.1, &mut vec![]);
        advance(&mut slow, 0.1, &mut vec![]);
    }
    for _ in 0..3000 {
        advance(&mut fine, 0.01, &mut vec![]);
    }
    assert_eq!(wet.food, fine.food);
    let Kind::Barley(a) = &wet.food.sources()[0].kind else {
        panic!()
    };
    let Kind::Barley(b) = &slow.food.sources()[0].kind else {
        panic!()
    };
    assert_eq!(a.growth_units, 2 * b.growth_units);
    let before = wet.food.clone();
    wet.sea_level = 2.;
    advance(&mut wet, 30., &mut vec![]);
    assert_eq!(wet.food, before);
    assert_eq!(status(&wet, &wet.food.sources()[0]), Status::Blocked);
    wet.sea_level = 0.;
    advance(&mut wet, 30., &mut vec![]);
    assert_eq!(wet.food.sources()[0].remaining, YIELD);
    let mature = wet.food.clone();
    advance(&mut wet, 600., &mut vec![]);
    assert_eq!(wet.food, mature); // No stockpiling future harvests while the plot is full.
    wet.sea_level = 2.;
    advance(&mut wet, 30., &mut vec![]);
    assert_eq!(wet.food, mature);
    assert!(!accessible(&wet, &wet.food.sources()[0]));
}
