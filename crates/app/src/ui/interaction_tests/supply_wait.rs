use super::*;

#[test]
fn waiting_inspector_and_supply_panels_show_the_committed_retry_without_advancing() {
    let mut session = Session::from_sim(genesis_sim_core::scenarios::living_settlement());
    session.clock.paused = true;
    for _ in 0..20 {
        session.step_once();
    }
    let id = session.sim.people.all()[0].id;
    let mut camera = Camera::default();
    session.inspect_person(id, &mut camera);
    let ctx = egui::Context::default();
    theme::install(&ctx);
    let mut front = crate::frontend::Frontend::default();
    let size = egui::vec2(1280., 1000.);
    let before = serde_json::to_vec(&*session.sim).unwrap();
    for _ in 0..3 {
        frame(&ctx, size, &mut session, &mut front, vec![]);
    }
    let (output, _) = frame(&ctx, size, &mut session, &mut front, vec![]);
    for label in [
        "Waiting for household supplies",
        "Next supply check · 1.0 logical s",
    ] {
        assert!(
            output
                .shapes
                .iter()
                .any(|s| text_position(&s.shape, label).is_some()),
            "{label}"
        );
    }
    for food in [false, true] {
        session.set_water_overview(true);
        session.supply_food = food;
        for _ in 0..3 {
            frame(&ctx, size, &mut session, &mut front, vec![]);
        }
        let (output, _) = frame(&ctx, size, &mut session, &mut front, vec![]);
        assert!(output.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.job.text.contains("Ada · supply retry in 1.0 s"))));
    }
    assert_eq!(before, serde_json::to_vec(&*session.sim).unwrap());
    assert_eq!(session.pending_count(), 0);
    session.step_once();
    assert_eq!(
        session.sim.people.get(id).unwrap().supply_retry_ms(),
        Some(900)
    );
}
