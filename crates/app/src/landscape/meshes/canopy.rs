//! Static optical-depth proxy used while preparing shared foliage meshes.
use super::{Vec3, foliage::Crown};

const RAYS: usize = 12;
const REACH: f32 = 8.;

fn thickness(crown: &Crown, point: Vec3, direction: Vec3) -> f32 {
    let p = (point - crown.center) / crown.extent;
    let d = direction / crown.extent;
    let a = d.length_squared();
    let b = p.dot(d);
    let discriminant = b * b - a * (p.length_squared() - 1.);
    if discriminant <= 0. {
        return 0.;
    }
    let radius = discriminant.sqrt();
    let enter = ((-b - radius) / a).max(0.);
    let exit = ((-b + radius) / a).min(REACH);
    (exit - enter).max(0.)
}

/// Cosine-weighted upper hemisphere, shared by all detail levels. Overlapping
/// leaf clusters add density; this approximates sky access, not individual leaves.
pub(super) fn openness(point: Vec3, crowns: &[(Crown, u64)]) -> f32 {
    let mut sum = 0.;
    for i in 0..RAYS {
        let radius = ((i as f32 + 0.5) / RAYS as f32).sqrt();
        let angle = i as f32 * 2.399_963_1;
        let direction = Vec3::new(
            radius * angle.cos(),
            (1. - radius * radius).sqrt(),
            radius * angle.sin(),
        );
        let depth: f32 = crowns
            .iter()
            .map(|(crown, _)| thickness(crown, point, direction))
            .sum();
        sum += (-depth * 0.45).exp();
    }
    (sum / RAYS as f32).clamp(0., 1.)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sky_access_decreases_beneath_and_inside_crowns() {
        let crowns = vec![(
            Crown {
                center: Vec3::ZERO,
                extent: Vec3::ONE,
                color: Vec3::ONE,
                spray_size: 0.1,
                counts: [12, 6, 3],
            },
            1,
        )];
        assert_eq!(openness(Vec3::ZERO, &[]), 1.);
        assert_eq!(openness(Vec3::Y * 1.01, &crowns), 1.);
        let top = openness(Vec3::Y * 0.8, &crowns);
        let core = openness(Vec3::ZERO, &crowns);
        let bottom = openness(-Vec3::Y * 0.8, &crowns);
        assert!(top > core && core > bottom);
        assert_eq!(thickness(&crowns[0].0, Vec3::Y * 2., Vec3::Y), 0.);
        assert!((thickness(&crowns[0].0, Vec3::Y * 2., -Vec3::Y) - 2.).abs() < 1e-5);
        assert_eq!(thickness(&crowns[0].0, Vec3::Y * 10., -Vec3::Y), 0.);
        for y in -20..20 {
            let value = openness(Vec3::new(0.3, y as f32 * 0.1, -0.2), &crowns);
            assert!(value.is_finite() && (0. ..=1.).contains(&value));
        }
    }
}
