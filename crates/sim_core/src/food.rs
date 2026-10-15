//! Finite wild-food sources and exact household portions; no decorative biomass ownership.
mod validation;
use crate::{SimState, identity::Allocator, people::People, spatial::Position};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
pub const CAPACITY: u32 = 12;
pub const REFILL_BELOW: u32 = 6;
pub const LOAD: u32 = 4;
pub const GATHER_MS: u64 = 2000;

/// Durable source identity, independent of its storage position or display mesh.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SourceId(pub u64);
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Source {
    pub id: SourceId,
    pub position: Position,
    pub remaining: u32,
    /// Initial finite harvest, for conservation accounting; not an inventory.
    pub initial: u32,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct HomeFood {
    pub stored: u32,
    pub consumed: u64,
    pub missed_portions: u64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Food {
    pub(crate) sources: Vec<Source>,
    pub(crate) homes: BTreeMap<u64, HomeFood>,
    ids: Allocator,
    elapsed_ms: u64,
    initialized: bool,
}
impl Default for Food {
    fn default() -> Self {
        Self {
            sources: Vec::new(),
            homes: BTreeMap::new(),
            ids: Allocator::new(),
            elapsed_ms: 0,
            initialized: true,
        }
    }
}
impl Food {
    pub(crate) fn missing() -> Self {
        Self {
            initialized: false,
            ..Self::default()
        }
    }
    pub(crate) fn is_missing(&self) -> bool {
        *self == Self::missing()
    }
    /// Committed source stocks; reservations remain owned by resident tasks.
    pub fn sources(&self) -> &[Source] {
        &self.sources
    }
    /// Resolve a persistent source, returning None for stale identity.
    pub fn source(&self, id: SourceId) -> Option<&Source> {
        self.sources.iter().find(|s| s.id == id)
    }
    /// Stock owned by a dwelling, addressed by its persistent building ID.
    pub fn home(&self, id: u64) -> Option<&HomeFood> {
        self.homes.get(&id)
    }
    /// Read-only stocks in canonical home-ID order.
    pub fn homes(&self) -> &BTreeMap<u64, HomeFood> {
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
        let portions = elapsed / 30_000;
        self.elapsed_ms = elapsed % 30_000;
        if portions == 0 {
            return;
        }
        for (&home, stock) in &mut self.homes {
            let count = people.all().iter().filter(|p| p.home == home).count() as u64;
            let demand = portions.saturating_mul(count);
            let used = demand.min(u64::from(stock.stored));
            stock.stored -= used as u32;
            stock.consumed += used;
            stock.missed_portions = stock.missed_portions.saturating_add(demand - used);
        }
    }
    pub(crate) fn collect(&mut self, id: SourceId, amount: u32) -> bool {
        let Some(source) = self.sources.iter_mut().find(|s| s.id == id) else {
            return false;
        };
        if amount == 0 || source.remaining < amount {
            return false;
        }
        source.remaining -= amount;
        true
    }
    pub(crate) fn deliver(&mut self, home: u64, amount: u32) -> bool {
        let Some(stock) = self.homes.get_mut(&home) else {
            return false;
        };
        if amount == 0 || amount > CAPACITY - stock.stored {
            return false;
        }
        stock.stored += amount;
        true
    }
}

/// Scenario-only initial harvest. Existing saves never acquire a new source on load.
pub(crate) fn populate(sim: &mut SimState) {
    use crate::{people::navigation, surface::walking, units::metres};
    sim.food.initialize(&sim.people);
    if !sim.food.sources.is_empty() {
        return;
    }
    let Some(first) = sim.people.all().first().map(|p| p.home_position) else {
        return;
    };
    let point = [6., 10., 14.]
        .into_iter()
        .flat_map(|r| {
            (0..32).map(move |i| {
                let a = i as f32 * std::f32::consts::TAU / 32.;
                Position::new(first.x + a.cos() * metres(r), first.z + a.sin() * metres(r))
            })
        })
        .find(|&p| walking::standable(sim, p) && navigation::route(sim, first, p).is_some());
    if let Some(position) = point
        && let Some(id) = sim.food.ids.allocate()
    {
        sim.food.sources.push(Source {
            id: SourceId(id),
            position,
            remaining: 256,
            initial: 256,
        });
    }
}
