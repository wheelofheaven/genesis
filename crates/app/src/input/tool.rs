use bevy::prelude::*;
use genesis_sim_core::actions::Action;

use crate::plugins::simulation::Simulation;
use crate::render::grid_render::GridDisplay;
use crate::ui::overlay::ActiveTool;

/// Queued actions from player input, consumed each sim tick.
#[derive(Resource, Default)]
pub struct PendingActions(pub Vec<Action>);

/// Converts mouse clicks into simulation actions based on the active tool.
pub fn handle_mouse_input(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    sim: Res<Simulation>,
    tool: Res<ActiveTool>,
    display: Res<GridDisplay>,
    mut pending: ResMut<PendingActions>,
) {
    if !mouse.pressed(MouseButton::Left) {
        return;
    }

    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };
    let Ok((camera, cam_transform)) = camera_q.single() else {
        return;
    };
    let Ok(world_pos) = camera.viewport_to_world_2d(cam_transform, cursor_pos) else {
        return;
    };

    // Convert world position to grid coordinates.
    let grid_x = ((world_pos.x / display.cell_size) + sim.0.grid.width as f32 / 2.0) as i32;
    let grid_y = ((-world_pos.y / display.cell_size) + sim.0.grid.height as f32 / 2.0) as i32;

    let action = match tool.0 {
        ToolKind::TerraformRaise => Action::Terraform {
            x: grid_x,
            y: grid_y,
            radius: 3,
            strength: 0.5,
        },
        ToolKind::TerraformLower => Action::Terraform {
            x: grid_x,
            y: grid_y,
            radius: 3,
            strength: -0.5,
        },
        ToolKind::SeedFungus => Action::SeedFungus {
            x: grid_x,
            y: grid_y,
            radius: 2,
        },
        ToolKind::SeedPlants => Action::SeedPlants {
            x: grid_x,
            y: grid_y,
            radius: 2,
        },
        ToolKind::BuildOutpost => {
            if mouse.just_pressed(MouseButton::Left) {
                Action::BuildOutpost {
                    x: grid_x,
                    y: grid_y,
                }
            } else {
                return;
            }
        }
    };

    pending.0.push(action);
}

/// Available player tools.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToolKind {
    #[default]
    TerraformRaise,
    TerraformLower,
    SeedFungus,
    SeedPlants,
    BuildOutpost,
}
