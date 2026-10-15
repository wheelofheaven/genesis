//! Disposable stylized figures. No activity or position is decided here.
use genesis_renderer::geometry::{Vertex, cuboid};
use genesis_sim_core::{SimState, people::Person, spatial::Position, units::metres};

pub fn mesh(sim: &SimState, out: &mut Vec<Vertex>, person: &Person, position: Position) {
    let base = crate::terrain::surface_point(sim, position);
    let palette = [
        [0.72, 0.36, 0.23],
        [0.25, 0.48, 0.57],
        [0.74, 0.63, 0.36],
        [0.38, 0.52, 0.32],
        [0.61, 0.38, 0.47],
        [0.75, 0.71, 0.57],
    ];
    let cloth = palette[(person.id.0 as usize - 1) % palette.len()];
    let skin = [0.61, 0.40, 0.27];
    let start = out.len();
    let mut part = |center: [f32; 3], size: [f32; 3], color| {
        cuboid(out, center.map(metres), size.map(metres), color)
    };
    part([0., 1.03, 0.], [0.48, 0.70, 0.30], cloth);
    part([0., 1.53, 0.], [0.29, 0.32, 0.29], skin);
    part([0., 1.68, 0.], [0.31, 0.10, 0.30], [0.18, 0.13, 0.10]);
    // Cosmetic gait from logical time; standing and blocked residents stay still.
    let walking = person.blocked.is_none() && !person.route().is_empty();
    let stride = if walking {
        (sim.time as f32 * 9. + person.id.0 as f32).sin() * 0.13
    } else {
        0.
    };
    for side in [-1., 1.] {
        part(
            [side * 0.13, 0.34, side * stride],
            [0.16, 0.68, 0.19],
            [0.31, 0.24, 0.18],
        );
        part(
            [side * 0.32, 1.02, -side * stride],
            [0.14, 0.58, 0.17],
            skin,
        );
    }
    let target = person
        .route()
        .first()
        .copied()
        .unwrap_or(person.destination());
    let angle = (target.x - position.x).atan2(target.z - position.z);
    let (s, c) = angle.sin_cos();
    for v in &mut out[start..] {
        let [x, y, z] = v.position;
        v.position = [base.x + c * x + s * z, base.y + y, base.z - s * x + c * z];
    }
}
