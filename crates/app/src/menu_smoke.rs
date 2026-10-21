//! Native regression path for menu transitions and asynchronous world replacement.
use crate::{
    frontend::{Frontend, Mode, Screen},
    session::Session,
};
use genesis_renderer::camera::Camera;
use std::time::Instant;

#[derive(Default)]
pub struct Check {
    stage: u8,
    began: Option<Instant>,
    time: f64,
    revision: u64,
    changed_at: u32,
    saved_state: Vec<u8>,
    saved_identity: String,
}
impl Check {
    /// Returns a capture suffix. The final capture happens only after completion.
    pub fn advance(
        &mut self,
        frame: u32,
        front: &mut Frontend,
        session: &mut Session,
        camera: &mut Camera,
    ) -> Result<Option<&'static str>, Box<dyn std::error::Error>> {
        let start = *self.began.get_or_insert_with(Instant::now);
        if start.elapsed().as_secs() > 90 {
            return Err("menu flow timed out".into());
        }
        if self.stage == 0 && frame == 10 {
            return Ok(Some("title.png"));
        }
        if self.stage == 0 && frame == 12 {
            front.begin_expedition(Mode::Settlement);
        }
        if self.stage == 0 && frame == 16 {
            return Ok(Some("destination.png"));
        }
        if self.stage == 0 && frame == 18 {
            front.enter_settings();
        }
        if self.stage == 0 && frame == 22 {
            return Ok(Some("settings.png"));
        }
        if self.stage == 0 && frame == 24 {
            front.escape();
            if front.screen != Screen::Destination || session.sim.time != 0. {
                return Err("menu advanced time or lost its return page".into());
            }
            front.screen = Screen::Setup;
        }
        if self.stage == 0 && frame == 30 {
            return Ok(Some("region.png"));
        }
        if self.stage == 0 && frame == 32 {
            front.screen = Screen::Briefing;
        }
        if self.stage == 0 && frame == 36 {
            return Ok(Some("briefing.png"));
        }
        if self.stage == 0 && frame == 38 {
            self.revision = session.terrain_revision;
            front.start();
            self.stage = 1;
        }
        if self.stage == 1 && front.screen == Screen::Arrival {
            if session.sim.buildings.is_empty()
                || session.terrain_revision == self.revision
                || session.sim.time != 0.
            {
                return Err(
                    "deployment lost its settlement, invalidation, or arrival pause".into(),
                );
            }
            self.stage = 5;
            return Ok(Some("arrival.png"));
        }
        if self.stage == 5 {
            front.escape();
            if !front.playing() {
                return Err("arrival could not be skipped".into());
            }
            session.clock.paused = true;
            session.landscape_tools = false;
            self.time = session.sim.time;
            front.escape();
            self.stage = 6;
            self.changed_at = frame;
            return Ok(None);
        }
        if self.stage == 6 && frame >= self.changed_at + 4 {
            self.stage = 2;
            self.changed_at = frame;
            return Ok(Some("pause.png"));
        }
        if self.stage == 2 && frame >= self.changed_at + 4 {
            front.enter_settings();
            self.stage = 7;
            self.changed_at = frame;
        }
        if self.stage == 7 && frame >= self.changed_at + 4 {
            self.stage = 8;
            self.changed_at = frame;
            return Ok(Some("surface-settings.png"));
        }
        if self.stage == 8 && frame >= self.changed_at + 4 {
            front.escape();
            if front.screen != Screen::Pause {
                return Err("surface settings lost the pause menu".into());
            }
            if session.sim.time != self.time {
                return Err("paused menu advanced simulation".into());
            }
            session.queue(genesis_sim_core::actions::Action::SeedPlants {
                x: 12,
                y: 12,
                radius: 1,
            });
            self.saved_state = serde_json::to_vec(&*session.sim)?;
            self.saved_identity = session.world_id.clone();
            if !front.save_expedition(session) {
                return Err(format!("menu save failed: {}", front.message).into());
            }
            session.step_once();
            front.screen = Screen::Home;
            if !front.has_session {
                return Err("main menu lost the session".into());
            }
            self.stage = 3;
            return Ok(Some("continue.png"));
        }
        if self.stage == 3 && frame.is_multiple_of(10) {
            front.request_load(session, camera);
            if front.screen != Screen::ConfirmLoad {
                return Err("loading over a live expedition skipped confirmation".into());
            }
            self.stage = 10;
            self.changed_at = frame;
            return Ok(None);
        }
        if self.stage == 10 && frame >= self.changed_at + 4 {
            self.stage = 11;
            self.changed_at = frame;
            return Ok(Some("load-confirmation.png"));
        }
        if self.stage == 11 && frame >= self.changed_at + 4 {
            front.escape();
            if front.screen != Screen::Home || session.sim.time == self.time {
                return Err("cancelled load changed the live expedition".into());
            }
            // Model a fresh application's placeholder world, retaining only the save path.
            front.has_session = false;
            session.replace_sim(genesis_sim_core::scenarios::coastal_outpost());
            front.request_load(session, camera);
            if !front.playing()
                || !front.has_session
                || !session.clock.paused
                || session.world_id != self.saved_identity
                || session.pending_count() != 1
                || serde_json::to_vec(&*session.sim)? != self.saved_state
            {
                return Err("menu load did not restore the saved expedition".into());
            }
            self.stage = 4;
            self.changed_at = frame;
        }
        // New egui Areas need a measured frame before they paint. Capture a
        // settled HUD, not the empty first frame after returning to play.
        if self.stage == 4 && frame >= self.changed_at + 4 {
            self.stage = 9;
        }
        Ok(None)
    }
    pub fn done(&self) -> bool {
        self.stage == 9
    }
}
