//! Exercise the generated valley's real drilling, collection and save boundaries.
use crate::{Result, input::Input, interaction::Object, session::Session, terrain, view::View};
use genesis_renderer::camera::Camera;
use genesis_sim_core::{actions::Action, spatial::Position, surface::walking, units::metres};
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
            let home = session
                .sim
                .buildings
                .first()
                .ok_or("water check needs a settlement")?
                .position;
            let mut site = None;
            'search: for radius in [8., 12., 16., 20., 24.] {
                for i in 0..32 {
                    let a = i as f32 * std::f32::consts::TAU / 32.;
                    let p = Position::new(
                        home.x + a.cos() * metres(radius),
                        home.z + a.sin() * metres(radius),
                    );
                    if walking::standable(&session.sim, p)
                        && session
                            .sim
                            .subsurface
                            .validate_well(
                                &session.sim.grid,
                                session.sim.sea_level,
                                &session.sim.buildings,
                                p,
                            )
                            .is_ok()
                    {
                        site = Some(p);
                        break 'search;
                    }
                }
            }
            session.queue(Action::DrillWell {
                position: site.ok_or("no valid nearby well site")?,
            });
            for _ in 0..600 {
                session.step_once();
                if session
                    .sim
                    .people
                    .all()
                    .iter()
                    .any(|p| p.carried_water > 0.)
                {
                    break;
                }
            }
            let person = session
                .sim
                .people
                .all()
                .iter()
                .find(|p| p.carried_water > 0.)
                .ok_or("no resident collected water")?;
            let id = person.id;
            let position = person.position;
            session.select_object(Some(Object::Person(id)), position);
            camera.center(terrain::surface_point(&session.sim, position));
            camera.set_visible_height(metres(16.));
        }
        5 | 24 => {
            let Some(Object::Person(id)) = session.selected_object else {
                return Err("no water carrier selected".into());
            };
            let person = session
                .sim
                .people
                .get(id)
                .ok_or("water carrier disappeared")?;
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
            if view.pick_object(session, camera, cursor, ground) != Some(Object::Person(id)) {
                return Err("water carrier picking failed".into());
            }
        }
        12 => {
            let Some(Object::Person(id)) = session.selected_object else {
                return Err("no water carrier selected".into());
            };
            let person = session.sim.people.get(id).ok_or("carrier missing")?;
            let home = person.home;
            let well = person
                .water_task
                .as_ref()
                .ok_or("collection task missing")?
                .well;
            let before = serde_json::to_vec(&*session.sim)?;
            session.queue(Action::SetWellActive {
                id: well,
                active: false,
            });
            let path = capture.with_extension("save.json");
            session.save_to(&path)?;
            for _ in 0..600 {
                session.step_once();
            }
            if session
                .sim
                .households
                .get(home)
                .is_none_or(|h| h.stored + h.consumed < 2.)
            {
                return Err("water was not delivered to its home".into());
            }
            let expected = serde_json::to_vec(&*session.sim)?;
            session.load_from(&path)?;
            if serde_json::to_vec(&*session.sim)? != before {
                return Err("cargo load changed the saved state".into());
            }
            for _ in 0..600 {
                session.step_once();
            }
            if serde_json::to_vec(&*session.sim)? != expected {
                return Err("cargo save continuation diverged".into());
            }
            session.load_from(&path)?;
            session.select_object(Some(Object::Person(id)), session.position);
            session.message = "Water collected · delivery and save continuity verified".into();
        }
        _ => {}
    }
    Ok(())
}
