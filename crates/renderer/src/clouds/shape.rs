//! Periodic cellular volume: rounded cloud lobes, built once, with filtered mips.
pub(super) const SIZE: u32 = 64;

fn hash(p: [i32; 3], salt: u32) -> f32 {
    let mut h = salt;
    for (v, k) in p.into_iter().zip([1597334677_u32, 3812015801, 958689151]) {
        h ^= (v.rem_euclid(4) as u32).wrapping_mul(k);
    }
    h = (h ^ (h >> 16)).wrapping_mul(2246822519);
    h = (h ^ (h >> 13)).wrapping_mul(3266489917);
    ((h ^ (h >> 16)) >> 8) as f32 / 16777216.
}

fn density(p: glam::Vec3) -> f32 {
    let p = p * 4.;
    let cell = p.floor().as_ivec3();
    let mut nearest = 4_f32;
    for z in -1..=1 {
        for y in -1..=1 {
            for x in -1..=1 {
                let c = cell + glam::IVec3::new(x, y, z);
                let a = c.to_array();
                let point = c.as_vec3() + glam::Vec3::new(hash(a, 17), hash(a, 739), hash(a, 2957));
                nearest = nearest.min(p.distance_squared(point));
            }
        }
    }
    (1. - nearest.sqrt()).clamp(0., 1.)
}

pub(super) fn levels() -> Vec<Vec<u8>> {
    let mut levels = vec![
        (0..SIZE.pow(3))
            .map(|i| {
                let p = glam::Vec3::new(
                    (i % SIZE) as f32 + 0.5,
                    ((i / SIZE) % SIZE) as f32 + 0.5,
                    (i / (SIZE * SIZE)) as f32 + 0.5,
                ) / SIZE as f32;
                (density(p) * 255.).round() as u8
            })
            .collect::<Vec<_>>(),
    ];
    let mut size = SIZE as usize;
    while size > 1 {
        let previous = &levels[levels.len() - 1];
        let half = size / 2;
        let mut next = Vec::with_capacity(half.pow(3));
        for z in 0..half {
            for y in 0..half {
                for x in 0..half {
                    let mut sum = 0_u32;
                    for dz in 0..2 {
                        for dy in 0..2 {
                            for dx in 0..2 {
                                sum += u32::from(
                                    previous
                                        [((z * 2 + dz) * size + y * 2 + dy) * size + x * 2 + dx],
                                );
                            }
                        }
                    }
                    next.push(((sum + 4) / 8) as u8);
                }
            }
        }
        levels.push(next);
        size = half;
    }
    levels
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn volume_is_repeatable_periodic_and_filtered() {
        for i in -30..30 {
            let p = glam::Vec3::new(i as f32 * 0.07, 0.13, -0.31);
            let d = density(p);
            assert!((0.0..=1.0).contains(&d));
            assert!((d - density(p + glam::Vec3::new(1., -1., 2.))).abs() < 1e-5);
        }
        let levels = levels();
        assert_eq!(levels, super::levels());
        assert_eq!(levels.iter().map(Vec::len).sum::<usize>(), 299593);
        let mean =
            |data: &[u8]| data.iter().map(|v| f64::from(*v)).sum::<f64>() / data.len() as f64;
        for (i, data) in levels.iter().enumerate() {
            assert_eq!(data.len(), ((SIZE >> i) as usize).pow(3));
            assert!((mean(data) - mean(&levels[0])).abs() < 2.);
        }
    }
}
