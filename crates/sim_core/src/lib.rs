pub mod actions;
pub mod config;
pub mod events;
pub mod grid;
pub mod rules;

use actions::Action;
use config::SimConfig;
use events::SimEvent;
use grid::Grid;
use rules::{apply_terraform, update_climate, update_life, update_water_depth};

/// The complete simulation state.
#[derive(Debug, Clone)]
pub struct SimState {
    pub grid: Grid,
    pub config: SimConfig,
    pub sea_level: f32,
    pub time: f64,
    pub stability_score: f32,
    pub outposts: Vec<(u32, u32)>,
    pub research_points: f32,
}

impl SimState {
    /// Create a new simulation from a config.
    pub fn new(config: SimConfig) -> Self {
        let grid = Grid::new(config.width, config.height);
        let sea_level = config.sea_level;
        Self {
            grid,
            config,
            sea_level,
            time: 0.0,
            stability_score: 0.0,
            outposts: Vec::new(),
            research_points: 0.0,
        }
    }

    /// Advance the simulation by one tick, processing player actions.
    /// Returns events generated during this step.
    pub fn step(&mut self, dt: f32, actions: &[Action]) -> Vec<SimEvent> {
        let mut events = Vec::new();

        // 1. Apply player actions.
        for action in actions {
            match action {
                Action::Terraform {
                    x,
                    y,
                    radius,
                    strength,
                } => {
                    apply_terraform(&mut self.grid, *x, *y, *radius, *strength);
                }
                Action::SeedFungus { x, y, radius } => {
                    self.seed_life(*x, *y, *radius, true);
                }
                Action::SeedPlants { x, y, radius } => {
                    self.seed_life(*x, *y, *radius, false);
                }
                Action::BuildOutpost { x, y } => {
                    let ux = (*x).rem_euclid(self.grid.width as i32) as u32;
                    let uy = (*y).rem_euclid(self.grid.height as i32) as u32;
                    if !self.outposts.contains(&(ux, uy)) {
                        self.outposts.push((ux, uy));
                        events.push(SimEvent::OutpostBuilt { x: ux, y: uy });
                    }
                }
            }
        }

        // 2. Update water depth.
        update_water_depth(&mut self.grid, self.sea_level);

        // 3. Update climate.
        update_climate(&mut self.grid);

        // 4. Update life.
        update_life(&mut self.grid, dt);

        // 5. Accumulate research points from outposts.
        self.research_points += self.outposts.len() as f32 * 0.1 * dt;

        // 6. Compute stability score.
        let new_stability = self.compute_stability();
        if (new_stability - self.stability_score).abs() > 0.05 {
            events.push(SimEvent::StabilityUpdate {
                score: new_stability,
            });
        }
        self.stability_score = new_stability;

        // 7. Check win condition.
        if self.stability_score >= 0.8 {
            events.push(SimEvent::BiosphereStabilized);
        }

        // 8. Advance time.
        self.time += dt as f64;

        events
    }

    /// Seed fungus or plants in a radius.
    fn seed_life(&mut self, cx: i32, cy: i32, radius: u32, is_fungus: bool) {
        let r = radius as i32;
        for dy in -r..=r {
            for dx in -r..=r {
                if dx * dx + dy * dy > r * r {
                    continue;
                }
                let cell = self.grid.get_mut(cx + dx, cy + dy);
                if cell.water_depth > 0.0 {
                    continue; // Can't seed underwater.
                }
                if is_fungus {
                    cell.fungus = (cell.fungus + 0.3).min(1.0);
                } else {
                    cell.plants = (cell.plants + 0.3).min(1.0);
                }
            }
        }
    }

    /// Compute a stability score [0.0, 1.0] based on land coverage and life.
    fn compute_stability(&self) -> f32 {
        let total_cells = (self.grid.width * self.grid.height) as f32;
        let mut land_cells = 0.0f32;
        let mut green_cells = 0.0f32;

        for cell in self.grid.cells() {
            if cell.water_depth == 0.0 {
                land_cells += 1.0;
                let greenness = cell.fungus * 0.3 + cell.plants * 0.7;
                if greenness > 0.3 {
                    green_cells += 1.0;
                }
            }
        }

        if land_cells < 1.0 {
            return 0.0;
        }

        // Score: ratio of green land to total cells, scaled.
        let green_ratio = green_cells / total_cells;
        // Need at least 15% of world to be green land for max score.
        (green_ratio / 0.15).min(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_sim_state() {
        let state = SimState::new(SimConfig::default());
        assert_eq!(state.grid.width, 128);
        assert_eq!(state.grid.height, 80);
        assert_eq!(state.time, 0.0);
    }

    #[test]
    fn test_deterministic_stepping() {
        let config = SimConfig {
            width: 16,
            height: 16,
            ..Default::default()
        };
        let actions = vec![
            Action::Terraform {
                x: 8,
                y: 8,
                radius: 3,
                strength: 2.0,
            },
            Action::SeedFungus {
                x: 8,
                y: 8,
                radius: 2,
            },
        ];

        let mut state_a = SimState::new(config.clone());
        let mut state_b = SimState::new(config);

        for _ in 0..10 {
            state_a.step(1.0, &actions);
            state_b.step(1.0, &actions);
        }

        // Both should produce identical grids.
        for i in 0..state_a.grid.cells().len() {
            let a = &state_a.grid.cells()[i];
            let b = &state_b.grid.cells()[i];
            assert_eq!(a.elevation, b.elevation);
            assert_eq!(a.fungus, b.fungus);
            assert_eq!(a.plants, b.plants);
            assert_eq!(a.fertility, b.fertility);
        }
    }

    #[test]
    fn test_stability_starts_at_zero() {
        let state = SimState::new(SimConfig::default());
        assert_eq!(state.stability_score, 0.0);
    }
}
