pub mod actions;
pub mod catalog;
pub mod config;
pub mod ecology;
pub mod events;
pub mod grid;
pub mod households;
mod identity;
pub mod paths;
pub mod people;
pub mod rules;
pub mod runtime;
pub mod save;
pub mod scenarios;
pub mod settlement;
pub mod spatial;
pub mod subsurface;
pub mod surface;
pub mod units;
pub mod worldgen;

use actions::Action;
use config::SimConfig;
use events::SimEvent;
use grid::Grid;
use rules::{terraform_patch, update_climate, update_life, update_water_depth};

/// The complete simulation state.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SimState {
    pub grid: Grid,
    pub config: SimConfig,
    pub sea_level: f32,
    pub time: f64,
    pub stability_score: f32,
    pub outposts: Vec<(u32, u32)>,
    pub research_points: f32,
    pub buildings: Vec<settlement::Building>,
    pub paths: Vec<settlement::Path>,
    #[serde(default = "people::People::legacy_missing")]
    pub people: people::People,
    #[serde(default = "households::Households::missing")]
    pub households: households::Households,
    pub subsurface: subsurface::Subsurface,
    pub geography: worldgen::Geography,
    #[serde(default)]
    pub(crate) building_ids: identity::Allocator,
    #[serde(default)]
    pub(crate) path_ids: identity::Allocator,
}

impl SimState {
    /// Create a new simulation from a config.
    pub fn new(config: SimConfig) -> Self {
        let grid = Grid::new(config.width, config.height);
        let sea_level = config.sea_level;
        let subsurface = subsurface::Subsurface::new(config.width, config.height);
        Self {
            geography: worldgen::Geography::default(),
            building_ids: identity::Allocator::new(),
            path_ids: identity::Allocator::new(),
            subsurface,
            grid,
            config,
            sea_level,
            time: 0.0,
            stability_score: 0.0,
            outposts: Vec::new(),
            research_points: 0.0,
            buildings: Vec::new(),
            paths: Vec::new(),
            people: people::People::default(),
            households: households::Households::default(),
        }
    }

    /// Advance the simulation by one tick, processing player actions.
    /// Returns events generated during this step.
    pub fn step(&mut self, dt: f32, actions: &[Action]) -> Vec<SimEvent> {
        let mut events = Vec::new();

        // 1. Apply player actions.
        for action in actions {
            match action {
                Action::CancelWaterTask { person } => {
                    if self.people.cancel_water(*person) {
                        events.push(SimEvent::WaterTaskCancelled {
                            person: *person,
                            time: self.time + f64::from(dt),
                        });
                    } else {
                        events.push(SimEvent::WaterTaskCancelRejected {
                            person: *person,
                            time: self.time + f64::from(dt),
                        });
                    }
                }
                Action::Terraform {
                    x,
                    y,
                    radius,
                    strength,
                } => match terraform_patch(&mut self.grid, *x, *y, *radius, *strength) {
                    Ok((_, None)) => events.push(SimEvent::TerrainUnchanged { x: *x, y: *y }),
                    Ok((changed_cells, Some(bounds))) => events.push(SimEvent::TerrainChanged {
                        x: *x,
                        y: *y,
                        changed_cells,
                        bounds,
                    }),
                    Err(reason) => events.push(SimEvent::TerraformRejected {
                        x: *x,
                        y: *y,
                        reason,
                    }),
                },
                Action::SeedFungus { x, y, radius } => {
                    rules::seed_life(&mut self.grid, *x, *y, *radius, true);
                }
                Action::SeedPlants { x, y, radius } => {
                    rules::seed_life(&mut self.grid, *x, *y, *radius, false);
                }
                Action::DrillWell { position } => {
                    match self.subsurface.validate_well(
                        &self.grid,
                        self.sea_level,
                        &self.buildings,
                        *position,
                    ) {
                        Ok(()) => match self.subsurface.drill(*position) {
                            Ok(id) => events.push(SimEvent::WellDrilled { id }),
                            Err(reason) => events.push(SimEvent::WellRejected { reason }),
                        },
                        Err(reason) => events.push(SimEvent::WellRejected { reason }),
                    }
                }
                Action::SetWellActive { id, active } => {
                    if let Some(well) = self.subsurface.wells.iter_mut().find(|w| w.id == *id) {
                        well.active = *active;
                        if !active {
                            well.last_yield = 0.;
                        }
                        events.push(SimEvent::WellOperationChanged {
                            id: *id,
                            active: *active,
                        });
                    } else {
                        events.push(SimEvent::WellRejected {
                            reason: subsurface::WellError::UnknownWell,
                        });
                    }
                }
                Action::BuildPath { waypoints, width } => {
                    match paths::validate(self, waypoints, *width) {
                        Ok(mut path) => {
                            let Some(id) = self.path_ids.allocate() else {
                                events.push(SimEvent::PathRejected {
                                    reason: paths::PathError::IdentityExhausted,
                                });
                                continue;
                            };
                            path.id = id;
                            // Construction clears vegetation/rocks across the walking corridor.
                            self.geography.cover.retain(|plant| {
                                !paths::on_path(
                                    &path,
                                    plant.position,
                                    plant.kind.collision_radius() * plant.size + units::metres(0.3),
                                )
                            });
                            self.paths.push(path);
                            events.push(SimEvent::PathBuilt { id });
                        }
                        Err(reason) => events.push(SimEvent::PathRejected { reason }),
                    }
                }
                Action::PlaceBuilding { position, rotation } => {
                    let mut building = settlement::Building {
                        id: 0,
                        position: *position,
                        rotation: rotation.rem_euclid(std::f32::consts::TAU),
                    };
                    match self.validate_building(&building) {
                        Ok(()) => {
                            let Some(id) = self.building_ids.allocate() else {
                                events.push(SimEvent::BuildingRejected {
                                    reason: settlement::PlacementError::IdentityExhausted,
                                });
                                continue;
                            };
                            building.id = id;
                            events.push(SimEvent::BuildingPlaced { id });
                            self.geography.cover.retain(|plant| {
                                paths::distance(plant.position, building.position)
                                    >= units::metres(if plant.kind.tree() { 6. } else { 4. })
                            });
                            self.buildings.push(building);
                        }
                        Err(reason) => events.push(SimEvent::BuildingRejected { reason }),
                    }
                }
                Action::BuildOutpost { x, y } => {
                    if self.grid.boundary == grid::Boundary::Clamp && !self.grid.contains(*x, *y) {
                        continue;
                    }
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

        self.subsurface.step(&self.grid, self.sea_level, dt);

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

        self.households.advance(&self.people, dt);
        let mut people = std::mem::take(&mut self.people);
        people.advance(self, dt, &mut events);
        self.people = people;

        // 8. Advance time.
        self.time += dt as f64;

        events
    }

    /// Shared validation for build previews and action processing.
    pub fn validate_building(
        &self,
        building: &settlement::Building,
    ) -> Result<(), settlement::PlacementError> {
        settlement::validate_surface(&self.grid, self.sea_level, &self.buildings, building, |p| {
            surface::elevation(self, p)
        })?;
        if self
            .paths
            .iter()
            .any(|path| paths::intersects_building(path, building))
        {
            return Err(settlement::PlacementError::Occupied);
        }
        if worldgen::river_distance(self, building.position) < 1.2 {
            return Err(settlement::PlacementError::Watercourse);
        }
        if self.subsurface.wells.iter().any(|w| {
            (w.position.x - building.position.x).powi(2)
                + (w.position.z - building.position.z).powi(2)
                < settlement::Building::WELL_CLEARANCE.powi(2)
        }) {
            Err(settlement::PlacementError::Occupied)
        } else {
            Ok(())
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
    fn continuous_build_commands_are_ordered_validated_and_reproducible() {
        use spatial::Position;
        let mut a = scenarios::coastal_outpost();
        assert_eq!(a.buildings.len(), 6);
        let mut b = a.clone();
        let build = Action::PlaceBuilding {
            position: Position::new(25.31, 20.27),
            rotation: 0.47,
        };
        let actions = [
            build.clone(),
            build,
            Action::PlaceBuilding {
                position: Position::new(0.5, 0.5),
                rotation: 0.,
            },
        ];
        let events = a.step(0.1, &actions);
        b.step(0.1, &actions);
        assert_eq!(a.buildings, b.buildings);
        assert_eq!(a.buildings.len(), 7);
        assert_eq!(
            a.buildings.last().unwrap().position,
            Position::new(25.31, 20.27)
        );
        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(e, SimEvent::BuildingPlaced { .. }))
                .count(),
            1
        );
        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(e, SimEvent::BuildingRejected { .. }))
                .count(),
            2
        );
    }

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
