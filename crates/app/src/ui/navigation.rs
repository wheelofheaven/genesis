use super::{
    icons::{self, Icon},
    theme::*,
};
use crate::session::Session;
use egui::{Align2, FontId, Sense, Shape, Stroke, Vec2};
use genesis_renderer::camera::Camera;

/// Metric scale at the focus plane; perspective makes distant objects smaller.
pub fn scale(ctx: &egui::Context, camera: &Camera) -> egui::Rect {
    let pixels_per_metre = ctx.content_rect().height()
        / camera.visible_height()
        / genesis_sim_core::units::METRES_PER_UNIT;
    let max_pixels = if ctx.content_rect().width() < 900. {
        60.
    } else {
        100.
    };
    let length = [1., 2., 5., 10., 20., 50., 100., 200., 500., 1000.]
        .into_iter()
        .rev()
        .find(|m| m * pixels_per_metre <= max_pixels)
        .unwrap_or(1.);
    egui::Area::new("map_scale".into())
        .anchor(Align2::LEFT_BOTTOM, [16., -16.])
        .movable(false)
        .fade_in(false)
        .show(ctx, |ui| {
            card().inner_margin(8).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 2.;
                ui.label(
                    egui::RichText::new(if camera.is_ground() {
                        "Eye · 1.7 m".into()
                    } else {
                        format!("{length:.0} m")
                    })
                    .size(11.)
                    .color(TEXT),
                )
                .on_hover_text(
                    "Scale at camera focus. Distant ground appears smaller in perspective.",
                );
                if camera.is_ground() {
                    return;
                }
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(length * pixels_per_metre, 7.),
                    egui::Sense::hover(),
                );
                let line = egui::Stroke::new(1.5, TEXT);
                ui.painter()
                    .line_segment([rect.left_center(), rect.right_center()], line);
                for x in [rect.left(), rect.right()] {
                    ui.painter().line_segment(
                        [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                        line,
                    );
                }
            });
        })
        .response
        .rect
}

pub fn show(ctx: &egui::Context, session: &mut Session, camera: &mut Camera) -> egui::Rect {
    egui::Area::new("navigation".into())
        .anchor(Align2::LEFT_BOTTOM, [16., -68.])
        .order(egui::Order::Foreground)
        .movable(false)
        .fade_in(false)
        .show(ctx, |ui| {
            card().inner_margin(10).show(ui, |ui| {
                ui.set_width(64.);
                let (rect, response) = ui.allocate_exact_size(Vec2::splat(64.), Sense::click());
                let painter = ui.painter_at(rect);
                let center = rect.center();
                painter.circle_stroke(center, 26., Stroke::new(1., LINE));
                let angle = camera.heading();
                let north = Vec2::new(angle.sin(), -angle.cos());
                let side = Vec2::new(-north.y, north.x);
                painter.add(Shape::convex_polygon(
                    vec![
                        center + north * 19.,
                        center - north * 7. + side * 5.,
                        center - north * 2.,
                        center - north * 7. - side * 5.,
                    ],
                    ACCENT,
                    Stroke::NONE,
                ));
                painter.circle_filled(center, 2., HIGHLIGHT);
                painter.text(
                    center + north * 29.,
                    Align2::CENTER_CENTER,
                    "N",
                    FontId::proportional(10.),
                    TEXT,
                );
                if response.on_hover_text("Face north").clicked() {
                    camera.face_north();
                }
                if icons::button(
                    ui,
                    Icon::Person,
                    "Ground",
                    camera.is_ground(),
                    false,
                    "Ground inspection · G · WASD move, right drag look, Esc return",
                )
                .clicked()
                {
                    session.stop_following();
                    crate::ground::toggle(&session.sim, session.position, camera);
                }
                if ctx.content_rect().width() < 900. || ctx.content_rect().height() < 650. {
                    return;
                }
                ui.add_space(2.);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.;
                    if icons::button(ui, Icon::Minus, "Zoom out", false, false, "Zoom out")
                        .clicked()
                    {
                        camera.zoom(-1.5);
                    }
                    if icons::button(ui, Icon::Plus, "Zoom in", false, false, "Zoom in").clicked() {
                        camera.zoom(1.5);
                    }
                });
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.;
                    if icons::button(
                        ui,
                        Icon::Focus,
                        "Focus",
                        false,
                        false,
                        "Focus selection · F",
                    )
                    .clicked()
                    {
                        session.stop_following();
                        camera.leave_ground();
                        camera.center(session.selected_point());
                    }
                    if icons::button(
                        ui,
                        Icon::Home,
                        "Home",
                        false,
                        false,
                        "Return to regional overview · R",
                    )
                    .clicked()
                    {
                        session.stop_following();
                        crate::terrain::home_camera(&session.sim, camera);
                    }
                });
            });
        })
        .response
        .rect
}

pub fn help(ctx: &egui::Context, open: &mut bool) -> Option<egui::Rect> {
    egui::Window::new("Find your way")
        .order(egui::Order::Foreground)
        .max_height((ctx.content_rect().height() - 100.).max(200.))
        .vscroll(true)
        .open(open)
        .anchor(Align2::CENTER_CENTER, [0., -10.])
        .resizable(false)
        .collapsible(false)
        .frame(card().inner_margin(22))
        .show(ctx, |ui| {
            ui.set_width(360.);
            muted(ui, "Pause whenever you want to take a closer look.");
            ui.add_space(8.);
            egui::Grid::new("control_guide")
                .num_columns(2)
                .spacing([24., 12.])
                .show(ui, |ui| {
                    for (key, action) in [
                        ("WASD / arrows", "Move · hold Shift for speed"),
                        ("Right drag", "Orbit / look around on the ground"),
                        ("G", "Ground inspection · Esc returns"),
                        ("Middle / Alt + left drag", "Pan across the landscape"),
                        ("Scroll", "Zoom toward the pointer"),
                        ("F / R", "Focus selection / return home"),
                        ("B", "Place a dwelling"),
                        ("P / Enter", "Draw path / build clicked route"),
                        ("Backspace", "Undo last path waypoint"),
                        ("V", "Survey groundwater and mineral deposits"),
                        ("Q / E", "Rotate a dwelling or orbit the view"),
                        ("Shift-click", "Build and keep placing"),
                        ("Esc", "Cancel the active tool"),
                        ("Space / 1, 2, 3", "Pause / change speed"),
                        ("F5 / F9", "Save world / load saved world"),
                    ] {
                        ui.label(egui::RichText::new(key).strong().color(TEXT));
                        ui.label(egui::RichText::new(action).color(MUTED));
                        ui.end_row();
                    }
                });
            ui.add_space(8.);
            ui.label(
                egui::RichText::new("Paused actions wait until you resume or step forward.")
                    .size(12.)
                    .color(HIGHLIGHT),
            );
        })
        .map(|r| r.response.rect)
}
