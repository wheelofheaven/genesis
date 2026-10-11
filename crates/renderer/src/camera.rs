//! Orbit camera math, independent of windowing and simulation.
use glam::{Mat4, Vec2, Vec3};

#[derive(Debug, Clone)]
pub struct Camera {
    pub focus: Vec3,
    yaw: f32,
    pitch: f32,
    view_height: f32,
    extent: f32,
    ground_view: Option<(Vec3, f32, f32, f32)>,
    eye_height: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            focus: Vec3::ZERO,
            yaw: 0.7,
            pitch: 0.8,
            view_height: 36.0,
            extent: 40.,
            ground_view: None,
            eye_height: 0.,
        }
    }
}

impl Camera {
    pub fn is_ground(&self) -> bool {
        self.ground_view.is_some()
    }
    pub fn enter_ground(&mut self, foot: Vec3, eye_height: f32) {
        if self.is_ground() {
            return;
        }
        self.ground_view = Some((self.focus, self.yaw, self.pitch, self.view_height));
        self.focus = foot;
        self.eye_height = eye_height.max(0.001);
        self.pitch = 0.04;
    }
    pub fn leave_ground(&mut self) {
        if let Some((focus, yaw, pitch, height)) = self.ground_view.take() {
            self.focus = focus;
            self.yaw = yaw;
            self.pitch = pitch;
            self.view_height = height;
        }
    }
    /// Eye height is supplied in world units by the application, independent
    /// of the environmental grid. Ground mode's focus is the observer's feet.
    pub fn eye(&self) -> Vec3 {
        if self.is_ground() {
            self.focus + Vec3::Y * self.eye_height
        } else {
            self.focus + self.direction() * self.distance()
        }
    }
    fn field_of_view(&self) -> f32 {
        if self.is_ground() {
            60_f32.to_radians()
        } else {
            Self::FIELD_OF_VIEW
        }
    }
    pub fn view_projection(&self, aspect: f32) -> Mat4 {
        let distance = self.distance();
        // Reversed floating-point depth retains thin surface details across
        // the range from individual doorways to the distant regional horizon.
        Mat4::perspective_infinite_reverse_rh(
            self.field_of_view(),
            aspect,
            if self.is_ground() {
                0.0025
            } else {
                (distance * 0.001).max(0.005)
            },
        ) * Mat4::look_at_rh(self.eye(), self.eye() - self.direction(), Vec3::Y)
    }
    const FIELD_OF_VIEW: f32 = std::f32::consts::FRAC_PI_4;
    fn distance(&self) -> f32 {
        self.view_height / (2. * (Self::FIELD_OF_VIEW * 0.5).tan())
    }

    pub fn direction(&self) -> Vec3 {
        Vec3::new(
            self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.cos() * self.pitch.cos(),
        )
    }

    pub fn frame_region(&mut self, extent: f32) {
        self.extent = extent;
        self.view_height = extent * 1.5;
    }
    pub fn set_visible_height(&mut self, height: f32) {
        self.view_height = height.clamp(1., self.extent * 2.5);
    }
    pub fn heading(&self) -> f32 {
        self.yaw
    }
    pub fn look_at_ground(&mut self, target: Vec3) {
        if self.is_ground() {
            let backward = self.focus - target;
            self.yaw = backward.x.atan2(backward.z);
        }
    }
    /// Vertical span of the focal plane, in world units.
    pub fn visible_height(&self) -> f32 {
        self.view_height
    }
    pub fn face_north(&mut self) {
        self.yaw = 0.;
    }

    pub fn orbit(&mut self, delta: Vec2) {
        self.yaw = (self.yaw - delta.x * 0.006).rem_euclid(std::f32::consts::TAU);
        self.pitch =
            (self.pitch + delta.y * 0.006).clamp(if self.is_ground() { -1.3 } else { 0.2 }, 1.5);
    }

    pub fn zoom(&mut self, lines: f32) {
        if self.is_ground() {
            return;
        }
        self.view_height = (self.view_height * (-lines.clamp(-20.0, 20.0) * 0.12).exp())
            .clamp(1.0, self.extent * 2.5);
    }

    /// Zoom about the pointer at the picked surface height, without input lag.
    pub fn zoom_at(&mut self, lines: f32, cursor: Vec2, size: Vec2, height: f32) {
        if self.is_ground() {
            return;
        }
        let before = self.ray(cursor, size).and_then(|(o, d)| {
            if d.y.abs() <= 1e-6 {
                return None;
            }
            let t = (height - o.y) / d.y;
            (t >= 0.).then(|| o + d * t)
        });
        let distance = self.distance();
        self.zoom(lines);
        if let Some(point) = before {
            // Move toward the picked point in all three dimensions. Retaining
            // the old focus altitude would send the eye below elevated ground.
            self.focus = point + (self.focus - point) * (self.distance() / distance);
            self.clamp_focus();
        }
    }
    pub fn center(&mut self, point: Vec3) {
        self.focus = point;
        self.clamp_focus();
    }
    fn clamp_focus(&mut self) {
        self.focus = self.focus.clamp(
            Vec3::new(-self.extent, -600., -self.extent),
            Vec3::new(self.extent, 450., self.extent),
        );
    }

    /// Ground-plane pan, in screen pixels. Bounds follow the current region.
    pub fn pan(&mut self, delta: Vec2, viewport_height: f32) {
        if self.is_ground() {
            return;
        }
        let right = Vec3::new(self.yaw.cos(), 0.0, -self.yaw.sin());
        let forward = Vec3::new(-self.yaw.sin(), 0.0, -self.yaw.cos());
        let scale = self.view_height / viewport_height.max(1.0);
        self.focus += (-right * delta.x + forward * delta.y / self.pitch.sin()) * scale;
        self.clamp_focus();
    }

    pub fn ray(&self, cursor: Vec2, size: Vec2) -> Option<(Vec3, Vec3)> {
        if size.x <= 0.0 || size.y <= 0.0 {
            return None;
        }
        let ndc = Vec2::new(cursor.x / size.x * 2.0 - 1.0, 1.0 - cursor.y / size.y * 2.0);
        // Analytic rays avoid cancellation when unprojecting a distant far
        // plane while inspecting a house far from the world origin.
        let forward = -self.direction();
        let right = forward.cross(Vec3::Y).normalize();
        let up = right.cross(forward);
        let tangent = (self.field_of_view() * 0.5).tan();
        let direction =
            (forward + right * ndc.x * (size.x / size.y) * tangent + up * ndc.y * tangent)
                .normalize();
        Some((self.eye(), direction))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ground_view_uses_eye_height_and_restores_orbit_exactly() {
        let mut camera = Camera::default();
        let before = camera.view_projection(1.5);
        let foot = Vec3::new(8., 5., 6.);
        camera.enter_ground(foot, 1.7 / 20.);
        assert!((camera.eye().y - foot.y - 1.7 / 20.).abs() < 1e-6);
        let (eye, ray) = camera
            .ray(Vec2::new(600., 400.), Vec2::new(1200., 800.))
            .unwrap();
        assert_eq!(eye, camera.eye());
        assert!((ray + camera.direction()).length() < 1e-5);
        camera.orbit(Vec2::new(20., -100.));
        camera.leave_ground();
        assert_eq!(before, camera.view_projection(1.5));
    }
    #[test]
    fn zoom_toward_elevated_ground_preserves_clearance_and_pointer_anchor() {
        let mut camera = Camera::default();
        camera.frame_region(192.);
        let cursor = Vec2::new(700., 360.);
        let size = Vec2::new(1280., 800.);
        let height = 80.;
        let (o, d) = camera.ray(cursor, size).unwrap();
        let target = o + d * ((height - o.y) / d.y);
        for _ in 0..5 {
            camera.zoom_at(5., cursor, size, height);
            let (o, d) = camera.ray(cursor, size).unwrap();
            assert!(o.y > height);
            let hit = o + d * ((height - o.y) / d.y);
            assert!(hit.distance(target) < 0.001);
        }
    }
    #[test]
    fn distant_objects_shrink_and_focus_scale_matches_navigation() {
        let camera = Camera::default();
        let matrix = camera.view_projection(1.);
        let right = (-camera.direction()).cross(Vec3::Y).normalize();
        let span = |center| {
            matrix
                .project_point3(center + right * 0.5)
                .distance(matrix.project_point3(center - right * 0.5))
        };
        let near = span(camera.focus);
        let distant = span(camera.focus - camera.direction() * camera.distance());
        assert!((near - 2. / camera.visible_height()).abs() < 1e-5);
        assert!((distant / near - 0.5).abs() < 1e-5);
    }
    #[test]
    fn zoom_keeps_point_under_pointer_and_pan_scales_with_viewport() {
        let mut c = Camera::default();
        let cursor = Vec2::new(710., 390.);
        let size = Vec2::new(1280., 800.);
        let ground = |c: &Camera| {
            let (o, d) = c.ray(cursor, size).unwrap();
            o + d * ((1.7 - o.y) / d.y)
        };
        let before = ground(&c);
        c.zoom_at(3., cursor, size, 1.7);
        assert!(ground(&c).distance(before) < 0.001);
        let mut a = Camera::default();
        let mut b = a.clone();
        a.pan(Vec2::new(50., -20.), 800.);
        b.pan(Vec2::new(100., -40.), 1600.);
        assert!(a.focus.distance(b.focus) < 1e-5);
    }
    #[test]
    fn enlarged_region_supports_distant_focus_and_house_scale_zoom() {
        let mut camera = Camera::default();
        camera.frame_region(192.);
        camera.center(Vec3::new(150., 30., -120.));
        assert_eq!(camera.focus, Vec3::new(150., 30., -120.));
        camera.set_visible_height(4.5);
        let (origin, direction) = camera
            .ray(Vec2::new(640., 400.), Vec2::new(1280., 800.))
            .unwrap();
        let hit = origin + direction * ((camera.focus.y - origin.y) / direction.y);
        assert!(hit.distance(camera.focus) < 0.001);
        camera.zoom(-20.);
        camera.zoom(-20.);
        assert_eq!(camera.visible_height(), 480.);
    }
    #[test]
    fn extreme_navigation_remains_finite_and_center_ray_targets_focus() {
        let mut camera = Camera::default();
        for _ in 0..100 {
            camera.orbit(Vec2::new(5000., -5000.));
            camera.zoom(100.);
            camera.pan(Vec2::splat(10000.), 0.);
        }
        let matrix = camera.view_projection(1.5);
        assert!(matrix.is_finite());
        let (origin, dir) = camera
            .ray(Vec2::new(480., 320.), Vec2::new(960., 640.))
            .unwrap();
        let ground = origin + dir * (-origin.y / dir.y);
        assert!(ground.distance(camera.focus) < 0.01);
        assert!(camera.ray(Vec2::ZERO, Vec2::ZERO).is_none());
    }
}
