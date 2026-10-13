use super::*;
use genesis_sim_core::grid::SampleBounds;

#[test]
fn batched_commits_outlive_hud_history_but_expired_ranges_force_full_refresh() {
    let mut s = Session::from_sim(genesis_sim_core::scenarios::coastal_outpost());
    s.clock.paused = true;
    for x in 2..14 {
        s.queue(Action::Terraform {
            x,
            y: 12,
            radius: 0,
            strength: 0.1,
        });
    }
    s.step_once();
    assert_eq!(s.terrain_revision, 12);
    assert_eq!(s.events.len(), 8);
    assert_eq!(
        s.terrain_changes_since(0),
        Some(SampleBounds {
            min: [2, 12],
            max: [13, 12]
        })
    );
    for _ in 0..21 {
        s.queue(Action::Terraform {
            x: 30,
            y: 20,
            radius: 0,
            strength: 0.1,
        });
        s.step_once();
    }
    assert_eq!(s.terrain_changes.len(), 32);
    assert_eq!(s.terrain_changes_since(0), None);
    assert_eq!(
        s.terrain_changes_since(12),
        Some(SampleBounds {
            min: [30, 20],
            max: [30, 20]
        })
    );
    assert_eq!(s.terrain_changes_since(s.terrain_revision), None);
}

#[test]
fn construction_and_world_replacement_break_local_revision_chains() {
    let mut s = Session::from_sim(genesis_sim_core::scenarios::coastal_outpost());
    s.clock.paused = true;
    s.queue(Action::Terraform {
        x: 15,
        y: 12,
        radius: 2,
        strength: 0.1,
    });
    s.queue(Action::PlaceBuilding {
        position: Position::new(25.31, 20.27),
        rotation: 0.47,
    });
    s.step_once();
    assert_eq!(s.terrain_revision, 2);
    assert_eq!(s.terrain_changes_since(0), None);
    let old_revision = s.terrain_revision;
    s.replace_sim(genesis_sim_core::scenarios::coastal_outpost());
    assert_eq!(s.terrain_changes_since(old_revision), None);
    s.clock.paused = true;
    s.queue(Action::Terraform {
        x: 15,
        y: 12,
        radius: 2,
        strength: 0.1,
    });
    s.step_once();
    assert_eq!(s.terrain_changes_since(old_revision), None);
    assert!(s.terrain_changes_since(old_revision + 1).is_some());
}
