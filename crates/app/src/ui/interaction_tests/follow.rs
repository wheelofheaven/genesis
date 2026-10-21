use super::*;
use crate::interaction::Object;

fn click(
    label: &str,
    size: egui::Vec2,
    ctx: &egui::Context,
    session: &mut Session,
    front: &mut crate::frontend::Frontend,
    camera: &mut Camera,
) {
    for _ in 0..3 {
        frame_with_camera(ctx, size, session, front, vec![], camera);
    }
    let (output, _) = frame_with_camera(ctx, size, session, front, vec![], camera);
    let point = output
        .shapes
        .iter()
        .find_map(|s| {
            let point = text_position(&s.shape, label)?;
            s.clip_rect.contains(point).then_some(point)
        })
        .unwrap_or_else(|| panic!("visible control missing: {label}"));
    for pressed in [true, false] {
        frame_with_camera(
            ctx,
            size,
            session,
            front,
            vec![
                egui::Event::PointerMoved(point),
                egui::Event::PointerButton {
                    pos: point,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            camera,
        );
    }
}

#[test]
fn supply_links_follow_controls_and_home_links_work_without_changing_paused_state() {
    for food in [false, true] {
        let ctx = egui::Context::default();
        theme::install(&ctx);
        let mut session = Session::from_sim(genesis_sim_core::scenarios::supplied_settlement());
        session.clock.paused = true;
        let id = session.sim.people.all()[0].id;
        let before = serde_json::to_vec(&*session.sim).unwrap();
        let mut front = crate::frontend::Frontend::default();
        let mut camera = Camera::default();
        session.set_water_overview(true);
        session.supply_food = food;
        click(
            "Inspect Ada",
            egui::vec2(1280., 1000.),
            &ctx,
            &mut session,
            &mut front,
            &mut camera,
        );
        assert_eq!(session.selected_object, Some(Object::Person(id)));
        assert!(!session.water_overview);
        assert!(session.following().is_none());
        assert_eq!(camera.focus, session.selected_point());

        let compact = egui::vec2(760., 540.);
        for label in ["Follow resident", "Stop following", "Follow resident"] {
            click(label, compact, &ctx, &mut session, &mut front, &mut camera);
            assert_eq!(
                session.following(),
                (label == "Follow resident").then_some(id)
            );
        }
        click(
            "Home · dwelling 1",
            compact,
            &ctx,
            &mut session,
            &mut front,
            &mut camera,
        );
        assert_eq!(session.selected_object, Some(Object::Building(1)));
        assert!(session.following().is_none());
        click(
            "Ada · Resting at home",
            compact,
            &ctx,
            &mut session,
            &mut front,
            &mut camera,
        );
        assert_eq!(session.selected_object, Some(Object::Person(id)));
        assert_eq!(before, serde_json::to_vec(&*session.sim).unwrap());
        assert_eq!(session.pending_count(), 0);
        assert_eq!(session.sim.completed_ticks(), 0);
    }
}

#[test]
fn opening_a_resident_from_the_bottom_of_supplies_keeps_follow_controls_visible() {
    let ctx = egui::Context::default();
    theme::install(&ctx);
    let mut session = Session::from_sim(genesis_sim_core::scenarios::supplied_settlement());
    session.clock.paused = true;
    session.set_water_overview(true);
    let mut front = crate::frontend::Frontend::default();
    let mut camera = Camera::default();
    let size = egui::vec2(760., 540.);
    for _ in 0..3 {
        frame_with_camera(&ctx, size, &mut session, &mut front, vec![], &mut camera);
    }
    let (_, bounds) = frame_with_camera(&ctx, size, &mut session, &mut front, vec![], &mut camera);
    let point = bounds[6].center();
    frame_with_camera(
        &ctx,
        size,
        &mut session,
        &mut front,
        vec![
            egui::Event::PointerMoved(point),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0., -3000.),
                modifiers: egui::Modifiers::NONE,
            },
        ],
        &mut camera,
    );
    for _ in 0..30 {
        frame_with_camera(&ctx, size, &mut session, &mut front, vec![], &mut camera);
    }
    click(
        "Inspect Eli",
        size,
        &ctx,
        &mut session,
        &mut front,
        &mut camera,
    );
    let id = session.sim.people.all()[5].id;
    assert_eq!(session.selected_object, Some(Object::Person(id)));
    click(
        "Follow resident",
        size,
        &ctx,
        &mut session,
        &mut front,
        &mut camera,
    );
    assert_eq!(session.following(), Some(id));
    assert_eq!(session.sim.completed_ticks(), 0);
}
