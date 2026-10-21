use super::*;
use genesis_sim_core::food::{
    Source,
    cultivation::{self, Kind, irrigation},
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
pub fn source(
    ui: &mut egui::Ui,
    sim: &genesis_sim_core::SimState,
    source: &Source,
) -> Option<Action> {
    let Kind::Barley(crop) = &source.kind else {
        return None;
    };
    ui.label(cultivation::status(sim, source).label());
    let progress = if source.remaining > 0 {
        1.
    } else {
        crop.growth_units as f32 / cultivation::GROWTH_UNITS as f32
    };
    ui.add(egui::ProgressBar::new(progress).show_percentage());
    ui.label(format!(
        "Growth moisture {:.0}% · produced {} portions",
        100. * cultivation::effective_moisture(sim, source),
        crop.produced
    ));
    muted(
        ui,
        "Full growth takes 60 logical seconds at 60% moisture; stops at 20%. Growth resumes after the standing harvest is cleared.",
    );
    ui.separator();
    ui.label("Irrigation");
    ui.label(format!(
        "Ambient moisture · {:.0}%",
        100. * cultivation::moisture(sim, source.position)
    ));
    if let Some(water) = &crop.soil_water {
        ui.label(format!(
            "Plot water · {:.2} / 4 u",
            water.stored as f64 / irrigation::UNITS as f64
        ));
        muted(
            ui,
            format!(
                "Applied {:.1} u · drained / evaporated {:.1} u",
                water.applied as f64 / irrigation::UNITS as f64,
                water.lost as f64 / irrigation::UNITS as f64
            ),
        );
    }
    let well = irrigation::candidate(sim, source.id);
    match well {
        Ok(id) => {
            muted(ui, format!("From well {id} · household claims protected"));
        }
        Err(reason) => {
            muted(ui, reason.to_string());
        }
    }
    let mut action = None;
    let mut tending = crop.tending;
    if ui
        .checkbox(&mut tending, "Residents tend this plot")
        .on_hover_text("Residents check at home departure after their own supply jobs. They water growing plots at 1 u or less when ambient moisture is below 60%. Stopping tending releases uncollected water; carried doses still deliver.")
        .changed()
    {
        action = Some(Action::SetCropTending {
            source: source.id,
            enabled: tending,
        });
    }
    if let Some(id) = sim.people.irrigator(source.id) {
        if let Some(p) = sim.people.get(id) {
            ui.label(format!("{} · {}", p.name, p.activity.label()));
            if let Some(issue) = p.irrigation_issue {
                ui.colored_label(ERROR, issue.label());
            }
        }
        muted(ui, "2 u of plot capacity reserved for delivery");
    } else if tending {
        muted(ui, "Watering checked at home departures");
    }
    ui.add_enabled_ui(well.is_ok(), |ui| {
        if button(
            ui,
            "Apply 2 u water",
            false,
            "Queue an expedition water application from the nearest eligible well",
        ) && let Ok(well) = well
        {
            action = Some(Action::IrrigateCrop {
                source: source.id,
                well,
            });
        }
    });
    muted(
        ui,
        "Expedition application within 30 m. Each dose supports 40 logical seconds of growth; water also drains while harvest waits.",
    );
    action
}
