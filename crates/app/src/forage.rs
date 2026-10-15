//! A small readable patch whose remaining fruit derives from finite domain stock.
use genesis_renderer::geometry::{Vertex, cuboid};
use genesis_sim_core::{SimState, food::Source, spatial::Position, units::metres};
pub fn mesh(sim: &SimState, out: &mut Vec<Vertex>, source: &Source) {
    for i in 0..5 {
        let a = i as f32 * std::f32::consts::TAU / 5.;
        let p = Position::new(
            source.position.x + a.cos() * metres(0.85),
            source.position.z + a.sin() * metres(0.85),
        );
        let base = crate::terrain::point(sim, p);
        cuboid(
            out,
            [base.x, base.y + metres(0.3), base.z],
            [metres(0.16), metres(0.6), metres(0.16)],
            [0.35, 0.26, 0.15],
        );
        cuboid(
            out,
            [base.x, base.y + metres(0.65), base.z],
            [metres(0.65), metres(0.6), metres(0.65)],
            if source.remaining == 0 {
                [0.43, 0.40, 0.24]
            } else {
                [0.34, 0.49, 0.20]
            },
        );
        if source.remaining > i * source.initial / 5 {
            for offset in [-0.18, 0.18] {
                cuboid(
                    out,
                    [base.x + metres(offset), base.y + metres(0.97), base.z],
                    [metres(0.16); 3],
                    [0.76, 0.37, 0.16],
                );
            }
        }
    }
}
