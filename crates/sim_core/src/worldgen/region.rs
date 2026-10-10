//! Place selection and a small local geographic frame, independent of rendering.
//! The anchors locate scenario windows; the current relief is procedural, not DEM data.
use crate::{spatial::Position, units::METRES_PER_UNIT};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Region {
    #[default]
    Island,
    LevantCoast,
    LevantHighlands,
}
impl Region {
    pub const ALL: [Self; 3] = [Self::LevantCoast, Self::LevantHighlands, Self::Island];
    pub fn name(self) -> &'static str {
        match self {
            Self::Island => "Mediterranean island",
            Self::LevantCoast => "Levant · coastal plain",
            Self::LevantHighlands => "Levant · inland hills",
        }
    }
    pub fn description(self) -> &'static str {
        match self {
            Self::Island => "An isolated island with a surrounding continental shelf.",
            Self::LevantCoast => {
                "Mediterranean water to the west, coastal lowlands and rising ground to the east."
            }
            Self::LevantHighlands => {
                "An inland window with ridges, valleys and drainage continuing beyond its edges."
            }
        }
    }
    pub fn anchor(self) -> Option<GeoPoint> {
        match self {
            Self::Island => None,
            Self::LevantCoast => Some(GeoPoint {
                latitude: 32.6,
                longitude: 34.93,
            }),
            Self::LevantHighlands => Some(GeoPoint {
                latitude: 32.0,
                longitude: 35.25,
            }),
        }
    }
    /// Local tangent approximation, suitable for kilometre-scale windows, not a
    /// global projection. Positive x is east; positive z is south.
    pub fn geographic_position(self, p: Position, width: u32, height: u32) -> Option<GeoPoint> {
        let anchor = self.anchor()?;
        let north = (height as f64 * 0.5 - p.z as f64) * METRES_PER_UNIT as f64;
        let east = (p.x as f64 - width as f64 * 0.5) * METRES_PER_UNIT as f64;
        let radians = 180. / std::f64::consts::PI / 6_371_008.8;
        Some(GeoPoint {
            latitude: anchor.latitude + north * radians,
            longitude: anchor.longitude + east * radians / anchor.latitude.to_radians().cos(),
        })
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GeoPoint {
    pub latitude: f64,
    pub longitude: f64,
}
