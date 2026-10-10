use serde::{Deserialize, Serialize};

/// Player actions fed into the simulation each tick.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    DrillWell {
        position: crate::spatial::Position,
    },
    SetWellActive {
        id: u64,
        active: bool,
    },
    /// Modify elevation at a position with a given radius and strength.
    Terraform {
        x: i32,
        y: i32,
        radius: u32,
        strength: f32,
    },
    /// Seed fungus at a position with a given radius.
    SeedFungus {
        x: i32,
        y: i32,
        radius: u32,
    },
    /// Seed plants at a position with a given radius.
    SeedPlants {
        x: i32,
        y: i32,
        radius: u32,
    },
    /// Place a freely positioned building; simulation validates its footprint.
    PlaceBuilding {
        position: crate::spatial::Position,
        rotation: f32,
    },
    /// Place a legacy grid outpost at a position.
    BuildOutpost {
        x: i32,
        y: i32,
    },
}
