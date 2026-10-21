use super::*;
use crate::{SimState, actions::Action, events::SimEvent, food::SourceId, scenarios};

fn demand() -> SimState {
    let mut sim = scenarios::water_settlement();
    for c in &mut sim.subsurface.columns {
        c.recharge = 0.;
    }
    sim.step(0., &[mode(PumpMode::FillBuffer)]);
    sim
}
fn mode(mode: PumpMode) -> Action {
    Action::SetWellPumpMode { id: 1, mode }
}
fn stored(s: &Subsurface) -> f64 {
    s.columns.iter().map(|c| f64::from(c.stored)).sum()
}

#[test]
fn full_buffer_preserves_aquifer_and_refills_after_collection() {
    let mut sim = demand();
    let before = stored(&sim.subsurface);
    for _ in 0..200 {
        sim.subsurface.step(&sim.grid, sim.sea_level, 0.1);
    }
    let w = &sim.subsurface.wells[0];
    let supply = w.supply.as_ref().unwrap();
    assert!(supply.stored >= supply::CAPACITY - FULL_MARGIN);
    assert_eq!(supply.released, 0.);
    assert!(supply.valid(w.recovered));
    assert_eq!(w.last_yield, 0.);
    assert!(w.active);
    assert_eq!(
        w.pumping_status(&sim.grid, sim.sea_level),
        "Buffer full · waiting for collection"
    );
    let recovered = w.recovered;
    for _ in 0..500 {
        sim.subsurface.step(&sim.grid, sim.sea_level, 0.1);
    }
    assert_eq!(sim.subsurface.wells[0].recovered, recovered);
    assert!((before - stored(&sim.subsurface) - recovered).abs() < 1e-9);
    assert!(sim.subsurface.wells[0].supply.as_mut().unwrap().collect(2.));
    for _ in 0..20 {
        sim.subsurface.step(&sim.grid, sim.sea_level, 0.1);
    }
    let w = &sim.subsurface.wells[0];
    assert!((w.recovered - recovered - 2.).abs() <= FULL_MARGIN);
    assert_eq!(w.supply.as_ref().unwrap().released, 0.);
    assert!((before - stored(&sim.subsurface) - w.recovered).abs() < 1e-9);
}

#[test]
fn fractional_and_edge_taps_never_overdraw_buffer_even_with_large_steps() {
    for position in [
        Position::new(8., 8.),
        Position::new(8.37, 8.71),
        Position::new(15.9, 15.9),
    ] {
        for room in [0.00011, 0.001, 0.13, 1.7, 20.] {
            let mut sim = demand();
            let w = &mut sim.subsurface.wells[0];
            w.position = position;
            w.recovered = supply::CAPACITY - room;
            w.supply.as_mut().unwrap().stored = w.recovered;
            let before = stored(&sim.subsurface);
            let initial = sim.subsurface.wells[0].recovered;
            for dt in [0.1, 1., 1e6, 0.1] {
                sim.subsurface.step(&sim.grid, sim.sea_level, dt);
            }
            let w = &sim.subsurface.wells[0];
            assert!(w.supply.as_ref().unwrap().valid(w.recovered));
            assert_eq!(
                w.supply.as_ref().unwrap().released,
                0.,
                "{position:?}, {room}"
            );
            assert!(w.recovered - initial <= room + 1e-12);
            assert!((before - stored(&sim.subsurface) - (w.recovered - initial)).abs() < 1e-9);
        }
    }
}

#[test]
fn operator_stop_and_submersion_override_policy_and_continuous_can_overflow() {
    let mut sim = demand();
    sim.step(
        0.,
        &[Action::SetWellActive {
            id: 1,
            active: false,
        }],
    );
    sim.subsurface.step(&sim.grid, sim.sea_level, 10.);
    assert_eq!(sim.subsurface.wells[0].recovered, 0.);
    assert_eq!(
        sim.subsurface.wells[0].pumping_status(&sim.grid, sim.sea_level),
        "Pump stopped"
    );
    sim.step(0., &[mode(PumpMode::Continuous)]);
    assert!(!sim.subsurface.wells[0].active);
    sim.step(
        0.,
        &[
            mode(PumpMode::FillBuffer),
            Action::SetWellActive {
                id: 1,
                active: true,
            },
        ],
    );
    sim.subsurface.step(&sim.grid, 2., 10.);
    assert_eq!(sim.subsurface.wells[0].recovered, 0.);
    assert_eq!(
        sim.subsurface.wells[0].pumping_status(&sim.grid, 2.),
        "Submerged · pumping blocked"
    );
    sim.subsurface.step(&sim.grid, sim.sea_level, 10.);
    assert!(sim.subsurface.wells[0].recovered > 0.);
    assert_eq!(
        sim.subsurface.wells[0].supply.as_ref().unwrap().released,
        0.
    );
    sim.step(0., &[mode(PumpMode::Continuous)]);
    sim.subsurface.step(&sim.grid, sim.sea_level, 10.);
    assert!(sim.subsurface.wells[0].supply.as_ref().unwrap().released > 0.);
    let events = sim.step(
        0.,
        &[Action::SetWellPumpMode {
            id: 999,
            mode: PumpMode::FillBuffer,
        }],
    );
    assert!(events.iter().any(|e| matches!(
        e,
        SimEvent::WellRejected {
            reason: super::super::WellError::UnknownWell
        }
    )));
}

#[test]
fn full_enabled_wells_keep_household_reservations_and_delivery_working() {
    let mut sim = demand();
    for _ in 0..200 {
        sim.subsurface.step(&sim.grid, sim.sea_level, 0.1);
    }
    let mut delivered = std::collections::BTreeSet::new();
    for _ in 0..1800 {
        for e in sim.step(0.1, &[]) {
            if let SimEvent::WaterDelivered { home, .. } = e {
                delivered.insert(home);
            }
        }
        assert!(sim.people.valid(&sim));
        assert!(sim.households.valid(&sim));
        assert!(sim.subsurface.wells[0].active);
        assert_eq!(
            sim.subsurface.wells[0].supply.as_ref().unwrap().released,
            0.
        );
    }
    assert_eq!(delivered.len(), 6);
}

#[test]
fn shared_aquifer_arbitration_is_stable_and_full_wells_leave_water_for_others() {
    let mut a = demand();
    a.step(
        0.,
        &[Action::DrillWell {
            position: Position::new(8.3, 8.),
        }],
    );
    assert_eq!(a.subsurface.wells.len(), 2);
    for c in &mut a.subsurface.columns {
        c.stored = 4.;
    }
    a.subsurface.wells[0].recovered = 20.;
    a.subsurface.wells[0].supply.as_mut().unwrap().stored = 20.;
    a.subsurface.wells[1].pump_mode = PumpMode::FillBuffer;
    let mut b = a.clone();
    b.subsurface.wells.reverse();
    for _ in 0..100 {
        a.subsurface.step(&a.grid, a.sea_level, 0.1);
        b.subsurface.step(&b.grid, b.sea_level, 0.1);
    }
    assert_eq!(a.subsurface.wells, b.subsurface.wells);
    assert_eq!(a.subsurface.wells[0].recovered, 20.);
    assert!(a.subsurface.wells[1].recovered > 4.);
    for (a, b) in a.subsurface.columns.iter().zip(&b.subsurface.columns) {
        assert_eq!(a.stored, b.stored);
    }
}

#[test]
fn conserving_pump_supports_more_dry_harvests_than_continuous_overflow() {
    let mut sim = scenarios::dry_cultivated_settlement();
    sim.step(
        0.,
        &[Action::SetCropTending {
            source: SourceId(1),
            enabled: true,
        }],
    );
    let mut continuous = sim.clone();
    sim.step(0., &[mode(PumpMode::FillBuffer)]);
    for _ in 0..6000 {
        sim.step(0.1, &[]);
        continuous.step(0.1, &[]);
        assert!(sim.people.valid(&sim));
        assert!(sim.households.valid(&sim));
        assert!(sim.food.valid(&sim));
    }
    assert_eq!(
        sim.subsurface.wells[0].supply.as_ref().unwrap().released,
        0.
    );
    assert!(
        continuous.subsurface.wells[0]
            .supply
            .as_ref()
            .unwrap()
            .released
            > 0.
    );
    assert!(sim.food.sources()[0].produced() > continuous.food.sources()[0].produced());
    assert!(sim.food.homes().values().all(|h| h.consumed > 0));
}

#[test]
fn depleted_columns_respect_room_smaller_than_the_last_representable_dose() {
    for stock in [0.1_f32, 1.7, 19.99, 1e9] {
        for position in [
            Position::new(8., 8.),
            Position::new(8.37, 8.71),
            Position::new(15.9, 15.9),
        ] {
            let mut sim = demand();
            for c in &mut sim.subsurface.columns {
                c.capacity = stock;
                c.stored = stock;
            }
            let room = f64::from(stock).min(20.) - 0.000_000_01;
            let before: Vec<_> = sim.subsurface.columns.iter().map(|c| c.stored).collect();
            let extracted = sim.subsurface.withdraw_to_buffer(position, 1e6, room);
            assert!(
                extracted <= room,
                "stock {stock}, room {room}, extracted {extracted}"
            );
            assert!(extracted >= 0.);
            let loss: f64 = before
                .iter()
                .zip(&sim.subsurface.columns)
                .map(|(&before, c)| f64::from(before) - f64::from(c.stored))
                .sum();
            assert_eq!(loss, extracted);
        }
    }
}
