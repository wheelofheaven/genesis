//! Application flow lives outside the simulation: menus never advance world time.
mod expedition;
#[cfg(test)]
mod interaction_tests;
mod orbit;
mod saves;
mod screens;
mod settings;
use crate::{session::Session, terrain};
use genesis_renderer::camera::Camera;
use genesis_sim_core::{
    SimState,
    worldgen::{self, Region},
};
pub use settings::Settings;
use std::sync::mpsc::{self, Receiver, TryRecvError};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Screen {
    Home,
    Destination,
    Setup,
    Briefing,
    Arrival,
    Settings,
    Controls,
    Playing,
    Pause,
    Loading,
    ConfirmNew,
    ConfirmQuit,
    ConfirmLoad,
}
pub use genesis_sim_core::catalog::ExpeditionMode as Mode;

pub struct Frontend {
    pub screen: Screen,
    pub has_session: bool,
    pub mode: Mode,
    pub planet_id: String,
    planet_texture: Option<egui::TextureHandle>,
    pub region: Region,
    pub seed: String,
    pub settings: Settings,
    pub draft: Settings,
    pub message: String,
    notice: String,
    pub quit: bool,
    return_to: Screen,
    quit_return: Screen,
    load_return: Screen,
    save_path: Result<std::path::PathBuf, String>,
    pub(crate) loading: Option<Receiver<SimState>>,
}
impl Default for Frontend {
    fn default() -> Self {
        let settings = Settings::load();
        Self {
            screen: Screen::Home,
            has_session: false,
            mode: Mode::default(),
            planet_id: genesis_sim_core::catalog::EARTH.id.into(),
            planet_texture: None,
            region: Region::LevantCoast,
            seed: worldgen::DEFAULT_SEED.to_string(),
            draft: settings.clone(),
            settings,
            message: String::new(),
            notice: String::new(),
            quit: false,
            return_to: Screen::Home,
            quit_return: Screen::Home,
            load_return: Screen::Home,
            save_path: crate::persistence::path().map_err(|e| e.to_string()),
            loading: None,
        }
    }
}
impl Frontend {
    pub fn playing(&self) -> bool {
        self.screen == Screen::Playing
    }
    pub fn begin_expedition(&mut self, mode: Mode) {
        self.mode = mode;
        self.screen = Screen::Destination;
        self.message.clear();
        self.notice.clear();
    }
    pub fn surface_backdrop(&self) -> bool {
        match self.screen {
            Screen::Playing | Screen::Arrival | Screen::Pause => true,
            Screen::Settings | Screen::Controls => self.return_to == Screen::Pause,
            Screen::ConfirmQuit => match self.quit_return {
                Screen::Pause | Screen::Playing | Screen::Arrival => true,
                Screen::Settings | Screen::Controls => self.return_to == Screen::Pause,
                Screen::ConfirmLoad => self.load_return == Screen::Pause,
                _ => false,
            },
            Screen::ConfirmLoad => self.load_return == Screen::Pause,
            _ => false,
        }
    }
    pub fn enter_settings(&mut self) {
        self.return_to = self.screen;
        self.draft = self.settings.clone();
        self.message.clear();
        self.notice.clear();
        self.screen = Screen::Settings;
    }
    pub fn request_quit(&mut self) {
        if self.screen == Screen::ConfirmQuit {
            return;
        }
        self.message.clear();
        self.notice.clear();
        if self.has_session {
            self.quit_return = self.screen;
            self.screen = Screen::ConfirmQuit;
        } else {
            self.quit = true;
        }
    }
    pub fn escape(&mut self) {
        self.screen = match self.screen {
            Screen::Playing => Screen::Pause,
            Screen::Pause => Screen::Playing,
            Screen::Settings | Screen::Controls => self.return_to,
            Screen::ConfirmQuit => self.quit_return,
            Screen::ConfirmLoad => self.load_return,
            Screen::Destination => Screen::Home,
            Screen::Setup => Screen::Destination,
            Screen::Briefing => Screen::Setup,
            Screen::Arrival => Screen::Playing,
            Screen::ConfirmNew => Screen::Briefing,
            Screen::Loading => {
                self.loading = None;
                Screen::Briefing
            }
            Screen::Home => Screen::Home,
        };
        self.message.clear();
        self.notice.clear();
    }
    pub fn start(&mut self) {
        self.notice.clear();
        let Ok(seed) = self.seed.parse::<u64>() else {
            self.message = "Enter a whole-number seed from 0 to 18446744073709551615.".into();
            return;
        };
        let Some(scenario) = genesis_sim_core::catalog::scenario(&self.planet_id, self.region)
        else {
            self.message = "This destination has no available regional scenario.".into();
            return;
        };
        let region = scenario.region;
        let inhabited = self.mode == Mode::Settlement;
        let (tx, rx) = mpsc::channel();
        match std::thread::Builder::new()
            .name("world-generation".into())
            .spawn(move || {
                let _ = tx.send(worldgen::generate_scenario(seed, region, inhabited));
            }) {
            Ok(_) => {
                self.loading = Some(rx);
                self.screen = Screen::Loading;
                self.message.clear();
            }
            Err(error) => self.message = format!("Could not start generation: {error}"),
        }
    }
    /// Only replace the active session after a complete new world arrives.
    pub fn poll(&mut self, session: &mut Session, camera: &mut Camera) {
        if self.screen != Screen::Loading {
            return;
        }
        let Some(rx) = &self.loading else { return };
        match rx.try_recv() {
            Ok(sim) => {
                session.replace_sim(sim);
                session.mode = self.mode;
                if self.mode == Mode::WorldBuilder {
                    session.landscape_tools = true;
                }
                *camera = Camera::default();
                terrain::home_camera(&session.sim, camera);
                self.loading = None;
                self.has_session = true;
                if self.mode == Mode::Settlement {
                    camera.center(session.selected_point());
                    camera.set_visible_height(12.);
                }
                self.screen = Screen::Arrival;
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.loading = None;
                self.message =
                    "World generation could not finish. Your previous session is still available."
                        .into();
                self.screen = Screen::Briefing;
            }
        }
    }
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        session: &mut Session,
        camera: &mut Camera,
    ) -> Vec<egui::Rect> {
        screens::show(self, ctx, session, camera)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancelled_generation_cannot_replace_a_resumable_session() {
        let mut front = Frontend {
            screen: Screen::Loading,
            has_session: true,
            ..Default::default()
        };
        let (tx, rx) = mpsc::channel();
        front.loading = Some(rx);
        front.escape();
        assert!(front.has_session);
        assert_eq!(front.screen, Screen::Briefing);
        assert!(
            tx.send(genesis_sim_core::scenarios::coastal_outpost())
                .is_err()
        );
    }
    #[test]
    fn unsupported_planets_never_launch_an_earth_recipe() {
        let mut front = Frontend {
            planet_id: "unimplemented-world".into(),
            screen: Screen::Briefing,
            ..Default::default()
        };
        front.start();
        assert!(front.loading.is_none());
        assert_eq!(front.screen, Screen::Briefing);
        assert!(!front.message.is_empty());
    }
    #[test]
    fn deployed_world_waits_at_arrival_and_invalidates_cached_geometry() {
        let mut session = Session::from_sim(genesis_sim_core::scenarios::coastal_outpost());
        session.terrain_revision = 42;
        let mut front = Frontend {
            screen: Screen::Loading,
            ..Default::default()
        };
        let (tx, rx) = mpsc::channel();
        front.loading = Some(rx);
        tx.send(genesis_sim_core::scenarios::coastal_outpost())
            .unwrap();
        front.poll(&mut session, &mut Camera::default());
        assert_eq!(front.screen, Screen::Arrival);
        assert_eq!(session.terrain_revision, 43);
        assert_eq!(session.sim.time, 0.);
        assert!(!front.playing());
        assert!(front.has_session);
        front.escape();
        assert!(front.playing());
    }
    #[test]
    fn cancelling_quit_from_settings_preserves_both_return_destinations() {
        let mut front = Frontend {
            screen: Screen::Pause,
            has_session: true,
            ..Default::default()
        };
        front.enter_settings();
        assert!(front.surface_backdrop());
        front.request_quit();
        assert_eq!(front.screen, Screen::ConfirmQuit);
        assert!(front.surface_backdrop());
        front.escape();
        assert_eq!(front.screen, Screen::Settings);
        front.escape();
        assert_eq!(front.screen, Screen::Pause);
    }
    #[test]
    fn menus_preserve_pause_state_and_pending_actions() {
        let mut session = Session::from_sim(genesis_sim_core::scenarios::coastal_outpost());
        session.clock.paused = true;
        session.queue(genesis_sim_core::actions::Action::SeedPlants {
            x: 12,
            y: 12,
            radius: 1,
        });
        let mut front = Frontend {
            screen: Screen::Playing,
            has_session: true,
            ..Default::default()
        };
        front.escape();
        front.enter_settings();
        front.escape();
        assert_eq!(front.screen, Screen::Pause);
        front.escape();
        assert!(front.playing());
        assert!(session.clock.paused);
        assert_eq!(session.pending_count(), 1);
    }
}
