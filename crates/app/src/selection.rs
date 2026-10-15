//! Disposable hover/selection geometry; the world mesh is never rebuilt for hover.
use crate::{interaction::Object, scene, session::Session, terrain};
use genesis_renderer::geometry::Vertex;
use genesis_sim_core::{settlement::Building, spatial::Position, units::metres};
use glam::Vec3;

pub fn object(
    session: &Session,
    cache: Option<&terrain::Cache>,
    out: &mut Vec<Vertex>,
    object: Object,
    selected: bool,
) {
    let sim = &session.sim;
    let color = if selected {
        [0.08, 0.82, 0.92]
    } else {
        [0.58, 0.91, 0.88]
    };
    match object {
        Object::Person(id) => {
            if let Some(person) = sim.people.get(id) {
                let p = session.person_position(person);
                let ring: Vec<_> = (0..=32)
                    .map(|i| {
                        let a = i as f32 * std::f32::consts::TAU / 32.;
                        Position::new(p.x + a.cos() * metres(0.65), p.z + a.sin() * metres(0.65))
                    })
                    .collect();
                scene::highlight(
                    sim,
                    cache,
                    out,
                    &ring,
                    metres(0.08),
                    color,
                    session.survey.seafloor,
                );
                if selected && !person.route().is_empty() {
                    let mut route = vec![p];
                    route.extend_from_slice(person.route());
                    scene::highlight(
                        sim,
                        cache,
                        out,
                        &route,
                        metres(0.08),
                        color,
                        session.survey.seafloor,
                    );
                }
            }
        }
        Object::Building(id) => {
            let Some(b) = sim.buildings.iter().find(|b| b.id == id) else {
                return;
            };
            // A small expanded shell tints visible faces without showing through terrain.
            let start = out.len();
            scene::house(sim, out, b);
            let center = terrain::point(sim, b.position) + Vec3::Y * Building::WALL_HEIGHT * 0.5;
            for v in &mut out[start..] {
                let p = Vec3::from_array(v.position);
                v.position = (center + (p - center) * 1.006).to_array();
                for (i, &c) in color.iter().enumerate() {
                    v.color[i] = v.color[i] * 0.60 + c * 0.40;
                }
            }
            let mut corners = b.footprint().to_vec();
            corners.push(corners[0]);
            scene::highlight(
                sim,
                cache,
                out,
                &corners,
                metres(if selected { 0.18 } else { 0.10 }),
                color,
                session.survey.seafloor,
            );
            // Roof perimeter remains readable when the foundation is behind a wall.
            let high = b
                .footprint()
                .iter()
                .map(|p| terrain::point(sim, *p).y)
                .fold(f32::NEG_INFINITY, f32::max);
            let start = out.len();
            scene::ribbon(
                sim,
                out,
                &corners,
                metres(if selected { 0.12 } else { 0.06 }),
                color,
            );
            for v in &mut out[start..] {
                v.position[1] = high + Building::WALL_HEIGHT + metres(0.51);
            }
        }
        Object::Path(id) => {
            if let Some(path) = sim.paths.iter().find(|p| p.id == id) {
                scene::highlight(
                    sim,
                    cache,
                    out,
                    &genesis_sim_core::paths::samples(path),
                    path.width + metres(0.16),
                    color,
                    session.survey.seafloor,
                );
                // Keep the path surface legible inside a narrow luminous edge.
                let start = out.len();
                scene::highlight(
                    sim,
                    cache,
                    out,
                    &genesis_sim_core::paths::samples(path),
                    (path.width - metres(0.12)).max(metres(0.4)),
                    if selected {
                        [0.36, 0.56, 0.49]
                    } else {
                        [0.55, 0.61, 0.46]
                    },
                    session.survey.seafloor,
                );
                for v in &mut out[start..] {
                    v.position[1] += metres(0.008);
                }
            }
        }
    }
}
pub fn draft(session: &Session, cache: Option<&terrain::Cache>, out: &mut Vec<Vertex>) {
    if let Some((path, valid)) = session.path_preview() {
        scene::highlight(
            &session.sim,
            cache,
            out,
            &genesis_sim_core::paths::samples(&path),
            path.width,
            if valid {
                [0.22, 0.80, 0.66]
            } else {
                [0.95, 0.22, 0.13]
            },
            session.survey.seafloor,
        );
    }
    for (i, p) in session.path.points.iter().enumerate() {
        let points: Vec<_> = (0..=24)
            .map(|n| {
                let a = n as f32 / 24. * std::f32::consts::TAU;
                Position::new(p.x + a.cos() * metres(0.65), p.z + a.sin() * metres(0.65))
            })
            .collect();
        scene::highlight(
            &session.sim,
            cache,
            out,
            &points,
            metres(0.12),
            if i == 0 {
                [0.9, 0.95, 0.8]
            } else {
                [0.12, 0.8, 0.9]
            },
            session.survey.seafloor,
        );
    }
}
