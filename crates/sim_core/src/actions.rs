use serde::{Deserialize, Serialize};

/// Player actions fed into the simulation each tick.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    /// Modify elevation at a position with a given radius and strength.
    Terraform {
        x: i32,
        y: i32,
        radius: u32,
        strength: f32,
    },
    /// Seed fungus at a position with a given radius.
    SeedFungus { x: i32, y: i32, radius: u32 },
    /// Seed plants at a position with a given radius.
    SeedPlants { x: i32, y: i32, radius: u32 },
    /// Place an outpost at a position.
    BuildOutpost { x: i32, y: i32 },
}
