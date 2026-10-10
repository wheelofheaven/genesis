use bevy::core_pipeline::core_3d::graph::Node3d;
use bevy::core_pipeline::fullscreen_material::FullscreenMaterial;
use bevy::prelude::*;
use bevy::render::extract_component::ExtractComponent;
use bevy::render::render_graph::{InternedRenderLabel, RenderLabel};
use bevy::render::render_resource::ShaderType;
use bevy::shader::ShaderRef;

/// Tilt-shift depth of field post-process for a miniature/diorama feel.
///
/// Applies a screen-space blur that increases with vertical distance from the
/// horizontal focus band, creating the illusion of shallow depth of field
/// commonly seen in miniature photography.
#[derive(Component, ExtractComponent, Clone, Copy, ShaderType)]
pub struct TiltShiftDoF {
    /// Normalized Y of the sharp focus band center (0.0 = top, 1.0 = bottom).
    pub focus_center: f32,
    /// Half-width of the sharp band in normalized screen coordinates.
    pub focus_width: f32,
    /// Maximum blur radius in pixels at the screen edges.
    pub max_blur_radius: f32,
    pub _padding: f32,
}

impl Default for TiltShiftDoF {
    fn default() -> Self {
        Self {
            focus_center: 0.5,
            focus_width: 0.2,
            max_blur_radius: 4.0,
            _padding: 0.0,
        }
    }
}

impl FullscreenMaterial for TiltShiftDoF {
    fn fragment_shader() -> ShaderRef {
        "shaders/postprocess/tilt_shift.wgsl".into()
    }

    fn node_edges() -> Vec<InternedRenderLabel> {
        vec![
            Node3d::Tonemapping.intern(),
            Self::node_label().intern(),
            Node3d::EndMainPassPostProcessing.intern(),
        ]
    }
}
