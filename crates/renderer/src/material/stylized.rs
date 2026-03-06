use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;

/// Stylized material with wrap diffuse, hemispheric ambient, Blinn-Phong specular,
/// and optional procedural noise detail.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct StylizedMaterial {
    #[uniform(0)]
    pub base_color: LinearRgba,
    #[uniform(0)]
    pub sky_color: LinearRgba,
    #[uniform(0)]
    pub ground_color: LinearRgba,
    #[uniform(0)]
    pub wrap_factor: f32,
    #[uniform(0)]
    pub specular_strength: f32,
    #[uniform(0)]
    pub specular_shininess: f32,
    #[uniform(0)]
    pub contact_shadow_strength: f32,
    /// World-space noise frequency for procedural detail.
    #[uniform(0)]
    pub detail_scale: f32,
    /// Color variation amount from noise (0.0 = off).
    #[uniform(0)]
    pub detail_strength: f32,
    /// Specular roughness variation from noise (0.0 = off).
    #[uniform(0)]
    pub detail_roughness: f32,
    /// Padding for 16-byte GPU alignment.
    #[uniform(0)]
    pub _padding: f32,
}

impl Default for StylizedMaterial {
    fn default() -> Self {
        Self {
            base_color: LinearRgba::WHITE,
            // Cool sky ambient.
            sky_color: LinearRgba::new(0.4, 0.5, 0.7, 1.0),
            // Warm ground bounce.
            ground_color: LinearRgba::new(0.3, 0.2, 0.1, 1.0),
            wrap_factor: 0.5,
            specular_strength: 0.3,
            specular_shininess: 32.0,
            contact_shadow_strength: 0.5,
            detail_scale: 0.1,
            detail_strength: 0.0,
            detail_roughness: 0.0,
            _padding: 0.0,
        }
    }
}

impl StylizedMaterial {
    /// Terrain surface — subtle noise-driven detail, vertex-color ready.
    pub fn terrain(base_color: LinearRgba) -> Self {
        Self {
            base_color,
            detail_scale: 0.08,
            detail_strength: 0.15,
            detail_roughness: 0.1,
            wrap_factor: 0.6,
            specular_strength: 0.15,
            specular_shininess: 16.0,
            ..default()
        }
    }

    /// Water surface — higher specular, tighter wrap.
    pub fn water(base_color: LinearRgba) -> Self {
        Self {
            base_color,
            detail_scale: 0.05,
            detail_strength: 0.08,
            detail_roughness: 0.05,
            wrap_factor: 0.3,
            specular_strength: 0.6,
            specular_shininess: 64.0,
            ..default()
        }
    }

    /// Stone surface — prominent noise detail, rough specular.
    pub fn stone(base_color: LinearRgba) -> Self {
        Self {
            base_color,
            detail_scale: 0.12,
            detail_strength: 0.25,
            detail_roughness: 0.2,
            wrap_factor: 0.5,
            specular_strength: 0.2,
            specular_shininess: 24.0,
            ..default()
        }
    }
}

impl Material for StylizedMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/stylized.wgsl".into()
    }
}
