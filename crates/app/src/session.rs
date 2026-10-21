//! Application session owning simulation, pending commands, and presentation choices.
mod follow;
use crate::clock::Clock;
use genesis_sim_core::{
    SimState, actions::Action, events::SimEvent, runtime::Simulation, settlement::Building,
    spatial::Position,
};
use std::{collections::VecDeque, time::Duration};

pub struct Session {
    pub mode: crate::frontend::Mode,
    pub sim: Simulation,
    previous_people: std::collections::BTreeMap<genesis_sim_core::people::PersonId, Position>,
    pub world_id: String,
    pub clock: Clock,
    pub selected: (i32, i32),
    pub position: Position,
    pub building_rotation: f32,
    pub placing: bool,
    pub crop_placing: bool,
    pub path: crate::path_tool::Draft,
    pub selected_object: Option<crate::interaction::Object>,
    following: Option<genesis_sim_core::people::PersonId>,
    pub hovered_object: Option<crate::interaction::Object>,
    pub hover: Option<Position>,
    pub overlay_dirty: bool,
    pub urgent_redraw: bool,
    pub terrain_revision: u64,
    terrain_changes: VecDeque<(u64, Option<genesis_sim_core::grid::SampleBounds>)>,
    pub help: bool,
    pub landscape_tools: bool,
    pub world_tools: bool,
    pub water_overview: bool,
    pub supply_food: bool,
    pub region_choice: genesis_sim_core::worldgen::Region,
    pub survey: crate::survey::Survey,
    pub message: String,
    pub message_is_error: bool,
    pending: Vec<Action>,
    pub events: VecDeque<SimEvent>,
    pub dirty: bool,
}
impl Default for Session {
    fn default() -> Self {
        Self::from_sim(genesis_sim_core::worldgen::generate(
            genesis_sim_core::worldgen::DEFAULT_SEED,
        ))
    }
}
impl Session {
    pub fn from_sim(sim: SimState) -> Self {
        Self::from_runtime(Simulation::new(sim))
    }
    fn from_runtime(sim: Simulation) -> Self {
        let position = sim.buildings.first().map_or(
            Position::new(sim.grid.width as f32 / 2., sim.grid.height as f32 / 2.),
            |b| b.position,
        );
        let region_choice = sim.geography.region;
        Self {
            mode: crate::frontend::Mode::Settlement,
            region_choice,
            sim,
            previous_people: Default::default(),
            world_id: genesis_storage::new_world_id(),
            clock: Clock::default(),
            selected: (position.x.round() as i32, position.z.round() as i32),
            position,
            building_rotation: 0.,
            placing: false,
            crop_placing: false,
            path: crate::path_tool::Draft::default(),
            selected_object: None,
            following: None,
            hovered_object: None,
            hover: None,
            overlay_dirty: true,
            urgent_redraw: true,
            terrain_revision: 0,
            terrain_changes: VecDeque::new(),
            help: false,
            landscape_tools: false,
            world_tools: false,
            water_overview: false,
            supply_food: false,
            survey: crate::survey::Survey::default(),
            message: String::new(),
            message_is_error: false,
            pending: Vec::new(),
            events: VecDeque::new(),
            dirty: true,
        }
    }
}
impl Session {
    /// Activate a generated world and reset client state. Every replacement
    /// advances the view generation, even when seed and object counts match.
    pub fn replace_sim(&mut self, sim: SimState) {
        let mut next = Self::from_sim(sim);
        next.mode = self.mode;
        self.replace(next);
    }

    fn replace(&mut self, mut next: Self) {
        next.terrain_revision = self.terrain_revision + 1;
        *self = next;
    }

    pub fn save_to(&self, path: &std::path::Path) -> genesis_storage::Result<()> {
        let mut snapshot =
            genesis_sim_core::save::Snapshot::capture(&self.sim, &self.world_id, &self.pending)?;
        snapshot.expedition_mode = Some(self.mode);
        genesis_storage::store(path, &snapshot)
    }

    pub fn load_from(&mut self, path: &std::path::Path) -> genesis_storage::Result<()> {
        let restored = genesis_storage::load(path)?;
        let mut next = Self::from_runtime(restored.simulation);
        next.world_id = restored.world_id;
        next.pending = restored.pending;
        next.mode = restored.expedition_mode.unwrap_or(self.mode);
        next.clock.paused = true;
        next.message = "World loaded · paused".into();
        self.replace(next);
        Ok(())
    }

    pub fn regenerate(&mut self, seed: u64) {
        let mode = self.mode;
        self.replace_sim(genesis_sim_core::worldgen::generate_scenario(
            seed,
            self.region_choice,
            mode == crate::frontend::Mode::Settlement,
        ));
        self.world_tools = true;
        self.message = format!("New region · seed {seed}");
    }
    pub fn queue(&mut self, action: Action) {
        self.pending.push(action);
    }
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }
    pub fn advance(&mut self, elapsed: Duration) {
        if self.clock.paused {
            self.previous_people.clear();
        }
        for _ in 0..self.clock.advance(elapsed) {
            self.tick();
        }
    }
    pub fn step_once(&mut self) {
        if self.clock.paused {
            self.tick();
        }
    }
    pub fn person_position(&self, person: &genesis_sim_core::people::Person) -> Position {
        let previous = self
            .previous_people
            .get(&person.id)
            .copied()
            .unwrap_or(person.position);
        let t = self.clock.interpolation();
        Position::new(
            previous.x + (person.position.x - previous.x) * t,
            previous.z + (person.position.z - previous.z) * t,
        )
    }
    fn tick(&mut self) {
        self.previous_people = self
            .sim
            .people
            .all()
            .iter()
            .map(|p| (p.id, p.position))
            .collect();
        self.urgent_redraw |= self
            .pending
            .iter()
            .any(|a| !matches!(a, Action::Terraform { .. }));
        let events = self.sim.advance(&self.pending);
        self.pending.clear();
        for event in events {
            match &event {
                SimEvent::CropTendingChanged { enabled, .. } => {
                    self.message = if *enabled {
                        "Resident tending enabled"
                    } else {
                        "Tending stopped; carried doses still deliver"
                    }
                    .into();
                    self.message_is_error = false;
                }
                SimEvent::CropTendingRejected { .. } => {
                    self.message = "Select a barley plot for tending".into();
                    self.message_is_error = true;
                }
                SimEvent::CropIrrigated { amount, well, .. } => {
                    self.message = format!("Applied {amount:.0} water units from well {well}");
                    self.message_is_error = false;
                }
                SimEvent::CropIrrigationRejected { reason, .. } => {
                    self.message = reason.to_string();
                    self.message_is_error = true;
                }
                SimEvent::CropPlanted { source, .. } => {
                    self.select_object(
                        Some(crate::interaction::Object::FoodSource(*source)),
                        self.position,
                    );
                    self.message = "Barley planted".into();
                    self.message_is_error = false;
                }
                SimEvent::CropPlantRejected { reason, .. } => {
                    self.message = reason.to_string();
                    self.message_is_error = true;
                }
                SimEvent::FoodTaskCancelled { .. } => {
                    self.message = "Gathering cancelled; any carried food returns home".into();
                    self.message_is_error = false;
                }
                SimEvent::FoodTaskCancelRejected { .. } => {
                    self.message = "No current food task for that resident".into();
                    self.message_is_error = true;
                }
                SimEvent::WaterTaskCancelled { .. } => {
                    self.message =
                        "Collection cancelled; resident returns home with any carried water".into();
                    self.message_is_error = false;
                }
                SimEvent::WaterTaskCancelRejected { .. } => {
                    self.message = "No current collection task for that resident".into();
                    self.message_is_error = true;
                }
                SimEvent::TerrainChanged { bounds, .. } => {
                    self.record_terrain_change(Some(*bounds));
                    self.urgent_redraw = true;
                    self.message = "Ground reshaped".into();
                    self.message_is_error = false;
                }
                SimEvent::TerrainUnchanged { .. } => {
                    self.message = "Ground unchanged".into();
                    self.message_is_error = false;
                }
                SimEvent::TerraformRejected { reason, .. } => {
                    self.message = reason.to_string();
                    self.message_is_error = true;
                }
                SimEvent::WellDrilled { id } => {
                    self.record_terrain_change(None);
                    self.message = format!("Borehole {id} online");
                    self.message_is_error = false;
                }
                SimEvent::WellRejected { reason } => {
                    self.message = reason.to_string();
                    self.message_is_error = true;
                }
                SimEvent::WellPumpModeChanged { id, mode } => {
                    self.message = format!("Borehole {id} · {}", mode.label());
                    self.message_is_error = false;
                }
                SimEvent::WellOperationChanged { id, active } => {
                    self.message = format!(
                        "Borehole {id} {}",
                        if *active { "enabled" } else { "stopped" }
                    );
                    self.message_is_error = false;
                }

                SimEvent::PathBuilt { id } => {
                    self.record_terrain_change(None);
                    let object = crate::interaction::Object::Path(*id);
                    self.select_object(Some(object), self.position);
                    self.message = format!("{} built", object.label());
                    self.message_is_error = false;
                }
                SimEvent::PathRejected { reason } => {
                    self.message = reason.to_string();
                    self.message_is_error = true;
                }
                SimEvent::BuildingPlaced { id } => {
                    self.select_object(
                        Some(crate::interaction::Object::Building(*id)),
                        self.position,
                    );
                    self.record_terrain_change(None);
                    self.message = format!("Dwelling {id} built");
                    self.message_is_error = false;
                }
                SimEvent::BuildingRejected { reason } => {
                    self.message = reason.to_string();
                    self.message_is_error = true;
                }
                _ => {}
            }
            if self.events.len() == 8 {
                self.events.pop_front();
            }
            self.events.push_back(event);
        }
        if let Some(crate::interaction::Object::Person(id)) = self.selected_object
            && let Some(p) = self.sim.people.get(id)
        {
            self.position = p.position;
            self.selected = (p.position.x.round() as i32, p.position.z.round() as i32);
        }
        self.dirty = true;
        self.overlay_dirty = true;
    }
    fn record_terrain_change(&mut self, bounds: Option<genesis_sim_core::grid::SampleBounds>) {
        self.terrain_revision += 1;
        if self.terrain_changes.len() == 32 {
            self.terrain_changes.pop_front();
        }
        self.terrain_changes
            .push_back((self.terrain_revision, bounds));
    }

    /// A complete terrain-only delta from a cache revision, or a full refresh.
    /// This bounded history is separate from the disposable eight-event HUD log.
    pub fn terrain_changes_since(
        &self,
        revision: u64,
    ) -> Option<genesis_sim_core::grid::SampleBounds> {
        if revision >= self.terrain_revision {
            return None;
        }
        let mut expected = revision.checked_add(1)?;
        let mut result: Option<genesis_sim_core::grid::SampleBounds> = None;
        for &(rev, bounds) in self.terrain_changes.iter().filter(|(r, _)| *r > revision) {
            if rev != expected {
                return None;
            }
            let bounds = bounds?;
            result = Some(result.map_or(bounds, |old| old.union(bounds)));
            expected += 1;
        }
        if expected != self.terrain_revision + 1 {
            return None;
        }
        result
    }

    pub fn selected_point(&self) -> glam::Vec3 {
        self.visible_point(self.position)
    }
    pub fn visible_point(&self, p: Position) -> glam::Vec3 {
        if self.survey.seafloor {
            crate::terrain::point(&self.sim, p)
        } else {
            crate::terrain::surface_point(&self.sim, p)
        }
    }
    pub fn select(&mut self, position: Position) {
        self.stop_following();
        self.selected_object = None;
        self.position = position;
        self.selected = (position.x.round() as i32, position.z.round() as i32);
        self.overlay_dirty = true;
    }
    pub fn set_water_overview(&mut self, open: bool) {
        self.set_placing(false);
        self.landscape_tools = false;
        self.world_tools = false;
        self.survey.open = false;
        self.water_overview = open;
    }
    pub fn set_placing(&mut self, placing: bool) {
        self.stop_following();
        self.water_overview = false;
        self.path.active = false;
        self.path.points.clear();
        self.hovered_object = None;
        self.placing = placing;
        if placing {
            self.landscape_tools = false;
            self.world_tools = false;
            self.survey.open = false;
        }
        self.overlay_dirty = true;
        self.message.clear();
        self.message_is_error = false;
    }
    pub fn set_survey(&mut self, open: bool) {
        self.survey.open = open;
        if open {
            self.set_placing(false);
            self.landscape_tools = false;
            self.world_tools = false;
            if !self.survey.active() {
                self.survey.water = true;
                self.refresh_survey();
            }
        }
    }
    pub fn refresh_survey(&mut self) {
        self.overlay_dirty = true;
        self.dirty = true;
        self.urgent_redraw = true;
    }
    pub fn rotate_building(&mut self, radians: f32) {
        self.building_rotation =
            (self.building_rotation + radians).rem_euclid(std::f32::consts::TAU);
        self.overlay_dirty = true;
    }
    pub fn select_object(&mut self, object: Option<crate::interaction::Object>, ground: Position) {
        use crate::interaction::Object;
        let p = match object {
            Some(Object::FoodSource(id)) => self.sim.food.source(id).map(|s| s.position),
            Some(Object::Person(id)) => self.sim.people.get(id).map(|p| p.position),
            Some(Object::Building(id)) => self
                .sim
                .buildings
                .iter()
                .find(|b| b.id == id)
                .map(|b| b.position),
            Some(Object::Path(id)) => self
                .sim
                .paths
                .iter()
                .find(|p| p.id == id)
                .and_then(|p| p.points.get(p.points.len() / 2))
                .copied(),
            None => None,
        };
        self.select(p.unwrap_or(ground));
        self.selected_object = object.filter(|_| p.is_some());
    }
    pub fn set_hovered_object(&mut self, object: Option<crate::interaction::Object>) {
        if self.hovered_object != object {
            self.hovered_object = object;
            self.overlay_dirty = true;
        }
    }
    pub fn set_hover(&mut self, hover: Option<Position>) {
        if self.hover != hover {
            self.hover = hover;
            if hover.is_none() {
                self.hovered_object = None;
            }
            self.overlay_dirty = true;
        }
    }
    pub fn build_hovered(&mut self, repeat: bool) {
        if !self.placing || self.hover.is_none() {
            return;
        }
        let candidate = self.candidate();
        if self.crop_placing {
            match genesis_sim_core::food::cultivation::validate(&self.sim, candidate.position) {
                Ok(()) => {
                    self.select(candidate.position);
                    self.queue(Action::PlantCrop {
                        position: candidate.position,
                    });
                    self.set_placing(repeat);
                    self.message = "Planting queued · resume or step to apply".into();
                }
                Err(reason) => {
                    self.message = reason.to_string();
                    self.message_is_error = true;
                }
            }
            return;
        }
        match self.sim.validate_building(&candidate) {
            Ok(()) => {
                self.select(candidate.position);
                self.queue(Action::PlaceBuilding {
                    position: candidate.position,
                    rotation: candidate.rotation,
                });
                self.set_placing(repeat);
                self.message = if self.clock.paused {
                    "Build queued · resume or step to apply"
                } else {
                    "Building queued"
                }
                .into();
            }
            Err(reason) => {
                self.message = reason.to_string();
                self.message_is_error = true;
            }
        }
    }
    pub fn candidate(&self) -> Building {
        Building {
            id: 0,
            position: self.hover.unwrap_or(self.position),
            rotation: self.building_rotation,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn path_selection_tracks_identity_across_repacking_and_rejects_removed_targets() {
        use crate::interaction::Object;
        let mut sim = genesis_sim_core::scenarios::coastal_outpost();
        let original = sim.paths[0].clone();
        let middle = original.points[original.points.len() / 2];
        let mut other = original.clone();
        other.id = 42;
        other.points = vec![Position::new(4., 4.), Position::new(5., 5.)];
        sim.paths.insert(0, other);
        let mut session = Session::from_sim(sim.clone());
        session.select_object(Some(Object::Path(original.id)), Position::new(0., 0.));
        assert_eq!(session.position, middle);
        assert_eq!(session.selected_object, Some(Object::Path(original.id)));
        sim.paths.retain(|p| p.id != original.id);
        let mut session = Session::from_sim(sim);
        let ground = Position::new(1., 1.);
        session.select_object(Some(Object::Path(original.id)), ground);
        assert_eq!(session.position, ground);
        assert_eq!(session.selected_object, None);
    }

    #[test]
    fn saved_session_restores_pending_intent_and_failed_load_keeps_active_world() {
        let dir = std::env::temp_dir().join(format!(
            "genesis-session-save-{}",
            genesis_storage::new_world_id()
        ));
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("world.json");
        let mut session = Session::from_sim(genesis_sim_core::scenarios::coastal_outpost());
        session.clock.paused = true;
        session.step_once();
        session.queue(Action::PlaceBuilding {
            position: Position::new(25.31, 20.27),
            rotation: 0.47,
        });
        let world_id = session.world_id.clone();
        session.save_to(&path).unwrap();
        assert_eq!(session.sim.completed_ticks(), 1);
        assert_eq!(session.pending_count(), 1);
        session.step_once();
        let revision = session.terrain_revision;
        assert_eq!(session.sim.buildings.len(), 7);
        session.load_from(&path).unwrap();
        assert_eq!(session.world_id, world_id);
        assert_eq!(session.sim.completed_ticks(), 1);
        assert_eq!(session.sim.buildings.len(), 6);
        assert_eq!(session.pending_count(), 1);
        assert!(
            session.clock.paused && session.urgent_redraw && session.terrain_revision > revision
        );
        session.step_once();
        assert_eq!(session.sim.buildings.len(), 7);
        assert_eq!(session.pending_count(), 0);
        let before = format!("{:?}", session.sim);
        std::fs::write(&path, b"{truncated").unwrap();
        assert!(session.load_from(&path).is_err());
        assert_eq!(format!("{:?}", session.sim), before);
        assert_eq!(session.world_id, world_id);
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn hover_and_repeat_building_are_responsive_without_rebuilding_the_world() {
        let mut session = Session::from_sim(genesis_sim_core::scenarios::coastal_outpost());
        session.clock.paused = true;
        session.dirty = false;
        session.urgent_redraw = false;
        session.overlay_dirty = false;
        session.set_placing(true);
        session.set_hover(Some(Position::new(25.31, 20.27)));
        session.rotate_building(0.47);
        assert!(!session.dirty && !session.urgent_redraw && session.overlay_dirty);
        let count = session.sim.buildings.len();
        session.build_hovered(true);
        assert!(session.placing);
        assert_eq!(session.pending_count(), 1);
        assert_eq!(session.sim.buildings.len(), count);
        session.step_once();
        assert_eq!(session.sim.buildings.len(), count + 1);
        session.build_hovered(false); // occupied site must neither enqueue nor leave build mode
        assert_eq!(session.pending_count(), 0);
        assert!(session.placing);
        session.set_hover(None);
        session.build_hovered(false);
        assert_eq!(session.pending_count(), 0);
    }
    #[test]
    fn terrain_invalidation_follows_commits_not_queued_intent() {
        let mut sim = genesis_sim_core::scenarios::coastal_outpost();
        sim.grid.boundary = genesis_sim_core::grid::Boundary::Clamp;
        let mut s = Session::from_sim(sim);
        s.clock.paused = true;
        s.urgent_redraw = false;
        let revision = s.terrain_revision;
        s.queue(Action::Terraform {
            x: i32::MAX,
            y: 0,
            radius: 3,
            strength: 1.,
        });
        s.step_once();
        assert_eq!(s.terrain_revision, revision);
        assert!(!s.urgent_redraw);
        assert!(s.message_is_error);
        s.queue(Action::Terraform {
            x: 15,
            y: 12,
            radius: 2,
            strength: 0.,
        });
        s.step_once();
        assert_eq!(s.terrain_revision, revision);
        assert!(!s.urgent_redraw);
        assert!(!s.message_is_error);
        s.queue(Action::Terraform {
            x: 15,
            y: 12,
            radius: 2,
            strength: 0.4,
        });
        s.queue(Action::Terraform {
            x: i32::MAX,
            y: 0,
            radius: 3,
            strength: 1.,
        });
        s.step_once();
        assert_eq!(s.terrain_revision, revision + 1);
        assert!(
            s.urgent_redraw,
            "a later rejection cannot hide an earlier commit"
        );
        assert!(s.message_is_error);
        assert_eq!(s.pending_count(), 0);
    }

    #[test]
    fn queued_actions_survive_pause_and_are_consumed_exactly_once() {
        let mut session = Session::from_sim(genesis_sim_core::scenarios::coastal_outpost());
        session.clock.paused = true;
        let before = session.sim.grid.get(15, 12).elevation;
        session.queue(Action::Terraform {
            x: 15,
            y: 12,
            radius: 2,
            strength: 0.5,
        });
        session.advance(Duration::from_secs(10));
        assert_eq!(session.sim.time, 0.0);
        assert_eq!(session.sim.grid.get(15, 12).elevation, before);
        session.step_once();
        assert!((session.sim.grid.get(15, 12).elevation - before - 0.5).abs() < 1e-5);
        assert_eq!(session.pending_count(), 0);
        session.clock.paused = false;
        session.advance(Duration::from_millis(200));
        assert!((session.sim.time - 0.3).abs() < 1e-6);
        assert!((session.sim.grid.get(15, 12).elevation - before - 0.5).abs() < 1e-5);
    }
    #[test]
    fn frame_partition_produces_identical_simulation_state() {
        let mut a = Session::from_sim(genesis_sim_core::scenarios::coastal_outpost());
        let mut b = Session::from_sim(genesis_sim_core::scenarios::coastal_outpost());
        let mut headless = Simulation::new(genesis_sim_core::scenarios::coastal_outpost());
        let actions = [
            Action::DrillWell {
                position: Position::new(18.25, 21.5),
            },
            Action::Terraform {
                x: 15,
                y: 12,
                radius: 2,
                strength: 0.4,
            },
        ];
        for action in &actions {
            a.queue(action.clone());
            b.queue(action.clone());
        }
        for tick in 0..10 {
            headless.advance(if tick == 0 { &actions } else { &[] });
        }
        for _ in 0..100 {
            a.advance(Duration::from_millis(10));
        }
        for _ in 0..10 {
            b.advance(Duration::from_millis(100));
        }
        assert_eq!(a.sim.time, b.sim.time);
        assert_eq!(a.sim.completed_ticks(), 10);
        assert_eq!(a.sim.completed_ticks(), headless.completed_ticks());
        assert_eq!(format!("{:?}", *a.sim), format!("{:?}", *headless));
        assert_eq!(format!("{:?}", *b.sim), format!("{:?}", *headless));
        for (a, b) in a.sim.grid.cells().iter().zip(b.sim.grid.cells()) {
            assert_eq!(a.fungus, b.fungus);
            assert_eq!(a.plants, b.plants);
            assert_eq!(a.fertility, b.fertility);
        }
    }
}

#[cfg(test)]
mod terrain_changes;

#[cfg(test)]
mod resident_tests {
    use super::*;
    #[test]
    fn irrigation_and_loss_are_independent_of_frame_partition() {
        use genesis_sim_core::{food::SourceId, food::cultivation::irrigation, scenarios};
        let mut initial = scenarios::dry_cultivated_settlement();
        for _ in 0..200 {
            initial.step(0.1, &[]);
        }
        let mut a = Session::from_sim(initial.clone());
        let mut b = Session::from_sim(initial);
        for session in [&mut a, &mut b] {
            session.queue(Action::IrrigateCrop {
                source: SourceId(1),
                well: 1,
            });
            session.queue(Action::IrrigateCrop {
                source: SourceId(1),
                well: 1,
            });
        }
        for _ in 0..8000 {
            a.advance(Duration::from_millis(10));
        }
        for _ in 0..800 {
            b.advance(Duration::from_millis(100));
        }
        assert_eq!(
            serde_json::to_vec(&*a.sim).unwrap(),
            serde_json::to_vec(&*b.sim).unwrap()
        );
        assert_eq!(irrigation::total_applied(&a.sim), 4.);
        assert_eq!(a.sim.food.sources()[0].produced(), 24);
    }
    #[test]
    fn render_pacing_and_inspection_do_not_change_residents() {
        let initial = genesis_sim_core::scenarios::supplied_settlement();
        let mut a = Session::from_sim(initial.clone());
        let mut b = Session::from_sim(initial);
        let id = a.sim.people.all()[0].id;
        a.select_object(Some(crate::interaction::Object::Person(id)), a.position);
        for _ in 0..18000 {
            a.advance(Duration::from_millis(10));
            let before = a.sim.people.clone();
            let _ = crate::scene::overlay(&a, None);
            assert_eq!(before, a.sim.people);
        }
        for _ in 0..1800 {
            b.advance(Duration::from_millis(100));
        }
        assert_eq!(a.sim.completed_ticks(), b.sim.completed_ticks());
        assert_eq!(a.sim.people, b.sim.people);
        assert_eq!(a.sim.households, b.sim.households);
        assert_eq!(a.sim.food, b.sim.food);
        assert!(a.sim.food.homes().values().all(|h| h.consumed > 0));
        assert_eq!(a.sim.subsurface.wells, b.sim.subsurface.wells);
        assert!(a.sim.households.all().values().all(|h| h.consumed > 0.));
        assert_eq!(a.position, a.sim.people.get(id).unwrap().position);
    }
    #[test]
    fn interpolation_picking_and_world_replacement_share_current_identity() {
        let mut s = Session::from_sim(genesis_sim_core::scenarios::living_settlement());
        for _ in 0..20 {
            s.advance(Duration::from_millis(100));
        }
        s.advance(Duration::from_millis(50));
        let p = &s.sim.people.all()[0];
        let displayed = s.person_position(p);
        let target = crate::terrain::surface_point(&s.sim, displayed)
            + glam::Vec3::Y * genesis_sim_core::units::metres(1.1);
        let hit = crate::interaction::pick_at(
            &s.sim,
            target + glam::Vec3::Y,
            -glam::Vec3::Y,
            None,
            |p| s.person_position(p),
        );
        assert_eq!(hit, Some(crate::interaction::Object::Person(p.id)));
        s.clock.paused = true;
        s.advance(Duration::from_millis(50));
        assert_eq!(
            s.person_position(&s.sim.people.all()[0]),
            s.sim.people.all()[0].position
        );
        s.replace_sim(genesis_sim_core::scenarios::living_settlement());
        assert!(s.previous_people.is_empty());
        assert!(s.selected_object.is_none());
    }
}
