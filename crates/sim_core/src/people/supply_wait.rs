//! A failed supply search delays leisure when the next household use is uncovered.
use super::{Activity, People, Person};
use crate::{SimState, households, paths::distance, surface::walking, units::metres};

const RETRY_MS: u64 = 2000;

impl People {
    /// Admit new work only at a safe home departure/retry boundary. Existing tasks
    /// are handled first by prepare_supplies and never interrupted by this policy.
    pub(super) fn supply_departure(&mut self, index: usize, sim: &SimState, ms: u64) -> bool {
        let p = &mut self.entries[index];
        if !matches!(p.activity, Activity::Resting | Activity::WaitingForSupplies)
            || p.remaining_ms > ms
            || distance(p.position, p.home_position) >= metres(0.01)
            || !walking::standable(sim, p.position)
        {
            return false;
        }
        if p.activity == Activity::WaitingForSupplies {
            p.activity = Activity::Resting;
        }
        true
    }

    pub(super) fn wait_if_short(&mut self, index: usize, sim: &SimState, ms: u64) {
        let p = &self.entries[index];
        if p.activity != Activity::Resting {
            return; // A food, water or productive tending task was admitted.
        }
        let residents = self
            .entries
            .iter()
            .filter(|other| other.home == p.home)
            .count();
        let food = sim.food.home(p.home).map_or(0, |h| h.stored);
        let water = sim.households.get(p.home).map_or(0., |h| h.stored);
        if u64::from(food) >= residents as u64
            && water >= residents as f64 * households::USE_PER_PERSON
        {
            return;
        }
        let p = &mut self.entries[index];
        p.activity = Activity::WaitingForSupplies;
        // The routine consumes this tick's ms after task preparation.
        p.remaining_ms = RETRY_MS.saturating_add(ms);
        p.route.clear();
        p.next = 0;
    }
}

pub(super) fn valid(p: &Person) -> bool {
    p.activity != Activity::WaitingForSupplies
        || (p.remaining_ms <= RETRY_MS
            && p.route.is_empty()
            && p.water_task.is_none()
            && p.food_task.is_none()
            && p.irrigation_task.is_none()
            && p.carried_food == 0
            && p.carried_water == 0.
            && distance(p.position, p.home_position) < metres(0.01))
}

#[cfg(test)]
mod tests;
