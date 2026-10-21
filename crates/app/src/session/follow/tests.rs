use super::*;
use genesis_sim_core::{scenarios, spatial::Position};
use std::time::Duration;

#[test]
fn tracking_matches_rendered_motion_pause_and_step_without_changing_the_world() {
    let mut session = Session::from_sim(scenarios::supplied_settlement());
    let mut reference = Session::from_sim(scenarios::supplied_settlement());
    let mut camera = Camera::default();
    let id = session.sim.people.all()[0].id;
    session.follow_person(id, &mut camera);
    camera.orbit(glam::Vec2::new(20., 10.));
    camera.zoom(2.);
    let direction = camera.direction();
    let height = camera.visible_height();
    let start = camera.focus;
    for _ in 0..250 {
        session.advance(Duration::from_millis(25));
        reference.advance(Duration::from_millis(25));
        session.update_follow(&mut camera);
        let p = session.sim.people.get(id).unwrap();
        let target = crate::terrain::surface_point(&session.sim, session.person_position(p));
        assert!(camera.focus.distance(target) < 1e-6);
        assert_eq!(camera.direction(), direction);
        assert_eq!(camera.visible_height(), height);
    }
    assert!(start.distance(camera.focus) > 0.01);
    session.clock.paused = true;
    reference.clock.paused = true;
    session.advance(Duration::from_secs(3));
    session.update_follow(&mut camera);
    let paused = camera.focus;
    session.advance(Duration::from_secs(3));
    session.update_follow(&mut camera);
    assert_eq!(paused, camera.focus);
    session.step_once();
    reference.step_once();
    session.update_follow(&mut camera);
    let p = session.sim.people.get(id).unwrap();
    assert_eq!(
        camera.focus,
        crate::terrain::surface_point(&session.sim, p.position)
    );
    assert_eq!(
        serde_json::to_vec(&*session.sim).unwrap(),
        serde_json::to_vec(&*reference.sim).unwrap()
    );
    assert_eq!(session.pending_count(), 0);
}

#[test]
fn tracking_releases_for_navigation_selection_tools_and_world_replacement() {
    use crate::input::Input;
    use winit::keyboard::KeyCode;
    let mut session = Session::from_sim(scenarios::supplied_settlement());
    let mut camera = Camera::default();
    let id = session.sim.people.all()[0].id;
    for key in [
        KeyCode::KeyF,
        KeyCode::KeyR,
        KeyCode::KeyG,
        KeyCode::Escape,
        KeyCode::KeyB,
        KeyCode::KeyP,
        KeyCode::KeyV,
    ] {
        session.follow_person(id, &mut camera);
        Input::default().key(key, true, &mut session, &mut camera);
        assert!(session.following().is_none(), "{key:?}");
    }
    session.follow_person(id, &mut camera);
    let mut input = Input::default();
    input.key(KeyCode::KeyW, true, &mut session, &mut camera);
    input.update_camera(&mut session, &mut camera, 0.1, 800.);
    assert!(session.following().is_none());
    session.follow_person(id, &mut camera);
    session.select_object(Some(Object::Building(1)), session.position);
    assert!(session.following().is_none());
    session.follow_person(id, &mut camera);
    session.set_water_overview(true);
    assert!(session.following().is_none());
    session.follow_person(id, &mut camera);
    session.selected_object = None;
    session.update_follow(&mut camera);
    assert!(session.following().is_none());
    session.follow_person(id, &mut camera);
    session.replace_sim(scenarios::supplied_settlement());
    session.update_follow(&mut camera);
    assert!(session.following().is_none()); // Reused numeric ID belongs to a new world.
    session.select(Position::new(1., 1.));
    session.follow_person(PersonId(u64::MAX), &mut camera);
    assert!(session.following().is_none());
    assert!(session.selected_object.is_none());
}

#[test]
fn following_is_transient_and_failed_load_preserves_the_live_camera_target() {
    let mut session = Session::from_sim(scenarios::supplied_settlement());
    let mut camera = Camera::default();
    let id = session.sim.people.all()[0].id;
    let path = std::env::temp_dir().join(format!(
        "genesis-follow-{}.json",
        genesis_storage::new_world_id()
    ));
    session.follow_person(id, &mut camera);
    session.save_to(&path).unwrap();
    let expected = serde_json::to_vec(&*session.sim).unwrap();
    assert!(session.load_from(&path.with_extension("absent")).is_err());
    assert_eq!(session.following(), Some(id));
    session.load_from(&path).unwrap();
    assert!(session.following().is_none());
    assert!(session.selected_object.is_none());
    assert_eq!(expected, serde_json::to_vec(&*session.sim).unwrap());
    std::fs::remove_file(path).unwrap();
}
