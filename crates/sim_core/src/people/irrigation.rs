//! Opt-in crop water hauling, admitted after the resident's own household jobs.
//! A task reserves one dose at the well and one dose of capacity at the plot.
#[cfg(test)]
mod tests;
mod validation;
use super::{Activity, People, Person, PersonId, navigation, water};
use crate::{
    SimState,
    events::SimEvent,
    food::{
        SourceId,
        cultivation::{self, Kind, irrigation as soil},
    },
    households,
    paths::distance,
    spatial::Position,
    surface::walking,
    units::metres,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IrrigationTask {
    pub source: SourceId,
    pub well: u64,
    pub pickup: Position,
    pub delivery: Position,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IrrigationIssue {
    Cancelled,
    Unavailable,
    RouteBlocked,
    DeliveryBlocked,
}
impl IrrigationIssue {
    pub fn label(self) -> &'static str {
        match self {
            Self::Cancelled => "Plot tending stopped; uncollected water released",
            Self::Unavailable => "Collection released: well or plot unavailable",
            Self::RouteBlocked => "Crop water collection released: route blocked",
            Self::DeliveryBlocked => "Carrying crop water; delivery blocked, retrying",
        }
    }
}
impl People {
    pub fn irrigator(&self, source: SourceId) -> Option<PersonId> {
        self.entries
            .iter()
            .find(|p| {
                p.irrigation_task
                    .as_ref()
                    .is_some_and(|t| t.source == source)
            })
            .map(|p| p.id)
    }
    pub fn reserved_irrigation(&self, source: SourceId) -> u64 {
        self.entries
            .iter()
            .filter_map(|p| p.irrigation_task.as_ref())
            .filter(|t| t.source == source)
            .count() as u64
            * soil::DOSE
    }
    pub(super) fn prepare_irrigation(&mut self, index: usize, sim: &SimState, ms: u64) {
        let p = &self.entries[index];
        if let Some(task) = &p.irrigation_task {
            // A collected dose must finish delivery even if tending is disabled,
            // the well stops, or the plot matures while the resident is walking.
            if p.carried_water == 0. {
                let enabled = sim
                    .food
                    .source(task.source)
                    .is_some_and(|s| matches!(&s.kind, Kind::Barley(c) if c.tending));
                let issue = if !enabled {
                    Some(IrrigationIssue::Cancelled)
                } else if !water::working(sim, task.well)
                    || sim
                        .food
                        .source(task.source)
                        .is_none_or(|s| !cultivation::accessible(sim, s))
                {
                    Some(IrrigationIssue::Unavailable)
                } else {
                    None
                };
                if let Some(issue) = issue {
                    release(&mut self.entries[index], issue);
                }
            }
            return;
        }
        if p.water_task.is_some()
            || p.food_task.is_some()
            || p.activity != Activity::Resting
            || p.remaining_ms > ms
        {
            return;
        }
        let mut plots: Vec<_> = sim
            .food
            .sources()
            .iter()
            .filter(|s| {
                matches!(&s.kind, Kind::Barley(c) if c.tending
                && c.soil_water.as_ref().is_some_and(|w| w.stored <= soil::UNITS
                    && w.applied <= 1_000_000_000_000_000 - soil::DOSE))
                    && s.remaining == 0
                    && self.irrigator(s.id).is_none()
                    && distance(p.position, s.position) <= metres(50.)
                    && cultivation::moisture(sim, s.position) < 0.6
                    && cultivation::accessible(sim, s)
            })
            .collect();
        plots.sort_by(|a, b| {
            distance(p.position, a.position)
                .total_cmp(&distance(p.position, b.position))
                .then(a.id.cmp(&b.id))
        });
        // Bound the route search just like food/water admission. One worker per
        // plot prevents repeated claims and keeps its reserved capacity simple.
        let mut searched_wells = 0;
        for plot in plots.into_iter().take(8) {
            let mut wells: Vec<_> = sim
                .subsurface
                .wells
                .iter()
                .filter(|w| {
                    water::working(sim, w.id)
                        && distance(w.position, plot.position) <= soil::RANGE
                        && distance(p.position, w.position) <= metres(50.)
                        && w.supply.as_ref().map_or(0., |s| s.stored) - self.reserved_water(w.id)
                            >= households::CONTAINER
                })
                .collect();
            wells.sort_by(|a, b| {
                distance(plot.position, a.position)
                    .total_cmp(&distance(plot.position, b.position))
                    .then(a.id.cmp(&b.id))
            });
            for well in wells.into_iter().take(8) {
                if searched_wells == 8 {
                    return;
                }
                searched_wells += 1;
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
                    if navigation::route(sim, pickup, plot.position).is_none() {
                        continue;
                    }
                    let p = &mut self.entries[index];
                    p.irrigation_task = Some(IrrigationTask {
                        source: plot.id,
                        well: well.id,
                        pickup,
                        delivery: plot.position,
                    });
                    p.activity = Activity::FetchingIrrigationWater;
                    p.irrigation_issue = None;
                    p.route = route;
                    p.next = 0;
                    p.remaining_ms = 0;
                    p.blocked = None;
                    return;
                }
            }
        }
    }
}
pub(super) fn release(p: &mut Person, issue: IrrigationIssue) {
    if p.carried_water == 0. {
        p.irrigation_task = None;
        p.activity = Activity::ReturningHome;
        p.irrigation_issue = Some(issue);
    } else {
        p.activity = Activity::WateringCrop;
        p.irrigation_issue = Some(IrrigationIssue::DeliveryBlocked);
    }
    p.route.clear();
    p.next = 0;
    p.remaining_ms = 2000;
}
pub(super) fn arrive(
    p: &mut Person,
    sim: &mut SimState,
    events: &mut Vec<SimEvent>,
    time: f64,
) -> bool {
    let Some(task) = p.irrigation_task.clone() else {
        return false;
    };
    if p.activity == Activity::FetchingIrrigationWater {
        if water::working(sim, task.well)
            && sim
                .subsurface
                .wells
                .iter_mut()
                .find(|w| w.id == task.well)
                .and_then(|w| w.supply.as_mut())
                .is_some_and(|s| s.collect(households::CONTAINER))
        {
            p.carried_water = households::CONTAINER;
            p.activity = Activity::WateringCrop;
            p.route.clear();
            p.next = 0;
            p.remaining_ms = 0;
            events.push(SimEvent::WaterCollected {
                person: p.id,
                well: task.well,
                amount: p.carried_water,
                time,
            });
        } else {
            release(p, IrrigationIssue::Unavailable);
        }
        return true;
    }
    if p.activity == Activity::WateringCrop {
        if soil::deliver(sim, task.source) {
            events.push(SimEvent::CropWatered {
                person: p.id,
                source: task.source,
                amount: p.carried_water,
                time,
            });
            p.carried_water = 0.;
            p.irrigation_task = None;
            p.irrigation_issue = None;
            p.activity = Activity::ReturningHome;
            p.route.clear();
            p.next = 0;
            p.remaining_ms = 0;
            p.blocked = None;
        } else {
            release(p, IrrigationIssue::DeliveryBlocked);
        }
        return true;
    }
    false
}
