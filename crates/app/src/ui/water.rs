//! Read-only household supply presentation. No planning or resource mutation.
mod food;
use super::theme::*;
use crate::{interaction::Object, session::Session};
use genesis_renderer::camera::Camera;
use genesis_sim_core::{
    SimState, households,
    people::{Activity, Person},
};
use std::collections::BTreeMap;

struct Home<'a> {
    id: u64,
    stock: &'a households::HomeWater,
    residents: Vec<&'a Person>,
}
struct Overview<'a> {
    homes: Vec<Home<'a>>,
    stored: f64,
    carried: f64,
    buffered: f64,
    reserved: f64,
    empty: usize,
}
impl<'a> Overview<'a> {
    fn read(sim: &'a SimState) -> Self {
        let mut residents: BTreeMap<u64, Vec<&Person>> = BTreeMap::new();
        for person in sim.people.all() {
            residents.entry(person.home).or_default().push(person);
        }
        let homes: Vec<_> = sim
            .households
            .all()
            .iter()
            .map(|(&id, stock)| {
                let mut people = residents.remove(&id).unwrap_or_default();
                people.sort_by_key(|p| p.id);
                Home {
                    id,
                    stock,
                    residents: people,
                }
            })
            .collect();
        Self {
            stored: homes.iter().map(|h| h.stock.stored).sum(),
            empty: homes.iter().filter(|h| h.stock.stored == 0.).count(),
            carried: sim.people.all().iter().map(|p| p.carried_water).sum(),
            buffered: sim
                .subsurface
                .wells
                .iter()
                .filter_map(|w| w.supply.as_ref())
                .map(|s| s.stored)
                .sum(),
            reserved: sim
                .people
                .all()
                .iter()
                .filter(|p| p.activity == Activity::FetchingWater)
                .filter_map(|p| p.water_task.as_ref())
                .map(|t| t.amount)
                .sum(),
            homes,
        }
    }
}
impl Home<'_> {
    fn label(&self) -> String {
        let mut text = format!(
            "Home {} · {:.1} / {:.0} u",
            self.id,
            self.stock.stored,
            households::CAPACITY
        );
        if let Some(p) = self.residents.iter().find(|p| p.water_task.is_some()) {
            if let Some(task) = &p.water_task {
                text.push_str(&format!(
                    "\n{} · {}",
                    p.name,
                    if p.carried_water > 0. {
                        "carrying water home"
                    } else {
                        "collecting water"
                    }
                ));
                text.push_str(&format!(
                    "\nWell {} · {:.1} u {}",
                    task.well,
                    task.amount,
                    if p.carried_water > 0. {
                        "carried"
                    } else {
                        "reserved"
                    }
                ));
                if let Some(blocked) = p.blocked {
                    text.push_str(&format!("\nBlocked: {}", blocked.label()));
                }
            }
        } else if let Some(p) = self.residents.iter().find(|p| p.food_task.is_some()) {
            text.push_str(&format!(
                "\n{} · {}\nWater reconsidered at home departure",
                p.name,
                p.activity.label()
            ));
        } else if self.stock.stored >= households::REFILL_BELOW {
            text.push_str("\nStock sufficient · no collection needed");
        } else if self.residents.is_empty() {
            text.push_str("\nNo resident collector");
        } else {
            text.push_str("\nLow stock · checks at home departure");
            for p in &self.residents {
                if let Some(issue) = p.water_issue {
                    text.push_str(&format!("\n{} · last attempt: {}", p.name, issue.label()));
                }
                if let Some(blocked) = p.blocked {
                    text.push_str(&format!("\n{} · blocked: {}", p.name, blocked.label()));
                }
            }
        }
        text
    }
}

pub fn panel(ui: &mut egui::Ui, session: &mut Session, camera: &mut Camera) {
    ui.horizontal(|ui| {
        if button(
            ui,
            "Water",
            !session.supply_food,
            "Household water and well stocks",
        ) {
            session.supply_food = false;
        }
        if button(
            ui,
            "Food",
            session.supply_food,
            "Household food and finite wild harvest",
        ) {
            session.supply_food = true;
        }
    });
    if session.supply_food {
        food::panel(ui, session, camera);
        return;
    }
    let view = Overview::read(&session.sim);
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(22., 22.), egui::Sense::hover());
        super::icons::paint(ui.painter(), rect, super::icons::Icon::Water, ACCENT);
        ui.heading("Water supply");
    });
    muted(
        ui,
        format!(
            "{} homes · {} empty · tick {}",
            view.homes.len(),
            view.empty,
            session.sim.completed_ticks()
        ),
    );
    ui.label(format!(
        "At homes {:.1} u · in transit {:.1} u",
        view.stored, view.carried
    ));
    ui.label(format!("Well buffers {:.1} u", view.buffered));
    muted(
        ui,
        format!("{:.1} u reserved within those buffers", view.reserved),
    );
    if view.homes.is_empty() {
        muted(
            ui,
            "No household water stores in this world. New Settlement worlds begin with residents.",
        );
    } else {
        muted(
            ui,
            "Select a home to focus it. Collection results update when residents try again.",
        );
    }
    let mut focus = None;
    for home in &view.homes {
        ui.separator();
        ui.push_id(home.id, |ui| {
            let response = ui.add(
                egui::Button::new(home.label())
                    .wrap()
                    .selected(session.selected_object == Some(Object::Building(home.id)))
                    .min_size(egui::vec2(ui.available_width(), 0.)),
            );
            if response.clicked() {
                focus = Some(home.id);
            }
            #[cfg(test)]
            ui.ctx()
                .data_mut(|d| d.insert_temp(egui::Id::new(("water_home", home.id)), response.rect));
        });
    }
    ui.separator();
    ui.collapsing(
        format!("Wells · {}", session.sim.subsurface.wells.len()),
        |ui| {
            if session.sim.subsurface.wells.is_empty() {
                muted(
                    ui,
                    "No wells. Use Survey → Hydrology to drill near the homes.",
                );
            }
            for well in &session.sim.subsurface.wells {
                ui.label(format!(
                    "Well {} · {}",
                    well.id,
                    if well.active {
                        "pump enabled"
                    } else {
                        "pump stopped"
                    }
                ));
                if let Some(stock) = &well.supply {
                    let reserved = session.sim.people.reserved_water(well.id);
                    muted(
                        ui,
                        format!("{:.1} u stored · {:.1} u reserved", stock.stored, reserved),
                    );
                    muted(
                        ui,
                        if stock.stored == 0. {
                            "Buffer empty"
                        } else if stock.stored - reserved < households::CONTAINER {
                            "Less than one unreserved container"
                        } else {
                            "Water in buffer"
                        },
                    );
                }
            }
        },
    );
    if let Some(id) = focus {
        focus_home(session, camera, id);
    }
}

pub(crate) fn focus_home(session: &mut Session, camera: &mut Camera, id: u64) {
    // Resolve against the active world at interaction time, never an array slot.
    if !session.sim.buildings.iter().any(|b| b.id == id) {
        return;
    }
    session.select_object(Some(Object::Building(id)), session.position);
    camera.leave_ground();
    camera.center(session.selected_point());
}

#[cfg(test)]
mod tests;
