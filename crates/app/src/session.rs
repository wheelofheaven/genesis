//! Application session owning simulation, pending commands, and presentation choices.
use crate::clock::{Clock, TICK};
use genesis_sim_core::{
    SimState, actions::Action, events::SimEvent, settlement::Building, spatial::Position,
};
use std::{collections::VecDeque, time::Duration};

pub struct Session {
    pub mode: crate::frontend::Mode,
    pub sim: SimState,
    pub clock: Clock,
    pub selected: (i32, i32),
    pub position: Position,
    pub building_rotation: f32,
    pub placing: bool,
    pub hover: Option<Position>,
    pub overlay_dirty: bool,
    pub urgent_redraw: bool,
    pub terrain_revision: u64,
    pub help: bool,
    pub landscape_tools: bool,
    pub world_tools: bool,
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
        let position = sim.buildings.first().map_or(
            Position::new(sim.grid.width as f32 / 2., sim.grid.height as f32 / 2.),
            |b| b.position,
        );
        let region_choice = sim.geography.region;
        Self {
            mode: crate::frontend::Mode::Settlement,
            region_choice,
            sim,
            clock: Clock::default(),
            selected: (position.x.round() as i32, position.z.round() as i32),
            position,
            building_rotation: 0.,
            placing: false,
            hover: None,
            overlay_dirty: true,
            urgent_redraw: true,
            terrain_revision: 0,
            help: false,
            landscape_tools: false,
            world_tools: false,
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
    pub fn regenerate(&mut self, seed: u64) {
        let revision = self.terrain_revision + 1;
        let mode = self.mode;
        *self = Self::from_sim(genesis_sim_core::worldgen::generate_scenario(
            seed,
            self.region_choice,
            mode == crate::frontend::Mode::Settlement,
        ));
        self.mode = mode;
        self.terrain_revision = revision;
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
        for _ in 0..self.clock.advance(elapsed) {
            self.tick();
        }
    }
    pub fn step_once(&mut self) {
        if self.clock.paused {
            self.tick();
        }
    }
    fn tick(&mut self) {
        if self
            .pending
            .iter()
            .any(|a| matches!(a, Action::Terraform { .. }))
        {
            self.terrain_revision += 1;
        }
        self.urgent_redraw |= !self.pending.is_empty();
        let events = self.sim.step(TICK.as_secs_f32(), &self.pending);
        self.pending.clear();
        for event in events {
            match &event {
                SimEvent::WellDrilled { id } => {
                    self.message = format!("Borehole {id} online");
                    self.message_is_error = false;
                }
                SimEvent::WellRejected { reason } => {
                    self.message = reason.to_string();
                    self.message_is_error = true;
                }
                SimEvent::WellOperationChanged { id, active } => {
                    self.message = format!(
                        "Borehole {id} {}",
                        if *active { "pumping" } else { "stopped" }
                    );
                    self.message_is_error = false;
                }

                SimEvent::BuildingPlaced { id } => {
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
        self.dirty = true;
        self.overlay_dirty = true;
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
        self.position = position;
        self.selected = (position.x.round() as i32, position.z.round() as i32);
        self.overlay_dirty = true;
    }
    pub fn set_placing(&mut self, placing: bool) {
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
    pub fn set_hover(&mut self, hover: Option<Position>) {
        if self.hover != hover {
            self.hover = hover;
            self.overlay_dirty = true;
        }
    }
    pub fn build_hovered(&mut self, repeat: bool) {
        if !self.placing || self.hover.is_none() {
            return;
        }
        let candidate = self.candidate();
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
        for _ in 0..100 {
            a.advance(Duration::from_millis(10));
        }
        for _ in 0..10 {
            b.advance(Duration::from_millis(100));
        }
        assert_eq!(a.sim.time, b.sim.time);
        for (a, b) in a.sim.grid.cells().iter().zip(b.sim.grid.cells()) {
            assert_eq!(a.fungus, b.fungus);
            assert_eq!(a.plants, b.plants);
            assert_eq!(a.fertility, b.fertility);
        }
    }
}
