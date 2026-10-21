//! Versioned domain snapshot. Encoding and file I/O belong to host storage.
mod migration;
mod validation;

use crate::{SimState, actions::Action, runtime::Simulation};
use serde::{Deserialize, Serialize};

/// Initial native save contract; incompatible future rules must bump metadata.
pub const SCHEMA: u32 = 10;
pub const RULES: &str = "genesis:rules/8";
pub const CONTENT: &str = "genesis:builtin/1";

/// Materialized world state with persistent per-domain identity allocators.
/// V1 append-only identities migrate on restore; captures always use V8.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub schema: u32,
    pub rules: String,
    pub content: String,
    pub world_id: String,
    pub completed_ticks: u64,
    /// None for historical saves or CPU scenarios without a player role.
    #[serde(default)]
    pub expedition_mode: Option<crate::catalog::ExpeditionMode>,
    /// Host-queued intent for the next tick, not already committed facts.
    pub pending: Vec<Action>,
    pub state: SimState,
}

#[derive(Debug)]
pub struct SaveError(pub(crate) String);

impl std::fmt::Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for SaveError {}

/// Fully staged and validated load. Clients replace their world only on success.
pub struct Restored {
    pub world_id: String,
    pub expedition_mode: Option<crate::catalog::ExpeditionMode>,
    pub simulation: Simulation,
    pub pending: Vec<Action>,
}

impl Snapshot {
    /// Capture at a completed boundary without consuming pending intent or
    /// advancing time. The host supplies the world's persistent identity.
    pub fn capture(
        sim: &Simulation,
        world_id: &str,
        pending: &[Action],
    ) -> Result<Self, SaveError> {
        let snapshot = Self {
            schema: SCHEMA,
            rules: RULES.into(),
            content: CONTENT.into(),
            world_id: world_id.into(),
            completed_ticks: sim.completed_ticks(),
            expedition_mode: None,
            pending: pending.to_vec(),
            state: sim.state.clone(),
        };
        snapshot.validate()?;
        Ok(snapshot)
    }

    /// Validate a record without mutating an active simulation.
    pub fn validate(&self) -> Result<(), SaveError> {
        validation::validate(self)
    }

    /// Rebuild disposable query caches only after bounded domain validation.
    pub fn restore(mut self) -> Result<Restored, SaveError> {
        migration::upgrade(&mut self)?;
        self.validate()?;
        for river in &mut self.state.geography.rivers {
            river.bounds = Some(river.points.iter().fold(
                [
                    f32::INFINITY,
                    f32::INFINITY,
                    f32::NEG_INFINITY,
                    f32::NEG_INFINITY,
                ],
                |b, p| [b[0].min(p.x), b[1].min(p.z), b[2].max(p.x), b[3].max(p.z)],
            ));
        }
        self.state.geography.channels = crate::surface::channels::Index::new(
            self.state.grid.width,
            self.state.grid.height,
            &self.state.geography.rivers,
        );
        Ok(Restored {
            world_id: self.world_id,
            expedition_mode: self.expedition_mode,
            pending: self.pending,
            simulation: Simulation {
                state: self.state,
                completed_ticks: self.completed_ticks,
            },
        })
    }
}
