use super::*;
use genesis_sim_core::{SimState, runtime::Simulation, scenarios, supply_outlook::Outlook};

#[test]
fn demand_follows_actual_occupancy_and_never_pools_other_homes_stock() {
    // Query-only fixture: share a home and give it partial stock. No domain step
    // is run with this synthetic distribution; conservation is tested elsewhere.
    let mut value = serde_json::to_value(scenarios::living_settlement()).unwrap();
    value["people"]["entries"][1]["home"] = 1.into();
    value["food"]["homes"]["1"]["stored"] = 1.into();
    value["households"]["homes"]["1"]["stored"] = 0.75.into();
    value["food"]["homes"]["3"]["stored"] = 12.into();
    value["households"]["homes"]["3"]["stored"] = 8.into();
    let mut sim: SimState = serde_json::from_value(value.clone()).unwrap();
    let a = Outlook::read(&sim);
    let home = &a.homes[0];
    assert_eq!(home.residents, 2);
    assert_eq!(home.food_shortfall(), 1);
    assert_eq!(home.water_shortfall(), 0.25);
    assert_eq!(a.homes.len(), 5);
    assert!(!a.homes.iter().any(|h| h.id == 2));
    assert_eq!(a.short_homes(), 4);
    sim.buildings.reverse();
    value["people"]["entries"].as_array_mut().unwrap().reverse();
    sim.people = serde_json::from_value(value["people"].clone()).unwrap();
    assert_eq!(a, Outlook::read(&sim));
}

#[test]
fn restored_outlook_matches_across_consumption_and_delivery_boundaries() {
    let mut run = Simulation::new(scenarios::supplied_settlement());
    for _ in 0..299 {
        run.advance(&[]);
    }
    let snapshot = Snapshot::capture(&run, "outlook", &[]).unwrap();
    let mut restored = decode(&encode(&snapshot).unwrap()).unwrap();
    for _ in 0..400 {
        assert_eq!(Outlook::read(&run), Outlook::read(&restored.simulation));
        run.advance(&[]);
        restored.simulation.advance(&[]);
    }
}
