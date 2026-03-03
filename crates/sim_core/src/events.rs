use serde::{Deserialize, Serialize};

/// Events emitted by the simulation during a step.
/// Used for expedition log, UI feedback, and debugging.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SimEvent {
    /// Land emerged above sea level.
    LandEmerged { x: u32, y: u32 },
    /// A biome changed at a cell.
    BiomeChanged {
        x: u32,
        y: u32,
        from: String,
        to: String,
    },
    /// Fungus started growing at a cell.
    FungusEstablished { x: u32, y: u32 },
    /// Plants started growing at a cell.
    PlantsEstablished { x: u32, y: u32 },
    /// Outpost was built.
    OutpostBuilt { x: u32, y: u32 },
    /// Stability score changed significantly.
    StabilityUpdate { score: f32 },
    /// Win condition reached.
    BiosphereStabilized,
}
