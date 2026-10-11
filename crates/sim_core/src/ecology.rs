//! Functional plant assemblages, not a historical species inventory.
//! Climate recipes are scenario data; habitat filters do not depend on Earth coordinates.
use crate::{
    SimState,
    spatial::{self, Position},
    subsurface::Rock,
    units::{METRES_PER_UNIT, metres},
    worldgen::Region,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CoverKind {
    Broadleaf,
    Pine,
    Riparian,
    EvergreenShrub,
    DryShrub,
    Limestone,
    Sandstone,
    Basalt,
}
impl CoverKind {
    pub const ALL: [Self; 8] = [
        Self::Broadleaf,
        Self::Pine,
        Self::Riparian,
        Self::EvergreenShrub,
        Self::DryShrub,
        Self::Limestone,
        Self::Sandstone,
        Self::Basalt,
    ];
    pub fn index(self) -> usize {
        Self::ALL.iter().position(|&k| k == self).unwrap_or(0)
    }
    pub fn rock(self) -> bool {
        matches!(self, Self::Limestone | Self::Sandstone | Self::Basalt)
    }
    pub fn tree(self) -> bool {
        matches!(self, Self::Broadleaf | Self::Pine | Self::Riparian)
    }
    /// Conservative solid footprint; shrubs remain traversable.
    pub fn collision_radius(self) -> f32 {
        metres(if self.rock() {
            1.7
        } else if self.tree() {
            0.35
        } else {
            0.
        })
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Broadleaf => "Evergreen broadleaf",
            Self::Pine => "Open pine",
            Self::Riparian => "Riparian broadleaf",
            Self::EvergreenShrub => "Evergreen shrub",
            Self::DryShrub => "Dry scrub",
            Self::Limestone => "Limestone",
            Self::Sandstone => "Sandstone",
            Self::Basalt => "Basalt",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Climate {
    /// Scenario annual mean at sea level; degrees Celsius, not a weather simulation.
    pub sea_level_temperature: f32,
    pub moisture_scale: f32,
    /// Dry-season severity (0–1), used to favour drought-tolerant growth forms.
    pub summer_drought: f32,
}
impl Default for Climate {
    fn default() -> Self {
        Self::for_region(Region::Island)
    }
}
impl Climate {
    pub fn for_region(region: Region) -> Self {
        match region {
            Region::LevantCoast => Self {
                sea_level_temperature: 20.,
                moisture_scale: 1.,
                summer_drought: 0.8,
            },
            Region::LevantHighlands => Self {
                sea_level_temperature: 19.,
                moisture_scale: 0.9,
                summer_drought: 0.85,
            },
            Region::Island => Self {
                sea_level_temperature: 18.,
                moisture_scale: 1.08,
                summer_drought: 0.7,
            },
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub struct Habitat {
    pub temperature: f32,
    pub moisture: f32,
    pub slope: f32,
    pub river_distance: f32,
    pub rock: Rock,
    pub drought: f32,
}
pub fn habitat(sim: &SimState, p: Position) -> Habitat {
    let h = spatial::elevation(&sim.grid, p);
    let step = metres(10.);
    let dx = (spatial::elevation(&sim.grid, Position::new(p.x + step, p.z))
        - spatial::elevation(&sim.grid, Position::new(p.x - step, p.z)))
        / (2. * step);
    let dz = (spatial::elevation(&sim.grid, Position::new(p.x, p.z + step))
        - spatial::elevation(&sim.grid, Position::new(p.x, p.z - step)))
        / (2. * step);
    let climate = sim.geography.climate;
    let river_distance = crate::worldgen::river_distance(sim, p);
    let riverside = (1. - river_distance / metres(45.)).clamp(0., 1.);
    Habitat {
        temperature: climate.sea_level_temperature
            - (h - sim.sea_level).max(0.) * METRES_PER_UNIT * 0.0065,
        moisture: (spatial::sample(&sim.grid, p, |c| c.moisture) * climate.moisture_scale
            + riverside * 0.18)
            .clamp(0., 1.),
        slope: dx.hypot(dz),
        river_distance,
        rock: sim.subsurface.reading(p).rock,
        drought: climate.summer_drought,
    }
}
/// Habitat suitability weights. Adding a planet can supply another climate and
/// assemblage without changing the renderer or mapping a latitude to a species.
pub fn weights(h: Habitat) -> [f32; 5] {
    let warm = ((h.temperature + 2.) / 12.).clamp(0., 1.);
    let trees = (1. - h.slope / 0.85).clamp(0., 1.);
    let moist = h.moisture;
    [
        warm * trees * (moist - 0.25).max(0.) * (1. - h.drought * 0.3),
        warm * trees
            * (0.28 + (0.65 - moist).max(0.) * 0.4)
            * (if h.rock == Rock::Limestone { 1.2 } else { 0.85 }),
        warm * trees
            * (moist - 0.5).max(0.)
            * (1. - h.river_distance / metres(50.)).clamp(0., 1.)
            * 4.,
        warm * (1. - h.slope / 1.2).clamp(0., 1.) * (0.2 + moist * 0.5),
        (1. - h.slope / 1.4).clamp(0., 1.) * (0.25 + h.drought * 0.3 + (0.6 - moist).max(0.)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn habitats_select_assemblages_without_requiring_an_earth_location() {
        let moist = Habitat {
            temperature: 20.,
            moisture: 0.8,
            slope: 0.1,
            river_distance: metres(12.),
            rock: Rock::Limestone,
            drought: 0.5,
        };
        let dry = Habitat {
            moisture: 0.2,
            river_distance: metres(200.),
            drought: 0.9,
            ..moist
        };
        assert!(weights(moist)[2] > weights(dry)[2]);
        assert!(weights(dry)[4] > weights(moist)[4]);
        let cold = Habitat {
            temperature: -10.,
            ..moist
        };
        assert!(weights(cold)[..4].iter().all(|&w| w == 0.));
        let steep = Habitat { slope: 1., ..moist };
        assert!(weights(steep)[..3].iter().all(|&w| w == 0.));
        assert!(weights(steep).iter().all(|w| w.is_finite() && *w >= 0.));
    }
}
