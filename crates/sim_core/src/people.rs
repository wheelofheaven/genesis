//! Persistent residents and a modest outdoor routine, independent of presentation.
mod food;
mod irrigation;
pub use irrigation::{IrrigationIssue, IrrigationTask};
pub(crate) mod navigation;
pub use food::{FoodIssue, FoodTask, SupplyKind};
mod water;
pub use water::{WaterIssue, WaterTask};
#[cfg(test)]
mod food_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod water_tests;
use crate::{
    SimState, events::SimEvent, identity::Allocator, paths::distance, spatial::Position,
    surface::walking, units::metres,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PersonId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Activity {
    Resting,
    FetchingFood,
    GatheringFood,
    CarryingFood,
    FetchingWater,
    CarryingWater,
    FetchingIrrigationWater,
    WateringCrop,
    GoingToMeeting,
    Visiting,
    ReturningHome,
}
impl Activity {
    pub fn label(self) -> &'static str {
        match self {
            Self::FetchingIrrigationWater => "Collecting crop water",
            Self::WateringCrop => "Carrying water to a crop",
            Self::FetchingWater => "Collecting household water",
            Self::CarryingWater => "Carrying water home",
            Self::FetchingFood => "Walking to gather food",
            Self::GatheringFood => "Harvesting food",
            Self::CarryingFood => "Carrying food home",
            Self::Resting => "Resting at home",
            Self::GoingToMeeting => "Walking to the meeting place",
            Self::Visiting => "Visiting the meeting place",
            Self::ReturningHome => "Returning home",
        }
    }
    fn travelling(self) -> bool {
        matches!(
            self,
            Self::GoingToMeeting
                | Self::ReturningHome
                | Self::FetchingWater
                | Self::CarryingWater
                | Self::FetchingIrrigationWater
                | Self::WateringCrop
                | Self::FetchingFood
                | Self::CarryingFood
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Blocked {
    MissingHome,
    UnsafeGround,
    NoLocalRoute,
}
impl Blocked {
    pub fn label(self) -> &'static str {
        match self {
            Self::MissingHome => "Home no longer exists",
            Self::UnsafeGround => "Ground underfoot is no longer safe",
            Self::NoLocalRoute => "No clear local route; retrying every two seconds",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Person {
    pub id: PersonId,
    pub name: String,
    pub home: u64,
    pub position: Position,
    pub home_position: Position,
    pub meeting: Position,
    pub activity: Activity,
    pub blocked: Option<Blocked>,
    #[serde(default)]
    pub water_task: Option<WaterTask>,
    #[serde(default)]
    pub irrigation_task: Option<IrrigationTask>,
    #[serde(default)]
    pub irrigation_issue: Option<IrrigationIssue>,
    #[serde(default)]
    pub carried_water: f64,
    #[serde(default)]
    pub water_issue: Option<WaterIssue>,
    #[serde(default)]
    pub food_task: Option<FoodTask>,
    #[serde(default)]
    pub carried_food: u32,
    #[serde(default)]
    pub food_issue: Option<FoodIssue>,
    #[serde(default = "SupplyKind::missing")]
    pub last_supply: SupplyKind,
    /// Remaining logical milliseconds until departure, return, or retry.
    remaining_ms: u64,
    /// Committed continuous route. Every movement still checks current terrain.
    route: Vec<Position>,
    next: usize,
}
impl Person {
    pub fn destination(&self) -> Position {
        if let Some(task) = &self.irrigation_task {
            return if self.activity == Activity::FetchingIrrigationWater {
                task.pickup
            } else {
                task.delivery
            };
        }
        if matches!(
            self.activity,
            Activity::FetchingFood | Activity::GatheringFood
        ) && let Some(task) = &self.food_task
        {
            return task.pickup;
        }
        if self.activity == Activity::FetchingWater
            && let Some(task) = &self.water_task
        {
            return task.pickup;
        }
        if matches!(self.activity, Activity::GoingToMeeting | Activity::Visiting) {
            self.meeting
        } else {
            self.home_position
        }
    }
    pub fn route(&self) -> &[Position] {
        &self.route[self.next..]
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct People {
    entries: Vec<Person>,
    ids: Allocator,
}
impl Default for People {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            ids: Allocator::new(),
        }
    }
}
impl People {
    pub(crate) fn legacy_missing() -> Self {
        Self {
            entries: Vec::new(),
            ids: Allocator::default(),
        }
    }
    pub fn all(&self) -> &[Person] {
        &self.entries
    }
    pub fn get(&self, id: PersonId) -> Option<&Person> {
        self.entries.iter().find(|p| p.id == id)
    }

    pub(crate) fn advance(&mut self, sim: &mut SimState, dt: f32, events: &mut Vec<SimEvent>) {
        if !dt.is_finite() || dt <= 0. {
            return;
        }
        // The shared runtime supplies exactly 100 ms. Legacy direct callers use
        // rounded milliseconds; no host clock or render rate enters this routine.
        let ms = (f64::from(dt) * 1000.).round() as u64;
        self.entries.sort_by_key(|p| p.id);
        for index in 0..self.entries.len() {
            let before = (self.entries[index].activity, self.entries[index].blocked);
            let water_before = self.entries[index].water_issue;
            let food_before = self.entries[index].food_issue;
            let irrigation_before = self.entries[index].irrigation_issue;
            self.prepare_supplies(index, sim, ms);
            let p = &mut self.entries[index];
            if !sim.buildings.iter().any(|b| b.id == p.home) {
                if p.water_task.is_some() {
                    water::release(p, WaterIssue::RouteBlocked);
                }
                if p.irrigation_task.is_some() {
                    irrigation::release(p, IrrigationIssue::RouteBlocked);
                }
                if p.food_task.is_some() {
                    food::release(p, FoodIssue::RouteBlocked);
                }
                p.blocked = Some(Blocked::MissingHome);
            } else if !walking::standable(sim, p.position) {
                if p.water_task.is_some() {
                    water::release(p, WaterIssue::RouteBlocked);
                }
                if p.irrigation_task.is_some() {
                    irrigation::release(p, IrrigationIssue::RouteBlocked);
                }
                if p.food_task.is_some() {
                    food::release(p, FoodIssue::RouteBlocked);
                }
                p.blocked = Some(Blocked::UnsafeGround);
                p.route.clear();
                p.next = 0;
            } else {
                p.remaining_ms = p.remaining_ms.saturating_sub(ms);
                if p.remaining_ms == 0 {
                    if !p.activity.travelling() && p.activity != Activity::GatheringFood {
                        p.activity = if p.activity == Activity::Resting {
                            Activity::GoingToMeeting
                        } else {
                            Activity::ReturningHome
                        };
                        p.route.clear();
                        p.next = 0;
                    }
                    if p.next == p.route.len() {
                        if distance(p.position, p.destination()) < metres(0.01) {
                            if !water::arrive(p, sim, events, sim.time + f64::from(dt))
                                && !food::arrive(p, sim, events, sim.time + f64::from(dt))
                                && !irrigation::arrive(p, sim, events, sim.time + f64::from(dt))
                            {
                                p.activity = if p.activity == Activity::GoingToMeeting {
                                    Activity::Visiting
                                } else {
                                    Activity::Resting
                                };
                                p.remaining_ms = if p.activity == Activity::Visiting {
                                    8000
                                } else {
                                    12000
                                };
                                p.blocked = None;
                            }
                        } else if let Some(route) =
                            navigation::route(sim, p.position, p.destination())
                        {
                            p.route = route;
                            p.next = 0;
                            p.blocked = None;
                        } else {
                            if p.water_task.is_some() {
                                water::release(p, WaterIssue::RouteBlocked);
                            }
                            if p.irrigation_task.is_some() {
                                irrigation::release(p, IrrigationIssue::RouteBlocked);
                            }
                            if p.food_task.is_some() {
                                food::release(p, FoodIssue::RouteBlocked);
                            }
                            p.blocked = Some(Blocked::NoLocalRoute);
                            p.remaining_ms = 2000;
                        }
                    }
                    if p.activity.travelling() && p.next < p.route.len() {
                        let target = p.route[p.next];
                        let d = distance(p.position, target);
                        let t = (metres(1.2) * dt / d.max(1e-8)).min(1.);
                        let next = Position::new(
                            p.position.x + (target.x - p.position.x) * t,
                            p.position.z + (target.z - p.position.z) * t,
                        );
                        let reached = walking::advance(sim, p.position, next);
                        p.position = reached;
                        if distance(reached, next) > metres(0.001) {
                            if p.water_task.is_some() {
                                water::release(p, WaterIssue::RouteBlocked);
                            }
                            p.route.clear();
                            p.next = 0;
                            p.remaining_ms = 2000;
                            if p.irrigation_task.is_some() {
                                irrigation::release(p, IrrigationIssue::RouteBlocked);
                            }
                            if p.food_task.is_some() {
                                food::release(p, FoodIssue::RouteBlocked);
                            }
                            p.blocked = Some(Blocked::NoLocalRoute);
                        } else {
                            p.blocked = None;
                            if distance(reached, target) < metres(0.001) {
                                p.next += 1;
                            }
                        }
                    }
                } else if !p.activity.travelling() && p.activity != Activity::GatheringFood {
                    p.blocked = None;
                }
            }
            if irrigation_before != p.irrigation_issue {
                events.push(SimEvent::IrrigationTaskStatusChanged {
                    person: p.id,
                    issue: p.irrigation_issue,
                    time: sim.time + f64::from(dt),
                });
            }
            if food_before != p.food_issue {
                events.push(SimEvent::FoodTaskStatusChanged {
                    person: p.id,
                    issue: p.food_issue,
                    time: sim.time + f64::from(dt),
                });
            }
            if water_before != p.water_issue {
                events.push(SimEvent::WaterTaskStatusChanged {
                    person: p.id,
                    issue: p.water_issue,
                    time: sim.time + f64::from(dt),
                });
            }
            if before != (p.activity, p.blocked) {
                events.push(SimEvent::PersonRoutineChanged {
                    id: p.id,
                    activity: p.activity,
                    blocked: p.blocked,
                    time: sim.time + f64::from(dt),
                });
            }
        }
    }
    pub(crate) fn valid(&self, sim: &SimState) -> bool {
        let inside = |p: Position| {
            p.x.is_finite()
                && p.z.is_finite()
                && p.x >= 0.
                && p.z >= 0.
                && p.x < sim.grid.width as f32
                && p.z < sim.grid.height as f32
        };
        let mut ids = std::collections::BTreeSet::new();
        self.food_valid(sim)
            && self.water_valid(sim)
            && self.irrigation_valid(sim)
            && self.entries.len() <= 10_000
            && self.entries.iter().map(|p| p.route.len()).sum::<usize>() <= 100_000
            && self.ids.covers(self.entries.iter().map(|p| p.id.0))
            && self.entries.iter().all(|p| {
                ids.insert(p.id)
                    && !p.name.is_empty()
                    && p.name.len() <= 80
                    && sim.buildings.iter().any(|b| b.id == p.home)
                    && inside(p.position)
                    && inside(p.home_position)
                    && inside(p.meeting)
                    && p.remaining_ms <= 12000
                    && p.route.len() <= navigation::MAX_NODES
                    && p.next <= p.route.len()
                    && p.route.iter().copied().all(inside)
                    && (p.activity.travelling() || p.next == p.route.len())
                    && p.route.last().is_none_or(|&q| {
                        distance(q, p.destination()) < metres(0.01) || !p.activity.travelling()
                    })
            })
    }
}

/// Scenario construction only: seed at most six residents on safe outdoor home
/// thresholds. Existing populations are retained; this is never a load migration.
pub(crate) fn populate(sim: &mut SimState) {
    if !sim.people.entries.is_empty() {
        return;
    }
    let names = ["Ada", "Neri", "Tamar", "Ilan", "Mira", "Eli"];
    let homes: Vec<_> = sim
        .buildings
        .iter()
        .take(6)
        .filter_map(|b| {
            let (s, c) = b.rotation.sin_cos();
            let p = Position::new(
                b.position.x - s * metres(3.5),
                b.position.z + c * metres(3.5),
            );
            walking::standable(sim, p).then_some((b.id, p))
        })
        .collect();
    let Some(&(_, first)) = homes.first() else {
        return;
    };
    // A common outdoor meeting point near the first home, chosen deterministically.
    let meeting = (0..32)
        .map(|i| {
            let a = i as f32 * std::f32::consts::TAU / 32.;
            Position::new(
                first.x + a.cos() * metres(4.),
                first.z + a.sin() * metres(4.),
            )
        })
        .find(|&p| walking::standable(sim, p))
        .unwrap_or(first);
    for (i, (home, position)) in homes.into_iter().enumerate() {
        let angle = i as f32 * std::f32::consts::TAU / 6.;
        let spot = Position::new(
            meeting.x + angle.cos() * metres(1.2),
            meeting.z + angle.sin() * metres(1.2),
        );
        let meeting = if walking::standable(sim, spot) {
            spot
        } else {
            meeting
        };
        let Some(id) = sim.people.ids.allocate() else {
            break;
        };
        sim.people.entries.push(Person {
            id: PersonId(id),
            name: names[i].into(),
            home,
            position,
            home_position: position,
            meeting,
            activity: Activity::Resting,
            blocked: None,
            water_task: None,
            irrigation_task: None,
            irrigation_issue: None,
            carried_water: 0.,
            water_issue: None,
            food_task: None,
            carried_food: 0,
            food_issue: None,
            last_supply: SupplyKind::default(),
            remaining_ms: 1000 + i as u64 * 1000,
            route: Vec::new(),
            next: 0,
        });
    }
    sim.households.initialize(&sim.people);
    sim.food.initialize(&sim.people);
}
