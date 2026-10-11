//! Native regression: roof picking, path preview, queued construction and selection.
use crate::{Result, input::Input, interaction::Object, session::Session, terrain, view::View};
use genesis_renderer::camera::Camera;
use genesis_sim_core::{paths, spatial::Position, units::metres};
use glam::{Vec2, Vec3};
use winit::keyboard::KeyCode;

fn pointer(view: &mut View, camera: &Camera, p: Vec3) -> Result<Vec2> {
    let size = view.window.inner_size();
    let ndc = camera
        .view_projection(size.width as f32 / size.height as f32)
        .project_point3(p);
    let cursor = Vec2::new(
        (ndc.x + 1.) * 0.5 * size.width as f32,
        (1. - ndc.y) * 0.5 * size.height as f32,
    );
    view.window
        .set_cursor_position(winit::dpi::PhysicalPosition::new(
            cursor.x as f64,
            cursor.y as f64,
        ))?;
    view.test_pointer(cursor);
    Ok(cursor)
}
pub fn advance(
    frame: u32,
    session: &mut Session,
    camera: &mut Camera,
    view: &mut View,
    input: &mut Input,
) -> Result<()> {
    match frame {
        1 => {
            view.window.focus_window();
            session.clock.paused = true;
            let b = &session.sim.buildings[0];
            camera.center(terrain::point(&session.sim, b.position));
            camera.set_visible_height(2.4);
        }
        3 => {
            let b = &session.sim.buildings[0];
            let expected = Object::Building(b.id);
            let roof = terrain::point(&session.sim, b.position) + Vec3::Y * metres(3.0);
            let cursor = pointer(view, camera, roof)?;
            input.cursor = Some(cursor);
            let ground = view.pick(session, camera, cursor);
            let hit = view.pick_object(session, camera, cursor, ground);
            if hit != Some(expected) {
                return Err(format!("roof picking failed: {hit:?}").into());
            }
            session.select_object(hit, session.position);
        }
        10 => {
            let mut route = None;
            'search: for path in &session.sim.paths {
                for &p in &path.points {
                    for i in 0..16 {
                        let a = i as f32 * std::f32::consts::TAU / 16.;
                        let q =
                            Position::new(p.x + a.cos() * metres(15.), p.z + a.sin() * metres(15.));
                        let r = Position::new(
                            q.x + (a + 0.5).cos() * metres(12.),
                            q.z + (a + 0.5).sin() * metres(12.),
                        );
                        let points = vec![p, q, r];
                        if paths::validate(&session.sim, &points, session.path.width).is_ok() {
                            route = Some(points);
                            break 'search;
                        }
                    }
                }
            }
            let points = route.ok_or("no valid footpath extension near the settlement")?;
            session.set_path_tool(true);
            for &p in &points {
                session.set_hover(Some(p));
                session.add_path_point();
            }
            let p = *session
                .path
                .points
                .last()
                .ok_or("draft missing waypoints")?;
            camera.center(terrain::point(&session.sim, session.path.points[1]));
            camera.set_visible_height(2.6);
            input.cursor = Some(pointer(view, camera, terrain::point(&session.sim, p))?);
        }
        20 => {
            let count = session.sim.paths.len();
            input.key(KeyCode::Enter, true, session, camera);
            input.key(KeyCode::Enter, false, session, camera);
            if session.pending_count() != 1 {
                return Err("Enter did not queue the path".into());
            }
            session.step_once();
            if session.sim.paths.len() != count + 1 {
                return Err(format!("path failed: {}", session.message).into());
            }
            session.set_path_tool(false);
            session.select_object(Some(Object::Path(count)), session.position);
            let p = session.position;
            input.cursor = Some(pointer(view, camera, terrain::point(&session.sim, p))?);
        }
        24 => {
            // Test the current projected target, independently of delayed OS warp
            // events or the user moving their pointer during this native check.
            let cursor = pointer(view, camera, terrain::point(&session.sim, session.position))?;
            input.cursor = Some(cursor);
            let ground = view.pick(session, camera, cursor);
            let hit = view.pick_object(session, camera, cursor, ground);
            if hit != session.selected_object {
                return Err(format!("path picking failed: {hit:?}, cursor {cursor:?}, ground {ground:?}, expected {:?} at {:?}",session.selected_object,session.position).into());
            }
        }
        _ => {}
    }
    Ok(())
}
