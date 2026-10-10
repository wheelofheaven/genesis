use crate::session::Session;
use genesis_renderer::camera::Camera;
use glam::Vec2;
use std::collections::HashSet;
use winit::keyboard::KeyCode;

#[derive(Default)]
pub struct Input {
    pub cursor: Option<Vec2>,
    pub orbit: bool,
    pub pan: bool,
    pub shift: bool,
    pub alt: bool,
    keys: HashSet<KeyCode>,
}
impl Input {
    /// Reserved game shortcuts; Tab and Enter remain available to the HUD.
    pub fn is_shortcut(code: KeyCode) -> bool {
        matches!(
            code,
            KeyCode::Space
                | KeyCode::Digit1
                | KeyCode::Digit2
                | KeyCode::Digit3
                | KeyCode::KeyB
                | KeyCode::KeyF
                | KeyCode::KeyR
                | KeyCode::KeyH
                | KeyCode::KeyV
                | KeyCode::KeyQ
                | KeyCode::KeyE
                | KeyCode::Escape
                | KeyCode::KeyW
                | KeyCode::KeyA
                | KeyCode::KeyS
                | KeyCode::KeyD
                | KeyCode::ArrowUp
                | KeyCode::ArrowDown
                | KeyCode::ArrowLeft
                | KeyCode::ArrowRight
        )
    }

    pub fn clear(&mut self) {
        self.orbit = false;
        self.pan = false;
        self.cursor = None;
        self.keys.clear();
        self.shift = false;
        self.alt = false;
    }
    pub fn key(
        &mut self,
        code: KeyCode,
        pressed: bool,
        session: &mut Session,
        camera: &mut Camera,
    ) {
        if !pressed {
            self.keys.remove(&code);
            return;
        }
        if !self.keys.insert(code) {
            return;
        }
        match code {
            KeyCode::Space => session.clock.paused = !session.clock.paused,
            KeyCode::Digit1 => session.clock.set_speed(1),
            KeyCode::Digit2 => session.clock.set_speed(4),
            KeyCode::Digit3 => session.clock.set_speed(16),
            KeyCode::KeyR => crate::terrain::home_camera(&session.sim, camera),
            KeyCode::KeyB => session.set_placing(!session.placing),
            KeyCode::Escape => {
                session.set_placing(false);
                session.help = false;
                session.landscape_tools = false;
                session.survey.open = false;
                session.world_tools = false;
            }
            KeyCode::KeyF => camera.center(session.selected_point()),
            KeyCode::KeyH => session.help = !session.help,
            KeyCode::KeyV => session.set_survey(!session.survey.open),
            KeyCode::KeyQ | KeyCode::KeyE => {
                let sign = if code == KeyCode::KeyQ { -1. } else { 1. };
                if session.placing {
                    session.rotate_building(sign * 15_f32.to_radians());
                } else {
                    camera.orbit(Vec2::new(sign * 35., 0.));
                }
            }
            _ => {}
        }
    }
    pub fn update_camera(&self, camera: &mut Camera, seconds: f32, height: f32) {
        let has = |a, b| self.keys.contains(&a) || self.keys.contains(&b);
        let x = i32::from(has(KeyCode::KeyA, KeyCode::ArrowLeft))
            - i32::from(has(KeyCode::KeyD, KeyCode::ArrowRight));
        let y = i32::from(has(KeyCode::KeyW, KeyCode::ArrowUp))
            - i32::from(has(KeyCode::KeyS, KeyCode::ArrowDown));
        let delta = Vec2::new(x as f32, y as f32).normalize_or_zero()
            * seconds.min(0.1)
            * height
            * if self.shift { 1.2 } else { 0.45 };
        camera.pan(delta, height);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shortcuts_cancel_without_exiting_and_focus_loss_stops_movement() {
        let mut input = Input::default();
        let mut session = Session::default();
        let mut camera = Camera::default();
        input.key(KeyCode::KeyB, true, &mut session, &mut camera);
        assert!(session.placing);
        input.key(KeyCode::KeyB, true, &mut session, &mut camera);
        assert!(session.placing); // held key does not toggle repeatedly
        input.key(KeyCode::Escape, true, &mut session, &mut camera);
        assert!(!session.placing);
        input.key(KeyCode::KeyW, true, &mut session, &mut camera);
        input.clear();
        let before = camera.focus;
        input.update_camera(&mut camera, 0.1, 800.);
        assert_eq!(before, camera.focus);
    }
}
