//! Explicit expedition transfer from well buffers to a plot-local water store.
//! Units are the existing abstract water units, not litres or calibrated soil physics.
use super::{Kind, SourceId, placement};
use crate::{SimState, paths::distance, spatial, units::metres};
use serde::{Deserialize, Serialize};

pub const UNITS: u64 = 1_000_000;
pub const DOSE: u64 = 2 * UNITS;
pub const CAPACITY: u64 = 4 * UNITS;
pub const RANGE: f32 = metres(30.);
/// Combined drainage/evaporation sink: 0.05 abstract units per logical second.
const LOSS_PER_MS: u64 = 50;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SoilWater {
    pub stored: u64,
    pub applied: u64,
    pub lost: u64,
}
impl SoilWater {
    pub(crate) fn valid(&self) -> bool {
        self.stored <= CAPACITY
            && self.stored.is_multiple_of(LOSS_PER_MS)
            && self.applied <= 1_000_000_000_000_000
            && self.applied.is_multiple_of(DOSE)
            && self.lost.checked_add(self.stored) == Some(self.applied)
    }
    /// Return the portion of the interval supplied with water, then book its loss.
    pub(super) fn advance(&mut self, ms: u64) -> u64 {
        let wet_ms = ms.min(self.stored / LOSS_PER_MS);
        let loss = self.stored.min(ms.saturating_mul(LOSS_PER_MS));
        self.stored -= loss;
        self.lost += loss;
        wet_ms
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Error {
    NotACrop,
    PlotBlocked,
    PlotFull,
    MissingWell,
    WellStopped,
    TooFar,
    WaterReservedOrEmpty,
    AccountingLimit,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NotACrop => "Select a barley plot",
            Self::PlotBlocked => "Plot is flooded, steep or obstructed",
            Self::PlotFull => "Plot needs room for 2 water units",
            Self::MissingWell => "Drill a well within 30 m of the plot",
            Self::WellStopped => "Well is stopped or submerged",
            Self::TooFar => "Well must be within 30 m of the plot",
            Self::WaterReservedOrEmpty => "Waiting for 2 unreserved water units",
            Self::AccountingLimit => "Plot has reached its lifetime water limit",
        })
    }
}
impl std::error::Error for Error {}

/// Shared preview/commit validation. Existing household reservations have priority.
pub fn validate(sim: &SimState, source: SourceId, well: u64) -> Result<(), Error> {
    let source = sim.food.source(source).ok_or(Error::NotACrop)?;
    let Kind::Barley(crop) = &source.kind else {
        return Err(Error::NotACrop);
    };
    let soil = crop.soil_water.as_ref().ok_or(Error::NotACrop)?;
    if placement::ground(sim, source.position).is_err() {
        return Err(Error::PlotBlocked);
    }
    if soil.stored + sim.people.reserved_irrigation(source.id) > CAPACITY - DOSE {
        return Err(Error::PlotFull);
    }
    if soil.applied + sim.people.reserved_irrigation(source.id) > 1_000_000_000_000_000 - DOSE {
        return Err(Error::AccountingLimit);
    }
    let well = sim
        .subsurface
        .wells
        .iter()
        .find(|w| w.id == well)
        .ok_or(Error::MissingWell)?;
    if distance(source.position, well.position) > RANGE {
        return Err(Error::TooFar);
    }
    if !well.active || spatial::elevation(&sim.grid, well.position) <= sim.sea_level + 0.08 {
        return Err(Error::WellStopped);
    }
    let available =
        well.supply.as_ref().map_or(0., |s| s.stored) - sim.people.reserved_water(well.id);
    if available < DOSE as f64 / UNITS as f64 {
        return Err(Error::WaterReservedOrEmpty);
    }
    Ok(())
}

/// Nearest eligible well, with stable identity as the distance tie-breaker.
/// If none is eligible, explain the nearest in-range candidate's rejection.
pub fn candidate(sim: &SimState, source: SourceId) -> Result<u64, Error> {
    let p = sim.food.source(source).ok_or(Error::NotACrop)?.position;
    let mut wells: Vec<_> = sim
        .subsurface
        .wells
        .iter()
        .filter(|w| distance(p, w.position) <= RANGE)
        .collect();
    wells.sort_by(|a, b| {
        distance(p, a.position)
            .total_cmp(&distance(p, b.position))
            .then(a.id.cmp(&b.id))
    });
    let mut reason = Error::MissingWell;
    for (i, well) in wells.iter().enumerate() {
        match validate(sim, source, well.id) {
            Ok(()) => return Ok(well.id),
            Err(error) if i == 0 => reason = error,
            Err(_) => {}
        }
    }
    Err(reason)
}

pub(crate) fn apply(sim: &mut SimState, source: SourceId, well: u64) -> Result<(), Error> {
    validate(sim, source, well)?;
    // Resolve both owners before mutation; no fallible operation follows withdrawal.
    let soil = sim
        .food
        .sources
        .iter_mut()
        .find(|s| s.id == source)
        .and_then(|s| match &mut s.kind {
            Kind::Barley(c) => c.soil_water.as_mut(),
            _ => None,
        })
        .ok_or(Error::NotACrop)?;
    let supply = sim
        .subsurface
        .wells
        .iter_mut()
        .find(|w| w.id == well)
        .and_then(|w| w.supply.as_mut())
        .ok_or(Error::MissingWell)?;
    if !supply.collect(DOSE as f64 / UNITS as f64) {
        return Err(Error::WaterReservedOrEmpty);
    }
    soil.stored += DOSE;
    soil.applied += DOSE;
    Ok(())
}

pub(crate) fn set_tending(sim: &mut SimState, source: SourceId, enabled: bool) -> bool {
    let Some(source) = sim.food.sources.iter_mut().find(|s| s.id == source) else {
        return false;
    };
    let Kind::Barley(crop) = &mut source.kind else {
        return false;
    };
    crop.tending = enabled;
    true
}

/// A carried dose already belongs to the resident; this only transfers ownership.
/// Delivery ignores the tending toggle: collected water keeps its destination claim.
pub(crate) fn deliver(sim: &mut SimState, source: SourceId) -> bool {
    let Some(source) = sim.food.sources.iter().find(|s| s.id == source) else {
        return false;
    };
    if placement::ground(sim, source.position).is_err() {
        return false;
    }
    let id = source.id;
    let Some(soil) = sim
        .food
        .sources
        .iter_mut()
        .find(|s| s.id == id)
        .and_then(|s| match &mut s.kind {
            Kind::Barley(c) => c.soil_water.as_mut(),
            _ => None,
        })
    else {
        return false;
    };
    if soil.stored > CAPACITY - DOSE || soil.applied > 1_000_000_000_000_000 - DOSE {
        return false;
    }
    soil.stored += DOSE;
    soil.applied += DOSE;
    true
}

/// Cumulative diversion, including water since lost from plot storage.
pub fn total_applied(sim: &SimState) -> f64 {
    sim.food
        .sources()
        .iter()
        .filter_map(|s| match &s.kind {
            Kind::Barley(c) => c.soil_water.as_ref(),
            _ => None,
        })
        .map(|s| s.applied as f64 / UNITS as f64)
        .sum()
}

#[cfg(test)]
mod tests;
