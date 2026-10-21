//! Transient resident tracking, using the same positions as rendered residents.
use super::Session;
use crate::interaction::Object;
use genesis_renderer::camera::Camera;
use genesis_sim_core::people::PersonId;

impl Session {
    pub fn following(&self) -> Option<PersonId> {
        self.following
    }

    pub fn stop_following(&mut self) {
        self.following = None;
    }

    /// Inspect by persistent identity without changing time or resident intent.
    pub fn inspect_person(&mut self, id: PersonId, camera: &mut Camera) {
        if self.sim.people.get(id).is_none() {
            return;
        }
        self.set_placing(false);
        self.landscape_tools = false;
        self.world_tools = false;
        self.survey.open = false;
        self.select_object(Some(Object::Person(id)), self.position);
        camera.leave_ground();
        camera.center(self.selected_point());
    }

    pub fn follow_person(&mut self, id: PersonId, camera: &mut Camera) {
        if self.sim.people.get(id).is_none() {
            return;
        }
        self.inspect_person(id, camera);
        self.following = Some(id);
        self.update_follow(camera);
    }

    /// Call after clock/input updates. Orbit and scale remain under player control.
    pub fn update_follow(&mut self, camera: &mut Camera) {
        let Some(id) = self.following else { return };
        if camera.is_ground() || self.selected_object != Some(Object::Person(id)) {
            self.stop_following();
            return;
        }
        let Some(person) = self.sim.people.get(id) else {
            self.stop_following();
            return;
        };
        camera.center(crate::terrain::surface_point(
            &self.sim,
            self.person_position(person),
        ));
    }
}

#[cfg(test)]
mod tests;
