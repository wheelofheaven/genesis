use crate::grid::Grid;

/// Apply a terraform action: modify elevation within a radius.
/// Positive strength raises land, negative lowers it.
pub fn apply_terraform(grid: &mut Grid, cx: i32, cy: i32, radius: u32, strength: f32) {
    if !strength.is_finite() {
        return;
    }
    let cx = cx.rem_euclid(grid.width as i32);
    let cy = cy.rem_euclid(grid.height as i32);
    let r = radius.min(grid.width.max(grid.height)) as i32;
    for dy in -r..=r {
        for dx in -r..=r {
            let dist_sq = (dx * dx + dy * dy) as f32;
            let radius_sq = (r * r) as f32;
            if dist_sq > radius_sq {
                continue;
            }
            // Falloff: stronger at center, weaker at edges.
            let falloff = if r == 0 {
                1.0
            } else {
                1.0 - (dist_sq / radius_sq).sqrt()
            };
            let cell = grid.get_mut(cx + dx, cy + dy);
            cell.elevation = (cell.elevation + strength * falloff).clamp(-40.0, 80.0);
        }
    }
}

/// Recompute water depth for all cells given the current sea level.
pub fn update_water_depth(grid: &mut Grid, sea_level: f32) {
    for cell in grid.cells_mut() {
        cell.water_depth = (sea_level - cell.elevation).max(0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brush_stays_finite_and_bounded_even_for_zero_radius() {
        let mut grid = Grid::new(8, 8);
        apply_terraform(&mut grid, 3, 3, 0, 2.);
        assert_eq!(grid.get(3, 3).elevation, 1.);
        for strength in [f32::NAN, f32::INFINITY, f32::MAX, -f32::MAX] {
            apply_terraform(&mut grid, 3, 3, 2, strength);
            assert!(
                grid.cells()
                    .iter()
                    .all(|c| c.elevation.is_finite() && (-40.0..=80.0).contains(&c.elevation))
            );
        }
    }

    #[test]
    fn test_terraform_raises_center() {
        let mut grid = Grid::new(16, 16);
        let before = grid.get(8, 8).elevation;
        apply_terraform(&mut grid, 8, 8, 2, 1.0);
        assert!(grid.get(8, 8).elevation > before);
    }

    #[test]
    fn test_terraform_falloff() {
        let mut grid = Grid::new(16, 16);
        apply_terraform(&mut grid, 8, 8, 3, 1.0);
        let center = grid.get(8, 8).elevation;
        let edge = grid.get(8 + 2, 8).elevation;
        assert!(center > edge, "Center should be higher than edge");
    }

    #[test]
    fn test_water_depth_clamped() {
        let mut grid = Grid::new(4, 4);
        grid.get_mut(0, 0).elevation = 5.0; // Well above sea level
        update_water_depth(&mut grid, 0.0);
        assert_eq!(grid.get(0, 0).water_depth, 0.0);
    }
}
