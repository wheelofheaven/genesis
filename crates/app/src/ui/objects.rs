use super::theme::*;
use crate::{interaction::Object, session::Session};
use genesis_sim_core::{paths, units::METRES_PER_UNIT};

pub fn selection(ui: &mut egui::Ui, session: &Session) {
    let Some(object) = session.selected_object else {
        return;
    };
    eyebrow(ui, "SELECTED OBJECT");
    ui.heading(object.label());
    match object {
        Object::Person(id) => {
            if let Some(p) = session.sim.people.get(id) {
                ui.heading(&p.name);
                ui.label(format!("Home · dwelling {}", p.home));
                ui.separator();
                ui.label(p.activity.label());
                let target = p.destination();
                ui.label(format!(
                    "Destination · {:.1}, {:.1} m",
                    target.x * METRES_PER_UNIT,
                    target.z * METRES_PER_UNIT
                ));
                if let Some(reason) = p.blocked {
                    ui.colored_label(ERROR, reason.label());
                }
                muted(
                    ui,
                    "F focuses this resident. The highlighted line is their current route.",
                );
            }
        }
        Object::Building(_) => {
            muted(ui, "Dwelling · single storey");
            ui.separator();
            ui.label("6.5 × 4.8 m footprint");
            ui.label("2.8 m walls");
            muted(
                ui,
                "F focuses this dwelling. G inspects it from the ground.",
            );
        }
        Object::Path(id) => {
            if let Some(path) = session.sim.paths.iter().find(|p| p.id == id) {
                muted(ui, "Unpaved footpath");
                ui.separator();
                ui.label(format!(
                    "{:.1} m long · {:.1} m wide",
                    paths::length(path) * METRES_PER_UNIT,
                    path.width * METRES_PER_UNIT
                ));
                muted(
                    ui,
                    "P extends the network. New endpoints snap to nearby paths.",
                );
            }
        }
    }
    ui.add_space(8.);
    muted(ui, "Click elsewhere to select. Esc clears selection.");
}
pub fn path_tool(ui: &mut egui::Ui, session: &mut Session) {
    ui.heading("Lay a footpath");
    muted(ui, "Click waypoints to shape a flowing route.");
    let mut width = session.path.width * METRES_PER_UNIT;
    if ui
        .add(
            egui::Slider::new(&mut width, 1.2..=4.)
                .step_by(0.2)
                .suffix(" m")
                .text("Width"),
        )
        .changed()
    {
        session.path.width = width / METRES_PER_UNIT;
        session.overlay_dirty = true;
    }
    ui.separator();
    let points = session.path_waypoints(true);
    if points.len() >= 2 {
        match session.path_validation(true) {
            Ok(path) => {
                ui.colored_label(
                    ACCENT,
                    format!(
                        "{:.1} m · Route clear",
                        paths::length(&path) * METRES_PER_UNIT
                    ),
                );
            }
            Err(e) => {
                ui.colored_label(ERROR, e.to_string());
            }
        }
    } else {
        muted(ui, "Click the ground to start.");
    }
    muted(
        ui,
        format!(
            "{} waypoints · endpoints snap within 1.5 m",
            session.path.points.len()
        ),
    );
    ui.add_space(6.);
    let valid = session.path_validation(false).is_ok();
    ui.add_enabled_ui(valid, |ui| {
        if button(
            ui,
            "Build path",
            false,
            "Build through the clicked waypoints · Enter",
        ) {
            session.finish_path();
        }
    });
    ui.add_enabled_ui(!session.path.points.is_empty(), |ui| {
        if button(
            ui,
            "Undo waypoint",
            false,
            "Remove the last waypoint · Backspace",
        ) {
            session.undo_path_point();
        }
    });
    muted(
        ui,
        "Enter builds the clicked route. Backspace undoes a point. Esc cancels.",
    );
    if button(ui, "Cancel path", false, "Discard this draft · Esc") {
        session.set_path_tool(false);
    }
}
