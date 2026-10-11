//! An inexpensive, art-directed regional atmosphere. No weather simulation or
//! planetary scattering solver: these parameters can later come from a planet.
use glam::Vec3;

/// Presentation parameters, independent of any future climate model.
#[derive(Clone, Copy)]
pub struct CloudLayer {
    pub height_metres: f32,
    pub cover: f32,
    pub shadow_strength: f32,
}

impl Default for CloudLayer {
    fn default() -> Self {
        Self {
            height_metres: 1800.,
            cover: 0.46,
            shadow_strength: 0.40,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Atmosphere {
    pub zenith: Vec3,
    pub horizon: Vec3,
    pub density: f32,
    pub scale_height_metres: f32,
    pub sea_level: f32,
    pub survey: bool,
    pub clouds: CloudLayer,
}

impl Default for Atmosphere {
    fn default() -> Self {
        Self {
            zenith: Vec3::new(0.12, 0.27, 0.50),
            horizon: Vec3::new(0.62, 0.72, 0.78),
            density: 0.00009,
            scale_height_metres: 900.,
            sea_level: 0.,
            survey: false,
            clouds: CloudLayer::default(),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct FrameUniform {
    pub projection: [f32; 16],
    pub direction_time: [f32; 4],
    pub light_projection: [f32; 16],
    pub far_light_projection: [f32; 16],
    pub eye_sea_level: [f32; 4],
    pub inverse_projection: [f32; 16],
    pub zenith: [f32; 4],
    pub horizon: [f32; 4],
    pub haze: [f32; 4],
    pub clouds: [f32; 4],
    pub cloud_motion: [f32; 4],
}

impl FrameUniform {
    pub fn new(
        camera: &crate::camera::Camera,
        aspect: f32,
        time: f32,
        atmosphere: Atmosphere,
    ) -> Self {
        let matrix = camera.view_projection(aspect);
        Self {
            projection: matrix.to_cols_array(),
            direction_time: camera.direction().extend(time).to_array(),
            light_projection: crate::shadows::Shadows::matrix(camera.focus).to_cols_array(),
            far_light_projection: crate::shadows::Shadows::cascade_matrix(camera.focus, 1)
                .to_cols_array(),
            eye_sea_level: camera.eye().extend(atmosphere.sea_level).to_array(),
            inverse_projection: matrix.inverse().to_cols_array(),
            zenith: atmosphere.zenith.extend(0.).to_array(),
            horizon: atmosphere.horizon.extend(0.).to_array(),
            haze: [
                atmosphere.density.max(0.),
                atmosphere.scale_height_metres.max(1.),
                if atmosphere.survey { 1. } else { 0. },
                0.,
            ],
            clouds: [
                atmosphere.sea_level * 20. + atmosphere.clouds.height_metres.max(1.),
                if atmosphere.survey {
                    0.
                } else {
                    atmosphere.clouds.cover.clamp(0., 1.)
                },
                atmosphere.clouds.shadow_strength.clamp(0., 1.),
                crate::clouds::TILE_METRES,
            ],
            // Fixed 8/4 m/s artistic drift. Both axes repeat at 4000 seconds;
            // periodic texture addressing makes that wrap continuous.
            cloud_motion: [
                time.rem_euclid(4000.) * 8.,
                time.rem_euclid(4000.) * 4.,
                0.,
                0.,
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn frame_layout_and_sky_reconstruction_match_camera() {
        assert_eq!(size_of::<FrameUniform>(), 368);
        let mut camera = crate::camera::Camera::default();
        camera.enter_ground(Vec3::new(100., 20., -120.), 0.085);
        let frame = FrameUniform::new(&camera, 1.6, 1., Atmosphere::default());
        let inverse = glam::Mat4::from_cols_array(&frame.inverse_projection);
        let p = inverse * glam::Vec4::new(0., 0., 0., 1.);
        let ray = p.truncate().normalize();
        assert!(ray.dot(-camera.direction()) > 0.9999);
        assert!(
            bytemuck::cast_slice::<_, f32>(&[frame])
                .iter()
                .all(|v| v.is_finite())
        );
    }

    #[test]
    fn cloud_survey_bypass_and_motion_are_bounded() {
        let camera = crate::camera::Camera::default();
        let atmosphere = Atmosphere {
            sea_level: 2.,
            survey: true,
            ..Default::default()
        };
        let frame = FrameUniform::new(&camera, 1., 4001., atmosphere);
        assert_eq!(frame.clouds[0], 1840.);
        assert_eq!(frame.clouds[1], 0.);
        assert_eq!(frame.cloud_motion, [8., 4., 0., 0.]);
        assert_eq!(
            frame.cloud_motion,
            FrameUniform::new(&camera, 1., 1., atmosphere).cloud_motion
        );
    }
}
