//! Human-eye-height inspection; this is not a simulated inhabitant or game mode.
use genesis_renderer::camera::Camera;
use genesis_sim_core::{SimState, spatial::Position, surface, units::metres};
use glam::{Vec2, Vec3};

pub fn toggle(sim: &SimState, selected: Position, camera: &mut Camera) {
    if camera.is_ground() {
        camera.leave_ground();
        return;
    }
    if surface::walking::standable(sim, selected) {
        camera.enter_ground(crate::terrain::point(sim, selected), metres(1.7));
        return;
    }
    let facing = sim
        .buildings
        .iter()
        .min_by(|a, b| {
            let distance = |p: Position| (selected.x - p.x).powi(2) + (selected.z - p.z).powi(2);
            distance(a.position).total_cmp(&distance(b.position))
        })
        .map_or(0., |b| b.rotation + std::f32::consts::FRAC_PI_2 + 0.35);
    for i in 0..512 {
        let angle = facing + i as f32 * 2.399963;
        let radius = metres(12.) + metres(0.6) * (i as f32).sqrt();
        let p = Position::new(
            selected.x + angle.cos() * radius,
            selected.z + angle.sin() * radius,
        );
        if surface::walking::standable(sim, p) {
            camera.enter_ground(crate::terrain::point(sim, p), metres(1.7));
            camera.look_at_ground(crate::terrain::point(sim, selected));
            return;
        }
    }
}
pub fn travel(sim: &SimState, camera: &mut Camera, input: Vec2, seconds: f32, fast: bool) {
    let right = Vec3::new(camera.heading().cos(), 0., -camera.heading().sin());
    let forward = Vec3::new(-camera.heading().sin(), 0., -camera.heading().cos());
    let motion = (-right * input.x + forward * input.y)
        * seconds.min(0.1)
        * metres(if fast { 4. } else { 1.4 });
    let from = Position::new(
        camera.focus.x + sim.grid.width as f32 / 2.,
        camera.focus.z + sim.grid.height as f32 / 2.,
    );
    let to = surface::walking::advance(
        sim,
        from,
        Position::new(from.x + motion.x, from.z + motion.z),
    );
    camera.focus = crate::terrain::point(sim, to);
}
