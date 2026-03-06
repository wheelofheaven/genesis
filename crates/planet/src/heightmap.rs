use bevy::prelude::Vec3;
use noise::{NoiseFn, Simplex};

/// Fixed seed for deterministic terrain generation.
const SEED: u32 = 42;

/// Sample a seamless heightmap at a point on the unit sphere.
///
/// Uses 3D simplex noise evaluated at the sphere surface, so there are no
/// seams at face boundaries. Two octaves blended 0.7/0.3 for natural detail.
pub fn sample_heightmap(point: Vec3, frequency: f32, amplitude: f32) -> f32 {
    let simplex = Simplex::new(SEED);

    let p = [
        (point.x * frequency) as f64,
        (point.y * frequency) as f64,
        (point.z * frequency) as f64,
    ];

    // Octave 1: base terrain shape.
    let octave1 = simplex.get(p);

    // Octave 2: finer detail at double frequency, 30% weight.
    let p2 = [p[0] * 2.0, p[1] * 2.0, p[2] * 2.0];
    let octave2 = simplex.get(p2);

    let combined = octave1 * 0.7 + octave2 * 0.3;
    combined as f32 * amplitude
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heightmap_bounded() {
        let amplitude = 5.0;
        // Sample many points on the unit sphere.
        for i in 0..1000 {
            let theta = (i as f32 / 1000.0) * std::f32::consts::TAU;
            let phi = (i as f32 / 1000.0) * std::f32::consts::PI;
            let point = Vec3::new(
                phi.sin() * theta.cos(),
                phi.sin() * theta.sin(),
                phi.cos(),
            );
            let h = sample_heightmap(point, 3.0, amplitude);
            assert!(
                h.abs() <= amplitude * 1.1,
                "height {h} exceeds expected range for amplitude {amplitude}"
            );
        }
    }

    #[test]
    fn test_heightmap_deterministic() {
        let point = Vec3::new(0.5, 0.3, 0.8).normalize();
        let h1 = sample_heightmap(point, 3.0, 2.0);
        let h2 = sample_heightmap(point, 3.0, 2.0);
        assert_eq!(h1, h2, "heightmap must be deterministic");
    }
}
