//! Read-only coverage of the next consumption cycle, from stock at each home.
//! Cargo and reservations never masquerade as delivered household supplies.
use crate::{SimState, households};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Home {
    pub id: u64,
    pub residents: u32,
    pub food: u32,
    pub water: f64,
    pub carried_food: u32,
    pub carried_water: f64,
}
impl Home {
    pub fn food_shortfall(&self) -> u32 {
        self.residents.saturating_sub(self.food)
    }
    pub fn water_shortfall(&self) -> f64 {
        (f64::from(self.residents) * households::USE_PER_PERSON - self.water).max(0.)
    }
    pub fn short(&self) -> bool {
        self.food_shortfall() > 0 || self.water_shortfall() > 0.
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Outlook {
    pub next_food_ms: Option<u64>,
    pub next_water_ms: Option<u64>,
    /// Occupied homes only, in persistent-ID order.
    pub homes: Vec<Home>,
}
impl Outlook {
    pub fn read(sim: &SimState) -> Self {
        let mut homes = BTreeMap::new();
        for person in sim.people.all() {
            let home = homes.entry(person.home).or_insert_with(|| Home {
                id: person.home,
                residents: 0,
                food: sim.food.home(person.home).map_or(0, |h| h.stored),
                water: sim.households.get(person.home).map_or(0., |h| h.stored),
                carried_food: 0,
                carried_water: 0.,
            });
            home.residents += 1;
            if person.food_task.is_some() {
                home.carried_food += person.carried_food;
            }
            // Crop water belongs to a plot, even when its carrier lives here.
            if person.water_task.is_some() {
                home.carried_water += person.carried_water;
            }
        }
        Self {
            next_food_ms: (!homes.is_empty()).then(|| sim.food.next_use_ms()),
            next_water_ms: (!homes.is_empty()).then(|| sim.households.next_use_ms()),
            homes: homes.into_values().collect(),
        }
    }
    pub fn short_homes(&self) -> usize {
        self.homes.iter().filter(|h| h.short()).count()
    }
    pub fn food_short_homes(&self) -> usize {
        self.homes.iter().filter(|h| h.food_shortfall() > 0).count()
    }
    pub fn water_short_homes(&self) -> usize {
        self.homes
            .iter()
            .filter(|h| h.water_shortfall() > 0.)
            .count()
    }
}

#[cfg(test)]
mod tests;
