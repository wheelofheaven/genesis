//! Monotonic per-domain identity allocation, independent of storage positions.
use serde::{Deserialize, Serialize};

/// Zero is reserved for previews and missing legacy save metadata. MAX marks
/// exhaustion; neither value is allocated to a committed object.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct Allocator(u64);

impl Allocator {
    pub(crate) fn new() -> Self {
        Self(1)
    }

    pub(crate) fn allocate(&mut self) -> Option<u64> {
        let next = self.0.checked_add(1)?;
        if self.0 == 0 {
            return None;
        }
        let id = self.0;
        self.0 = next;
        Some(id)
    }

    pub(crate) fn covers(&self, ids: impl Iterator<Item = u64>) -> bool {
        self.0 > 0 && ids.into_iter().all(|id| id > 0 && id < self.0)
    }

    /// V1 used append-only sequential identities. Only its validated migration
    /// may reconstruct the allocation counter from the object count.
    pub(crate) fn legacy(count: usize) -> Self {
        Self(count as u64 + 1)
    }
}

#[cfg(test)]
mod tests;
