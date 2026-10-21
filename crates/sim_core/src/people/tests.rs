use super::*;
use crate::{runtime::Simulation, scenarios};

// Social/navigation tests begin with genuinely covered homes; empty homes now
// remain available for supply work instead of leaving for a social visit.
fn covered_settlement() -> SimState {
    let mut sim = scenarios::supplied_settlement();
    let homes: Vec<_> = sim.people.all().iter().map(|p| p.home).collect();
    for home in homes {
        assert!(sim.food.deliver(home, crate::food::CAPACITY));
        sim.food.sources[0].remaining -= crate::food::CAPACITY;
        assert!(sim.households.deliver(home, crate::households::CAPACITY));
        sim.subsurface.wells[0].recovered += crate::households::CAPACITY;
        sim.subsurface.wells[0].supply.as_mut().unwrap().collected += crate::households::CAPACITY;
    }
    assert!(sim.food.valid(&sim) && sim.households.valid(&sim));
    sim
}
#[test]
fn residents_complete_routines_and_survive_repacking_deterministically() {
    let mut a = covered_settlement();
    assert_eq!(a.people.all().len(), 6);
    let mut b = a.clone();
    b.people.entries.reverse();
    b.buildings.reverse();
    let mut visited = std::collections::BTreeSet::new();
    let mut returned = std::collections::BTreeSet::new();
    for _ in 0..1200 {
        a.step(0.1, &[]);
        b.step(0.1, &[]);
        for p in a.people.all() {
            assert_eq!(Some(p), b.people.get(p.id));
            assert!(p.blocked.is_none(), "{} {:?}", p.name, p.blocked);
            if p.activity == Activity::Visiting {
                visited.insert(p.id);
            }
            if p.activity == Activity::Resting && visited.contains(&p.id) {
                returned.insert(p.id);
            }
        }
    }
    assert_eq!(visited.len(), 6);
    assert_eq!(returned.len(), 6);
}
#[test]
fn blocked_ground_stops_movement_and_recovers_without_teleporting() {
    let mut sim = scenarios::living_settlement();
    for _ in 0..20 {
        sim.step(0.1, &[]);
    }
    let positions: Vec<_> = sim.people.all().iter().map(|p| p.position).collect();
    sim.sea_level = 2.;
    for _ in 0..30 {
        sim.step(0.1, &[]);
    }
    for (p, position) in sim.people.all().iter().zip(positions) {
        assert_eq!(p.position, position);
        assert_eq!(p.blocked, Some(Blocked::UnsafeGround));
    }
    sim.sea_level = 0.;
    for _ in 0..100 {
        sim.step(0.1, &[]);
    }
    assert!(sim.people.all().iter().all(|p| p.blocked.is_none()));
}
#[test]
fn snapshot_continues_routes_and_scheduled_departures() {
    let mut a = Simulation::new(covered_settlement());
    for _ in 0..35 {
        a.advance(&[]);
    }
    let mut b = crate::save::Snapshot::capture(&a, "residents", &[])
        .unwrap()
        .restore()
        .unwrap()
        .simulation;
    for _ in 0..800 {
        assert_eq!(
            format!("{:?}", a.advance(&[])),
            format!("{:?}", b.advance(&[]))
        );
        assert_eq!(a.people, b.people);
    }
}
#[test]
fn navigation_detours_around_a_dwelling_and_refuses_unbounded_search() {
    let sim = scenarios::living_settlement();
    let from = Position::new(7.5, 7.2);
    let to = Position::new(7.5, 8.);
    let path = navigation::route(&sim, from, to).unwrap();
    let mut p = from;
    assert!(path.len() > 2);
    for next in path {
        assert!(distance(walking::advance(&sim, p, next), next) < metres(0.001));
        p = next;
    }
    assert_eq!(p, to);
    assert!(navigation::route(&sim, from, Position::new(15., 15.)).is_none());
}

#[test]
fn changed_route_obstacle_stops_a_committed_journey() {
    let mut sim = covered_settlement();
    for _ in 0..15 {
        sim.step(0.1, &[]);
    }
    let p = sim.people.all()[0].clone();
    assert!(!p.route().is_empty());
    let direction = p.destination();
    let d = distance(p.position, direction);
    let center = Position::new(
        p.position.x + (direction.x - p.position.x) * metres(2.3) / d,
        p.position.z + (direction.z - p.position.z) * metres(2.3) / d,
    );
    sim.geography.cover.push(crate::worldgen::Cover {
        position: center,
        size: 1.,
        kind: crate::ecology::CoverKind::Limestone,
        seed: 1,
    });
    assert!(walking::standable(&sim, p.position));
    for _ in 0..6 {
        sim.step(0.1, &[]);
    }
    let next = sim.people.get(p.id).unwrap();
    assert_eq!(next.blocked, Some(Blocked::NoLocalRoute));
    assert!(walking::standable(&sim, next.position));
    assert!(distance(p.position, next.position) < metres(0.31));
    assert_ne!(next.activity, Activity::Visiting);
}
