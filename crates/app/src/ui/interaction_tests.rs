use super::*;

fn frame(
    ctx: &egui::Context,
    size: egui::Vec2,
    session: &mut Session,
    front: &mut crate::frontend::Frontend,
    events: Vec<egui::Event>,
) -> (egui::FullOutput, Vec<egui::Rect>) {
    let mut occupied = Vec::new();
    let output = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
            events,
            ..Default::default()
        },
        |ctx| {
            occupied = show(ctx, session, &mut Camera::default(), front);
        },
    );
    (output, occupied)
}

#[test]
fn hud_stays_on_screen_without_overlapping_at_supported_scales() {
    for size in [
        egui::vec2(563., 400.),
        egui::vec2(760., 540.),
        egui::vec2(900., 540.),
        egui::vec2(1100., 800.),
        egui::vec2(1120., 540.),
        egui::vec2(1280., 800.),
    ] {
        for tool in 0..7 {
            let ctx = egui::Context::default();
            theme::install(&ctx);
            let mut session = Session::from_sim(genesis_sim_core::scenarios::coastal_outpost());
            session.path.active = tool == 5;
            if tool == 6 {
                session.selected_object = Some(crate::interaction::Object::Building(1));
            }
            session.placing = tool == 1;
            session.landscape_tools = tool == 2;
            session.survey.open = tool == 3;
            session.survey.water = true;
            session.survey.minerals = true;
            session.world_tools = tool == 4;
            session.message = "Borehole 1 online".into();
            let mut front = crate::frontend::Frontend::default();
            // egui Areas settle measured dimensions on the following frame.
            for _ in 0..3 {
                frame(&ctx, size, &mut session, &mut front, vec![]);
            }
            let (_, rects) = frame(&ctx, size, &mut session, &mut front, vec![]);
            let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, size).expand(1.);
            for (i, rect) in rects.iter().enumerate() {
                assert!(
                    screen.contains_rect(*rect),
                    "{size:?}, tool {tool}, panel {i}: {rect:?}"
                );
                for (j, other) in rects.iter().enumerate().skip(i + 1) {
                    assert!(
                        !rect.shrink(1.).intersects(other.shrink(1.)),
                        "{size:?}, tool {tool}: panels {i} {rect:?} and {j} {other:?} overlap"
                    );
                }
            }
        }
    }
}

fn text_position(shape: &egui::Shape, label: &str) -> Option<egui::Pos2> {
    match shape {
        egui::Shape::Text(text) if text.galley.job.text == label => {
            Some(text.pos + text.galley.size() * 0.5)
        }
        egui::Shape::Vec(shapes) => shapes.iter().find_map(|s| text_position(s, label)),
        _ => None,
    }
}

#[test]
fn dock_switches_tools_and_menu_is_reachable() {
    let ctx = egui::Context::default();
    theme::install(&ctx);
    let mut session = Session::from_sim(genesis_sim_core::scenarios::coastal_outpost());
    let mut front = crate::frontend::Frontend::default();
    for (label, expected) in [
        ("Paths", 6),
        ("Build", 1),
        ("Survey", 2),
        ("Landscape", 3),
        ("Explore", 0),
        ("Region", 4),
        ("Menu", 5),
    ] {
        let size = egui::vec2(760., 540.);
        frame(&ctx, size, &mut session, &mut front, vec![]);
        let (output, _) = frame(&ctx, size, &mut session, &mut front, vec![]);
        let position = output
            .shapes
            .iter()
            .find_map(|s| text_position(&s.shape, label))
            .or_else(|| {
                ctx.data(|d| {
                    d.get_temp::<egui::Rect>(egui::Id::new(("icon_button", label)))
                        .map(|r| r.center())
                })
            })
            .expect(label);
        for pressed in [true, false] {
            frame(
                &ctx,
                size,
                &mut session,
                &mut front,
                vec![
                    egui::Event::PointerMoved(position),
                    egui::Event::PointerButton {
                        pos: position,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
        assert_eq!(session.path.active, expected == 6);
        assert_eq!(session.placing, expected == 1);
        assert_eq!(session.survey.open, expected == 2);
        assert_eq!(session.landscape_tools, expected == 3);
        assert_eq!(session.world_tools, expected == 4 || expected == 5);
        if expected == 5 {
            assert_eq!(front.screen, crate::frontend::Screen::Pause);
        }
    }
}
