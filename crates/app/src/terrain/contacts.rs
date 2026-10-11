//! Cosmetic ground occlusion from the same exposed scenery used by the renderer.
use genesis_renderer::contact::{EXTENT_METRES, Field, Footprint};
use genesis_sim_core::{SimState, ecology::CoverKind, units::metres, worldgen::noise::unit};
use glam::{Vec2, Vec3};

pub(super) fn build(sim: &SimState, eye: Vec3) -> Field {
    let center = Vec2::new(eye.x, eye.z) * 20.;
    let clearance = crate::scene::Clearance::new(sim);
    let mut footprints = Vec::new();
    for plant in &sim.geography.cover {
        let p = Vec2::new(
            plant.position.x - sim.grid.width as f32 * 0.5,
            plant.position.z - sim.grid.height as f32 * 0.5,
        ) * 20.;
        if (p - center).abs().max_element() > EXTENT_METRES * 0.5 + 16.
            || clearance.hides_cover(plant)
            || super::point(sim, plant.position).y < sim.sea_level + metres(0.3)
        {
            continue;
        }
        let rotation = unit(plant.seed, 955) * std::f32::consts::TAU;
        let (radius, strength) = if plant.kind.tree() {
            (Vec2::new(0.9, 0.8), 0.52)
        } else if plant.kind.rock() {
            (Vec2::new(1.8, 1.4), 0.46)
        } else {
            (Vec2::new(1.25, 1.1), 0.30)
        };
        footprints.push(Footprint {
            center: p,
            radius: radius * plant.size,
            rotation,
            strength,
        });
        if plant.kind.tree() {
            let radius = match plant.kind {
                CoverKind::Broadleaf => 3.5,
                CoverKind::Pine => 3.0,
                _ => 2.5,
            };
            footprints.push(Footprint {
                center: p,
                radius: Vec2::new(radius, radius * 0.88) * plant.size,
                rotation,
                strength: 0.16,
            });
        }
    }
    Field::bake(center, footprints)
}

#[cfg(test)]
mod tests {
    use super::*;
    use genesis_sim_core::{
        config::SimConfig,
        settlement::{Building, Path},
        spatial::Position,
        worldgen::Cover,
    };

    #[test]
    fn ground_contact_disappears_with_hidden_removed_or_submerged_scenery() {
        let mut sim = SimState::new(SimConfig {
            width: 8,
            height: 8,
            ..Default::default()
        });
        for cell in sim.grid.cells_mut() {
            cell.elevation = 2.;
        }
        let position = Position::new(4., 4.);
        let cover = Cover {
            position,
            size: 1.,
            seed: 7,
            kind: CoverKind::Pine,
        };
        let eye = Vec3::new(0., 2.085, 0.);
        let contact = |sim: &SimState| build(sim, eye).at(Vec2::ZERO);
        for kind in CoverKind::ALL {
            sim.geography.cover = vec![Cover { kind, ..cover }];
            assert!(contact(&sim) > 0, "{}", kind.name());
        }
        sim.buildings.push(Building {
            id: 1,
            position,
            rotation: 0.7,
        });
        assert_eq!(contact(&sim), 0);
        sim.buildings.clear();
        sim.paths.push(Path {
            points: vec![Position::new(3., 4.), Position::new(5., 4.)],
            width: metres(1.2),
        });
        assert_eq!(contact(&sim), 0);
        sim.paths.clear();
        sim.sea_level = 3.;
        assert_eq!(contact(&sim), 0);
        sim.sea_level = 0.;
        sim.geography.cover.clear();
        assert_eq!(contact(&sim), 0);
    }
}
