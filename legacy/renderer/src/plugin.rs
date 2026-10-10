use bevy::prelude::*;

use crate::camera::CameraPlugin;
use crate::material::MaterialSetupPlugin;
use crate::postprocess::PostProcessPlugin;

/// Composite plugin that adds the custom renderer systems.
pub struct RendererPlugin;

impl Plugin for RendererPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((CameraPlugin, MaterialSetupPlugin, PostProcessPlugin));
    }
}
