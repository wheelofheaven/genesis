//! Disposable CPU scene work. One request is in flight; the view validates its
//! revision, camera bucket, survey and edit epoch before publishing any result.
use crate::{scene, survey::Survey, terrain};
use genesis_renderer::world_mesh::PreparedWorld;
use genesis_sim_core::SimState;
use std::sync::{Arc, mpsc};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Tag {
    pub revision: u64,
    pub key: terrain::lod::Key,
    pub survey: Survey,
    pub epoch: u64,
}

pub(super) struct Ready {
    pub cache: Arc<terrain::Cache>,
    pub picker: Option<terrain::Picker>,
    pub world: PreparedWorld,
}

struct Pending {
    tag: Tag,
    receiver: mpsc::Receiver<Ready>,
}

impl Pending {
    fn spawn(sim: SimState, tag: Tag, cache: Option<Arc<terrain::Cache>>) -> Self {
        let (sender, receiver) = mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let picker_needed = cache.is_none();
            let cache = cache.unwrap_or_else(|| Arc::new(terrain::Cache::at(&sim, tag.key)));
            let picker = picker_needed.then(|| terrain::Picker::from_cache(&sim, &cache));
            let vertices = scene::mesh(&sim, tag.survey, &cache, &cache.rivers);
            let world = PreparedWorld::new(&vertices);
            let _ = sender.send(Ready {
                cache,
                picker,
                world,
            });
        });
        Self { tag, receiver }
    }
}

/// Owns one outstanding request, including a completed but unconsumed result.
/// Stale work keeps the slot until completion; replacement never detaches it
/// and starts an unbounded chain of workers. No pending input queue is retained.
#[derive(Default)]
pub(super) struct Preparation {
    pending: Option<Pending>,
}

pub(super) enum Completion {
    Waiting,
    Ready(Ready),
    Discarded,
}

impl Preparation {
    pub fn is_idle(&self) -> bool {
        self.pending.is_none()
    }

    /// Refuse admission before cloning the input when the worker slot is busy.
    pub fn request(
        &mut self,
        sim: &SimState,
        tag: Tag,
        cache: Option<Arc<terrain::Cache>>,
    ) -> bool {
        if !self.is_idle() {
            return false;
        }
        self.pending = Some(Pending::spawn(sim.clone(), tag, cache));
        true
    }

    /// Only validated results escape to the GPU/picking publisher. A failed or
    /// stale completion releases the slot and asks the view to refresh.
    pub fn poll(&mut self, current: Tag, urgent: bool) -> Completion {
        let Some(pending) = &self.pending else {
            return Completion::Waiting;
        };
        let completion = match pending.receiver.try_recv() {
            Ok(ready) if !urgent && pending.tag == current => Completion::Ready(ready),
            Ok(_) | Err(mpsc::TryRecvError::Disconnected) => Completion::Discarded,
            Err(mpsc::TryRecvError::Empty) => return Completion::Waiting,
        };
        self.pending = None;
        completion
    }
}

#[cfg(test)]
mod tests;
