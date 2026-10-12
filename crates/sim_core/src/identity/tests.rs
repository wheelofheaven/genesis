use super::*;
use crate::{
    SimState,
    actions::Action,
    config::SimConfig,
    events::SimEvent,
    paths::PathError,
    settlement::{Building, PlacementError},
    spatial::Position,
    subsurface::WellError,
};

fn flat() -> SimState {
    let mut s = SimState::new(SimConfig {
        width: 32,
        height: 32,
        ..Default::default()
    });
    for c in s.grid.cells_mut() {
        c.elevation = 1.;
    }
    for c in &mut s.subsurface.columns {
        c.capacity = 20.;
        c.stored = 0.01;
        c.permeability = 1.;
    }
    s
}
fn commands(offset: f32) -> [Action; 3] {
    [
        Action::PlaceBuilding {
            position: Position::new(4. + offset * 3., 4.),
            rotation: 0.,
        },
        Action::DrillWell {
            position: Position::new(12.1 + offset * 0.8, 12.1),
        },
        Action::BuildPath {
            waypoints: vec![
                Position::new(18. + offset * 3., 18.),
                Position::new(19. + offset * 3., 19.),
            ],
            width: Building::PATH_WIDTH,
        },
    ]
}

#[test]
fn removed_highest_ids_are_never_reused_and_stale_well_commands_fail() {
    let mut s = flat();
    s.step(0., &commands(0.));
    s.step(0., &commands(1.));
    assert_eq!(
        (s.buildings.len(), s.paths.len(), s.subsurface.wells.len()),
        (2, 2, 2)
    );
    assert_eq!(s.buildings.pop().unwrap().id, 2);
    assert_eq!(s.paths.pop().unwrap().id, 2);
    assert_eq!(s.subsurface.wells.pop().unwrap().id, 2);
    let events = s.step(0., &commands(1.));
    assert!(matches!(events[0], SimEvent::BuildingPlaced { id: 3 }));
    assert!(matches!(events[1], SimEvent::WellDrilled { id: 3 }));
    assert!(matches!(events[2], SimEvent::PathBuilt { id: 3 }));
    let events = s.step(
        0.,
        &[Action::SetWellActive {
            id: 2,
            active: false,
        }],
    );
    assert!(matches!(
        events[0],
        SimEvent::WellRejected {
            reason: WellError::UnknownWell
        }
    ));
    assert!(s.subsurface.wells.iter().all(|w| w.active));
}

#[test]
fn repacking_preserves_well_arbitration_under_scarcity() {
    let mut s = flat();
    s.step(0., &commands(0.));
    s.step(0., &commands(1.));
    let mut reordered = s.clone();
    reordered.subsurface.wells.reverse();
    s.step(0.1, &[]);
    reordered.step(0.1, &[]);
    assert_eq!(s.subsurface.wells, reordered.subsurface.wells);
    assert!(s.subsurface.wells.iter().all(|w| w.recovered > 0.));
    for (a, b) in s
        .subsurface
        .columns
        .iter()
        .zip(&reordered.subsurface.columns)
    {
        assert_eq!(a.stored, b.stored);
    }
}

#[test]
fn exhaustion_rejects_without_clearing_cover_or_creating_objects() {
    let mut s = flat();
    s.building_ids = Allocator(u64::MAX);
    s.path_ids = Allocator(u64::MAX);
    s.subsurface.well_ids = Allocator(u64::MAX);
    s.geography.cover = [Position::new(4., 4.), Position::new(18.5, 18.5)]
        .map(|position| crate::worldgen::Cover {
            position,
            size: 1.,
            kind: crate::ecology::CoverKind::DryShrub,
            seed: 42,
        })
        .to_vec();
    let before = s.geography.cover.clone();
    let events = s.step(0., &commands(0.));
    assert!(matches!(
        events[0],
        SimEvent::BuildingRejected {
            reason: PlacementError::IdentityExhausted
        }
    ));
    assert!(matches!(
        events[1],
        SimEvent::WellRejected {
            reason: WellError::IdentityExhausted
        }
    ));
    assert!(matches!(
        events[2],
        SimEvent::PathRejected {
            reason: PathError::IdentityExhausted
        }
    ));
    assert!(s.buildings.is_empty() && s.paths.is_empty() && s.subsurface.wells.is_empty());
    assert_eq!(before, s.geography.cover);
    assert_eq!(s.building_ids, Allocator(u64::MAX));
}
