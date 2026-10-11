//! Freeform footpaths: shared preview recipe, bounded validation and construction.
use crate::{
    SimState,
    settlement::{Building, Path},
    spatial::Position,
    surface,
    units::metres,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PathError {
    InvalidRoute,
    TooLong,
    OutsideMap,
    Underwater,
    SteepSlope,
    Occupied,
    Watercourse,
    Duplicate,
}
impl std::fmt::Display for PathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidRoute => "Choose 2–64 distinct waypoints and a width of 1.2–4 m",
            Self::TooLong => "Keep each path below 500 m",
            Self::OutsideMap => "Keep the whole path inside the region",
            Self::Underwater => "Route the path over dry land",
            Self::SteepSlope => "Choose a gentler route (maximum slope 35%)",
            Self::Occupied => "Route around dwellings and boreholes",
            Self::Watercourse => "A bridge is needed to cross this watercourse",
            Self::Duplicate => "This path already exists",
        })
    }
}
pub fn distance(a: Position, b: Position) -> f32 {
    (a.x - b.x).hypot(a.z - b.z)
}
pub fn nearest(p: Position, a: Position, b: Position) -> Position {
    let dx = b.x - a.x;
    let dz = b.z - a.z;
    let t = (((p.x - a.x) * dx + (p.z - a.z) * dz) / (dx * dx + dz * dz).max(1e-12)).clamp(0., 1.);
    Position::new(a.x + t * dx, a.z + t * dz)
}
pub fn length(path: &Path) -> f32 {
    path.points.windows(2).map(|p| distance(p[0], p[1])).sum()
}
pub fn on_path(path: &Path, p: Position, margin: f32) -> bool {
    path.points
        .windows(2)
        .any(|s| distance(p, nearest(p, s[0], s[1])) <= path.width * 0.5 + margin)
}
/// Segment versus expanded rotated footprint, including long sparse segments.
pub fn intersects_building(path: &Path, b: &Building) -> bool {
    let (s, c) = b.rotation.sin_cos();
    let local = |p: Position| {
        [
            (p.x - b.position.x) * c + (p.z - b.position.z) * s,
            (p.z - b.position.z) * c - (p.x - b.position.x) * s,
        ]
    };
    let extent = [
        Building::HALF_WIDTH + path.width * 0.5,
        Building::HALF_DEPTH + path.width * 0.5,
    ];
    path.points.windows(2).any(|pair| {
        let a = local(pair[0]);
        let z = local(pair[1]);
        let mut lo: f32 = 0.;
        let mut hi: f32 = 1.;
        for axis in 0..2 {
            let d = z[axis] - a[axis];
            if d.abs() < 1e-9 {
                if a[axis].abs() > extent[axis] {
                    return false;
                }
            } else {
                let t0 = (-extent[axis] - a[axis]) / d;
                let t1 = (extent[axis] - a[axis]) / d;
                lo = lo.max(t0.min(t1));
                hi = hi.min(t0.max(t1));
                if lo > hi {
                    return false;
                }
            }
        }
        true
    })
}
/// Rounded corners preserve endpoints; samples are at most half a metre apart.
pub fn route(waypoints: &[Position], width: f32) -> Result<Path, PathError> {
    if !(2..=64).contains(&waypoints.len())
        || !width.is_finite()
        || !(metres(1.2)..=metres(4.)).contains(&width)
        || waypoints
            .iter()
            .any(|p| !p.x.is_finite() || !p.z.is_finite())
        || waypoints
            .windows(2)
            .any(|p| distance(p[0], p[1]) < metres(0.5))
    {
        return Err(PathError::InvalidRoute);
    }
    if waypoints
        .windows(2)
        .map(|p| distance(p[0], p[1]))
        .sum::<f32>()
        > metres(500.)
    {
        return Err(PathError::TooLong);
    }
    let mut controls = waypoints.to_vec();
    for _ in 0..2 {
        let mut next = vec![controls[0]];
        for pair in controls.windows(2) {
            for t in [0.25, 0.75] {
                next.push(Position::new(
                    pair[0].x + (pair[1].x - pair[0].x) * t,
                    pair[0].z + (pair[1].z - pair[0].z) * t,
                ));
            }
        }
        next.push(controls[controls.len() - 1]);
        controls = next;
    }
    Ok(Path {
        points: controls,
        width,
    })
}
/// Fine samples are temporary for validation and drawing, never world storage.
pub fn samples(path: &Path) -> Vec<Position> {
    let Some(first) = path.points.first() else {
        return Vec::new();
    };
    let mut points = vec![*first];
    for pair in path.points.windows(2) {
        let steps = (distance(pair[0], pair[1]) / metres(0.5))
            .ceil()
            .clamp(1., 2000.) as usize;
        for i in 1..=steps {
            let t = i as f32 / steps as f32;
            points.push(Position::new(
                pair[0].x + (pair[1].x - pair[0].x) * t,
                pair[0].z + (pair[1].z - pair[0].z) * t,
            ));
        }
    }
    points
}
/// Preview and committed actions use exactly the same terrain/obstacle checks.
pub fn validate(sim: &SimState, waypoints: &[Position], width: f32) -> Result<Path, PathError> {
    let path = route(waypoints, width)?;
    if sim.paths.contains(&path) {
        return Err(PathError::Duplicate);
    }
    let radius = width * 0.5;
    let mut previous: Option<([f32; 3], Position)> = None;
    let samples = samples(&path);
    for (i, &p) in samples.iter().enumerate() {
        let a = samples[i.saturating_sub(1)];
        let b = samples[(i + 1).min(samples.len() - 1)];
        let d = distance(a, b).max(1e-9);
        let side = Position::new(-(b.z - a.z) / d * radius, (b.x - a.x) / d * radius);
        let mut heights = [0.; 3];
        for (j, offset) in [-1., 0., 1.].into_iter().enumerate() {
            let q = Position::new(p.x + side.x * offset, p.z + side.z * offset);
            if q.x < 0. || q.z < 0. || q.x >= sim.grid.width as f32 || q.z >= sim.grid.height as f32
            {
                return Err(PathError::OutsideMap);
            }
            heights[j] = surface::elevation(sim, q);
            if heights[j] <= sim.sea_level + metres(0.1) {
                return Err(PathError::Underwater);
            }
        }
        if (heights[0] - heights[2]).abs() > width * 0.35 + metres(0.02) {
            return Err(PathError::SteepSlope);
        }
        if let Some((old, q)) = previous {
            let limit = distance(p, q) * 0.35 + metres(0.02);
            if heights.iter().zip(old).any(|(a, b)| (a - b).abs() > limit) {
                return Err(PathError::SteepSlope);
            }
        }
        previous = Some((heights, p));
        for b in &sim.buildings {
            let (s, c) = b.rotation.sin_cos();
            let dx = p.x - b.position.x;
            let dz = p.z - b.position.z;
            if (dx * c + dz * s).abs() < Building::HALF_WIDTH + radius + metres(0.1)
                && (dz * c - dx * s).abs() < Building::HALF_DEPTH + radius + metres(0.1)
            {
                return Err(PathError::Occupied);
            }
        }
        if sim
            .subsurface
            .wells
            .iter()
            .any(|w| distance(w.position, p) < radius + metres(0.6))
        {
            return Err(PathError::Occupied);
        }
        if crate::worldgen::river_distance(sim, p) < radius + metres(7.) {
            return Err(PathError::Watercourse);
        }
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{actions::Action, config::SimConfig, events::SimEvent};
    fn flat() -> SimState {
        let mut sim = SimState::new(SimConfig {
            width: 32,
            height: 32,
            ..Default::default()
        });
        for c in sim.grid.cells_mut() {
            c.elevation = 1.;
        }
        sim
    }
    fn points() -> Vec<Position> {
        vec![
            Position::new(4., 4.),
            Position::new(5., 4.3),
            Position::new(6., 4.),
        ]
    }
    #[test]
    fn rounded_routes_preserve_endpoints_and_validate_interiors() {
        let mut sim = flat();
        let points = points();
        let path = validate(&sim, &points, Building::PATH_WIDTH).unwrap();
        assert_eq!(path.points.first(), points.first());
        assert_eq!(path.points.last(), points.last());
        assert!(
            samples(&path)
                .windows(2)
                .all(|s| distance(s[0], s[1]) <= metres(0.501))
        );
        let middle = path.points[path.points.len() / 2];
        sim.buildings.push(Building {
            id: 1,
            position: middle,
            rotation: 0.7,
        });
        assert_eq!(
            validate(&sim, &points, Building::PATH_WIDTH),
            Err(PathError::Occupied)
        );
        sim.buildings.clear();
        sim.sea_level = 2.;
        assert_eq!(
            validate(&sim, &points, Building::PATH_WIDTH),
            Err(PathError::Underwater)
        );
        sim.sea_level = 0.;
        for z in 0..32 {
            for x in 0..32 {
                sim.grid.get_mut(x, z).elevation = x as f32;
            }
        }
        assert_eq!(
            validate(&sim, &points, Building::PATH_WIDTH),
            Err(PathError::SteepSlope)
        );
    }
    #[test]
    fn invalid_and_unbounded_routes_are_rejected_without_mutating_the_world() {
        let mut sim = flat();
        for points in [
            vec![],
            vec![Position::new(f32::NAN, 2.), Position::new(3., 4.)],
            vec![Position::new(1., 1.), Position::new(31., 1.)],
            vec![Position::new(-1., 4.), Position::new(3., 4.)],
        ] {
            let events = sim.step(
                0.1,
                &[Action::BuildPath {
                    waypoints: points,
                    width: Building::PATH_WIDTH,
                }],
            );
            assert!(
                events
                    .iter()
                    .any(|e| matches!(e, SimEvent::PathRejected { .. }))
            );
            assert!(sim.paths.is_empty());
        }
        assert_eq!(
            route(&points(), f32::INFINITY),
            Err(PathError::InvalidRoute)
        );
        assert_eq!(
            route(&[Position::new(2., 2.); 2], Building::PATH_WIDTH),
            Err(PathError::InvalidRoute)
        );
    }
    #[test]
    fn construction_is_deterministic_ordered_and_rejects_duplicates() {
        let mut a = flat();
        let mut b = a.clone();
        let action = Action::BuildPath {
            waypoints: points(),
            width: Building::PATH_WIDTH,
        };
        let events = a.step(0.1, &[action.clone(), action.clone()]);
        b.step(0.1, &[action.clone(), action]);
        assert_eq!(a.paths, b.paths);
        assert_eq!(a.paths.len(), 1);
        assert!(matches!(events[0], SimEvent::PathBuilt { index: 0 }));
        assert!(matches!(
            events[1],
            SimEvent::PathRejected {
                reason: PathError::Duplicate
            }
        ));
        assert_eq!(
            a.validate_building(&Building {
                id: 1,
                position: a.paths[0].points[5],
                rotation: 0.4
            }),
            Err(crate::settlement::PlacementError::Occupied)
        );
    }
}
