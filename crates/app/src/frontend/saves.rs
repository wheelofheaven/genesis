//! Menu storage operations use the same staged load and atomic writer as F5/F9.
use super::{Frontend, Screen};
use crate::{persistence, session::Session};
use genesis_renderer::camera::Camera;

impl Frontend {
    /// Capture runs use a sibling slot rather than the player's saved expedition.
    pub(crate) fn set_save_path(&mut self, path: std::path::PathBuf) {
        self.save_path = Ok(path);
    }

    pub(crate) fn save_expedition(&mut self, session: &Session) -> bool {
        self.message.clear();
        self.notice.clear();
        if !self.has_session {
            self.message = "Begin an expedition before saving.".into();
            return false;
        }
        let result = self
            .save_path
            .as_ref()
            .map_err(Clone::clone)
            .and_then(|path| persistence::save_at(session, path).map_err(|e| e.to_string()));
        match result {
            Ok(()) => {
                self.notice = format!("Expedition saved · tick {}", session.sim.completed_ticks());
                true
            }
            Err(error) => {
                self.message = format!("Save failed: {error}. Your expedition is still open.");
                false
            }
        }
    }

    pub(crate) fn request_load(&mut self, session: &mut Session, camera: &mut Camera) {
        self.message.clear();
        self.notice.clear();
        if self.has_session {
            self.load_return = self.screen;
            self.screen = Screen::ConfirmLoad;
        } else {
            self.load_expedition(session, camera);
        }
    }

    pub(crate) fn load_expedition(&mut self, session: &mut Session, camera: &mut Camera) {
        self.message.clear();
        self.notice.clear();
        let result = self.save_path.as_ref().map_err(Clone::clone).and_then(|path| {
            if !path.try_exists().map_err(|e| e.to_string())? {
                return Err("No saved expedition yet. Begin one, then choose Save expedition from the pause menu".into());
            }
            persistence::load_at(session, camera, path).map_err(|e| e.to_string())
        });
        match result {
            Ok(()) => {
                self.loading = None;
                self.has_session = true;
                self.mode = session.mode;
                self.region = session.region_choice;
                self.seed = session.sim.config.seed.to_string();
                self.planet_id = genesis_sim_core::catalog::EARTH.id.into();
                self.screen = Screen::Playing;
            }
            Err(error) => {
                self.message = format!("Load failed: {error}");
            }
        }
    }
}
