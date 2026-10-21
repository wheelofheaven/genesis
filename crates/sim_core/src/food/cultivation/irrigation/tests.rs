use super::*;
use crate::{actions::Action, events::SimEvent, food::cultivation, scenarios, spatial::Position};

fn dry() -> SimState {
    let mut sim = scenarios::cultivated_settlement();
    for c in sim.grid.cells_mut() {
        c.rainfall = 0.01;
    }
    for _ in 0..200 {
        sim.step(0.1, &[]);
    }
    sim
}
fn soil(sim: &SimState) -> &SoilWater {
    let Kind::Barley(c) = &sim.food.sources()[0].kind else {
        panic!()
    };
    c.soil_water.as_ref().unwrap()
}
fn action() -> Action {
    Action::IrrigateCrop {
        source: SourceId(1),
        well: 1,
    }
}

#[test]
fn irrigation_enables_dry_harvest_and_conserves_water_through_loss_and_delivery() {
    let mut sim = dry();
    let mut control = sim.clone();
    assert_eq!(
        cultivation::status(&sim, &sim.food.sources()[0]),
        cultivation::Status::Dry
    );
    assert!(candidate(&sim, SourceId(1)).is_ok());
    let before = sim.subsurface.wells[0].supply.as_ref().unwrap().stored;
    let events = sim.step(0., &[action(), action(), action()]);
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, SimEvent::CropIrrigated { .. }))
            .count(),
        2
    );
    assert!(events.iter().any(|e| matches!(
        e,
        SimEvent::CropIrrigationRejected {
            reason: Error::PlotFull,
            ..
        }
    )));
    assert_eq!(
        sim.subsurface.wells[0].supply.as_ref().unwrap().stored,
        before - 4.
    );
    assert_eq!(soil(&sim).stored, CAPACITY);
    for _ in 0..2000 {
        sim.step(0.1, &[]);
        control.step(0.1, &[]);
        assert!(sim.households.valid(&sim));
        assert!(sim.food.valid(&sim));
        assert!(sim.people.valid(&sim));
        assert!(soil(&sim).valid());
    }
    assert_eq!(soil(&sim).stored, 0);
    assert_eq!(soil(&sim).lost, CAPACITY);
    assert_eq!(
        sim.food.sources()[0].produced(),
        u64::from(cultivation::YIELD)
    );
    assert_eq!(control.food.sources()[0].produced(), 0);
    assert!(sim.food.homes().values().any(|h| h.consumed > 0));
}

#[test]
fn ordered_transfers_preserve_reservations_and_rejections_are_atomic() {
    let mut sim = scenarios::cultivated_settlement();
    for c in sim.grid.cells_mut() {
        c.rainfall = 0.01;
    }
    for _ in 0..100 {
        sim.step(0.1, &[]);
    }
    assert!(sim.people.reserved_water(1) > 0.);
    // Restrict only the available buffer, booking the removed excess as release.
    let reserved = sim.people.reserved_water(1);
    let supply = sim.subsurface.wells[0].supply.as_mut().unwrap();
    supply.released += supply.stored - reserved;
    supply.stored = reserved;
    assert_eq!(
        validate(&sim, SourceId(1), 1),
        Err(Error::WaterReservedOrEmpty)
    );
    let before = sim.food.clone();
    let wells = sim.subsurface.wells.clone();
    assert!(
        sim.step(0., &[action()])
            .iter()
            .any(|e| matches!(e, SimEvent::CropIrrigationRejected { .. }))
    );
    assert_eq!(sim.food, before);
    assert_eq!(sim.subsurface.wells, wells);
    assert!(sim.people.valid(&sim));
    assert!(sim.households.valid(&sim));
    for _ in 0..100 {
        sim.step(0.1, &[]);
    }
    let events = sim.step(
        0.,
        &[
            Action::SetWellActive {
                id: 1,
                active: false,
            },
            action(),
        ],
    );
    assert!(events.iter().any(|e| matches!(
        e,
        SimEvent::CropIrrigationRejected {
            reason: Error::WellStopped,
            ..
        }
    )));
    assert_eq!(soil(&sim).applied, 0);
    assert_eq!(validate(&sim, SourceId(999), 1), Err(Error::NotACrop));
    assert_eq!(validate(&sim, SourceId(1), 999), Err(Error::MissingWell));
    sim.subsurface.wells[0].active = true;
    sim.subsurface.wells[0].position = Position::new(12., 12.);
    assert_eq!(validate(&sim, SourceId(1), 1), Err(Error::TooFar));
    sim.sea_level = 2.;
    assert_eq!(validate(&sim, SourceId(1), 1), Err(Error::PlotBlocked));
}

#[test]
fn draining_limits_growth_to_wet_time_and_continues_on_blocked_and_mature_plots() {
    let mut coarse = dry();
    coarse.step(0., &[action()]);
    let mut fine = coarse.clone();
    cultivation::advance(&mut coarse, 50., &mut vec![]);
    for _ in 0..500 {
        cultivation::advance(&mut fine, 0.1, &mut vec![]);
    }
    assert_eq!(coarse.food, fine.food);
    let Kind::Barley(c) = &coarse.food.sources()[0].kind else {
        panic!()
    };
    assert_eq!(c.growth_units, 40_000_000);
    assert_eq!(soil(&coarse).stored, 0);
    coarse.step(0., &[action()]);
    coarse.sea_level = 2.;
    cultivation::advance(&mut coarse, 10., &mut vec![]);
    assert_eq!(soil(&coarse).stored, 1_500_000);
    assert_eq!(
        cultivation::status(&coarse, &coarse.food.sources()[0]),
        cultivation::Status::Blocked
    );
    coarse.sea_level = 0.;
    cultivation::advance(&mut coarse, 20., &mut vec![]);
    assert_eq!(coarse.food.sources()[0].remaining, cultivation::YIELD);
    cultivation::advance(&mut coarse, 10., &mut vec![]);
    assert_eq!(soil(&coarse).stored, 0);
    assert!(soil(&coarse).valid());
}

#[test]
fn source_choice_is_stable_and_skips_an_unusable_nearer_well() {
    let mut sim = dry();
    sim.step(
        0.,
        &[Action::DrillWell {
            position: Position::new(7., 8.),
        }],
    );
    assert_eq!(sim.subsurface.wells.len(), 2);
    for _ in 0..100 {
        sim.step(0.1, &[]);
    }
    assert_eq!(candidate(&sim, SourceId(1)), Ok(1));
    sim.subsurface.wells.reverse();
    assert_eq!(candidate(&sim, SourceId(1)), Ok(1));
    sim.step(
        0.,
        &[Action::SetWellActive {
            id: 1,
            active: false,
        }],
    );
    assert_eq!(candidate(&sim, SourceId(1)), Ok(2));
    let wild = scenarios::supplied_settlement();
    assert_eq!(
        validate(&wild, wild.food.sources()[0].id, 1),
        Err(Error::NotACrop)
    );
}
