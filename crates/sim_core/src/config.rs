use serde::{Deserialize, Serialize};

/// Configuration for a simulation instance. Immutable after creation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimConfig {
    /// Grid width in cells.
    pub width: u32,
    /// Grid height in cells.
    pub height: u32,
    /// Initial sea level.
    pub sea_level: f32,
    /// Deterministic RNG seed.
    pub seed: u64,
    /// Simulation tick duration in abstract time units.
    pub tick_dt: f32,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            width: 128,
            height: 80,
            sea_level: 0.0,
            seed: 42,
            tick_dt: 1.0,
        }
    }
}
