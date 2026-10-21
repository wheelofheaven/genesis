use serde::{Deserialize, Serialize};

/// Player actions fed into the simulation each tick.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    SetWellPumpMode {
        id: u64,
        mode: crate::subsurface::PumpMode,
    },
    SetCropTending {
        source: crate::food::SourceId,
        enabled: bool,
    },
    /// Apply a fixed dose of unreserved well water to a nearby crop.
    IrrigateCrop {
        source: crate::food::SourceId,
        well: u64,
    },
    PlantCrop {
        position: crate::spatial::Position,
    },
    CancelFoodTask {
        person: crate::people::PersonId,
    },
    CancelWaterTask {
        person: crate::people::PersonId,
    },
    /// Construct a curved footpath through continuous waypoints.
    BuildPath {
        waypoints: Vec<crate::spatial::Position>,
        width: f32,
    },
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
