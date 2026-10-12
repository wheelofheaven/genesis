use crate::atmosphere::FrameUniform;

/// Exactly the inputs read by sky_lookup.wgsl. Rotation, wind and horizontal
/// travel do not affect the directional table; no altitude quantization is used.
#[derive(Clone, Copy, PartialEq)]
pub(super) struct Key {
    altitude_km: f32,
    rayleigh: [f32; 4],
    mie: [f32; 4],
    layers: [f32; 4],
}
impl Key {
    pub fn new(frame: &FrameUniform) -> Self {
        Self {
            altitude_km: ((frame.eye_sea_level[1] - frame.eye_sea_level[3]) * 0.02)
                .clamp(0., frame.air_layers[1] * 0.9),
            rayleigh: frame.rayleigh,
            mie: frame.mie,
            layers: frame.air_layers,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{atmosphere::Atmosphere, camera::Camera};
    #[test]
    fn sky_cache_tracks_transport_inputs_without_camera_or_wind_churn() {
        let camera = Camera::default();
        let frame = FrameUniform::new(&camera, 1.6, 0., Atmosphere::default(), glam::Vec2::ZERO);
        let key = Key::new(&frame);
        let mut other = frame;
        other.cloud_motion[0] += 500.;
        other.clouds[1] = 0.9;
        other.eye_sea_level[0] += 500.;
        other.eye_sea_level[2] -= 500.;
        other.direction_time[0] += 1.;
        assert!(key == Key::new(&other));
        for change in 0..8 {
            let mut other = frame;
            match change {
                0 => other.eye_sea_level[1] += 1.,
                1 => other.eye_sea_level[3] += 1.,
                2 => other.rayleigh[0] += 0.1,
                3 => other.mie[0] += 0.1,
                i => other.air_layers[i - 4] += 0.1,
            }
            assert!(key != Key::new(&other));
        }
    }
}
