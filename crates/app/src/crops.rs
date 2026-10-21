//! Disposable rows of barley, sampled onto the authoritative ground surface.
use genesis_renderer::geometry::{Vertex, cuboid};
use genesis_sim_core::{
    SimState,
    food::{
        Source,
        cultivation::{self, Kind},
    },
    spatial::Position,
    units::metres,
};
pub fn mesh(sim: &SimState, out: &mut Vec<Vertex>, source: &Source) {
    let Kind::Barley(crop) = &source.kind else {
        return;
    };
    let progress = if source.remaining > 0 {
        1.
    } else {
        crop.growth_units as f32 / cultivation::GROWTH_UNITS as f32
    };
    let dry = cultivation::effective_moisture(sim, source) <= 0.2;
    let color = if source.remaining > 0 {
        [0.78, 0.64, 0.26]
    } else if dry {
        [0.48, 0.40, 0.24]
    } else {
        [0.38, 0.57, 0.24]
    };
    let mut index = 0;
    for x in -3..=3_i32 {
        for z in -3..=3_i32 {
            if x * x + z * z > 9 {
                continue;
            }
            let p = Position::new(
                source.position.x + metres(x as f32 * 0.5),
                source.position.z + metres(z as f32 * 0.5),
            );
            let base = crate::terrain::point(sim, p);
            cuboid(
                out,
                [base.x, base.y + metres(0.015), base.z],
                [metres(0.36), metres(0.03), metres(0.36)],
                [0.33, 0.27, 0.18],
            );
            // Standing rows thin with the remaining harvest; ground markers stay pickable.
            if source.remaining == 0 || index * cultivation::YIELD < source.remaining * 29 {
                let h = metres(0.1 + 0.85 * progress);
                cuboid(
                    out,
                    [base.x, base.y + h * 0.5, base.z],
                    [metres(0.07), h, metres(0.07)],
                    color,
                );
                if progress > 0.5 {
                    cuboid(
                        out,
                        [base.x, base.y + h, base.z],
                        [metres(0.14), metres(0.17), metres(0.14)],
                        color,
                    );
                }
            }
            index += 1;
        }
    }
}
