use serde::{Deserialize, Serialize};

/// Events emitted by the simulation during a step.
/// Used for expedition log, UI feedback, and debugging.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SimEvent {
    WaterTaskStatusChanged {
        person: crate::people::PersonId,
        issue: Option<crate::people::WaterIssue>,
        time: f64,
    },
    WaterCollected {
        person: crate::people::PersonId,
        well: u64,
        amount: f64,
        time: f64,
    },
    WaterDelivered {
        person: crate::people::PersonId,
        home: u64,
        amount: f64,
        time: f64,
    },
    WaterTaskCancelled {
        person: crate::people::PersonId,
        time: f64,
    },
    WaterTaskCancelRejected {
        person: crate::people::PersonId,
        time: f64,
    },
    PersonRoutineChanged {
        id: crate::people::PersonId,
        activity: crate::people::Activity,
        blocked: Option<crate::people::Blocked>,
        time: f64,
    },
    TerrainChanged {
        x: i32,
        y: i32,
        changed_cells: usize,
        bounds: crate::grid::SampleBounds,
    },
    TerrainUnchanged {
        x: i32,
        y: i32,
    },
    TerraformRejected {
        x: i32,
        y: i32,
        reason: crate::rules::TerraformError,
    },
    PathBuilt {
        id: u64,
    },
    PathRejected {
        reason: crate::paths::PathError,
    },
    WellDrilled {
        id: u64,
    },
    WellRejected {
        reason: crate::subsurface::WellError,
    },
    WellOperationChanged {
        id: u64,
        active: bool,
    },
    BuildingPlaced {
        id: u64,
    },
    BuildingRejected {
        reason: crate::settlement::PlacementError,
    },
    /// Land emerged above sea level.
    LandEmerged {
        x: u32,
        y: u32,
    },
    /// A biome changed at a cell.
    BiomeChanged {
        x: u32,
        y: u32,
        from: String,
        to: String,
    },
    /// Fungus started growing at a cell.
    FungusEstablished {
        x: u32,
        y: u32,
    },
    /// Plants started growing at a cell.
    PlantsEstablished {
        x: u32,
        y: u32,
    },
    /// Outpost was built.
    OutpostBuilt {
        x: u32,
        y: u32,
    },
    /// Stability score changed significantly.
    StabilityUpdate {
        score: f32,
    },
    /// Win condition reached.
    BiosphereStabilized,
}
