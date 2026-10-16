use super::*;
use genesis_sim_core::food::{
    Source,
    cultivation::{self, Kind},
};

pub fn placement(ui: &mut egui::Ui, session: &mut Session) {
    ui.heading("Plant barley");
    muted(ui, "4 m across · continuous placement");
    muted(
        ui,
        "Regrows after each harvest. Dry ground slows growth; residents harvest and carry portions home.",
    );
    let p = session.candidate().position;
    if session.hover.is_some() {
        match cultivation::validate(&session.sim, p) {
            Ok(()) => {
                ui.colored_label(ACCENT, "Ready to plant");
            }
            Err(reason) => {
                ui.colored_label(ERROR, reason.to_string());
            }
        }
        ui.label(format!(
            "Soil moisture · {:.0}%",
            100. * cultivation::moisture(&session.sim, p)
        ));
        if cultivation::moisture(&session.sim, p) <= 0.2 {
            ui.colored_label(ERROR, "Too dry to grow here");
        }
    } else {
        muted(ui, "Move onto the landscape");
    }
    muted(
        ui,
        "Click to plant. Shift-click repeats. No seed cost in this prototype.",
    );
    if button(ui, "Cancel placement", false, "Return to exploration · Esc") {
        session.set_placing(false);
    }
}
pub fn source(ui: &mut egui::Ui, sim: &genesis_sim_core::SimState, source: &Source) {
    let Kind::Barley(crop) = &source.kind else {
        return;
    };
    ui.label(cultivation::status(sim, source).label());
    let progress = if source.remaining > 0 {
        1.
    } else {
        crop.growth_units as f32 / cultivation::GROWTH_UNITS as f32
    };
    ui.add(egui::ProgressBar::new(progress).show_percentage());
    ui.label(format!(
        "Moisture {:.0}% · produced {} portions",
        100. * cultivation::moisture(sim, source.position),
        crop.produced
    ));
    muted(
        ui,
        "Full growth takes 60 logical seconds at 60% moisture; stops at 20%. Growth resumes after the standing harvest is cleared.",
    );
}
