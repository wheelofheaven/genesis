//! Water stocks owned by occupied dwellings, in the existing abstract water units.
use crate::{SimState, people::People};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
pub const CAPACITY: f64 = 8.;
pub const REFILL_BELOW: f64 = 4.;
pub const CONTAINER: f64 = 2.;
const USE_INTERVAL_MS: u64 = 30_000;
pub const USE_PER_PERSON: f64 = 0.5;
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct HomeWater {
    pub stored: f64,
    /// Cumulative modeled sink, never available stock.
    pub consumed: f64,
    pub missed_portions: u64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Households {
    homes: BTreeMap<u64, HomeWater>,
    elapsed_ms: u64,
    initialized: bool,
}
impl Default for Households {
    fn default() -> Self {
        Self {
            homes: BTreeMap::new(),
            elapsed_ms: 0,
            initialized: true,
        }
    }
}
impl Households {
    pub(crate) fn missing() -> Self {
        Self {
            initialized: false,
            ..Self::default()
        }
    }
    pub(crate) fn is_missing(&self) -> bool {
        *self == Self::missing()
    }
    /// Logical time remaining until the next household consumption boundary.
    pub fn next_use_ms(&self) -> u64 {
        USE_INTERVAL_MS - self.elapsed_ms
    }
    pub fn get(&self, home: u64) -> Option<&HomeWater> {
        self.homes.get(&home)
    }
    pub fn all(&self) -> &BTreeMap<u64, HomeWater> {
        &self.homes
    }
    pub(crate) fn initialize(&mut self, people: &People) {
        for p in people.all() {
            self.homes.entry(p.home).or_default();
        }
    }
    pub(crate) fn advance(&mut self, people: &People, dt: f32) {
        if !dt.is_finite() || dt <= 0. || people.all().is_empty() {
            return;
        }
        self.initialize(people);
        let ms = (f64::from(dt) * 1000.).round() as u64;
        let elapsed = self.elapsed_ms.saturating_add(ms);
        let portions = elapsed / USE_INTERVAL_MS;
        self.elapsed_ms = elapsed % USE_INTERVAL_MS;
        if portions == 0 {
            return;
        }
        for (&home, stock) in &mut self.homes {
            let residents = people.all().iter().filter(|p| p.home == home).count() as u64;
            let count = portions.saturating_mul(residents);
            let demand = count as f64 * USE_PER_PERSON;
            let used = stock.stored.min(demand);
            stock.stored -= used;
            stock.consumed += used;
            stock.missed_portions = stock
                .missed_portions
                .saturating_add(((demand - used) / USE_PER_PERSON).ceil() as u64);
        }
    }
    pub(crate) fn deliver(&mut self, home: u64, amount: f64) -> bool {
        let Some(stock) = self.homes.get_mut(&home) else {
            return false;
        };
        if !amount.is_finite() || amount <= 0. || stock.stored + amount > CAPACITY {
            return false;
        }
        stock.stored += amount;
        true
    }
    pub(crate) fn valid(&self, sim: &SimState) -> bool {
        let collected: f64 = sim
            .subsurface
            .wells
            .iter()
            .filter_map(|w| w.supply.as_ref())
            .map(|w| w.collected)
            .sum();
        let accounted: f64 = self
            .homes
            .values()
            .map(|h| h.stored + h.consumed)
            .sum::<f64>()
            + sim
                .people
                .all()
                .iter()
                .map(|p| p.carried_water)
                .sum::<f64>()
            + crate::food::cultivation::irrigation::total_applied(sim);
        (collected - accounted).abs() <= 1e-8 * (1. + collected)
            && self.initialized
            && self.elapsed_ms < USE_INTERVAL_MS
            && self.homes.len() <= 10_000
            && self.homes.iter().all(|(id, w)| {
                sim.buildings.iter().any(|b| b.id == *id)
                    && w.stored.is_finite()
                    && (0. ..=CAPACITY).contains(&w.stored)
                    && w.consumed.is_finite()
                    && (0. ..=1e15).contains(&w.consumed)
            })
            && sim
                .people
                .all()
                .iter()
                .all(|p| self.homes.contains_key(&p.home))
    }
}
