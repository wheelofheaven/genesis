//! Orbit camera math, independent of windowing and simulation.
use glam::{Mat4, Vec2, Vec3};

#[derive(Debug, Clone)]
pub struct Camera {
    pub focus: Vec3,
    yaw: f32,
    pitch: f32,
    view_height: f32,
    extent: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            focus: Vec3::ZERO,
            yaw: 0.7,
            pitch: 0.8,
            view_height: 36.0,
            extent: 40.,
        }
    }
}

impl Camera {
    pub fn view_projection(&self, aspect: f32) -> Mat4 {
        let half = self.view_height * 0.5;
        let direction = self.direction();
        Mat4::orthographic_rh(-half * aspect, half * aspect, -half, half, 0.1, 2400.0)
            * Mat4::look_at_rh(self.focus + direction * 1000.0, self.focus, Vec3::Y)
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
    pub fn visible_height(&self) -> f32 {
        self.view_height
    }
    pub fn face_north(&mut self) {
        self.yaw = 0.;
    }

    pub fn orbit(&mut self, delta: Vec2) {
        self.yaw = (self.yaw - delta.x * 0.006).rem_euclid(std::f32::consts::TAU);
        self.pitch = (self.pitch + delta.y * 0.006).clamp(0.2, 1.5);
    }

    pub fn zoom(&mut self, lines: f32) {
        self.view_height = (self.view_height * (-lines.clamp(-20.0, 20.0) * 0.12).exp())
            .clamp(1.0, self.extent * 2.5);
    }

    /// Zoom about the pointer at the picked surface height, without input lag.
    pub fn zoom_at(&mut self, lines: f32, cursor: Vec2, size: Vec2, height: f32) {
        let hit = |camera: &Self| {
            camera
                .ray(cursor, size)
                .and_then(|(o, d)| (d.y.abs() > 1e-6).then(|| o + d * ((height - o.y) / d.y)))
        };
        let before = hit(self);
        self.zoom(lines);
        if let (Some(a), Some(b)) = (before, hit(self)) {
            self.focus += a - b;
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
        let inverse = self.view_projection(size.x / size.y).inverse();
        let near = inverse.project_point3(ndc.extend(0.0));
        let far = inverse.project_point3(ndc.extend(1.0));
        Some((near, (far - near).normalize()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
