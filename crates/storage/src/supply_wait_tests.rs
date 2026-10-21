use super::*;
use genesis_sim_core::{actions::Action, people::Activity, runtime::Simulation, scenarios};

#[test]
fn v10_upgrade_preserves_all_materialized_state_and_pending_intent() {
    let bytes = include_bytes!("../fixtures/v10-conserving.json");
    let mut old: serde_json::Value = serde_json::from_slice(bytes).unwrap();
    old["pending"] = serde_json::json!([{"SetWellActive": {"id":1,"active":false}}]);
    let loaded = decode(&serde_json::to_vec(&old).unwrap()).unwrap();
    let expected: genesis_sim_core::SimState =
        serde_json::from_value(old["state"].clone()).unwrap();
    assert_eq!(
        serde_json::to_value(&*loaded.simulation).unwrap(),
        serde_json::to_value(expected).unwrap()
    );
    assert_eq!(loaded.simulation.completed_ticks(), 500);
    assert_eq!(loaded.world_id, old["world_id"].as_str().unwrap());
    assert!(matches!(
        loaded.pending.as_slice(),
        [Action::SetWellActive {
            id: 1,
            active: false
        }]
    ));
    let snapshot =
        Snapshot::capture(&loaded.simulation, &loaded.world_id, &loaded.pending).unwrap();
    assert_eq!(snapshot.schema, 11);
    assert_eq!(snapshot.rules, "genesis:rules/9");
    old["rules"] = "genesis:rules/9".into();
    assert!(decode(&serde_json::to_vec(&old).unwrap()).is_err());
}

#[test]
fn waiting_saves_preserve_retry_phase_and_reject_invalid_or_legacy_waiting() {
    let mut run = Simulation::new(scenarios::water_settlement());
    run.advance(&[Action::SetWellActive {
        id: 1,
        active: false,
    }]);
    for _ in 1..15 {
        run.advance(&[]);
    }
    assert_eq!(run.people.all()[0].activity, Activity::WaitingForSupplies);
    let pending = [Action::SetWellActive {
        id: 1,
        active: true,
    }];
    let snapshot = Snapshot::capture(&run, "supply-wait", &pending).unwrap();
    let mut loaded = decode(&encode(&snapshot).unwrap()).unwrap();
    assert_eq!(
        run.people.all()[0].supply_retry_ms(),
        loaded.simulation.people.all()[0].supply_retry_ms()
    );
    run.advance(&pending);
    loaded.simulation.advance(&loaded.pending);
    for _ in 0..700 {
        assert_eq!(
            serde_json::to_vec(&*run).unwrap(),
            serde_json::to_vec(&*loaded.simulation).unwrap()
        );
        run.advance(&[]);
        loaded.simulation.advance(&[]);
    }
    let good = serde_json::to_value(snapshot).unwrap();
    for (pointer, value) in [
        (
            "/state/people/entries/0/remaining_ms",
            serde_json::json!(2001),
        ),
        ("/state/people/entries/0/position/x", serde_json::json!(3.)),
        ("/state/people/entries/0/carried_food", serde_json::json!(1)),
        (
            "/state/people/entries/0/route",
            serde_json::json!([{"x":3.,"z":3.}]),
        ),
    ] {
        let mut bad = good.clone();
        *bad.pointer_mut(pointer).unwrap() = value;
        assert!(
            decode(&serde_json::to_vec(&bad).unwrap()).is_err(),
            "{pointer}"
        );
    }
    let mut legacy = good;
    legacy["schema"] = 10.into();
    legacy["rules"] = "genesis:rules/8".into();
    assert!(decode(&serde_json::to_vec(&legacy).unwrap()).is_err());
}
