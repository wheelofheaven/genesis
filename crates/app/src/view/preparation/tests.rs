use super::*;
use crate::session::Session;
use genesis_sim_core::{config::SimConfig, spatial::Position};
use std::time::{Duration, Instant};

fn world(height: f32) -> SimState {
    let mut sim = SimState::new(SimConfig {
        width: 8,
        height: 8,
        ..Default::default()
    });
    for cell in sim.grid.cells_mut() {
        cell.elevation = height;
    }
    sim.step(0., &[]);
    sim
}
fn tag(session: &Session) -> Tag {
    Tag {
        revision: session.terrain_revision,
        key: terrain::lod::Key([0, 1, 0]),
        survey: session.survey,
        epoch: 7,
    }
}
fn wait(preparation: &mut Preparation, tag: Tag) -> Ready {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match preparation.poll(tag, false) {
            Completion::Ready(ready) => return ready,
            Completion::Discarded => panic!("current result unexpectedly discarded"),
            Completion::Waiting => {
                assert!(Instant::now() < deadline, "terrain worker deadline");
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    }
}
fn delayed(tag: Tag) -> (Preparation, mpsc::SyncSender<Ready>) {
    let (sender, receiver) = mpsc::sync_channel(1);
    (
        Preparation {
            pending: Some(Pending { tag, receiver }),
        },
        sender,
    )
}
struct File(std::path::PathBuf);
impl File {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!(
            "genesis-preparation-{}.json",
            genesis_storage::new_world_id()
        )))
    }
}
impl Drop for File {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[test]
fn stale_work_cannot_replace_edits_surveys_or_camera_detail() {
    let session = Session::from_sim(world(2.));
    let old = tag(&session);
    let cache = Arc::new(terrain::Cache::at(&session.sim, old.key));
    for (current, urgent) in [
        (old, true),
        (
            Tag {
                revision: old.revision + 1,
                ..old
            },
            false,
        ),
        (
            Tag {
                key: terrain::lod::Key([1; 3]),
                ..old
            },
            false,
        ),
        (
            Tag {
                epoch: old.epoch + 1,
                ..old
            },
            false,
        ),
        (
            Tag {
                survey: Survey {
                    water: true,
                    ..old.survey
                },
                ..old
            },
            false,
        ),
    ] {
        let (mut preparation, sender) = delayed(old);
        sender
            .send(Ready {
                cache: Arc::clone(&cache),
                picker: None,
                world: PreparedWorld::new(&[]),
            })
            .expect("held result");
        assert!(matches!(
            preparation.poll(current, urgent),
            Completion::Discarded
        ));
        assert!(preparation.is_idle());
        assert!(matches!(
            preparation.poll(current, urgent),
            Completion::Waiting
        ));
    }
}

#[test]
fn worker_keeps_colour_refresh_on_the_same_cache_and_builds_matching_picker_for_movement() {
    let mut sim = world(2.);
    sim.geography.version = 9;
    let tag = tag(&Session::from_sim(sim.clone()));
    let mut preparation = Preparation::default();
    assert!(preparation.request(&sim, tag, None));
    let fresh = wait(&mut preparation, tag);
    assert_eq!(fresh.cache.key, tag.key);
    let picker = fresh.picker.expect("new terrain needs new picking data");
    let hit = picker
        .pick(glam::Vec3::new(0.1, 5., 0.1), -glam::Vec3::Y, false)
        .expect("new terrain is pickable");
    assert!((hit.x - 4.1).abs() < 1e-5 && (hit.z - 4.1).abs() < 1e-5);
    sim.step(0.1, &[]);
    assert!(preparation.request(&sim, tag, Some(Arc::clone(&fresh.cache))));
    let refresh = wait(&mut preparation, tag);
    assert!(Arc::ptr_eq(&fresh.cache, &refresh.cache));
    assert!(
        refresh.picker.is_none(),
        "colours must not rebuild picking data"
    );
}

#[test]
fn delayed_old_world_result_is_rejected_after_repeated_loads_and_latest_world_converges() {
    let mut session = Session::from_sim(world(2.));
    let old_tag = tag(&session);
    let mut worker = Preparation::default();
    assert!(worker.request(&session.sim, old_tag, None));
    let old_result = wait(&mut worker, old_tag);
    assert!(
        (old_result
            .cache
            .height_at(&session.sim, Position::new(4., 4.))
            - 2.)
            .abs()
            < 1e-5
    );
    // Deterministically withhold delivery. No sleep or worker race chooses when
    // the old result arrives relative to the following real save/load boundary.
    let (mut preparation, sender) = delayed(old_tag);
    let file = File::new();
    let next = Session::from_sim(world(5.));
    next.save_to(&file.0).unwrap();
    for _ in 0..3 {
        session.load_from(&file.0).unwrap();
        assert_eq!(
            session.world_id, next.world_id,
            "same saved identity can be reloaded"
        );
        assert!(!preparation.request(&session.sim, tag(&session), None));
        assert!(matches!(
            preparation.poll(tag(&session), false),
            Completion::Waiting
        ));
        assert!(!preparation.is_idle());
    }
    assert_eq!(session.terrain_revision, old_tag.revision + 3);
    let before = format!("{:?}", session.sim);
    sender.send(old_result).expect("late delivery");
    assert!(
        !preparation.request(&session.sim, tag(&session), None),
        "unconsumed completion still occupies the slot"
    );
    assert!(matches!(
        preparation.poll(tag(&session), false),
        Completion::Discarded
    ));
    assert!(preparation.request(&session.sim, tag(&session), None));
    let fresh = wait(&mut preparation, tag(&session));
    assert!((fresh.cache.height_at(&session.sim, Position::new(4., 4.)) - 5.).abs() < 1e-5);
    let hit = fresh
        .picker
        .unwrap()
        .pick(
            glam::Vec3::new(0.1, 8., 0.1),
            glam::Vec3::new(0.2, -1., 0.).normalize(),
            false,
        )
        .unwrap();
    assert!(
        (hit.x - 4.7).abs() < 1e-4 && (hit.z - 4.1).abs() < 1e-4,
        "picker uses the loaded height, not the old world's height"
    );
    assert_eq!(
        format!("{:?}", session.sim),
        before,
        "derived work cannot mutate the world"
    );
}

#[test]
fn failed_load_preserves_current_work_and_generated_replacement_invalidates_it() {
    let mut session = Session::from_sim(world(2.));
    let current = tag(&session);
    let identity = session.world_id.clone();
    let mut preparation = Preparation::default();
    assert!(preparation.request(&session.sim, current, None));
    let file = File::new();
    std::fs::write(&file.0, b"{truncated").unwrap();
    assert!(session.load_from(&file.0).is_err());
    assert_eq!(tag(&session), current);
    assert_eq!(session.world_id, identity);
    let result = wait(&mut preparation, current);
    let (mut preparation, sender) = delayed(current);
    session.replace_sim(world(4.));
    assert_eq!(session.terrain_revision, current.revision + 1);
    assert_ne!(session.world_id, identity);
    sender.send(result).expect("late result");
    assert!(matches!(
        preparation.poll(tag(&session), false),
        Completion::Discarded
    ));
}

#[test]
fn disconnected_worker_releases_the_slot_for_recovery() {
    let session = Session::from_sim(world(2.));
    let tag = tag(&session);
    let (mut preparation, sender) = delayed(tag);
    drop(sender);
    assert!(matches!(
        preparation.poll(tag, false),
        Completion::Discarded
    ));
    assert!(preparation.request(&session.sim, tag, None));
    assert!(wait(&mut preparation, tag).picker.is_some());
}
