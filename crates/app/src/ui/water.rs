//! Read-only household supply presentation. No planning or resource mutation.
mod food;
mod outlook;
use super::theme::*;
use crate::{interaction::Object, session::Session};
use genesis_renderer::camera::Camera;
use genesis_sim_core::{SimState, households, people::Person};
use std::collections::BTreeMap;

struct Home<'a> {
    id: u64,
    stock: &'a households::HomeWater,
    residents: Vec<&'a Person>,
    needs: Option<genesis_sim_core::supply_outlook::Home>,
}
struct Overview<'a> {
    homes: Vec<Home<'a>>,
    outlook: genesis_sim_core::supply_outlook::Outlook,
    stored: f64,
    carried: f64,
    buffered: f64,
    reserved: f64,
    empty: usize,
}
impl<'a> Overview<'a> {
    fn read(sim: &'a SimState) -> Self {
        let outlook = genesis_sim_core::supply_outlook::Outlook::read(sim);
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
                    needs: outlook
                        .homes
                        .binary_search_by_key(&id, |h| h.id)
                        .ok()
                        .map(|i| outlook.homes[i].clone()),
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
                .subsurface
                .wells
                .iter()
                .map(|w| sim.people.reserved_water(w.id))
                .sum(),
            homes,
            outlook,
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
        if let Some(home) = &self.needs {
            text.push_str(&format!("\n{}", outlook::home_line(home, false)));
        }
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
        } else if let Some(p) = self
            .residents
            .iter()
            .find(|p| p.food_task.is_some() || p.irrigation_task.is_some())
        {
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
            text.push_str(
                if self.residents.iter().any(|p| p.supply_retry_ms().is_some()) {
                    "\nWaiting at home for supplies"
                } else {
                    "\nLow stock · checks at home departure"
                },
            );
            for p in &self.residents {
                if let Some(ms) = p.supply_retry_ms() {
                    text.push_str(&format!(
                        "\n{} · supply retry in {:.1} s",
                        p.name,
                        ms as f64 / 1000.
                    ));
                }
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
    let mut view = Overview::read(&session.sim);
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
    outlook::summary(ui, &view.outlook, false);
    ui.label(format!(
        "At homes {:.1} u · in transit {:.1} u",
        view.stored, view.carried
    ));
    ui.label(format!("Well buffers {:.1} u", view.buffered));
    let irrigated = genesis_sim_core::food::cultivation::irrigation::total_applied(&session.sim);
    if irrigated > 0. {
        muted(
            ui,
            format!("Applied to crops · {irrigated:.1} u total (includes losses)"),
        );
    }
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
    let mut resident = None;
    view.homes.sort_by_key(|h| {
        (
            !h.needs.as_ref().is_some_and(|n| n.water_shortfall() > 0.),
            h.id,
        )
    });
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
            if let Some(id) = resident_links(ui, &home.residents) {
                resident = Some(id);
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
                ui.label(format!("Well {} · {}", well.id, well.pump_mode.label()));
                muted(
                    ui,
                    well.pumping_status(&session.sim.grid, session.sim.sea_level),
                );
                if let Some(stock) = &well.supply {
                    let reserved = session.sim.people.reserved_water(well.id);
                    muted(
                        ui,
                        format!("{:.1} u stored · {:.1} u reserved", stock.stored, reserved),
                    );
                    muted(ui, format!("Released water · {:.1} u", stock.released));
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
    if let Some(id) = resident {
        session.inspect_person(id, camera);
    }
}

fn resident_links(
    ui: &mut egui::Ui,
    residents: &[&Person],
) -> Option<genesis_sim_core::people::PersonId> {
    let mut selected = None;
    for person in residents {
        if ui
            .small_button(format!("Inspect {}", person.name))
            .on_hover_text("See this resident's activity and route, or follow their journey")
            .clicked()
        {
            selected = Some(person.id);
        }
    }
    selected
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
