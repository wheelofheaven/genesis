use super::*;
use genesis_sim_core::{actions::Action, runtime::Simulation, scenarios, subsurface::PumpMode};

#[test]
fn v9_keeps_continuous_pumps_and_all_tending_commitments() {
    let bytes = include_bytes!("../fixtures/v9-tending.json");
    let old: serde_json::Value = serde_json::from_slice(bytes).unwrap();
    let restored = decode(bytes).unwrap();
    let expected: genesis_sim_core::SimState =
        serde_json::from_value(old["state"].clone()).unwrap();
    assert_eq!(
        serde_json::to_value(&*restored.simulation).unwrap(),
        serde_json::to_value(expected).unwrap()
    );
    assert_eq!(restored.simulation.completed_ticks(), 500);
    assert_eq!(
        restored.simulation.subsurface.wells[0].pump_mode,
        PumpMode::Continuous
    );
    let mut bad = old.clone();
    bad["state"]["subsurface"]["wells"][0]["pump_mode"] = "FillBuffer".into();
    assert!(decode(&serde_json::to_vec(&bad).unwrap()).is_err());
    let mut bad = old;
    bad["pending"] = serde_json::json!([{"SetWellPumpMode":{"id":1,"mode":"Continuous"}}]);
    assert!(decode(&serde_json::to_vec(&bad).unwrap()).is_err());
}

#[test]
fn pump_modes_round_trip_at_full_and_working_buffers_with_pending_changes() {
    let mut state = scenarios::water_settlement();
    state.step(
        0.,
        &[Action::SetWellPumpMode {
            id: 1,
            mode: PumpMode::FillBuffer,
        }],
    );
    // Start from a legitimately filled buffer before the residents' first departure.
    for _ in 0..200 {
        state.subsurface.step(&state.grid, state.sea_level, 0.1);
    }
    assert_eq!(state.subsurface.wells[0].last_yield, 0.);
    assert!(state.subsurface.wells[0].supply.as_ref().unwrap().stored > 19.999);
    let mut run = Simulation::new(state);
    for ticks in [0, 100, 900] {
        for _ in 0..ticks {
            run.advance(&[]);
        }
        for mode in [PumpMode::Continuous, PumpMode::FillBuffer] {
            let pending = [Action::SetWellPumpMode { id: 1, mode }];
            let snapshot = Snapshot::capture(&run, "pump-policy", &pending).unwrap();
            let mut loaded = decode(&encode(&snapshot).unwrap()).unwrap();
            let mut direct = snapshot.clone().restore().unwrap();
            for s in [&mut loaded, &mut direct] {
                s.simulation.advance(&s.pending);
                for _ in 0..500 {
                    s.simulation.advance(&[]);
                }
                Snapshot::capture(&s.simulation, "pump-policy", &[]).unwrap();
            }
            assert_eq!(
                serde_json::to_vec(&*loaded.simulation).unwrap(),
                serde_json::to_vec(&*direct.simulation).unwrap()
            );
            assert_eq!(loaded.simulation.subsurface.wells[0].pump_mode, mode);
            let mut bad = serde_json::to_value(snapshot).unwrap();
            bad["state"]["subsurface"]["wells"][0]["pump_mode"] = "unknown".into();
            assert!(decode(&serde_json::to_vec(&bad).unwrap()).is_err());
        }
    }
}
