use super::*;
use crate::{
    actions::Action, events::SimEvent, food, runtime::Simulation, save::Snapshot, scenarios,
};
fn check(sim: &SimState) {
    assert!(sim.people.valid(sim), "invalid people at {}", sim.time);
    assert!(sim.food.valid(sim), "food accounting at {}", sim.time);
    assert!(sim.households.valid(sim));
}
#[test]
fn households_get_both_supplies_and_finite_food_eventually_runs_out() {
    let mut sim = scenarios::supplied_settlement();
    sim.food.sources[0].remaining = 48;
    sim.food.sources[0].initial = 48;
    let mut food_homes = std::collections::BTreeSet::new();
    let mut water_homes = std::collections::BTreeSet::new();
    for _ in 0..3600 {
        for event in sim.step(0.1, &[]) {
            match event {
                SimEvent::FoodDelivered { home, .. } => {
                    food_homes.insert(home);
                }
                SimEvent::WaterDelivered { home, .. } => {
                    water_homes.insert(home);
                }
                _ => {}
            }
        }
        check(&sim);
    }
    assert_eq!(food_homes.len(), 6);
    assert_eq!(water_homes.len(), 6);
    assert_eq!(sim.food.sources[0].remaining, 0);
    assert!(
        sim.food
            .homes
            .values()
            .all(|h| h.consumed > 0 && h.missed_portions > 0)
    );
    assert!(sim.households.all().values().all(|h| h.consumed > 0.));
    assert!(sim.people.all().iter().all(|p| p.carried_food == 0));
}
#[test]
fn scarce_food_claims_are_stable_and_water_failure_does_not_starve_food() {
    let mut a = scenarios::supplied_settlement();
    a.subsurface.wells[0].active = false;
    a.food.sources[0].remaining = food::LOAD;
    a.food.sources[0].initial = food::LOAD;
    for p in &mut a.people.entries {
        p.remaining_ms = 0;
    }
    let mut b = a.clone();
    b.people.entries.reverse();
    b.buildings.reverse();
    a.step(0.1, &[]);
    b.step(0.1, &[]);
    assert_eq!(a.people, b.people);
    assert_eq!(a.food, b.food);
    assert_eq!(a.people.reserved_food(a.food.sources[0].id), food::LOAD);
    assert!(a.people.entries[0].food_task.is_some());
    assert!(a.people.entries[0].water_task.is_none());
    check(&a);
    let id = a.people.entries[0].id;
    let before = a.food.clone();
    let events = a.step(
        0.,
        &[
            Action::CancelFoodTask { person: id },
            Action::CancelFoodTask { person: id },
        ],
    );
    assert_eq!(before, a.food);
    assert_eq!(a.people.reserved_food(a.food.sources[0].id), 0);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, SimEvent::FoodTaskCancelled { .. }))
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, SimEvent::FoodTaskCancelRejected { .. }))
    );
    assert!(!events.iter().any(|e| matches!(
        e,
        SimEvent::FoodCollected { .. } | SimEvent::FoodDelivered { .. }
    )));
}
#[test]
fn gathering_save_resumes_at_same_commit_and_blocked_cargo_is_retained() {
    let mut sim = scenarios::supplied_settlement();
    sim.subsurface.wells[0].active = false;
    let mut run = Simulation::new(sim);
    for _ in 0..600 {
        run.advance(&[]);
        if run
            .people
            .all()
            .iter()
            .any(|p| p.activity == Activity::GatheringFood)
        {
            break;
        }
    }
    assert!(
        run.people
            .all()
            .iter()
            .any(|p| p.activity == Activity::GatheringFood)
    );
    let mut restored = Snapshot::capture(&run, "gathering", &[])
        .unwrap()
        .restore()
        .unwrap()
        .simulation;
    for _ in 0..500 {
        run.advance(&[]);
        restored.advance(&[]);
    }
    assert_eq!(run.people, restored.people);
    assert_eq!(run.food, restored.food);

    let mut sim = scenarios::supplied_settlement();
    sim.subsurface.wells[0].active = false;
    for _ in 0..600 {
        sim.step(0.1, &[]);
        if sim.people.all().iter().any(|p| p.carried_food > 0) {
            break;
        }
    }
    let id = sim
        .people
        .all()
        .iter()
        .find(|p| p.carried_food > 0)
        .unwrap()
        .id;
    let home = sim.people.get(id).unwrap().home;
    for cell in sim.grid.cells_mut() {
        cell.elevation = -1.;
    }
    sim.step(0.1, &[Action::CancelFoodTask { person: id }]);
    assert_eq!(sim.people.get(id).unwrap().carried_food, food::LOAD);
    assert_eq!(
        sim.people.get(id).unwrap().food_issue,
        Some(FoodIssue::DeliveryBlocked)
    );
    assert!(
        sim.people
            .all()
            .iter()
            .filter(|p| p.carried_food == 0)
            .all(|p| p.food_task.is_none())
    );
    check(&sim);
    for cell in sim.grid.cells_mut() {
        cell.elevation = 1.;
    }
    for _ in 0..600 {
        sim.step(0.1, &[]);
    }
    let stock = sim.food.home(home).unwrap();
    assert!(u64::from(stock.stored) + stock.consumed >= u64::from(food::LOAD));
    check(&sim);
}
#[test]
fn tied_reserves_alternate_and_active_tasks_never_switch_resources() {
    let mut sim = scenarios::supplied_settlement();
    sim.people.entries.truncate(1);
    let p = &mut sim.people.entries[0];
    p.remaining_ms = 0;
    let w = &mut sim.subsurface.wells[0];
    w.recovered = 20.;
    w.supply.as_mut().unwrap().stored = 20.;
    let mut other = sim.clone();
    other.people.entries[0].last_supply = SupplyKind::Water;
    sim.step(0.1, &[]);
    other.step(0.1, &[]);
    assert!(sim.people.entries[0].water_task.is_some());
    assert!(other.people.entries[0].food_task.is_some());
    other.people.entries[0].last_supply = SupplyKind::Food;
    other.step(0.1, &[]);
    assert!(other.people.entries[0].food_task.is_some());
    assert!(other.people.entries[0].water_task.is_none());
    check(&sim);
    check(&other);
}
