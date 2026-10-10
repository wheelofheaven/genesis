// Module temporarily unused: egui/UI disabled due to bevy_egui Metal crash on macOS 26.
#![allow(dead_code)]

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
    display: Option<Res<GridDisplay>>,
    mut pending: ResMut<PendingActions>,
) {
    // GridDisplay is only available when GridRenderPlugin is active.
    let Some(display) = display else {
        return;
    };

    // Don't process left-click when right mouse is held (orbit camera).
    if !mouse.pressed(MouseButton::Left) || mouse.pressed(MouseButton::Right) {
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

    // Cast a ray from the camera through the cursor position.
    let Ok(ray) = camera.viewport_to_world(cam_transform, cursor_pos) else {
        return;
    };

    // Intersect with the Y=0 ground plane.
    if ray.direction.y.abs() < 1e-6 {
        return; // Ray parallel to ground plane.
    }
    let t = -ray.origin.y / ray.direction.y;
    if t < 0.0 {
        return; // Intersection behind the camera.
    }
    let hit = ray.origin + ray.direction * t;

    // Convert world XZ position to grid coordinates.
    // Grid center is at world origin. Grid Y maps to world -Z.
    let grid_x = ((hit.x / display.cell_size) + sim.0.grid.width as f32 / 2.0) as i32;
    let grid_y = ((-hit.z / display.cell_size) + sim.0.grid.height as f32 / 2.0) as i32;

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
