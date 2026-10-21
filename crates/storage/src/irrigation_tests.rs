use super::*;
use genesis_sim_core::{
    actions::Action,
    food::{SourceId, cultivation::irrigation},
    runtime::Simulation,
    scenarios,
};

#[test]
fn v6_crops_migrate_without_changing_growth_stock_or_resident_commitments() {
    let bytes = include_bytes!("../fixtures/v6-crops.json");
    let old: serde_json::Value = serde_json::from_slice(bytes).unwrap();
    let restored = decode(bytes).unwrap();
    let mut expected = old["state"].clone();
    expected["food"]["sources"][0]["kind"]["Barley"]["soil_water"] =
        serde_json::json!({"stored":0,"applied":0,"lost":0});
    let expected: genesis_sim_core::SimState = serde_json::from_value(expected).unwrap();
    assert_eq!(
        serde_json::to_value(&*restored.simulation).unwrap(),
        serde_json::to_value(expected).unwrap()
    );
    assert_eq!(restored.simulation.completed_ticks(), 100);
    assert_eq!(irrigation::total_applied(&restored.simulation), 0.);
    let mut corrupt = old;
    corrupt["state"]["food"]["sources"][0]["kind"]["Barley"]["soil_water"] =
        serde_json::json!({"stored":0,"applied":0,"lost":0});
    assert!(decode(&serde_json::to_vec(&corrupt).unwrap()).is_err());
}

#[test]
fn irrigation_pending_and_active_storage_continue_exactly_and_reject_corruption() {
    let mut state = scenarios::cultivated_settlement();
    for c in state.grid.cells_mut() {
        c.rainfall = 0.01;
    }
    let mut run = Simulation::new(state);
    for _ in 0..200 {
        run.advance(&[]);
    }
    let command = Action::IrrigateCrop {
        source: SourceId(1),
        well: 1,
    };
    for phase in [0, 1, 400, 800] {
        if phase == 1 {
            run.advance(std::slice::from_ref(&command));
        } else {
            for _ in 0..phase {
                run.advance(&[]);
            }
        }
        let snapshot =
            Snapshot::capture(&run, "irrigation", std::slice::from_ref(&command)).unwrap();
        let mut loaded = decode(&encode(&snapshot).unwrap()).unwrap();
        let mut direct = snapshot.clone().restore().unwrap();
        for s in [&mut loaded, &mut direct] {
            s.simulation.advance(&s.pending);
            for _ in 0..1500 {
                s.simulation.advance(&[]);
            }
            Snapshot::capture(&s.simulation, "irrigation", &[]).unwrap();
        }
        assert_eq!(
            serde_json::to_vec(&*loaded.simulation).unwrap(),
            serde_json::to_vec(&*direct.simulation).unwrap()
        );
        let good = serde_json::to_value(snapshot).unwrap();
        for value in [
            serde_json::Value::Null,
            serde_json::json!({"stored":4_000_050,"applied":6_000_000,"lost":1_999_950}),
            serde_json::json!({"stored":1,"applied":2_000_000,"lost":1_999_999}),
            serde_json::json!({"stored":0,"applied":2_000_000,"lost":0}),
            serde_json::json!({"stored":0,"applied":u64::MAX,"lost":u64::MAX}),
        ] {
            let mut bad = good.clone();
            bad["state"]["food"]["sources"][0]["kind"]["Barley"]["soil_water"] = value;
            assert!(decode(&serde_json::to_vec(&bad).unwrap()).is_err());
        }
        let mut bad = good.clone();
        bad["state"]["food"]["sources"][0]["kind"]["Barley"]
            .as_object_mut()
            .unwrap()
            .remove("soil_water");
        assert!(decode(&serde_json::to_vec(&bad).unwrap()).is_err());
        // Internally conserved plot water must also come from real well withdrawals.
        let mut bad = good;
        let water = &mut bad["state"]["food"]["sources"][0]["kind"]["Barley"]["soil_water"];
        water["applied"] = (water["applied"].as_u64().unwrap() + irrigation::DOSE).into();
        water["lost"] = (water["lost"].as_u64().unwrap() + irrigation::DOSE).into();
        assert!(decode(&serde_json::to_vec(&bad).unwrap()).is_err());
    }
}

#[test]
fn v8_defaults_to_manual_tending_and_rejects_future_commitments() {
    let bytes = include_bytes!("../fixtures/v8-irrigation.json");
    let old: serde_json::Value = serde_json::from_slice(bytes).unwrap();
    let restored = decode(bytes).unwrap();
    let expected: genesis_sim_core::SimState =
        serde_json::from_value(old["state"].clone()).unwrap();
    assert_eq!(
        serde_json::to_value(&*restored.simulation).unwrap(),
        serde_json::to_value(expected).unwrap()
    );
    assert!(
        restored
            .simulation
            .people
            .all()
            .iter()
            .all(|p| p.irrigation_task.is_none())
    );
    assert_eq!(restored.simulation.completed_ticks(), 500);
    assert_eq!(irrigation::total_applied(&restored.simulation), 4.);
    let mut bad = old.clone();
    bad["state"]["food"]["sources"][0]["kind"]["Barley"]["tending"] = true.into();
    assert!(decode(&serde_json::to_vec(&bad).unwrap()).is_err());
    let mut bad = old;
    bad["pending"] = serde_json::json!([{"SetCropTending":{"source":1,"enabled":true}}]);
    assert!(decode(&serde_json::to_vec(&bad).unwrap()).is_err());
}

#[test]
fn resident_watering_saves_preserve_claims_cargo_and_pending_stop_exactly() {
    use genesis_sim_core::people::Activity;
    let mut state = scenarios::cultivated_settlement();
    for c in state.grid.cells_mut() {
        c.rainfall = 0.01;
    }
    let mut run = Simulation::new(state);
    run.advance(&[Action::SetCropTending {
        source: SourceId(1),
        enabled: true,
    }]);
    for phase in [Activity::FetchingIrrigationWater, Activity::WateringCrop] {
        for _ in 0..2000 {
            if run.people.all().iter().any(|p| p.activity == phase) {
                break;
            }
            run.advance(&[]);
        }
        assert!(run.people.all().iter().any(|p| p.activity == phase));
        let snapshot = Snapshot::capture(
            &run,
            "resident-watering",
            &[Action::SetCropTending {
                source: SourceId(1),
                enabled: false,
            }],
        )
        .unwrap();
        let encoded = encode(&snapshot).unwrap();
        let mut loaded = decode(&encoded).unwrap();
        let mut direct = snapshot.clone().restore().unwrap();
        for s in [&mut loaded, &mut direct] {
            s.simulation.advance(&s.pending);
            for _ in 0..1000 {
                s.simulation.advance(&[]);
            }
            Snapshot::capture(&s.simulation, "resident-watering", &[]).unwrap();
        }
        assert_eq!(
            serde_json::to_vec(&*loaded.simulation).unwrap(),
            serde_json::to_vec(&*direct.simulation).unwrap()
        );
        assert_eq!(
            irrigation::total_applied(&loaded.simulation),
            if phase == Activity::WateringCrop {
                2.
            } else {
                0.
            }
        );
        let good = serde_json::to_value(&snapshot).unwrap();
        let index = run
            .people
            .all()
            .iter()
            .position(|p| p.activity == phase)
            .unwrap();
        for (field, value) in [
            ("source", serde_json::json!(999)),
            ("well", serde_json::json!(999)),
            ("pickup", serde_json::json!({"x":-1,"z":0})),
            ("delivery", serde_json::json!({"x":0,"z":0})),
        ] {
            let mut bad = good.clone();
            bad["state"]["people"]["entries"][index]["irrigation_task"][field] = value;
            assert!(
                decode(&serde_json::to_vec(&bad).unwrap()).is_err(),
                "{field}"
            );
        }
        let mut bad = good.clone();
        bad["state"]["people"]["entries"][index]["irrigation_task"] = serde_json::Value::Null;
        assert!(decode(&serde_json::to_vec(&bad).unwrap()).is_err());
        let mut bad = good.clone();
        bad["state"]["people"]["entries"][index]["carried_water"] = 1.into();
        assert!(decode(&serde_json::to_vec(&bad).unwrap()).is_err());
        // Duplicate capacity ownership is invalid even if total water balances.
        let mut bad = good.clone();
        let other = (index + 1) % run.people.all().len();
        let task = bad["state"]["people"]["entries"][index]["irrigation_task"].clone();
        bad["state"]["people"]["entries"][other]["irrigation_task"] = task;
        assert!(decode(&serde_json::to_vec(&bad).unwrap()).is_err());
        let mut bad = good;
        bad["schema"] = 8.into();
        bad["rules"] = "genesis:rules/6".into();
        assert!(decode(&serde_json::to_vec(&bad).unwrap()).is_err());
    }
}
