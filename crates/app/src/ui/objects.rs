use super::theme::*;
use crate::{interaction::Object, session::Session};
use genesis_sim_core::{paths, units::METRES_PER_UNIT};

pub fn selection(ui: &mut egui::Ui, session: &mut Session) {
    let Some(object) = session.selected_object else {
        return;
    };
    eyebrow(ui, "SELECTED OBJECT");
    ui.heading(object.label());
    match object {
        Object::FoodSource(id) => {
            if let Some(source) = session.sim.food.source(id) {
                ui.label(format!(
                    "{} portions remaining · {} reserved",
                    source.remaining,
                    session.sim.people.reserved_food(id)
                ));
                ui.heading(source.label());
                if matches!(
                    source.kind,
                    genesis_sim_core::food::cultivation::Kind::Barley(_)
                ) {
                    if let Some(action) = super::cultivation::source(ui, &session.sim, source) {
                        session.queue(action);
                    }
                } else {
                    muted(
                        ui,
                        "Finite wild-food harvest. Residents gather four portions per trip. No regrowth yet.",
                    );
                    if source.remaining == 0 {
                        ui.colored_label(ERROR, "Depleted");
                    }
                }
            }
        }
        Object::Person(id) => {
            if let Some(p) = session.sim.people.get(id) {
                ui.heading(&p.name);
                ui.label(format!("Home · dwelling {}", p.home));
                ui.separator();
                ui.label(p.activity.label());
                if let Some(home) = session.sim.households.get(p.home) {
                    ui.label(format!(
                        "Household water · {:.1} / {:.0} u",
                        home.stored,
                        genesis_sim_core::households::CAPACITY
                    ));
                }
                if let Some(task) = &p.water_task {
                    ui.label(format!(
                        "Well {} · {:.1} u {}",
                        task.well,
                        task.amount,
                        if p.carried_water > 0. {
                            "being carried"
                        } else {
                            "reserved"
                        }
                    ));
                }
                if let Some(task) = &p.irrigation_task {
                    ui.label(format!(
                        "Well {} → plot {} · 2 u {}",
                        task.well,
                        task.source.0,
                        if p.carried_water > 0. {
                            "carried"
                        } else {
                            "reserved"
                        }
                    ));
                }
                if let Some(issue) = p.irrigation_issue {
                    ui.colored_label(ERROR, format!("Last tending issue: {}", issue.label()));
                }
                if let Some(issue) = p.water_issue {
                    ui.colored_label(ERROR, format!("Last issue: {}", issue.label()));
                }
                if let Some(home) = session.sim.food.home(p.home) {
                    ui.label(format!(
                        "Household food · {} / {} portions",
                        home.stored,
                        genesis_sim_core::food::CAPACITY
                    ));
                }
                if let Some(task) = &p.food_task {
                    ui.label(format!(
                        "Patch {} · {} portions {}",
                        task.source.0,
                        task.amount,
                        if p.carried_food > 0 {
                            "carried"
                        } else {
                            "reserved"
                        }
                    ));
                }
                if let Some(issue) = p.food_issue {
                    ui.colored_label(ERROR, format!("Last issue: {}", issue.label()));
                }
                let food_cancel = p.food_task.is_some() && p.carried_food == 0;
                let target = p.destination();
                ui.label(format!(
                    "Destination · {:.1}, {:.1} m",
                    target.x * METRES_PER_UNIT,
                    target.z * METRES_PER_UNIT
                ));
                if let Some(reason) = p.blocked {
                    ui.colored_label(ERROR, reason.label());
                }
                let has_task = p.water_task.is_some();
                let carrying = p.carried_water > 0.;
                if has_task && !carrying && ui.button("Cancel collection").clicked() {
                    session
                        .queue(genesis_sim_core::actions::Action::CancelWaterTask { person: id });
                }
                if food_cancel && ui.button("Cancel gathering").clicked() {
                    session.queue(genesis_sim_core::actions::Action::CancelFoodTask { person: id });
                }
                muted(
                    ui,
                    "F focuses this resident. Highlighted lines show the current route.",
                );
            }
        }
        Object::Building(id) => {
            if let Some(home) = session.sim.food.home(id) {
                ui.label(format!(
                    "Food · {} / {} portions",
                    home.stored,
                    genesis_sim_core::food::CAPACITY
                ));
                ui.label(format!(
                    "Eaten {} · missed portions {}",
                    home.consumed, home.missed_portions
                ));
                if let Some(p) = session.sim.people.food_collector(id) {
                    ui.label(format!("Resident {} is gathering for this home", p.0));
                }
                ui.separator();
            }
            if let Some(home) = session.sim.households.get(id) {
                ui.label(format!(
                    "Household water · {:.1} / {:.0} u",
                    home.stored,
                    genesis_sim_core::households::CAPACITY
                ));
                ui.label(format!(
                    "Used {:.1} u · missed portions {}",
                    home.consumed, home.missed_portions
                ));
                if let Some(person) = session.sim.people.home_collector(id) {
                    ui.label(format!("Resident {} is supplying this home", person.0));
                } else if home.stored < genesis_sim_core::households::REFILL_BELOW {
                    muted(
                        ui,
                        "Low water · residents look for a working well at their next departure.",
                    );
                }
                ui.separator();
            }
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
