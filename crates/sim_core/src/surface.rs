//! Authoritative local ground, independent of rendering LOD and climate cells.
//! The regular sub-metre lattice is queried lazily; it is never allocated for
//! the whole region. Coordinates retain the shared 20 m world-unit convention.
use crate::{
    SimState,
    spatial::{self, Position},
    units::metres,
    worldgen::noise::noise,
};
pub mod walking;

pub const SUBDIVISIONS: u32 = 32;
pub const SPACING: f32 = 1. / SUBDIVISIONS as f32; // 0.625 m

/// Ground at a canonical lattice vertex. Detail is anchored to world space and
/// seed, never to the observer, mesh density, frame time, or ecology tick.
pub fn vertex_height(sim: &SimState, p: Position) -> f32 {
    let base = spatial::elevation(&sim.grid, p);
    if sim.geography.version == 0 {
        return base;
    }
    let x = p.x * crate::units::METRES_PER_UNIT;
    let z = p.z * crate::units::METRES_PER_UNIT;
    let mut detail = noise(sim.config.seed ^ 0x713, x / 9., z / 9.) * metres(0.18)
        + noise(sim.config.seed ^ 0x719, x / 2.5, z / 2.5) * metres(0.045);
    // Preserve the existing regional continuation and shore elevations.
    let edge =
        p.x.min(p.z)
            .min(sim.grid.width as f32 - p.x)
            .min(sim.grid.height as f32 - p.z);
    detail *= edge.clamp(0., 1.) * ((base - sim.sea_level).abs() / metres(2.)).clamp(0., 1.);
    // Suppress small hummocks under existing foundations and worn paths.
    // This is not construction grading: regional slope remains authoritative.
    for b in &sim.buildings {
        let (s, c) = b.rotation.sin_cos();
        let dx = p.x - b.position.x;
        let dz = p.z - b.position.z;
        let outside = ((dx * c + dz * s).abs() - crate::settlement::Building::HALF_WIDTH)
            .max((dz * c - dx * s).abs() - crate::settlement::Building::HALF_DEPTH);
        detail *= (outside / metres(1.5)).clamp(0., 1.);
    }
    for path in &sim.paths {
        for pair in path.points.windows(2) {
            let margin = path.width * 0.5 + metres(0.5);
            if p.x < pair[0].x.min(pair[1].x) - margin
                || p.x > pair[0].x.max(pair[1].x) + margin
                || p.z < pair[0].z.min(pair[1].z) - margin
                || p.z > pair[0].z.max(pair[1].z) + margin
            {
                continue;
            }
            let dx = pair[1].x - pair[0].x;
            let dz = pair[1].z - pair[0].z;
            let t = (((p.x - pair[0].x) * dx + (p.z - pair[0].z) * dz)
                / (dx * dx + dz * dz).max(1e-9))
            .clamp(0., 1.);
            let distance =
                ((p.x - pair[0].x - t * dx).powi(2) + (p.z - pair[0].z - t * dz).powi(2)).sqrt();
            detail *= ((distance - path.width * 0.5) / metres(0.5)).clamp(0., 1.);
        }
    }
    (base + detail).clamp(crate::units::MIN_ELEVATION, crate::units::MAX_ELEVATION)
}

/// Fixed triangle topology shared by local rendering, placement and walking.
/// Coarser visual meshes approximate this surface; they never change it.
pub fn elevation(sim: &SimState, p: Position) -> f32 {
    let d = if sim.geography.version == 0 {
        2.
    } else {
        SUBDIVISIONS as f32
    };
    let x = (p.x * d).floor() / d;
    let z = (p.z * d).floor() / d;
    let tx = (p.x - x) * d;
    let tz = (p.z - z) * d;
    let a = vertex_height(sim, Position::new(x, z));
    if tx == 0. && tz == 0. {
        return a;
    }
    let b = vertex_height(sim, Position::new(x + 1. / d, z));
    let c = vertex_height(sim, Position::new(x, z + 1. / d));
    if tx + tz <= 1. {
        a + tx * (b - a) + tz * (c - a)
    } else {
        let e = vertex_height(sim, Position::new(x + 1. / d, z + 1. / d));
        e + (1. - tx) * (c - e) + (1. - tz) * (b - e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detail_is_bounded_continuous_and_independent_of_ticks() {
        let mut sim = SimState::new(crate::config::SimConfig {
            width: 16,
            height: 16,
            ..Default::default()
        });
        sim.geography.version = 6;
        for c in sim.grid.cells_mut() {
            c.elevation = 2.;
        }
        let points: Vec<_> = (0..100)
            .map(|i| Position::new(4. + i as f32 * 0.0037, 5.137))
            .collect();
        let before: Vec<_> = points.iter().map(|&p| elevation(&sim, p)).collect();
        assert!(before.windows(2).any(|p| (p[0] - p[1]).abs() > 1e-5));
        assert!(
            before
                .iter()
                .all(|h| (h - 2.).abs() <= metres(0.225) + 1e-5)
        );
        for (p, h) in points.iter().zip(&before) {
            assert!((elevation(&sim, Position::new(p.x + 0.0001, p.z)) - h).abs() < metres(0.01));
        }
        sim.step(0.1, &[]);
        assert_eq!(
            before,
            points
                .iter()
                .map(|&p| elevation(&sim, p))
                .collect::<Vec<_>>()
        );
        sim.config.seed += 1;
        assert_ne!(
            before,
            points
                .iter()
                .map(|&p| elevation(&sim, p))
                .collect::<Vec<_>>()
        );
    }
}
