use super::*;
use genesis_sim_core::{actions::Action, scenarios};

#[test]
fn overview_counts_physical_stock_once_and_keeps_committed_state_unchanged() {
    let mut sim = scenarios::water_settlement();
    for _ in 0..600 {
        sim.step(0.1, &[]);
        if sim.people.all().iter().any(|p| p.carried_water > 0.) {
            break;
        }
    }
    assert!(sim.people.all().iter().any(|p| p.carried_water > 0.));
    let before = serde_json::to_vec(&sim).unwrap();
    let view = Overview::read(&sim);
    assert_eq!(view.homes.len(), 6);
    assert!(view.carried > 0.);
    assert_eq!(view.reserved, sim.people.reserved_water(1));
    let supply = sim.subsurface.wells[0].supply.as_ref().unwrap();
    let consumed: f64 = sim.households.all().values().map(|h| h.consumed).sum();
    assert!((view.stored + view.carried + consumed - supply.collected).abs() < 1e-8);
    assert_eq!(view.buffered, supply.stored);
    assert_eq!(serde_json::to_vec(&sim).unwrap(), before);
    assert!(
        view.homes
            .iter()
            .any(|h| h.label().contains("carrying water home"))
    );
    sim.buildings.reverse();
    assert_eq!(
        Overview::read(&sim)
            .homes
            .iter()
            .map(|h| h.id)
            .collect::<Vec<_>>(),
        vec![1, 2, 3, 4, 5, 6]
    );
    let empty = scenarios::coastal_outpost();
    let view = Overview::read(&empty);
    assert!(view.homes.is_empty());
    assert_eq!(
        view.carried + view.stored + view.reserved + view.buffered,
        0.
    );
}

#[test]
fn household_rows_distinguish_collection_from_last_failed_attempt() {
    let mut sim = scenarios::water_settlement();
    for _ in 0..20 {
        sim.step(0.1, &[]);
    }
    assert!(
        Overview::read(&sim).homes[0]
            .label()
            .contains("collecting water")
    );
    sim.step(
        0.1,
        &[Action::SetWellActive {
            id: 1,
            active: false,
        }],
    );
    let label = Overview::read(&sim).homes[0].label();
    assert!(
        label.contains("last attempt: Collection cancelled: well stopped or submerged"),
        "{label}"
    );
    assert!(!label.contains("reserved"));

    let mut no_water = scenarios::water_settlement();
    for c in &mut no_water.subsurface.columns {
        c.permeability = 0.;
    }
    for _ in 0..20 {
        no_water.step(0.1, &[]);
    }
    assert!(
        Overview::read(&no_water).homes[0]
            .label()
            .contains("last attempt: Waiting for enough unreserved well water")
    );

    // Simulate a recorded route failure without invoking planning from the view.
    let mut value = serde_json::to_value(&no_water.people).unwrap();
    value["entries"][0]["water_issue"] = serde_json::json!("RouteBlocked");
    no_water.people = serde_json::from_value(value).unwrap();
    assert!(
        Overview::read(&no_water).homes[0]
            .label()
            .contains("last attempt: Collection released: route blocked")
    );
}
