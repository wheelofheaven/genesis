//! One repeating gameplay crop cycle, using climate moisture as a condition, not water stock.
//! Barley automatically resows after clearing the standing harvest in this initial model.
mod placement;
use super::{Source, SourceId};
use crate::{SimState, events::SimEvent, spatial, spatial::Position, units::metres};
pub(crate) use placement::overlaps_building;
pub use placement::{Error, footprint, validate};
use serde::{Deserialize, Serialize};

pub const RADIUS: f32 = metres(2.);
pub const YIELD: u32 = 24;
pub const GROWTH_UNITS: u64 = 60_000 * 1000;
pub const MAX_PLOTS: usize = 64;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum Kind {
    /// Only legacy migration may resolve this missing discriminator.
    #[default]
    Legacy,
    Wild,
    Barley(Crop),
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Crop {
    /// Accumulated logical milliseconds weighted by an integer 0..1000 growth rate.
    pub growth_units: u64,
    /// Cumulative portions created, including stock still standing in the plot.
    pub produced: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Growing,
    Dry,
    Blocked,
    Harvestable,
}
impl Status {
    pub fn label(self) -> &'static str {
        match self {
            Self::Growing => "Growing",
            Self::Dry => "Too dry · growth paused",
            Self::Blocked => "Flooded, steep or obstructed · growth and harvest paused",
            Self::Harvestable => "Harvestable",
        }
    }
}
/// Read the same moisture proxy used by ecology; no separate moisture authority.
pub fn moisture(sim: &SimState, p: Position) -> f32 {
    spatial::sample(&sim.grid, p, |c| c.moisture).clamp(0., 1.)
}
fn rate(sim: &SimState, p: Position) -> u64 {
    (((moisture(sim, p) - 0.2) / 0.4).clamp(0., 1.) * 1000.).round() as u64
}
pub fn status(sim: &SimState, source: &Source) -> Status {
    if placement::ground(sim, source.position).is_err() {
        Status::Blocked
    } else if source.remaining > 0 {
        Status::Harvestable
    } else if rate(sim, source.position) == 0 {
        Status::Dry
    } else {
        Status::Growing
    }
}
/// Same harvestability query for task admission and pickup after terrain changes.
pub fn accessible(sim: &SimState, source: &Source) -> bool {
    if matches!(source.kind, Kind::Barley(_)) {
        placement::ground(sim, source.position).is_ok()
    } else {
        crate::surface::walking::standable(sim, source.position)
    }
}
pub(crate) fn plant(sim: &mut SimState, p: Position) -> Result<SourceId, Error> {
    validate(sim, p)?;
    let id = sim.food.ids.allocate().ok_or(Error::Capacity)?;
    sim.food.sources.push(Source {
        id: SourceId(id),
        position: p,
        remaining: 0,
        initial: 0,
        kind: Kind::Barley(Crop::default()),
    });
    Ok(SourceId(id))
}
pub(crate) fn advance(sim: &mut SimState, dt: f32, events: &mut Vec<SimEvent>) {
    if !dt.is_finite() || dt <= 0. {
        return;
    }
    let ms = (f64::from(dt) * 1000.).round() as u64;
    for i in 0..sim.food.sources.len() {
        let source = &sim.food.sources[i];
        if !matches!(source.kind, Kind::Barley(_)) || source.remaining > 0 {
            continue;
        }
        let rate = if status(sim, source) == Status::Growing {
            rate(sim, source.position)
        } else {
            0
        };
        let source = &mut sim.food.sources[i];
        let Kind::Barley(crop) = &mut source.kind else {
            continue;
        };
        crop.growth_units = crop
            .growth_units
            .saturating_add(ms.saturating_mul(rate))
            .min(GROWTH_UNITS);
        if crop.growth_units == GROWTH_UNITS {
            crop.growth_units = 0;
            crop.produced += u64::from(YIELD);
            source.remaining = YIELD;
            events.push(SimEvent::CropMatured {
                source: source.id,
                portions: YIELD,
                time: sim.time + f64::from(dt),
            });
        }
    }
}

#[cfg(test)]
mod tests;
