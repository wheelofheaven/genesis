pub mod tilt_shift;
pub mod vignette;

use bevy::core_pipeline::fullscreen_material::FullscreenMaterialPlugin;
use bevy::prelude::*;

use tilt_shift::TiltShiftDoF;
use vignette::VignettePostProcess;

pub struct PostProcessPlugin;

impl Plugin for PostProcessPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            FullscreenMaterialPlugin::<TiltShiftDoF>::default(),
            FullscreenMaterialPlugin::<VignettePostProcess>::default(),
        ));
    }
}
