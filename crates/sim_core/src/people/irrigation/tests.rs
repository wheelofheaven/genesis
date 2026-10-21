use super::*;
use crate::{actions::Action, scenarios};

fn dry() -> SimState {
    let mut sim = scenarios::cultivated_settlement();
    for c in sim.grid.cells_mut() {
        c.rainfall = 0.01;
    }
    sim
}
fn toggle(enabled: bool) -> Action {
    Action::SetCropTending {
        source: SourceId(1),
        enabled,
    }
}
fn valid(sim: &SimState) {
    assert!(sim.people.valid(sim));
    assert!(sim.households.valid(sim));
    assert!(sim.food.valid(sim));
    for well in &sim.subsurface.wells {
        assert!(well.supply.as_ref().unwrap().valid(well.recovered));
    }
}
fn task_phase(carrying: bool) -> (SimState, PersonId) {
    let mut sim = dry();
    sim.step(0.1, &[toggle(true)]);
    for _ in 0..2000 {
        sim.step(0.1, &[]);
        valid(&sim);
        if let Some(p) = sim
            .people
            .all()
            .iter()
            .find(|p| p.irrigation_task.is_some() && (p.carried_water > 0.) == carrying)
        {
            let id = p.id;
            return (sim, id);
        }
    }
    panic!("no irrigation task in requested phase");
}

#[test]
fn opt_in_hauling_sustains_dry_harvests_and_conserves_every_transfer() {
    let mut sim = dry();
    // A larger finite aquifer lets this test cover multiple crop cycles.
    for c in &mut sim.subsurface.columns {
        c.capacity = 1200.;
        c.stored = 1200.;
    }
    let mut control = sim.clone();
    sim.step(0., &[toggle(true)]);
    let mut delivered = 0;
    for _ in 0..6000 {
        for e in sim.step(0.1, &[]) {
            if matches!(e, SimEvent::CropWatered { .. }) {
                delivered += 1;
            }
        }
        control.step(0.1, &[]);
        valid(&sim);
    }
    assert!(delivered >= 4, "{delivered}");
    assert!(sim.food.sources()[0].produced() >= 48);
    assert_eq!(control.food.sources()[0].produced(), 0);
    assert_eq!(soil::total_applied(&control), 0.);
    assert_eq!(soil::total_applied(&sim), f64::from(delivered) * 2.);
    assert!(sim.food.homes().values().all(|h| h.consumed > 0));
}

#[test]
fn household_jobs_win_and_plot_claims_are_unique_and_stable() {
    let mut a = dry();
    a.subsurface.wells[0].recovered = 20.;
    a.subsurface.wells[0].supply.as_mut().unwrap().stored = 20.;
    for p in &mut a.people.entries {
        p.remaining_ms = 0;
    }
    let mut b = a.clone();
    b.people.entries.reverse();
    for sim in [&mut a, &mut b] {
        sim.step(0.1, &[toggle(true)]);
    }
    assert_eq!(a.people, b.people);
    assert!(a.people.all().iter().all(|p| p.water_task.is_some()));
    assert!(a.people.irrigator(SourceId(1)).is_none());
    for _ in 0..2500 {
        a.step(0.1, &[]);
        b.step(0.1, &[]);
        valid(&a);
        assert_eq!(a.people, b.people);
        assert!(
            a.people
                .all()
                .iter()
                .filter(|p| p.irrigation_task.is_some())
                .count()
                <= 1
        );
    }
    assert!(soil::total_applied(&a) > 0.);
}

#[test]
fn expedition_application_protects_both_worker_claims() {
    let (mut sim, _) = task_phase(false);
    assert_eq!(sim.people.reserved_irrigation(SourceId(1)), soil::DOSE);
    let stocked = sim.clone();
    let reserved = sim.people.reserved_water(1);
    let supply = sim.subsurface.wells[0].supply.as_mut().unwrap();
    supply.released += supply.stored - reserved;
    supply.stored = reserved;
    assert_eq!(
        soil::validate(&sim, SourceId(1), 1),
        Err(soil::Error::WaterReservedOrEmpty)
    );
    valid(&sim);
    // Independently test the reserved destination capacity with the original stock.
    let mut sim = stocked;
    soil::apply(&mut sim, SourceId(1), 1).unwrap();
    assert_eq!(
        soil::apply(&mut sim, SourceId(1), 1),
        Err(soil::Error::PlotFull)
    );
    valid(&sim);
    for _ in 0..1000 {
        sim.step(0.1, &[]);
        valid(&sim);
    }
    assert!(soil::total_applied(&sim) >= 4.);
}

#[test]
fn stopping_before_pickup_releases_claims_without_withdrawing_water() {
    let (mut sim, id) = task_phase(false);
    sim.step(0.1, &[toggle(false)]);
    assert!(sim.people.get(id).unwrap().irrigation_task.is_none());
    assert_eq!(
        sim.people.get(id).unwrap().irrigation_issue,
        Some(IrrigationIssue::Cancelled)
    );
    assert_eq!(sim.people.reserved_irrigation(SourceId(1)), 0);
    for _ in 0..1000 {
        sim.step(0.1, &[]);
        valid(&sim);
    }
    assert_eq!(soil::total_applied(&sim), 0.);
    let (mut sim, id) = task_phase(false);
    sim.step(
        0.1,
        &[Action::SetWellActive {
            id: 1,
            active: false,
        }],
    );
    assert!(sim.people.get(id).unwrap().irrigation_task.is_none());
    assert_eq!(
        sim.people.get(id).unwrap().irrigation_issue,
        Some(IrrigationIssue::Unavailable)
    );
    valid(&sim);
}

#[test]
fn blocked_carried_water_survives_stopped_tending_and_recovers() {
    let (mut sim, id) = task_phase(true);
    sim.step(
        0.1,
        &[
            toggle(false),
            Action::SetWellActive {
                id: 1,
                active: false,
            },
        ],
    );
    assert_eq!(sim.people.get(id).unwrap().carried_water, 2.);
    let level = sim.sea_level;
    sim.sea_level = 2.;
    for _ in 0..30 {
        sim.step(0.1, &[]);
        valid(&sim);
    }
    let p = sim.people.get(id).unwrap();
    assert_eq!(p.carried_water, 2.);
    assert_eq!(p.irrigation_issue, Some(IrrigationIssue::DeliveryBlocked));
    assert_eq!(sim.people.reserved_irrigation(SourceId(1)), soil::DOSE);
    sim.sea_level = level;
    for _ in 0..1000 {
        sim.step(0.1, &[]);
        valid(&sim);
    }
    assert_eq!(soil::total_applied(&sim), 2.);
    assert_eq!(sim.people.get(id).unwrap().carried_water, 0.);
    assert!(sim.people.irrigator(SourceId(1)).is_none());
}

#[test]
fn humid_plots_do_not_consume_irrigation_and_invalid_toggles_reject() {
    let mut sim = scenarios::cultivated_settlement();
    for c in sim.grid.cells_mut() {
        c.rainfall = 100.;
    }
    sim.step(0.1, &[toggle(true)]);
    assert!(cultivation::moisture(&sim, sim.food.sources()[0].position) >= 0.6);
    for _ in 0..1800 {
        sim.step(0.1, &[]);
        valid(&sim);
    }
    assert_eq!(soil::total_applied(&sim), 0.);
    assert!(
        sim.step(
            0.,
            &[Action::SetCropTending {
                source: SourceId(999),
                enabled: true
            }]
        )
        .iter()
        .any(|e| matches!(e, SimEvent::CropTendingRejected { .. }))
    );
}
