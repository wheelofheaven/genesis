//! Independent building footprints and authored path centerlines.
use crate::{
    grid::Grid,
    spatial::{Position, elevation},
    units::metres,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Building {
    pub id: u64,
    pub position: Position,
    /// Radians about the vertical axis.
    pub rotation: f32,
}
impl Building {
    pub const HALF_WIDTH: f32 = metres(6.5) * 0.5;
    pub const HALF_DEPTH: f32 = metres(4.8) * 0.5;
    pub const WALL_HEIGHT: f32 = metres(2.8);
    pub const MAX_FOUNDATION_STEP: f32 = metres(1.2);
    pub const CLEARANCE: f32 = metres(1.);
    pub const WELL_CLEARANCE: f32 = metres(5.);
    pub const TREE_CLEARANCE: f32 = metres(6.);
    pub const PATH_WIDTH: f32 = metres(1.2);
    pub const ENTRANCE_WIDTH: f32 = metres(0.9);
    pub fn footprint(&self) -> [Position; 4] {
        let (s, c) = self.rotation.sin_cos();
        [(-1., -1.), (1., -1.), (1., 1.), (-1., 1.)].map(|(x, z)| {
            Position::new(
                self.position.x + c * x * Self::HALF_WIDTH - s * z * Self::HALF_DEPTH,
                self.position.z + s * x * Self::HALF_WIDTH + c * z * Self::HALF_DEPTH,
            )
        })
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Path {
    /// Persistent identity; zero only for uncommitted preview geometry.
    #[serde(default)]
    pub id: u64,
    pub points: Vec<Position>,
    pub width: f32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlacementError {
    OutsideMap,
    Underwater,
    SteepSlope,
    Occupied,
    InvalidPosition,
    Watercourse,
    IdentityExhausted,
}
impl std::fmt::Display for PlacementError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::OutsideMap => "Footprint extends beyond the map",
            Self::Underwater => "Choose dry land",
            Self::SteepSlope => "Ground is too steep",
            Self::Occupied => "Keep buildings, paths and boreholes clear",
            Self::InvalidPosition => "Invalid position or rotation",
            Self::Watercourse => "Keep the watercourse clear",
            Self::IdentityExhausted => "Dwelling identity capacity exhausted",
        })
    }
}

/// Check the entire rotated footprint (including interior), not a grid cell.
pub fn validate(
    grid: &Grid,
    sea: f32,
    buildings: &[Building],
    candidate: &Building,
) -> Result<(), PlacementError> {
    validate_surface(grid, sea, buildings, candidate, |p| elevation(grid, p))
}

/// Validate against the shared physical surface, independent of rendering LOD.
pub fn validate_surface(
    grid: &Grid,
    sea: f32,
    buildings: &[Building],
    candidate: &Building,
    height: impl Fn(Position) -> f32,
) -> Result<(), PlacementError> {
    let p = candidate.position;
    if !p.x.is_finite() || !p.z.is_finite() || !candidate.rotation.is_finite() {
        return Err(PlacementError::InvalidPosition);
    }
    let corners = candidate.footprint();
    if corners
        .iter()
        .any(|p| p.x < 0. || p.z < 0. || p.x >= grid.width as f32 || p.z >= grid.height as f32)
    {
        return Err(PlacementError::OutsideMap);
    }
    let mut low = f32::INFINITY;
    let mut high = f32::NEG_INFINITY;
    for i in 0..=4 {
        for j in 0..=4 {
            let u = i as f32 / 4.;
            let v = j as f32 / 4.;
            let point = Position::new(
                corners[0].x
                    + (corners[1].x - corners[0].x) * u
                    + (corners[3].x - corners[0].x) * v,
                corners[0].z
                    + (corners[1].z - corners[0].z) * u
                    + (corners[3].z - corners[0].z) * v,
            );
            let h = height(point);
            low = low.min(h);
            high = high.max(h);
        }
    }
    if low <= sea + 0.08 {
        return Err(PlacementError::Underwater);
    }
    if high - low > Building::MAX_FOUNDATION_STEP {
        return Err(PlacementError::SteepSlope);
    }
    if buildings.iter().any(|other| overlap(candidate, other)) {
        return Err(PlacementError::Occupied);
    }
    Ok(())
}

fn overlap(a: &Building, b: &Building) -> bool {
    let ca = a.footprint();
    let cb = b.footprint();
    for angle in [a.rotation, b.rotation] {
        let (s, c) = angle.sin_cos();
        for (x, z) in [(c, s), (-s, c)] {
            let interval = |corners: &[Position; 4]| {
                corners
                    .iter()
                    .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), p| {
                        let v = p.x * x + p.z * z;
                        (lo.min(v), hi.max(v))
                    })
            };
            let (al, ah) = interval(&ca);
            let (bl, bh) = interval(&cb);
            if ah + Building::CLEARANCE < bl || bh + Building::CLEARANCE < al {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    fn flat() -> Grid {
        let mut g = Grid::new(16, 16);
        for c in g.cells_mut() {
            c.elevation = 1.;
        }
        g
    }
    #[test]
    fn rotated_fractional_footprints_and_rejections() {
        let mut g = flat();
        let a = Building {
            id: 1,
            position: Position::new(5.23, 6.71),
            rotation: 0.72,
        };
        assert_eq!(validate(&g, 0., &[], &a), Ok(()));
        assert_eq!(
            validate(&g, 0., std::slice::from_ref(&a), &a),
            Err(PlacementError::Occupied)
        );
        let mut b = a.clone();
        b.position.x += metres(2.);
        assert_eq!(
            validate(&g, 0., std::slice::from_ref(&a), &b),
            Err(PlacementError::Occupied)
        );
        b.position.x = a.position.x + metres(10.);
        assert_eq!(validate(&g, 0., std::slice::from_ref(&a), &b), Ok(()));
        b.position.x = 0.;
        assert_eq!(validate(&g, 0., &[], &b), Err(PlacementError::OutsideMap));
        b.position.x = f32::NAN;
        assert_eq!(
            validate(&g, 0., &[], &b),
            Err(PlacementError::InvalidPosition)
        );
        assert_eq!(validate(&g, 2., &[], &a), Err(PlacementError::Underwater));
        for z in 0..16 {
            for x in 0..16 {
                g.get_mut(x, z).elevation = x as f32;
            }
        }
        assert_eq!(validate(&g, 0., &[], &a), Err(PlacementError::SteepSlope));
    }
}
