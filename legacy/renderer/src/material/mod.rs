pub mod stylized;

use bevy::prelude::*;

use stylized::StylizedMaterial;

pub struct MaterialSetupPlugin;

impl Plugin for MaterialSetupPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<StylizedMaterial>::default());
    }
}
