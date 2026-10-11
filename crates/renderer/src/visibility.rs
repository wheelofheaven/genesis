//! Conservative clip-volume tests, shared by camera and light visibility.
use glam::{Mat4, Vec3, Vec4};

pub struct Frustum {
    planes: [Vec4; 6],
}

impl Frustum {
    /// WebGPU clip space: -w <= x,y <= w and 0 <= z <= w.
    /// An infinite reverse projection has one degenerate (unbounded) plane.
    pub fn new(matrix: Mat4) -> Self {
        let r = matrix.transpose();
        let planes = [
            r.w_axis + r.x_axis,
            r.w_axis - r.x_axis,
            r.w_axis + r.y_axis,
            r.w_axis - r.y_axis,
            r.z_axis,
            r.w_axis - r.z_axis,
        ]
        .map(|p| {
            let length = p.truncate().length();
            if length > 1e-8 { p / length } else { Vec4::W }
        });
        Self { planes }
    }

    pub fn sphere(&self, center: Vec3, radius: f32) -> bool {
        self.planes
            .iter()
            .all(|p| p.truncate().dot(center) + p.w >= -radius)
    }

    pub fn bounds(&self, min: Vec3, max: Vec3) -> bool {
        let center = (min + max) * 0.5;
        let half = (max - min) * 0.5;
        self.planes
            .iter()
            .all(|p| p.truncate().dot(center) + p.w + p.truncate().abs().dot(half) >= 0.)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reverse_infinite_projection_keeps_horizon_and_intersecting_bounds() {
        let f = Frustum::new(Mat4::perspective_infinite_reverse_rh(1., 1., 0.1));
        assert!(f.sphere(Vec3::new(0., 0., -1_000_000.), 1.));
        assert!(!f.sphere(Vec3::new(0., 0., 5.), 1.));
        assert!(!f.sphere(Vec3::new(10., 0., -1.), 0.1));
        assert!(f.bounds(Vec3::new(-100., -1., -10.), Vec3::new(100., 1., 10.)));
    }
    #[test]
    fn orthographic_projection_tests_both_depth_limits() {
        let f = Frustum::new(Mat4::orthographic_rh(-8., 8., -8., 8., 0.1, 80.));
        assert!(f.sphere(Vec3::new(8.5, 0., -10.), 1.));
        assert!(!f.sphere(Vec3::new(10., 0., -10.), 1.));
        assert!(!f.sphere(Vec3::new(0., 0., -82.), 1.));
        assert!(!f.sphere(Vec3::new(0., 0., 2.), 1.));
    }
}
