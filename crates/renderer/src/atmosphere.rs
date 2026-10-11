//! Regional atmospheric presentation. A scattering lookup and cloud volume share
//! these parameters; simulation weather and planetary composition remain separate.
use glam::{Vec2, Vec3};

/// Presentation parameters, independent of any future climate model.
#[derive(Clone, Copy)]
pub struct CloudLayer {
    pub height_metres: f32,
    pub cover: f32,
    pub shadow_strength: f32,
    pub thickness_metres: f32,
    pub extinction_per_metre: f32,
    pub wind_metres_per_second: Vec2,
}

impl Default for CloudLayer {
    fn default() -> Self {
        Self {
            height_metres: 1500.,
            cover: 0.46,
            shadow_strength: 0.40,
            thickness_metres: 1100.,
            extinction_per_metre: 0.008,
            wind_metres_per_second: Vec2::new(8., 4.),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Atmosphere {
    /// Sea-level vertical molecular optical depth (RGB), a planet parameter.
    pub rayleigh: Vec3,
    /// Relative aerosol loading for the single-scattering sky lookup.
    pub aerosol: f32,
    pub planet_radius_metres: f32,
    pub atmosphere_height_metres: f32,
    pub molecular_scale_height_metres: f32,
    pub aerosol_scale_height_metres: f32,
    pub density: f32,
    pub scale_height_metres: f32,
    pub sea_level: f32,
    pub survey: bool,
    pub clouds: CloudLayer,
}

impl Default for Atmosphere {
    fn default() -> Self {
        Self {
            rayleigh: Vec3::new(0.0464, 0.108, 0.2648),
            aerosol: 1.,
            planet_radius_metres: 6_371_000.,
            atmosphere_height_metres: 80_000.,
            molecular_scale_height_metres: 8000.,
            aerosol_scale_height_metres: 1200.,
            density: 0.00009,
            scale_height_metres: 900.,
            sea_level: 0.,
            survey: false,
            clouds: CloudLayer::default(),
        }
    }
}

impl Atmosphere {
    /// Visual inspection profiles, not a forecast or a persisted weather model.
    pub fn preview(name: &str) -> Option<Self> {
        let mut atmosphere = Self::default();
        match name {
            "fair" => {}
            "clear" => {
                atmosphere.clouds.cover = 0.;
                atmosphere.aerosol = 0.55;
            }
            "overcast" => {
                atmosphere.clouds.cover = 0.92;
                atmosphere.clouds.height_metres = 900.;
                atmosphere.clouds.thickness_metres = 1800.;
                atmosphere.clouds.extinction_per_metre = 0.012;
                atmosphere.clouds.shadow_strength = 0.72;
                atmosphere.clouds.wind_metres_per_second = Vec2::new(12., 6.);
                atmosphere.density = 0.00016;
                atmosphere.aerosol = 2.;
            }
            _ => return None,
        }
        Some(atmosphere)
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
    pub rayleigh: [f32; 4],
    pub mie: [f32; 4],
    pub air_layers: [f32; 4],
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
            direction_time: camera.direction().extend(finite(time, 0.)).to_array(),
            light_projection: crate::shadows::Shadows::matrix(camera.focus).to_cols_array(),
            far_light_projection: crate::shadows::Shadows::cascade_matrix(camera.focus, 1)
                .to_cols_array(),
            eye_sea_level: camera
                .eye()
                .extend(finite(atmosphere.sea_level, 0.))
                .to_array(),
            inverse_projection: matrix.inverse().to_cols_array(),
            rayleigh: Vec3::from_array(
                atmosphere
                    .rayleigh
                    .to_array()
                    .map(|v| finite(v, 0.1).clamp(0.001, 4.)),
            )
            .extend(0.)
            .to_array(),
            mie: [0.015 * finite(atmosphere.aerosol, 1.).clamp(0., 8.); 4],
            air_layers: [
                finite(atmosphere.planet_radius_metres, 6_371_000.).clamp(500_000., 100_000_000.)
                    * 0.001,
                finite(atmosphere.atmosphere_height_metres, 80_000.).clamp(10_000., 500_000.)
                    * 0.001,
                finite(atmosphere.molecular_scale_height_metres, 8000.).clamp(100., 50_000.)
                    * 0.001,
                finite(atmosphere.aerosol_scale_height_metres, 1200.).clamp(100., 20_000.) * 0.001,
            ],
            haze: [
                finite(atmosphere.density, 0.00009).clamp(0., 1.),
                finite(atmosphere.scale_height_metres, 900.).clamp(1., 100_000.),
                if atmosphere.survey { 1. } else { 0. },
                0.,
            ],
            clouds: [
                finite(atmosphere.sea_level, 0.) * 20.
                    + finite(atmosphere.clouds.height_metres, 1500.).clamp(100., 16000.),
                if atmosphere.survey {
                    0.
                } else {
                    finite(atmosphere.clouds.cover, 0.46).clamp(0., 1.)
                },
                finite(atmosphere.clouds.shadow_strength, 0.4).clamp(0., 1.),
                crate::clouds::TILE_METRES,
            ],
            // Wrap displacement, not time: arbitrary wind stays continuous across
            // the periodic field. f64 multiplication avoids losing the fraction.
            cloud_motion: [
                drift(time, atmosphere.clouds.wind_metres_per_second.x),
                drift(time, atmosphere.clouds.wind_metres_per_second.y),
                finite(atmosphere.clouds.thickness_metres, 1100.).clamp(100., 6000.),
                finite(atmosphere.clouds.extinction_per_metre, 0.008).clamp(0., 0.05),
            ],
        }
    }
}

fn finite(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}
fn drift(time: f32, velocity: f32) -> f32 {
    (f64::from(finite(time, 0.)) * f64::from(finite(velocity, 0.).clamp(-150., 150.)))
        .rem_euclid(f64::from(crate::clouds::TILE_METRES)) as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn frame_layout_and_sky_reconstruction_match_camera() {
        assert_eq!(size_of::<FrameUniform>(), 384);
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
        assert_eq!(frame.clouds[0], 1540.);
        assert_eq!(frame.clouds[1], 0.);
        assert_eq!(frame.cloud_motion, [8., 4., 1100., 0.008]);
        assert_eq!(
            frame.cloud_motion,
            FrameUniform::new(&camera, 1., 1., atmosphere).cloud_motion
        );
    }

    #[test]
    fn weather_inputs_stay_finite_and_arbitrary_wind_wraps_continuously() {
        let camera = crate::camera::Camera::default();
        for profile in ["clear", "fair", "overcast"] {
            let atmosphere = Atmosphere::preview(profile).expect("known preview");
            let frame = FrameUniform::new(&camera, 1.6, 31., atmosphere);
            assert!(
                bytemuck::cast_slice::<_, f32>(&[frame])
                    .iter()
                    .all(|v| v.is_finite())
            );
        }
        assert!(Atmosphere::preview("rain").is_none());
        let mut atmosphere = Atmosphere {
            rayleigh: Vec3::splat(f32::NAN),
            aerosol: f32::INFINITY,
            planet_radius_metres: f32::NAN,
            ..Default::default()
        };
        atmosphere.clouds.cover = f32::NAN;
        atmosphere.clouds.thickness_metres = -1.;
        atmosphere.clouds.extinction_per_metre = f32::INFINITY;
        atmosphere.clouds.wind_metres_per_second = Vec2::new(f32::NAN, f32::INFINITY);
        let frame = FrameUniform::new(&camera, 1.6, 0., atmosphere);
        assert!(
            bytemuck::cast_slice::<_, f32>(&[frame])
                .iter()
                .all(|v| v.is_finite())
        );
        assert_eq!(frame.cloud_motion[2], 100.);
        for velocity in [-7.25_f32, 3.125, 12.75] {
            let period = crate::clouds::TILE_METRES / velocity.abs();
            let before = drift(period - 0.01, velocity);
            let after = drift(period + 0.01, velocity);
            let delta = (after - before + 8000.).rem_euclid(16000.) - 8000.;
            assert!((delta - velocity * 0.02).abs() < 0.01);
        }
    }
}
