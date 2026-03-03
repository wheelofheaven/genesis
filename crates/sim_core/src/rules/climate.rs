use crate::grid::{Biome, Grid};

/// Update climate proxies (temperature, moisture) and derive biomes.
pub fn update_climate(grid: &mut Grid) {
    let height = grid.height;
    let width = grid.width;

    // First pass: compute temp and moisture into a buffer to avoid aliasing.
    let mut temp_buf = vec![0.0f32; (width * height) as usize];
    let mut moisture_buf = vec![0.0f32; (width * height) as usize];

    for y in 0..height as i32 {
        for x in 0..width as i32 {
            let idx = grid.index(x, y);
            let cell = grid.get(x, y);

            // Temperature: based on latitude band + elevation.
            // Latitude 0 = equator (hot), latitude ±1 = poles (cold).
            let lat = (y as f32 / height as f32 - 0.5).abs() * 2.0; // 0..1
            let base_temp = 1.0 - lat; // hot at equator, cold at poles
            let elevation_cooling = (cell.elevation.max(0.0) * 0.1).min(0.4);
            temp_buf[idx] = (base_temp - elevation_cooling).clamp(0.0, 1.0);

            // Moisture: based on adjacency to water.
            let mut water_neighbors = 0u32;
            for (nx, ny) in grid.neighbors4(x, y) {
                if grid.get(nx, ny).water_depth > 0.0 {
                    water_neighbors += 1;
                }
            }
            let water_factor = water_neighbors as f32 / 4.0;
            let self_water = if cell.water_depth > 0.0 { 1.0 } else { 0.0 };
            moisture_buf[idx] = (water_factor * 0.6 + self_water * 0.4).clamp(0.0, 1.0);
        }
    }

    // Second pass: apply buffers and derive biomes.
    for y in 0..height as i32 {
        for x in 0..width as i32 {
            let idx = grid.index(x, y);
            let cell = grid.get_mut(x, y);
            cell.temp = temp_buf[idx];
            cell.moisture = moisture_buf[idx];
            cell.biome = derive_biome(cell.elevation, cell.temp, cell.moisture);
        }
    }
}

/// Derive biome from cell properties.
fn derive_biome(elevation: f32, temp: f32, moisture: f32) -> Biome {
    if elevation < -0.1 {
        return Biome::Ocean;
    }
    if elevation < 0.1 {
        return Biome::Beach;
    }
    if elevation > 3.0 {
        return Biome::Mountain;
    }
    if temp < 0.25 {
        return Biome::Tundra;
    }
    if moisture < 0.2 {
        return Biome::Desert;
    }
    if moisture > 0.5 && temp > 0.4 {
        return Biome::Forest;
    }
    Biome::Grassland
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ocean_biome_for_submerged() {
        assert_eq!(derive_biome(-1.0, 0.5, 0.5), Biome::Ocean);
    }

    #[test]
    fn test_mountain_biome_for_high_elevation() {
        assert_eq!(derive_biome(4.0, 0.5, 0.5), Biome::Mountain);
    }

    #[test]
    fn test_desert_biome_for_dry() {
        assert_eq!(derive_biome(0.5, 0.5, 0.1), Biome::Desert);
    }

    #[test]
    fn test_tundra_biome_for_cold() {
        assert_eq!(derive_biome(0.5, 0.1, 0.5), Biome::Tundra);
    }

    #[test]
    fn test_climate_update_no_panic() {
        let mut grid = Grid::new(16, 16);
        update_climate(&mut grid);
        // Should not panic and all temps should be in range.
        for cell in grid.cells() {
            assert!((0.0..=1.0).contains(&cell.temp));
            assert!((0.0..=1.0).contains(&cell.moisture));
        }
    }
}
