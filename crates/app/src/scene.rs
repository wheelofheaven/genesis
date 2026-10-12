//! Read-only adapter: world positions are simulation data, meshes are disposable.
mod buildings;
mod clearance;
pub(crate) use clearance::Clearance;
mod paths;
use crate::{session::Session, terrain};
pub(crate) use buildings::house;
pub(crate) use buildings::picking_mesh as house_picking_mesh;
use genesis_renderer::geometry::{Vertex, cuboid};
use genesis_sim_core::{SimState, settlement::Building, spatial::Position, units::metres};
mod markings;
pub(crate) use markings::ribbon;
pub(crate) use paths::highlight;

pub fn mesh(
    sim: &SimState,
    layers: crate::survey::Survey,
    terrain_cache: &terrain::Cache,
    cover: &[Vertex],
) -> Vec<Vertex> {
    let mut out = Vec::new();
    terrain_cache.mesh(sim, &mut out, layers);
    out.extend_from_slice(cover);
    for path in &sim.paths {
        paths::surface(
            sim,
            terrain_cache,
            &mut out,
            &genesis_sim_core::paths::samples(path),
            path.width,
        );
    }
    if layers.minerals {
        for d in &sim.subsurface.deposits {
            for radius in [1., 0.7] {
                let points: Vec<_> = (0..=80)
                    .map(|i| {
                        let a = i as f32 / 80. * std::f32::consts::TAU;
                        d.outline(a, radius)
                    })
                    .collect();
                ribbon(
                    sim,
                    &mut out,
                    &points,
                    if radius == 1. { 0.1 } else { 0.045 },
                    crate::survey::mineral_color(d.mineral),
                );
            }
        }
    }
    for well in &sim.subsurface.wells {
        let p = terrain::surface_point(sim, well.position);
        cuboid(
            &mut out,
            [p.x, p.y + metres(0.1), p.z],
            [metres(0.7), metres(0.2), metres(0.7)],
            [0.3, 0.4, 0.43],
        );
        cuboid(
            &mut out,
            [p.x, p.y + metres(0.65), p.z],
            [metres(0.12), metres(1.1), metres(0.12)],
            if well.active {
                [0.1, 0.72, 0.9]
            } else {
                [0.4, 0.4, 0.4]
            },
        );
        cuboid(
            &mut out,
            [p.x, p.y + metres(1.2), p.z],
            [metres(0.6), metres(0.12), metres(0.12)],
            [0.65, 0.78, 0.79],
        );
    }
    for b in &sim.buildings {
        if layers.active() {
            house(sim, &mut out, b);
        } else {
            buildings::support(sim, &mut out, b);
        }
        if let Some(points) = paths::entrance(sim, b) {
            paths::surface(
                sim,
                terrain_cache,
                &mut out,
                &points,
                Building::ENTRANCE_WIDTH,
            );
        }
    }
    out
}

pub fn overlay(session: &Session, cache: Option<&terrain::Cache>) -> Vec<Vertex> {
    let sim = &session.sim;
    let mut out = Vec::new();
    if session.placing && session.hover.is_none() {
        return out;
    }
    if session.placing {
        let b = session.candidate();
        let valid = sim.validate_building(&b).is_ok();
        if valid {
            house(sim, &mut out, &b);
            for v in &mut out {
                v.color = [
                    v.color[0] * 0.5 + 0.1,
                    v.color[1] * 0.5 + 0.45,
                    v.color[2] * 0.5 + 0.2,
                    1.,
                ];
            }
        }
        let corners = b.footprint().map(|p| {
            Position::new(
                b.position.x + (p.x - b.position.x) * 1.2,
                b.position.z + (p.z - b.position.z) * 1.2,
            )
        });
        let mut points = Vec::new();
        for i in 0..4 {
            let a = corners[i];
            let b = corners[(i + 1) % 4];
            for j in 0..8 {
                let t = j as f32 / 8.;
                points.push(Position::new(a.x + (b.x - a.x) * t, a.z + (b.z - a.z) * t));
            }
        }
        points.push(corners[0]);
        highlight(
            sim,
            cache,
            &mut out,
            &points,
            metres(0.15),
            if valid {
                [0.35, 0.85, 0.44]
            } else {
                [0.95, 0.23, 0.13]
            },
            session.survey.seafloor,
        );
    } else if session.path.active {
        crate::selection::draft(session, cache, &mut out);
    } else if session.selected_object.is_none() {
        let points: Vec<_> = (0..=64)
            .map(|i| {
                let a = i as f32 / 64. * std::f32::consts::TAU;
                Position::new(
                    session.position.x + a.cos() * metres(2.),
                    session.position.z + a.sin() * metres(2.),
                )
            })
            .collect();
        highlight(
            sim,
            cache,
            &mut out,
            &points,
            metres(0.08),
            [0.2, 0.85, 0.95],
            session.survey.seafloor,
        );
    }
    if !session.placing && !session.path.active {
        if let Some(object) = session.selected_object {
            crate::selection::object(session, cache, &mut out, object, true);
        }
        if let Some(object) = session.hovered_object
            && Some(object) != session.selected_object
        {
            crate::selection::object(session, cache, &mut out, object, false);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ground_and_invalid_building_markers_follow_the_selected_survey_surface() {
        let mut sim = SimState::new(genesis_sim_core::config::SimConfig {
            width: 8,
            height: 8,
            ..Default::default()
        });
        sim.geography.version = 8;
        sim.sea_level = 5.;
        for z in 0..8 {
            for x in 0..8 {
                sim.grid.get_mut(x, z).elevation = 2. + ((x * 3 + z) % 5) as f32 * 0.02;
            }
        }
        let mut session = Session::from_sim(sim);
        session.position = Position::new(4., 4.);
        session.hover = Some(session.position);
        session.building_rotation = 0.7;
        for eye in [glam::Vec3::new(0., 2.1, 0.), glam::Vec3::new(0., 100., 0.)] {
            let cache = terrain::Cache::at(&session.sim, terrain::lod::Key::at(eye));
            for placing in [false, true] {
                session.placing = placing;
                for seafloor in [false, true] {
                    session.survey.seafloor = seafloor;
                    let mesh = overlay(&session, Some(&cache));
                    assert!(!mesh.is_empty());
                    assert!(mesh.len() < 20_000);
                    for tri in mesh.chunks_exact(3) {
                        let p = tri
                            .iter()
                            .map(|v| glam::Vec3::from_array(v.position))
                            .sum::<glam::Vec3>()
                            / 3.;
                        let expected = if seafloor {
                            cache.height_at(&session.sim, Position::new(p.x + 4., p.z + 4.))
                                + metres(0.04)
                        } else {
                            session.sim.sea_level + metres(0.06)
                        };
                        assert!((p.y - expected).abs() < 2e-5);
                        for v in tri {
                            let color = if placing {
                                [0.95, 0.23, 0.13, 1.]
                            } else {
                                [0.2, 0.85, 0.95, 1.]
                            };
                            assert_eq!(v.color, color);
                        }
                    }
                }
            }
        }
    }
}
