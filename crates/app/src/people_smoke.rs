//! Native checks for the actual generated settlement, selection and save continuity.
use crate::{Result, input::Input, interaction::Object, session::Session, terrain, view::View};
use genesis_renderer::camera::Camera;
use genesis_sim_core::units::metres;
use glam::{Vec2, Vec3};

pub fn advance(
    frame: u32,
    session: &mut Session,
    camera: &mut Camera,
    view: &mut View,
    input: &mut Input,
    capture: &std::path::Path,
) -> Result<()> {
    match frame {
        1 => {
            session.clock.paused = true;
            if session.sim.people.all().len() != 6 {
                return Err("expected six generated residents".into());
            }
            for _ in 0..35 {
                session.step_once();
            }
            let p = &session.sim.people.all()[0];
            camera.center(terrain::surface_point(&session.sim, p.position));
            camera.set_visible_height(metres(16.));
        }
        5 | 24 => {
            let person = &session.sim.people.all()[0];
            let expected = Object::Person(person.id);
            let target =
                terrain::surface_point(&session.sim, person.position) + Vec3::Y * metres(1.1);
            camera.center(target);
            let size = view.window.inner_size();
            let ndc = camera
                .view_projection(size.width as f32 / size.height as f32)
                .project_point3(target);
            let cursor = Vec2::new(
                (ndc.x + 1.) * 0.5 * size.width as f32,
                (1. - ndc.y) * 0.5 * size.height as f32,
            );
            input.cursor = Some(cursor);
            view.test_pointer(cursor);
            let ground = view.pick(session, camera, cursor);
            let hit = view.pick_object(session, camera, cursor, ground);
            if hit != Some(expected) {
                return Err(format!("resident picking: {hit:?}, expected {expected:?}").into());
            }
            session.select_object(hit, session.position);
        }
        12 => {
            let path = capture.with_extension("save.json");
            session.save_to(&path)?;
            let expected = session.sim.people.clone();
            for _ in 0..20 {
                session.step_once();
            }
            if expected == session.sim.people {
                return Err("residents did not advance".into());
            }
            session.load_from(&path)?;
            if expected != session.sim.people {
                return Err("resident restore changed state".into());
            }
            let p = &session.sim.people.all()[0];
            session.select_object(Some(Object::Person(p.id)), p.position);
        }
        _ => {}
    }
    Ok(())
}
