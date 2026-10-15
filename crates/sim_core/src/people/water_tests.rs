use super::*;
use crate::{
    actions::Action, events::SimEvent, households, runtime::Simulation, save::Snapshot, scenarios,
};
fn total(sim: &SimState) -> f64 {
    sim.subsurface
        .columns
        .iter()
        .map(|c| f64::from(c.stored))
        .sum::<f64>()
        + sim
            .subsurface
            .wells
            .iter()
            .filter_map(|w| w.supply.as_ref())
            .map(|s| s.stored + s.released)
            .sum::<f64>()
        + sim
            .households
            .all()
            .values()
            .map(|h| h.stored + h.consumed)
            .sum::<f64>()
        + sim
            .people
            .all()
            .iter()
            .map(|p| p.carried_water)
            .sum::<f64>()
}
fn scarce() -> SimState {
    let mut s = scenarios::water_settlement();
    assert_eq!(s.subsurface.wells.len(), 1);
    for c in &mut s.subsurface.columns {
        c.recharge = 0.;
        c.permeability = 0.;
    }
    let w = &mut s.subsurface.wells[0];
    w.recovered = 2.;
    w.supply.as_mut().unwrap().stored = 2.;
    for p in &mut s.people.entries {
        p.remaining_ms = 0;
    }
    s
}
#[test]
fn water_moves_between_single_owners_and_feeds_households() {
    let mut s = scenarios::water_settlement();
    for c in &mut s.subsurface.columns {
        c.recharge = 0.;
    }
    let before = total(&s);
    let mut delivered = std::collections::BTreeSet::new();
    for _ in 0..1800 {
        for e in s.step(0.1, &[]) {
            if let SimEvent::WaterDelivered { home, .. } = e {
                delivered.insert(home);
            }
        }
        assert!(s.people.valid(&s));
        assert!(s.households.valid(&s));
        assert!(
            s.subsurface
                .wells
                .iter()
                .all(|w| w.supply.as_ref().unwrap().valid(w.recovered))
        );
        assert!((total(&s) - before).abs() < 0.01, "{}", total(&s) - before);
    }
    assert_eq!(delivered.len(), 6);
    assert!(s.households.all().values().all(|h| h.consumed > 0.));
    assert!(s.subsurface.wells[0].supply.as_ref().unwrap().released > 0.);
}
#[test]
fn scarce_stock_and_shared_homes_have_one_stable_claim() {
    let mut a = scarce();
    let mut b = a.clone();
    b.people.entries.reverse();
    b.buildings.reverse();
    a.step(0.1, &[]);
    b.step(0.1, &[]);
    assert_eq!(a.people, b.people);
    assert_eq!(a.people.reserved_water(1), 2.);
    assert!(a.people.get(PersonId(1)).unwrap().water_task.is_some());
    assert_eq!(
        a.people
            .all()
            .iter()
            .filter(|p| p.water_task.is_some())
            .count(),
        1
    );
    let mut s = scarce();
    s.subsurface.wells[0].recovered = 20.;
    s.subsurface.wells[0].supply.as_mut().unwrap().stored = 20.;
    let home = s.people.entries[0].home;
    for p in &mut s.people.entries {
        p.home = home;
    }
    s.step(0.1, &[]);
    assert_eq!(
        s.people
            .all()
            .iter()
            .filter(|p| p.water_task.is_some())
            .count(),
        1
    );
}
#[test]
fn cancellation_and_stopped_wells_release_uncollected_water() {
    for cancel in [true, false] {
        let mut s = scarce();
        s.people.entries.truncate(1);
        s.step(0.1, &[]);
        assert_eq!(s.people.reserved_water(1), 2.);
        let action = if cancel {
            Action::CancelWaterTask {
                person: PersonId(1),
            }
        } else {
            Action::SetWellActive {
                id: 1,
                active: false,
            }
        };
        let events = s.step(0.1, &[action]);
        assert_eq!(s.people.reserved_water(1), 0.);
        assert_eq!(s.people.all()[0].carried_water, 0.);
        assert_eq!(s.subsurface.wells[0].supply.as_ref().unwrap().stored, 2.);
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, SimEvent::WaterCollected { .. }))
        );
        assert_eq!(
            s.people.all()[0].water_issue,
            Some(if cancel {
                WaterIssue::Cancelled
            } else {
                WaterIssue::WellStopped
            })
        );
    }
    let mut s = scarce();
    let before = s.people.clone();
    let events = s.step(
        0.,
        &[Action::CancelWaterTask {
            person: PersonId(999),
        }],
    );
    assert_eq!(s.people, before);
    assert!(matches!(
        events[0],
        SimEvent::WaterTaskCancelRejected { .. }
    ));
}
#[test]
fn terrain_block_releases_fetch_claim_but_retains_carried_water() {
    let mut s = scarce();
    s.people.entries.truncate(1);
    s.step(0.1, &[]);
    let mut rock = crate::worldgen::Cover {
        position: s.people.entries[0].position,
        size: 1.,
        kind: crate::ecology::CoverKind::Limestone,
        seed: 1,
    };
    s.geography.cover.push(rock.clone());
    s.step(0.1, &[]);
    assert_eq!(s.people.reserved_water(1), 0.);
    assert_eq!(
        s.people.entries[0].water_issue,
        Some(WaterIssue::RouteBlocked)
    );
    s.geography.cover.clear();
    for _ in 0..1000 {
        s.step(0.1, &[]);
        if s.people.entries[0].carried_water > 0. {
            break;
        }
    }
    assert_eq!(s.people.entries[0].carried_water, households::CONTAINER);
    rock.position = s.people.entries[0].position;
    s.geography.cover.push(rock);
    let before = total(&s);
    for _ in 0..30 {
        s.step(0.1, &[]);
    }
    assert_eq!(s.people.entries[0].carried_water, households::CONTAINER);
    assert!(s.people.home_collector(s.people.entries[0].home).is_some());
    assert_eq!(
        s.people.entries[0].water_issue,
        Some(WaterIssue::DeliveryBlocked)
    );
    assert!((before - total(&s)).abs() < 1e-7);
}
#[test]
fn carried_water_survives_save_cancel_and_delivery_with_pump_stopped() {
    let mut state = scarce();
    state.people.entries.truncate(1);
    let mut a = Simulation::new(state);
    for _ in 0..300 {
        a.advance(&[]);
        if a.people.all()[0].carried_water > 0. {
            break;
        }
    }
    assert_eq!(a.people.all()[0].carried_water, 2.);
    let pending = [
        Action::SetWellActive {
            id: 1,
            active: false,
        },
        Action::CancelWaterTask {
            person: PersonId(1),
        },
    ];
    let saved = Snapshot::capture(&a, "water", &pending).unwrap();
    let mut loaded = saved.restore().unwrap();
    a.advance(&pending);
    loaded.simulation.advance(&loaded.pending);
    for _ in 0..300 {
        a.advance(&[]);
        loaded.simulation.advance(&[]);
    }
    assert_eq!(a.people, loaded.simulation.people);
    assert_eq!(a.households, loaded.simulation.households);
    let h = a.households.get(a.people.all()[0].home).unwrap();
    assert_eq!(h.stored + h.consumed, 2.);
    assert_eq!(a.people.all()[0].carried_water, 0.);
}
