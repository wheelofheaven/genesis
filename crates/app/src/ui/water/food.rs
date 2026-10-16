use super::*;
use genesis_sim_core::food;
pub(super) fn panel(ui: &mut egui::Ui, session: &mut Session, camera: &mut Camera) {
    let view = Overview::read(&session.sim);
    ui.heading("Food supply");
    muted(
        ui,
        format!(
            "{} homes · tick {}",
            view.homes.len(),
            session.sim.completed_ticks()
        ),
    );
    let stored: u64 = session
        .sim
        .food
        .homes()
        .values()
        .map(|h| u64::from(h.stored))
        .sum();
    let carried: u64 = session
        .sim
        .people
        .all()
        .iter()
        .map(|p| u64::from(p.carried_food))
        .sum();
    let remaining: u64 = session
        .sim
        .food
        .sources()
        .iter()
        .map(|s| u64::from(s.remaining))
        .sum();
    let reserved: u64 = session
        .sim
        .food
        .sources()
        .iter()
        .map(|s| u64::from(session.sim.people.reserved_food(s.id)))
        .sum();
    ui.label(format!("At homes {stored} · in transit {carried}"));
    ui.label(format!("At sources {remaining} · reserved {reserved}"));
    muted(ui, "Whole portions. Reservations are part of source stock.");
    muted(
        ui,
        "Lower relative reserves first; equal reserves alternate. Unavailable tasks fall back to the other supply.",
    );
    if view.homes.is_empty() {
        muted(ui, "No inhabited households in this world.");
    }
    let mut focus = None;
    for home in &view.homes {
        let Some(stock) = session.sim.food.home(home.id) else {
            continue;
        };
        let mut label = format!(
            "Home {} · {} / {} portions",
            home.id,
            stock.stored,
            food::CAPACITY
        );
        if let Some(p) = home.residents.iter().find(|p| p.food_task.is_some()) {
            label.push_str(&format!("\n{} · {}", p.name, p.activity.label()));
            if let Some(t) = &p.food_task {
                label.push_str(&format!(
                    "\nPatch {} · {} {}",
                    t.source.0,
                    t.amount,
                    if p.carried_food > 0 {
                        "carried"
                    } else {
                        "reserved"
                    }
                ));
            }
            if let Some(reason) = p.blocked {
                label.push_str(&format!("\nBlocked: {}", reason.label()));
            }
        } else if let Some(p) = home.residents.iter().find(|p| p.water_task.is_some()) {
            label.push_str(&format!(
                "\n{} · {}\nFood reconsidered at home departure",
                p.name,
                p.activity.label()
            ));
        } else if stock.stored >= food::REFILL_BELOW {
            label.push_str("\nStock sufficient");
        } else {
            label.push_str("\nLow stock · checks at home departure");
            for p in &home.residents {
                if let Some(issue) = p.food_issue {
                    label.push_str(&format!("\n{} · last attempt: {}", p.name, issue.label()));
                }
                if let Some(reason) = p.blocked {
                    label.push_str(&format!("\n{} · blocked: {}", p.name, reason.label()));
                }
            }
        }
        ui.separator();
        if ui
            .add(
                egui::Button::new(label)
                    .wrap()
                    .selected(session.selected_object == Some(Object::Building(home.id)))
                    .min_size(egui::vec2(ui.available_width(), 0.)),
            )
            .clicked()
        {
            focus = Some(Object::Building(home.id));
        }
    }
    ui.separator();
    ui.label("Food sources");
    if session.sim.food.sources().is_empty() {
        muted(
            ui,
            "No source in this world. Use Build → Barley plot to plant food.",
        );
    }
    for source in session.sim.food.sources() {
        if ui
            .button(format!(
                "{} {} · {} portions{}",
                source.label(),
                source.id.0,
                source.remaining,
                if source.remaining == 0
                    && !matches!(source.kind, food::cultivation::Kind::Barley(_))
                {
                    " · depleted"
                } else {
                    ""
                }
            ))
            .clicked()
        {
            focus = Some(Object::FoodSource(source.id));
        }
        if matches!(source.kind, food::cultivation::Kind::Barley(_)) {
            ui.label(food::cultivation::status(&session.sim, source).label());
        }
    }
    if let Some(object) = focus {
        session.select_object(Some(object), session.position);
        camera.leave_ground();
        camera.center(session.selected_point());
    }
}
