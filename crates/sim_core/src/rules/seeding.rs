//! Legacy life brushes; environmental growth remains in `life`.
use crate::grid::{Boundary, Grid};

/// Apply the existing per-visit coverage increment to dry samples.
/// Eligibility uses committed water depth, just as before extraction: later
/// tick stages refresh depth and growth. A brush outside a bounded map is a no-op.
pub(crate) fn seed_life(grid: &mut Grid, cx: i32, cy: i32, radius: u32, is_fungus: bool) {
    let (w, h) = (i64::from(grid.width), i64::from(grid.height));
    if w == 0 || h == 0 {
        return;
    }
    // Normalize wrapped centers and widen before offsets. External commands may
    // use either i32 extreme; neither overflow nor clamping may choose a sample.
    let (cx, cy) = match grid.boundary {
        Boundary::Wrap => (i64::from(cx).rem_euclid(w), i64::from(cy).rem_euclid(h)),
        Boundary::Clamp => (i64::from(cx), i64::from(cy)),
    };
    let r = i64::from(radius.min(grid.width.max(grid.height)));
    if grid.boundary == Boundary::Clamp && (cx + r < 0 || cy + r < 0 || cx - r >= w || cy - r >= h)
    {
        return;
    }
    for dy in -r..=r {
        for dx in -r..=r {
            let (x, y) = (cx + dx, cy + dy);
            if grid.boundary == Boundary::Clamp && (x < 0 || y < 0 || x >= w || y >= h) {
                continue;
            }
            if dx * dx + dy * dy > r * r {
                continue;
            }
            let index = (y.rem_euclid(h) * w + x.rem_euclid(w)) as usize;
            let cell = &mut grid.cells_mut()[index];
            if cell.water_depth > 0.0 {
                continue;
            }
            // Wrapped brushes intentionally retain repeated visits on small maps.
            let coverage = if is_fungus {
                &mut cell.fungus
            } else {
                &mut cell.plants
            };
            *coverage = (*coverage + 0.3).min(1.0);
        }
    }
}

#[cfg(test)]
mod tests;
