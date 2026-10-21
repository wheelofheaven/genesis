use super::*;
use crate::{actions::Action, events::SimEvent, people::PersonId, scenarios};

#[test]
fn empty_homes_wait_and_retry_on_exact_logical_boundaries_without_event_spam() {
    let mut sim = scenarios::living_settlement();
    for p in &mut sim.people.entries {
        p.remaining_ms = 0;
    }
    let start: Vec<_> = sim.people.entries.iter().map(|p| p.position).collect();
    let events = sim.step(0.1, &[]);
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(
                e,
                SimEvent::PersonRoutineChanged {
                    activity: Activity::WaitingForSupplies,
                    ..
                }
            ))
            .count(),
        6
    );
    for remaining in (0..20).rev() {
        assert!(
            sim.people
                .entries
                .iter()
                .all(|p| p.supply_retry_ms() == Some((remaining + 1) * 100))
        );
        let events = sim.step(0.1, &[]);
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, SimEvent::PersonRoutineChanged { .. }))
        );
    }
    assert!(
        sim.people
            .entries
            .iter()
            .all(|p| p.supply_retry_ms() == Some(2000))
    );
    for _ in 0..600 {
        sim.step(0.1, &[]);
    }
    for (p, position) in sim.people.entries.iter().zip(start) {
        assert_eq!(p.position, position);
        assert_eq!(p.activity, Activity::WaitingForSupplies);
    }
    assert!(sim.people.valid(&sim));
    assert!(sim.food.homes().values().all(|h| h.missed_portions > 0));
}

#[test]
fn resumed_well_is_reconsidered_at_retry_and_all_homes_receive_deliveries() {
    let mut sim = scenarios::water_settlement();
    for _ in 0..200 {
        sim.subsurface.step(&sim.grid, sim.sea_level, 0.1);
    }
    sim.subsurface.wells[0].active = false;
    for p in &mut sim.people.entries {
        p.remaining_ms = 0;
    }
    sim.step(0.1, &[]);
    sim.step(
        0.,
        &[Action::SetWellActive {
            id: 1,
            active: true,
        }],
    );
    for _ in 0..19 {
        sim.step(0.1, &[]);
        assert!(sim.people.entries.iter().all(|p| p.water_task.is_none()));
    }
    sim.step(0.1, &[]);
    assert!(sim.people.entries.iter().all(|p| p.water_task.is_some()));
    for _ in 0..1200 {
        sim.step(0.1, &[]);
        assert!(sim.people.valid(&sim));
        assert!(sim.households.valid(&sim));
    }
    assert!(sim.households.all().values().all(|h| h.consumed > 0.));
}

#[test]
fn harvest_readiness_ends_waiting_and_repacking_preserves_claim_order() {
    let mut sim = scenarios::supplied_settlement();
    sim.subsurface.wells[0].active = false;
    // Keep the original harvest standing but reserved by nobody; temporarily
    // remove availability to exercise a newly replenished source at a retry.
    let harvest = sim.food.sources[0].initial;
    sim.food.sources[0].initial = 0;
    sim.food.sources[0].remaining = 0;
    for p in &mut sim.people.entries {
        p.remaining_ms = 0;
    }
    sim.step(0.1, &[]);
    assert!(
        sim.people
            .entries
            .iter()
            .all(|p| p.activity == Activity::WaitingForSupplies)
    );
    sim.food.sources[0].initial = harvest;
    sim.food.sources[0].remaining = harvest;
    let mut other = sim.clone();
    other.people.entries.reverse();
    other.buildings.reverse();
    for _ in 0..600 {
        sim.step(0.1, &[]);
        other.step(0.1, &[]);
        assert_eq!(sim.people, other.people);
        assert_eq!(sim.food, other.food);
        assert!(sim.people.valid(&sim));
        assert!(sim.food.valid(&sim));
    }
    assert!(sim.food.homes().values().all(|h| h.consumed > 0));
}

#[test]
fn covered_homes_keep_social_visits_but_shared_home_demand_is_not_invented() {
    let mut sim = scenarios::supplied_settlement();
    sim.subsurface.wells[0].active = false;
    sim.people.entries.truncate(2);
    let home = sim.people.entries[0].home;
    let position = sim.people.entries[0].home_position;
    for p in &mut sim.people.entries {
        p.home = home;
        p.position = position;
        p.home_position = position;
        p.remaining_ms = 0;
    }
    // Two residents, one meal: enough for neither the household nor leisure.
    sim.food.sources[0].initial = 1;
    sim.food.sources[0].remaining = 0;
    assert!(sim.food.deliver(home, 1));
    assert!(sim.households.deliver(home, 1.));
    sim.subsurface.wells[0].recovered = 1.;
    sim.subsurface.wells[0].supply.as_mut().unwrap().collected = 1.;
    sim.step(0.1, &[]);
    assert!(
        sim.people
            .entries
            .iter()
            .all(|p| p.activity == Activity::WaitingForSupplies)
    );
    sim.food.sources[0].initial += 1;
    assert!(sim.food.deliver(home, 1));
    for _ in 0..20 {
        sim.step(0.1, &[]);
    }
    assert!(
        sim.people
            .entries
            .iter()
            .all(|p| p.activity == Activity::GoingToMeeting)
    );
    assert!(
        sim.people
            .get(PersonId(1))
            .unwrap()
            .supply_retry_ms()
            .is_none()
    );
    assert!(sim.people.valid(&sim));
    assert!(sim.food.valid(&sim));
    assert!(sim.households.valid(&sim));
}

#[test]
fn growing_crops_become_collectable_during_retries_without_interrupting_water_jobs() {
    let mut sim = scenarios::cultivated_settlement();
    sim.step(
        0.,
        &[Action::SetWellActive {
            id: 1,
            active: false,
        }],
    );
    let mut waiting = std::collections::BTreeSet::new();
    let mut supplied = std::collections::BTreeSet::new();
    for tick in 0..1800 {
        if tick == 100 {
            sim.step(
                0.,
                &[Action::SetWellActive {
                    id: 1,
                    active: true,
                }],
            );
        }
        for p in sim.people.all() {
            if p.activity == Activity::WaitingForSupplies {
                waiting.insert(p.id);
            }
        }
        for event in sim.step(0.1, &[]) {
            if let SimEvent::FoodDelivered { person, .. } = event {
                supplied.insert(person);
            }
        }
        assert!(sim.people.valid(&sim));
        assert!(sim.food.valid(&sim));
        assert!(sim.households.valid(&sim));
    }
    assert!(!waiting.is_empty());
    assert_eq!(supplied.len(), 6);
    assert!(waiting.is_subset(&supplied));
    assert!(sim.households.all().values().all(|h| h.consumed > 0.));
}
