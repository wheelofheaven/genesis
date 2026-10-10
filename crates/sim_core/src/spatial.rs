//! Continuous positions over an environmental field with explicit boundaries. No renderer types.
use crate::grid::{Cell, Grid};
use serde::{Deserialize, Serialize};

/// World coordinates in environmental-sample units; origin is the map's corner.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Position {
    pub x: f32,
    pub z: f32,
}
impl Position {
    pub const fn new(x: f32, z: f32) -> Self {
        Self { x, z }
    }
}

/// Convex cubic B-spline interpolation: continuous slopes and no overshoot.
/// Samples are environmental measurements, not tile/building boundaries.
pub fn sample(grid: &Grid, p: Position, field: impl Fn(&Cell) -> f32) -> f32 {
    fn weights(t: f32) -> [f32; 4] {
        [
            (1. - t).powi(3) / 6.,
            (3. * t.powi(3) - 6. * t * t + 4.) / 6.,
            (-3. * t.powi(3) + 3. * t * t + 3. * t + 1.) / 6.,
            t.powi(3) / 6.,
        ]
    }
    let coordinate = |v: f32, size: u32| match grid.boundary {
        crate::grid::Boundary::Wrap => v.rem_euclid(size as f32),
        crate::grid::Boundary::Clamp => v.clamp(0., size as f32 - 1.),
    };
    let x = coordinate(p.x, grid.width);
    let z = coordinate(p.z, grid.height);
    let wx = weights(x.fract());
    let wz = weights(z.fract());
    let mut result = 0.;
    for (j, b) in wz.into_iter().enumerate() {
        for (i, a) in wx.into_iter().enumerate() {
            result += a
                * b
                * field(grid.get(
                    x.floor() as i32 + i as i32 - 1,
                    z.floor() as i32 + j as i32 - 1,
                ));
        }
    }
    result
}

/// Height of the shared terrain surface, in world units.
pub fn elevation(grid: &Grid, p: Position) -> f32 {
    sample(grid, p, |c| c.elevation)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn surface_is_bounded_continuous_and_wraps() {
        let mut grid = Grid::new(8, 8);
        grid.get_mut(3, 3).elevation = 4.;
        for i in 0..800 {
            let p = Position::new(i as f32 / 100., 3.2);
            let h = elevation(&grid, p);
            assert!((-1.0001..=4.).contains(&h));
            assert!((h - elevation(&grid, Position::new(p.x + 8., p.z))).abs() < 1e-5);
            assert!((h - elevation(&grid, Position::new(p.x + 0.001, p.z))).abs() < 0.01);
        }
    }
}
