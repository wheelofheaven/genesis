//! Repeatable material inspection cameras using existing world placements.
use crate::{session::Session, terrain};
use genesis_renderer::camera::Camera;
use genesis_sim_core::{
    ecology::CoverKind, paths::distance, spatial::Position, surface::walking, units::metres,
};
use glam::Vec3;

pub(crate) fn rock(
    session: &mut Session,
    camera: &mut Camera,
    kind: Option<CoverKind>,
) -> crate::Result<()> {
    let sim = &session.sim;
    let clearance = crate::scene::Clearance::new(sim);
    let rock = sim
        .geography
        .cover
        .iter()
        .filter(|p| {
            p.kind.rock()
                && kind.is_none_or(|kind| kind == p.kind)
                && p.size >= 0.8
                && !clearance.hides_cover(p)
                && terrain::point(sim, p.position).y > sim.sea_level + metres(0.3)
        })
        .min_by(|a, b| {
            distance(a.position, session.position)
                .total_cmp(&distance(b.position, session.position))
        })
        .ok_or("no exposed rock of the requested kind to inspect")?;
    let p = rock.position;
    let stand = (0..64)
        .map(|i| {
            let a = 2.6 + i as f32 * 2.399_963;
            let radius = metres(3.5 * rock.size + 1.);
            Position::new(p.x + a.cos() * radius, p.z + a.sin() * radius)
        })
        .find(|p| walking::standable(sim, *p))
        .ok_or("no accessible ground near this rock")?;
    camera.enter_ground(terrain::point(sim, stand), metres(1.7));
    camera.look_at_ground(terrain::point(sim, p) + Vec3::Y * metres(0.45 * rock.size));
    eprintln!(
        "rock inspection: {}, size {:.3}, seed {}",
        rock.kind.name(),
        rock.size,
        rock.seed
    );
    session.clock.paused = true;
    Ok(())
}
