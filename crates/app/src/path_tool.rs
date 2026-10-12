//! Drafts are presentation state; only a validated Action changes the world.
use crate::session::Session;
use genesis_sim_core::{
    actions::Action,
    paths,
    settlement::{Building, Path},
    spatial::Position,
    units::metres,
};

struct Cached {
    points: Vec<Position>,
    width: f32,
    revision: u64,
    result: Result<Path, paths::PathError>,
}
pub struct Draft {
    pub active: bool,
    pub points: Vec<Position>,
    pub width: f32,
    cache: std::cell::RefCell<Vec<Cached>>,
}
impl Default for Draft {
    fn default() -> Self {
        Self {
            active: false,
            points: Vec::new(),
            width: Building::PATH_WIDTH,
            cache: Default::default(),
        }
    }
}
impl Session {
    pub fn set_path_tool(&mut self, active: bool) {
        self.set_placing(false);
        self.path.active = active;
        if active {
            self.landscape_tools = false;
            self.survey.open = false;
            self.world_tools = false;
        }
        self.overlay_dirty = true;
    }
    pub fn path_endpoint(&self) -> Option<Position> {
        let p = self.hover?;
        self.sim
            .paths
            .iter()
            .flat_map(|path| path.points.windows(2))
            .map(|s| paths::nearest(p, s[0], s[1]))
            .filter(|q| paths::distance(p, *q) < metres(1.5))
            .min_by(|a, b| paths::distance(p, *a).total_cmp(&paths::distance(p, *b)))
            .or(Some(p))
    }
    pub fn path_waypoints(&self, preview: bool) -> Vec<Position> {
        let mut points = self.path.points.clone();
        if preview
            && let Some(p) = self.path_endpoint()
            && points
                .last()
                .is_none_or(|q| paths::distance(*q, p) >= metres(0.5))
        {
            points.push(p);
        }
        points
    }
    pub fn path_validation(&self, preview: bool) -> Result<Path, paths::PathError> {
        let points = self.path_waypoints(preview);
        let mut cache = self.path.cache.borrow_mut();
        if let Some(entry) = cache.iter().find(|e| {
            e.points == points && e.width == self.path.width && e.revision == self.terrain_revision
        }) {
            return entry.result.clone();
        }
        let result = paths::validate(&self.sim, &points, self.path.width);
        if cache.len() == 2 {
            cache.remove(0);
        }
        cache.push(Cached {
            points,
            width: self.path.width,
            revision: self.terrain_revision,
            result: result.clone(),
        });
        result
    }
    pub fn path_preview(&self) -> Option<(Path, bool)> {
        match self.path_validation(true) {
            Ok(path) => Some((path, true)),
            Err(_) => paths::route(&self.path_waypoints(true), self.path.width)
                .ok()
                .map(|p| (p, false)),
        }
    }
    pub fn add_path_point(&mut self) {
        if !self.path.active {
            return;
        }
        if let Some(p) = self.path_endpoint() {
            if self.path.points.len() >= 64 {
                self.message = "Finish this path before adding more points".into();
                self.message_is_error = true;
                return;
            }
            if self
                .path
                .points
                .last()
                .is_none_or(|q| paths::distance(*q, p) >= metres(0.5))
            {
                self.path.points.push(p);
                self.message.clear();
                self.overlay_dirty = true;
            }
        }
    }
    pub fn undo_path_point(&mut self) {
        self.path.points.pop();
        self.overlay_dirty = true;
    }
    pub fn finish_path(&mut self) {
        if !self.path.active {
            return;
        }
        match self.path_validation(false) {
            Ok(_) => {
                self.queue(Action::BuildPath {
                    waypoints: self.path.points.clone(),
                    width: self.path.width,
                });
                self.path.points.clear();
                self.message = "Path queued".into();
                self.message_is_error = false;
                self.overlay_dirty = true;
            }
            Err(e) => {
                self.message = e.to_string();
                self.message_is_error = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn drafting_undo_cancel_and_paused_commit_keep_world_changes_explicit() {
        let mut sim = genesis_sim_core::SimState::new(genesis_sim_core::config::SimConfig {
            width: 16,
            height: 16,
            ..Default::default()
        });
        for c in sim.grid.cells_mut() {
            c.elevation = 1.;
        }
        let mut s = Session::from_sim(sim);
        s.clock.paused = true;
        s.set_path_tool(true);
        for p in [
            Position::new(4., 4.),
            Position::new(5., 4.2),
            Position::new(6., 4.),
        ] {
            s.set_hover(Some(p));
            s.add_path_point();
        }
        s.undo_path_point();
        assert_eq!(s.path.points.len(), 2);
        s.finish_path();
        assert_eq!(s.pending_count(), 1);
        assert!(s.sim.paths.is_empty());
        s.step_once();
        assert_eq!(s.sim.paths.len(), 1);
        assert!(s.path.points.is_empty());
        assert_eq!(s.selected_object, Some(crate::interaction::Object::Path(1)));
        s.step_once();
        assert_eq!(s.sim.paths.len(), 1);
        s.set_hover(Some(Position::new(7., 7.)));
        s.add_path_point();
        s.set_placing(true);
        assert!(!s.path.active);
        assert!(s.path.points.is_empty());
    }
}
