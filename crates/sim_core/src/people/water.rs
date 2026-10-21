//! Small collection transactions. A task owns claims; a person owns water in transit.
use super::{Activity, People, Person, PersonId, navigation};
use crate::{
    SimState, events::SimEvent, households, paths::distance, spatial::Position, surface::walking,
    units::metres,
};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WaterTask {
    pub well: u64,
    pub pickup: Position,
    pub amount: f64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WaterIssue {
    NoWorkingWell,
    WellEmpty,
    NoReachableWell,
    WellStopped,
    RouteBlocked,
    DeliveryBlocked,
    Cancelled,
}
impl WaterIssue {
    pub fn label(self) -> &'static str {
        match self {
            Self::NoWorkingWell => "Household needs water; drill or resume a nearby well",
            Self::WellEmpty => "Waiting for enough unreserved well water",
            Self::NoReachableWell => "No reachable well within the local walking range",
            Self::WellStopped => "Collection cancelled: well stopped or submerged",
            Self::DeliveryBlocked => "Carrying water; the route home is blocked",
            Self::RouteBlocked => "Collection released: route blocked",
            Self::Cancelled => "Collection cancelled; returning home",
        }
    }
}
pub(super) fn working(sim: &SimState, id: u64) -> bool {
    sim.subsurface
        .wells
        .iter()
        .find(|w| w.id == id)
        .is_some_and(|w| {
            w.active && crate::spatial::elevation(&sim.grid, w.position) > sim.sea_level + 0.08
        })
}
impl People {
    /// Derived reservation totals have no independently mutable copy.
    pub fn reserved_water(&self, well: u64) -> f64 {
        self.entries
            .iter()
            .filter(|p| p.activity == Activity::FetchingWater)
            .filter_map(|p| p.water_task.as_ref())
            .filter(|t| t.well == well)
            .map(|t| t.amount)
            .sum::<f64>()
            + self
                .entries
                .iter()
                .filter(|p| p.activity == Activity::FetchingIrrigationWater)
                .filter_map(|p| p.irrigation_task.as_ref())
                .filter(|t| t.well == well)
                .count() as f64
                * households::CONTAINER
    }
    pub fn home_collector(&self, home: u64) -> Option<PersonId> {
        self.entries
            .iter()
            .find(|p| p.home == home && p.water_task.is_some())
            .map(|p| p.id)
    }
    pub(crate) fn cancel_water(&mut self, id: PersonId) -> bool {
        let Some(p) = self
            .entries
            .iter_mut()
            .find(|p| p.id == id && p.water_task.is_some())
        else {
            return false;
        };
        release(p, WaterIssue::Cancelled);
        true
    }
    pub(crate) fn prepare_water(&mut self, index: usize, sim: &SimState, ms: u64) {
        let p = &self.entries[index];
        if p.food_task.is_some() {
            return;
        }
        if let Some(t) = &p.water_task {
            if p.activity == Activity::FetchingWater && !working(sim, t.well) {
                release(&mut self.entries[index], WaterIssue::WellStopped);
            }
            return;
        }
        if p.activity != Activity::Resting || p.remaining_ms > ms {
            return;
        }
        let Some(home) = sim.households.get(p.home) else {
            return;
        };
        if home.stored >= households::REFILL_BELOW || self.home_collector(p.home).is_some() {
            return;
        }
        let mut wells: Vec<_> = sim
            .subsurface
            .wells
            .iter()
            .filter(|w| {
                w.active && crate::spatial::elevation(&sim.grid, w.position) > sim.sea_level + 0.08
            })
            .collect();
        wells.sort_by(|a, b| {
            distance(p.position, a.position)
                .total_cmp(&distance(p.position, b.position))
                .then(a.id.cmp(&b.id))
        });
        let mut issue = WaterIssue::NoWorkingWell;
        // Bounded source search; stable distance/ID order is gameplay policy.
        for well in wells.into_iter().take(8) {
            if distance(p.position, well.position) > metres(50.) {
                issue = WaterIssue::NoReachableWell;
                continue;
            }
            let available =
                well.supply.as_ref().map_or(0., |s| s.stored) - self.reserved_water(well.id);
            if available < households::CONTAINER {
                issue = WaterIssue::WellEmpty;
                continue;
            }
            issue = WaterIssue::NoReachableWell;
            for i in 0..16 {
                let a = i as f32 * std::f32::consts::TAU / 16.;
                let pickup = Position::new(
                    well.position.x + a.cos() * metres(0.8),
                    well.position.z + a.sin() * metres(0.8),
                );
                if !walking::standable(sim, pickup) {
                    continue;
                }
                let Some(route) = navigation::route(sim, p.position, pickup) else {
                    continue;
                };
                let resident = &mut self.entries[index];
                resident.water_task = Some(WaterTask {
                    well: well.id,
                    pickup,
                    amount: households::CONTAINER,
                });
                resident.activity = Activity::FetchingWater;
                resident.route = route;
                resident.next = 0;
                resident.remaining_ms = 0;
                resident.blocked = None;
                resident.water_issue = None;
                return;
            }
        }
        self.entries[index].water_issue = Some(issue);
    }
    pub(crate) fn water_valid(&self, sim: &SimState) -> bool {
        let mut claimed = std::collections::BTreeSet::new();
        self.entries.iter().all(|p| {
            if let Some(t) = &p.water_task {
                let pickup = t.pickup;
                claimed.insert(p.home)
                    && t.amount == households::CONTAINER
                    && pickup.x.is_finite()
                    && pickup.z.is_finite()
                    && pickup.x >= 0.
                    && pickup.z >= 0.
                    && pickup.x < sim.grid.width as f32
                    && pickup.z < sim.grid.height as f32
                    && sim
                        .subsurface
                        .wells
                        .iter()
                        .any(|w| w.id == t.well && distance(w.position, pickup) <= metres(1.))
                    && sim
                        .households
                        .get(p.home)
                        .is_some_and(|h| h.stored + t.amount <= households::CAPACITY)
                    && match p.activity {
                        Activity::FetchingWater => p.carried_water == 0.,
                        Activity::CarryingWater => p.carried_water == t.amount,
                        _ => false,
                    }
            } else {
                (p.carried_water == 0. || p.irrigation_task.is_some())
                    && !matches!(
                        p.activity,
                        Activity::FetchingWater | Activity::CarryingWater
                    )
            }
        }) && sim
            .subsurface
            .wells
            .iter()
            .all(|w| self.reserved_water(w.id) <= w.supply.as_ref().map_or(0., |s| s.stored))
    }
    pub(crate) fn legacy_water(&self) -> bool {
        self.entries.iter().all(|p| {
            p.water_task.is_none()
                && p.carried_water == 0.
                && p.water_issue.is_none()
                && !matches!(
                    p.activity,
                    Activity::FetchingWater | Activity::CarryingWater
                )
        })
    }
}
/// Before collection, cancellation releases both claims. After collection, the
/// delivery commitment survives cancellation so neither cargo nor capacity vanishes.
pub(super) fn release(p: &mut Person, issue: WaterIssue) {
    if p.carried_water == 0. {
        p.water_task = None;
        p.activity = Activity::ReturningHome;
    } else {
        p.activity = Activity::CarryingWater;
    }
    p.route.clear();
    p.next = 0;
    p.remaining_ms = 2000;
    p.water_issue = Some(
        if p.carried_water > 0. && issue == WaterIssue::RouteBlocked {
            WaterIssue::DeliveryBlocked
        } else {
            issue
        },
    );
}
pub(super) fn arrive(
    p: &mut Person,
    sim: &mut SimState,
    events: &mut Vec<SimEvent>,
    time: f64,
) -> bool {
    if p.activity == Activity::FetchingWater {
        let Some(t) = p.water_task.clone() else {
            return false;
        };
        if !working(sim, t.well) {
            release(p, WaterIssue::WellStopped);
            return true;
        }
        let collected = sim
            .subsurface
            .wells
            .iter_mut()
            .find(|w| w.id == t.well)
            .and_then(|w| w.supply.as_mut())
            .is_some_and(|s| s.collect(t.amount));
        if collected {
            p.carried_water = t.amount;
            p.activity = Activity::CarryingWater;
            p.route.clear();
            p.next = 0;
            p.remaining_ms = 0;
            events.push(SimEvent::WaterCollected {
                person: p.id,
                well: t.well,
                amount: t.amount,
                time,
            });
        } else {
            release(p, WaterIssue::WellEmpty);
        }
        return true;
    }
    if p.activity == Activity::CarryingWater {
        if sim.households.deliver(p.home, p.carried_water) {
            events.push(SimEvent::WaterDelivered {
                person: p.id,
                home: p.home,
                amount: p.carried_water,
                time,
            });
            p.last_supply = super::SupplyKind::Water;
            p.carried_water = 0.;
            p.water_task = None;
            p.water_issue = None;
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
