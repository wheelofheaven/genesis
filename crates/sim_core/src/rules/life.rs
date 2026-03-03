use crate::grid::{Biome, Grid};

/// Update life (fungus and plants) spread and decay.
pub fn update_life(grid: &mut Grid, dt: f32) {
    let width = grid.width;
    let height = grid.height;

    // Read current state into buffers.
    let mut fungus_delta = vec![0.0f32; (width * height) as usize];
    let mut plants_delta = vec![0.0f32; (width * height) as usize];
    let mut fertility_delta = vec![0.0f32; (width * height) as usize];

    for y in 0..height as i32 {
        for x in 0..width as i32 {
            let idx = grid.index(x, y);
            let cell = grid.get(x, y);

            // Skip ocean cells — no land life.
            if cell.biome == Biome::Ocean {
                fungus_delta[idx] = -cell.fungus; // Decay any stranded life
                plants_delta[idx] = -cell.plants;
                continue;
            }

            // --- Fungus ---
            // Grows in moderate moisture, not extreme temps.
            let fungus_suitability = fungus_growth_rate(cell.temp, cell.moisture);
            // Diffusion: average neighbor fungus contributes to growth.
            let mut neighbor_fungus = 0.0;
            for (nx, ny) in grid.neighbors4(x, y) {
                neighbor_fungus += grid.get(nx, ny).fungus;
            }
            neighbor_fungus /= 4.0;

            let fungus_growth = (fungus_suitability * 0.05 + neighbor_fungus * 0.02) * dt;
            let fungus_decay = if fungus_suitability < 0.1 {
                0.03 * dt
            } else {
                0.0
            };
            fungus_delta[idx] = fungus_growth - fungus_decay;

            // --- Plants ---
            // Require fertility threshold.
            let plant_suitability = plant_growth_rate(cell.temp, cell.moisture, cell.fertility);
            let mut neighbor_plants = 0.0;
            for (nx, ny) in grid.neighbors4(x, y) {
                neighbor_plants += grid.get(nx, ny).plants;
            }
            neighbor_plants /= 4.0;

            let plant_growth = (plant_suitability * 0.03 + neighbor_plants * 0.015) * dt;
            let plant_decay = if plant_suitability < 0.05 {
                0.02 * dt
            } else {
                0.0
            };
            plants_delta[idx] = plant_growth - plant_decay;

            // --- Fertility feedback ---
            // Fungus slowly builds fertility; plants slightly too.
            fertility_delta[idx] = (cell.fungus * 0.01 + cell.plants * 0.005) * dt;
        }
    }

    // Apply deltas.
    for y in 0..height as i32 {
        for x in 0..width as i32 {
            let idx = grid.index(x, y);
            let cell = grid.get_mut(x, y);
            cell.fungus = (cell.fungus + fungus_delta[idx]).clamp(0.0, 1.0);
            cell.plants = (cell.plants + plants_delta[idx]).clamp(0.0, 1.0);
            cell.fertility = (cell.fertility + fertility_delta[idx]).clamp(0.0, 1.0);
        }
    }
}

/// Fungus growth rate based on temperature and moisture suitability.
fn fungus_growth_rate(temp: f32, moisture: f32) -> f32 {
    // Best in moderate conditions: temp 0.3-0.7, moisture 0.3-0.8.
    let temp_factor = 1.0 - ((temp - 0.5).abs() * 2.5).min(1.0);
    let moisture_factor = 1.0 - ((moisture - 0.55).abs() * 2.0).min(1.0);
    (temp_factor * moisture_factor).max(0.0)
}

/// Plant growth rate based on temperature, moisture, and fertility.
fn plant_growth_rate(temp: f32, moisture: f32, fertility: f32) -> f32 {
    let temp_factor = 1.0 - ((temp - 0.6).abs() * 2.5).min(1.0);
    let moisture_factor = 1.0 - ((moisture - 0.5).abs() * 2.5).min(1.0);
    let fertility_gate = if fertility > 0.1 { fertility } else { 0.0 };
    (temp_factor * moisture_factor * fertility_gate).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fungus_growth_rate_moderate() {
        let rate = fungus_growth_rate(0.5, 0.55);
        assert!(
            rate > 0.5,
            "Should grow well in moderate conditions: {rate}"
        );
    }

    #[test]
    fn test_fungus_growth_rate_extreme() {
        let rate = fungus_growth_rate(0.0, 0.0);
        assert!(
            rate < 0.1,
            "Should barely grow in extreme conditions: {rate}"
        );
    }

    #[test]
    fn test_plant_needs_fertility() {
        let rate = plant_growth_rate(0.6, 0.5, 0.0);
        assert_eq!(rate, 0.0, "No growth without fertility");
    }

    #[test]
    fn test_life_stays_bounded() {
        let mut grid = Grid::new(8, 8);
        // Set up a land cell with fungus at max.
        let cell = grid.get_mut(4, 4);
        cell.elevation = 1.0;
        cell.water_depth = 0.0;
        cell.fungus = 1.0;
        cell.plants = 1.0;
        cell.fertility = 1.0;
        cell.temp = 0.5;
        cell.moisture = 0.5;
        cell.biome = Biome::Grassland;

        // Run many ticks.
        for _ in 0..1000 {
            update_life(&mut grid, 1.0);
        }

        for cell in grid.cells() {
            assert!(cell.fungus >= 0.0 && cell.fungus <= 1.0);
            assert!(cell.plants >= 0.0 && cell.plants <= 1.0);
            assert!(cell.fertility >= 0.0 && cell.fertility <= 1.0);
        }
    }
}
