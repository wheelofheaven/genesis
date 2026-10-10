//! Drive the real egui widgets so navigation isn't tested only by assigning states.
use super::*;
use egui::{Event, Pos2};

fn draw(
    front: &mut Frontend,
    ctx: &egui::Context,
    session: &mut Session,
    camera: &mut Camera,
    events: Vec<Event>,
) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                Pos2::ZERO,
                egui::vec2(1280., 800.),
            )),
            events,
            ..Default::default()
        },
        |ctx| {
            front.show(ctx, session, camera);
        },
    )
}
fn text_position(shape: &egui::Shape, label: &str) -> Option<Pos2> {
    match shape {
        egui::Shape::Text(text) if text.galley.job.text == label => {
            Some(text.pos + text.galley.size() * 0.5)
        }
        egui::Shape::Vec(shapes) => shapes.iter().find_map(|s| text_position(s, label)),
        _ => None,
    }
}
fn click(
    front: &mut Frontend,
    ctx: &egui::Context,
    session: &mut Session,
    camera: &mut Camera,
    label: &str,
) {
    // A new screen gets its own layout before pointer events arrive.
    draw(front, ctx, session, camera, vec![]);
    let output = draw(front, ctx, session, camera, vec![]);
    let position = output
        .shapes
        .iter()
        .find_map(|s| text_position(&s.shape, label))
        .unwrap_or_else(|| panic!("missing visible control: {label}"));
    for pressed in [true, false] {
        draw(
            front,
            ctx,
            session,
            camera,
            vec![
                Event::PointerMoved(position),
                Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
}
#[test]
fn expedition_widgets_preserve_region_and_allow_back_navigation() {
    let ctx = egui::Context::default();
    crate::ui::theme::install(&ctx);
    let mut front = Frontend::default();
    let mut session = Session::from_sim(genesis_sim_core::scenarios::coastal_outpost());
    let mut camera = Camera::default();
    click(
        &mut front,
        &ctx,
        &mut session,
        &mut camera,
        "Begin expedition",
    );
    assert_eq!(front.screen, Screen::Destination);
    click(
        &mut front,
        &ctx,
        &mut session,
        &mut camera,
        "Select a region",
    );
    assert_eq!(front.screen, Screen::Setup);
    click(
        &mut front,
        &ctx,
        &mut session,
        &mut camera,
        Region::LevantHighlands.name(),
    );
    click(
        &mut front,
        &ctx,
        &mut session,
        &mut camera,
        "Review expedition",
    );
    assert_eq!(front.screen, Screen::Briefing);
    assert_eq!(front.region, Region::LevantHighlands);
    click(
        &mut front,
        &ctx,
        &mut session,
        &mut camera,
        "Back to regions",
    );
    assert_eq!(front.screen, Screen::Setup);
    assert_eq!(front.region, Region::LevantHighlands);
    assert_eq!(session.sim.time, 0.);
}
