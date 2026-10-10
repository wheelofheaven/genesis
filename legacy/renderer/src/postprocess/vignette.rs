use bevy::core_pipeline::core_3d::graph::Node3d;
use bevy::core_pipeline::fullscreen_material::FullscreenMaterial;
use bevy::prelude::*;
use bevy::render::extract_component::ExtractComponent;
use bevy::render::render_graph::{InternedRenderLabel, RenderLabel};
use bevy::render::render_resource::ShaderType;
use bevy::shader::ShaderRef;

use super::tilt_shift::TiltShiftDoF;

/// Vignette darkening post-process effect.
#[derive(Component, ExtractComponent, Clone, Copy, ShaderType, Default)]
pub struct VignettePostProcess {
    pub intensity: f32,
    pub radius: f32,
    pub softness: f32,
    pub _padding: f32,
}

impl VignettePostProcess {
    pub fn new(intensity: f32, radius: f32, softness: f32) -> Self {
        Self {
            intensity,
            radius,
            softness,
            _padding: 0.0,
        }
    }
}

impl FullscreenMaterial for VignettePostProcess {
    fn fragment_shader() -> ShaderRef {
        "shaders/postprocess/vignette.wgsl".into()
    }

    fn node_edges() -> Vec<InternedRenderLabel> {
        vec![
            TiltShiftDoF::node_label().intern(),
            Self::node_label().intern(),
            Node3d::EndMainPassPostProcessing.intern(),
        ]
    }
}
