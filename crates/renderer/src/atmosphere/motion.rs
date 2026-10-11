//! Integrate the current layer wind rather than multiplying it by world age.
//! This is presentation state; a future weather simulation supplies the velocity.
use glam::Vec2;

#[derive(Default)]
pub(crate) struct CloudMotion {
    time: f64,
    displacement: [f64; 2],
}

impl CloudMotion {
    /// Rewinding the presentation clock explicitly starts a new deterministic preview.
    pub fn update(&mut self, time: f64, wind: Vec2) -> Vec2 {
        if !time.is_finite() || time < 0. {
            return Vec2::from_array(self.displacement.map(|v| v as f32));
        }
        if time < self.time {
            *self = Self::default();
        }
        let dt = time - self.time;
        self.time = time;
        for (position, velocity) in self.displacement.iter_mut().zip(wind.to_array()) {
            let velocity = super::finite(velocity, 0.).clamp(-150., 150.) as f64;
            // Reduce time before multiplication, including for extremely large clocks.
            if velocity != 0. {
                let period = f64::from(crate::clouds::TILE_METRES) / velocity.abs();
                *position = (*position + dt.rem_euclid(period) * velocity)
                    .rem_euclid(f64::from(crate::clouds::TILE_METRES));
            }
        }
        Vec2::from_array(self.displacement.map(|v| v as f32))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changing_wind_preserves_position_and_calm_stops_motion() {
        let mut motion = CloudMotion::default();
        assert_eq!(
            motion.update(100., Vec2::new(8., 4.)),
            Vec2::new(800., 400.)
        );
        assert_eq!(
            motion.update(100., Vec2::new(-10., 3.)),
            Vec2::new(800., 400.)
        );
        assert_eq!(
            motion.update(110., Vec2::new(-10., 3.)),
            Vec2::new(700., 430.)
        );
        assert_eq!(motion.update(200., Vec2::ZERO), Vec2::new(700., 430.));
        assert_eq!(
            motion.update(201., Vec2::new(0., -5.)),
            Vec2::new(700., 425.)
        );
    }

    #[test]
    fn integration_is_frame_rate_independent_and_wraps_without_jumps() {
        let wind = Vec2::new(-7.25, 3.125);
        let mut motion = CloudMotion::default();
        for frame in 1..=6000 {
            motion.update(frame as f64 / 60., wind);
        }
        let expected = CloudMotion::default().update(100., wind);
        assert!((motion.update(100., wind) - expected).length() < 0.001);
        let period = 16000. / 7.25;
        let before = motion.update(period - 0.01, wind);
        let after = motion.update(period + 0.01, wind);
        let dx = (after.x - before.x + 8000.).rem_euclid(16000.) - 8000.;
        assert!((dx + 7.25 * 0.02).abs() < 0.002);
        assert_eq!(motion.update(1., wind), Vec2::new(15992.75, 3.125));
        let previous = motion.update(1., wind);
        assert_eq!(motion.update(f64::NAN, wind), previous);
        assert_eq!(motion.update(2., Vec2::splat(f32::NAN)), previous);
        assert!(motion.update(f64::MAX, wind).is_finite());
    }
}
