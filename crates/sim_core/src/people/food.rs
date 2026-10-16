//! A second explicit task: reserve, walk, gather, carry, deliver.
mod validation;
use super::{Activity, People, Person, PersonId, navigation};
use crate::{
    SimState, events::SimEvent, food as stock, paths::distance, spatial::Position, units::metres,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SupplyKind {
    Water,
    #[default]
    Food,
    /// Decode-only sentinel; accepted solely by the pre-food save migration.
    Uninitialized,
}
impl SupplyKind {
    pub(crate) fn missing() -> Self {
        Self::Uninitialized
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoodTask {
    pub source: stock::SourceId,
    pub pickup: Position,
    pub amount: u32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FoodIssue {
    NoSource,
    Growing,
    Dry,
    SourceBlocked,
    Empty,
    NoRoute,
    RouteBlocked,
    DeliveryBlocked,
    Cancelled,
}
impl FoodIssue {
    pub fn label(self) -> &'static str {
        match self {
            Self::NoSource => "No food source in this world",
            Self::Growing => "Waiting for crops to mature",
            Self::Dry => "Crop growth paused by dry ground",
            Self::SourceBlocked => "Crop ground is flooded or obstructed",
            Self::Empty => "Not enough unreserved food remains",
            Self::NoRoute => "No reachable food source within local walking range",
            Self::RouteBlocked => "Gathering released: route or source blocked",
            Self::DeliveryBlocked => "Carrying food; the route home is blocked",
            Self::Cancelled => "Gathering cancelled; returning home",
        }
    }
}
impl People {
    /// Claims against source stock; carried portions are no longer reserved there.
    pub fn reserved_food(&self, source: stock::SourceId) -> u32 {
        self.entries
            .iter()
            .filter(|p| p.carried_food == 0)
            .filter_map(|p| p.food_task.as_ref())
            .filter(|t| t.source == source)
            .map(|t| t.amount)
            .sum()
    }
    /// The active collection/delivery claim for this dwelling, if any.
    pub fn food_collector(&self, home: u64) -> Option<PersonId> {
        self.entries
            .iter()
            .find(|p| p.home == home && p.food_task.is_some())
            .map(|p| p.id)
    }
    pub(crate) fn cancel_food(&mut self, id: PersonId) -> bool {
        let Some(p) = self
            .entries
            .iter_mut()
            .find(|p| p.id == id && p.food_task.is_some())
        else {
            return false;
        };
        release(p, FoodIssue::Cancelled);
        true
    }
    pub(crate) fn prepare_supplies(&mut self, index: usize, sim: &SimState, ms: u64) {
        let p = &self.entries[index];
        if p.water_task.is_some() {
            self.prepare_water(index, sim, ms);
            return;
        }
        if let Some(task) = &p.food_task {
            if p.carried_food == 0
                && sim
                    .food
                    .source(task.source)
                    .is_none_or(|s| !stock::cultivation::accessible(sim, s))
            {
                release(&mut self.entries[index], FoodIssue::RouteBlocked);
            }
            return;
        }
        // Lower fraction of the refill threshold first; alternate equal reserves.
        // Failed admission falls through to the other resource at this same boundary.
        let food = sim
            .food
            .home(p.home)
            .map_or(1., |h| f64::from(h.stored) / f64::from(stock::REFILL_BELOW));
        let water = sim
            .households
            .get(p.home)
            .map_or(1., |h| h.stored / crate::households::REFILL_BELOW);
        let food_first = food < water || (food == water && p.last_supply == SupplyKind::Water);
        if food_first {
            self.prepare_food(index, sim, ms);
        }
        self.prepare_water(index, sim, ms);
        if !food_first {
            self.prepare_food(index, sim, ms);
        }
    }
    fn prepare_food(&mut self, index: usize, sim: &SimState, ms: u64) {
        let p = &self.entries[index];
        if p.water_task.is_some()
            || p.food_task.is_some()
            || p.activity != Activity::Resting
            || p.remaining_ms > ms
        {
            return;
        }
        if sim
            .food
            .home(p.home)
            .is_none_or(|h| h.stored >= stock::REFILL_BELOW)
            || self.food_collector(p.home).is_some()
        {
            return;
        }
        let mut sources: Vec<_> = sim.food.sources().iter().collect();
        sources.sort_by(|a, b| {
            distance(p.position, a.position)
                .total_cmp(&distance(p.position, b.position))
                .then(a.id.cmp(&b.id))
        });
        let mut issue = FoodIssue::NoSource;
        for source in sources.into_iter().take(8) {
            if source
                .remaining
                .saturating_sub(self.reserved_food(source.id))
                < stock::LOAD
            {
                issue = if matches!(source.kind, stock::cultivation::Kind::Barley(_))
                    && source.remaining == 0
                {
                    match stock::cultivation::status(sim, source) {
                        stock::cultivation::Status::Dry => FoodIssue::Dry,
                        stock::cultivation::Status::Blocked => FoodIssue::SourceBlocked,
                        _ => FoodIssue::Growing,
                    }
                } else {
                    FoodIssue::Empty
                };
                continue;
            }
            issue = FoodIssue::NoRoute;
            if distance(p.position, source.position) > metres(50.)
                || !stock::cultivation::accessible(sim, source)
            {
                continue;
            }
            let Some(route) = navigation::route(sim, p.position, source.position) else {
                continue;
            };
            let p = &mut self.entries[index];
            p.food_task = Some(FoodTask {
                source: source.id,
                pickup: source.position,
                amount: stock::LOAD,
            });
            p.activity = Activity::FetchingFood;
            p.route = route;
            p.next = 0;
            p.remaining_ms = 0;
            p.blocked = None;
            p.food_issue = None;
            return;
        }
        self.entries[index].food_issue = Some(issue);
    }
}
pub(super) fn release(p: &mut Person, issue: FoodIssue) {
    if p.carried_food == 0 {
        p.food_task = None;
        p.activity = Activity::ReturningHome;
    } else {
        p.activity = Activity::CarryingFood;
    }
    p.route.clear();
    p.next = 0;
    p.remaining_ms = 2000;
    p.food_issue = Some(if p.carried_food > 0 && issue == FoodIssue::RouteBlocked {
        FoodIssue::DeliveryBlocked
    } else {
        issue
    });
}
pub(super) fn arrive(
    p: &mut Person,
    sim: &mut SimState,
    events: &mut Vec<SimEvent>,
    time: f64,
) -> bool {
    if p.activity == Activity::FetchingFood {
        p.activity = Activity::GatheringFood;
        p.remaining_ms = stock::GATHER_MS;
        p.route.clear();
        p.next = 0;
        p.blocked = None;
        return true;
    }
    if p.activity == Activity::GatheringFood {
        let Some(t) = p.food_task.clone() else {
            return false;
        };
        if sim
            .food
            .source(t.source)
            .is_none_or(|s| !stock::cultivation::accessible(sim, s))
        {
            release(p, FoodIssue::RouteBlocked);
        } else if sim.food.collect(t.source, t.amount) {
            p.carried_food = t.amount;
            p.activity = Activity::CarryingFood;
            p.route.clear();
            p.next = 0;
            p.remaining_ms = 0;
            events.push(SimEvent::FoodCollected {
                person: p.id,
                source: t.source,
                amount: t.amount,
                time,
            });
        } else {
            release(p, FoodIssue::Empty);
        }
        return true;
    }
    if p.activity == Activity::CarryingFood {
        if sim.food.deliver(p.home, p.carried_food) {
            events.push(SimEvent::FoodDelivered {
                person: p.id,
                home: p.home,
                amount: p.carried_food,
                time,
            });
            p.carried_food = 0;
            p.food_task = None;
            p.food_issue = None;
            p.last_supply = SupplyKind::Food;
            p.activity = Activity::Resting;
            p.remaining_ms = 12000;
            p.route.clear();
            p.next = 0;
            p.blocked = None;
        }
        return true;
    }
    false
}
