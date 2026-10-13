use crate::{
    grid::{Boundary, Grid, SampleBounds},
    units::{MAX_ELEVATION, MIN_ELEVATION},
};

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A rejected brush request has no elevation effects. Normal tick systems still run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerraformError {
    NonFiniteStrength,
    OutsideMap,
    EmptyGrid,
}

impl std::fmt::Display for TerraformError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NonFiniteStrength => "Terrain strength must be finite",
            Self::OutsideMap => "Choose ground inside the region",
            Self::EmptyGrid => "Terrain has no environmental samples",
        })
    }
}

/// Legacy setup helper. Live actions use the fallible operation and emit outcomes.
pub fn apply_terraform(grid: &mut Grid, cx: i32, cy: i32, radius: u32, strength: f32) {
    let _ = try_apply_terraform(grid, cx, cy, radius, strength);
}

/// Validate and stage one brush, then commit its complete elevation patch.
/// Returns the number of distinct samples whose final elevation changed.
/// There is no externally held plan that can be applied to a newer world.
/// Water depth/climate/life are updated by the existing later tick stages.
pub fn try_apply_terraform(
    grid: &mut Grid,
    cx: i32,
    cy: i32,
    radius: u32,
    strength: f32,
) -> Result<usize, TerraformError> {
    terraform_patch(grid, cx, cy, radius, strength).map(|(count, _)| count)
}

pub(crate) fn terraform_patch(
    grid: &mut Grid,
    cx: i32,
    cy: i32,
    radius: u32,
    strength: f32,
) -> Result<(usize, Option<SampleBounds>), TerraformError> {
    if !strength.is_finite() {
        return Err(TerraformError::NonFiniteStrength);
    }
    let (w, h) = (i64::from(grid.width), i64::from(grid.height));
    if w == 0 || h == 0 {
        return Err(TerraformError::EmptyGrid);
    }
    // Widen before adding offsets: even extreme external coordinates are safe.
    let (cx, cy) = match grid.boundary {
        Boundary::Wrap => (i64::from(cx).rem_euclid(w), i64::from(cy).rem_euclid(h)),
        Boundary::Clamp => (i64::from(cx), i64::from(cy)),
    };
    let r = i64::from(radius.min(grid.width.max(grid.height)));
    if grid.boundary == Boundary::Clamp && (cx + r < 0 || cy + r < 0 || cx - r >= w || cy - r >= h)
    {
        return Err(TerraformError::OutsideMap);
    }
    let mut patch = BTreeMap::new();
    let mut intersects = false;
    for dy in -r..=r {
        for dx in -r..=r {
            let (x, y) = (cx + dx, cy + dy);
            if grid.boundary == Boundary::Clamp && (x < 0 || y < 0 || x >= w || y >= h) {
                continue;
            }
            let dist_sq = (dx * dx + dy * dy) as f32;
            let radius_sq = (r * r) as f32;
            if dist_sq > radius_sq {
                continue;
            }
            intersects = true;
            let falloff = if r == 0 {
                1.0
            } else {
                1.0 - (dist_sq / radius_sq).sqrt()
            };
            let index = (y.rem_euclid(h) * w + x.rem_euclid(w)) as usize;
            // Wrapped brushes may visit one sample repeatedly. Preserve the
            // original stencil order and per-visit clamping, staging each result.
            let old = patch
                .get(&index)
                .copied()
                .unwrap_or(grid.cells()[index].elevation);
            let new = (old + strength * falloff).clamp(MIN_ELEVATION, MAX_ELEVATION);
            if new.to_bits() != old.to_bits() {
                patch.insert(index, new);
            }
        }
    }
    if !intersects {
        return Err(TerraformError::OutsideMap);
    }
    patch.retain(|&index, value| value.to_bits() != grid.cells()[index].elevation.to_bits());
    let changed = patch.len();
    let mut bounds: Option<SampleBounds> = None;
    for (index, elevation) in patch {
        let point = [index as u32 % grid.width, index as u32 / grid.width];
        let cell = SampleBounds {
            min: point,
            max: point,
        };
        bounds = Some(bounds.map_or(cell, |old| old.union(cell)));
        grid.cells_mut()[index].elevation = elevation;
    }
    Ok((changed, bounds))
}

/// Recompute water depth for all cells given the current sea level.
pub fn update_water_depth(grid: &mut Grid, sea_level: f32) {
    for cell in grid.cells_mut() {
        cell.water_depth = (sea_level - cell.elevation).max(0.0);
    }
}

#[cfg(test)]
mod tests;
