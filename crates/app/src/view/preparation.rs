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

pub(super) struct Pending {
    pub tag: Tag,
    pub receiver: mpsc::Receiver<Ready>,
}

impl Pending {
    pub fn spawn(sim: SimState, tag: Tag, cache: Option<Arc<terrain::Cache>>) -> Self {
        let (sender, receiver) = mpsc::channel();
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

    pub fn matches(&self, current: Tag, urgent: bool) -> bool {
        !urgent && self.tag == current
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_work_cannot_replace_edits_surveys_or_camera_detail() {
        let tag = Tag {
            revision: 1,
            key: terrain::lod::Key([0; 3]),
            survey: Survey::default(),
            epoch: 7,
        };
        let (_sender, receiver) = mpsc::channel();
        let pending = Pending { tag, receiver };
        assert!(pending.matches(tag, false));
        assert!(!pending.matches(tag, true));
        assert!(!pending.matches(Tag { revision: 2, ..tag }, false));
        assert!(!pending.matches(
            Tag {
                key: terrain::lod::Key([1; 3]),
                ..tag
            },
            false
        ));
        assert!(!pending.matches(Tag { epoch: 8, ..tag }, false));
        assert!(!pending.matches(
            Tag {
                survey: Survey {
                    water: true,
                    ..tag.survey
                },
                ..tag
            },
            false
        ));
    }

    #[test]
    fn worker_keeps_colour_refresh_on_the_same_cache_and_builds_matching_picker_for_movement() {
        let mut sim = SimState::new(genesis_sim_core::config::SimConfig {
            width: 8,
            height: 8,
            ..Default::default()
        });
        sim.geography.version = 9;
        for cell in sim.grid.cells_mut() {
            cell.elevation = 2.;
        }
        let tag = Tag {
            revision: 0,
            key: terrain::lod::Key([0, 1, 0]),
            survey: Survey::default(),
            epoch: 0,
        };
        let fresh = Pending::spawn(sim.clone(), tag, None)
            .receiver
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("terrain worker");
        assert_eq!(fresh.cache.key, tag.key);
        let picker = fresh.picker.expect("new terrain needs new picking data");
        let hit = picker
            .pick(glam::Vec3::new(0.1, 5., 0.1), -glam::Vec3::Y, false)
            .expect("new terrain is pickable");
        assert!((hit.x - 4.1).abs() < 1e-5 && (hit.z - 4.1).abs() < 1e-5);
        sim.step(0.1, &[]);
        let refresh = Pending::spawn(sim, tag, Some(Arc::clone(&fresh.cache)))
            .receiver
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("colour worker");
        assert!(Arc::ptr_eq(&fresh.cache, &refresh.cache));
        assert!(
            refresh.picker.is_none(),
            "colours must not rebuild picking data"
        );
    }
}
