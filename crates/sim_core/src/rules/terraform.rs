use crate::grid::Grid;

/// Apply a terraform action: modify elevation within a radius.
/// Positive strength raises land, negative lowers it.
pub fn apply_terraform(grid: &mut Grid, cx: i32, cy: i32, radius: u32, strength: f32) {
    let r = radius as i32;
    for dy in -r..=r {
        for dx in -r..=r {
            let dist_sq = (dx * dx + dy * dy) as f32;
            let radius_sq = (r * r) as f32;
            if dist_sq > radius_sq {
                continue;
            }
            // Falloff: stronger at center, weaker at edges.
            let falloff = 1.0 - (dist_sq / radius_sq).sqrt();
            let cell = grid.get_mut(cx + dx, cy + dy);
            cell.elevation += strength * falloff;
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
