//! Packed periodic cloud structure: large lobes, billows, erosion, and body variation.
//! RGBA channels share one filtered texture fetch; generated once at startup.
pub(super) const SIZE: u32 = 64;

fn hash(p: [i32; 3], salt: u32, period: i32) -> f32 {
    let mut h = salt;
    for (v, k) in p.into_iter().zip([1597334677_u32, 3812015801, 958689151]) {
        h ^= (v.rem_euclid(period) as u32).wrapping_mul(k);
    }
    h = (h ^ (h >> 16)).wrapping_mul(2246822519);
    h = (h ^ (h >> 13)).wrapping_mul(3266489917);
    ((h ^ (h >> 16)) >> 8) as f32 / 16777216.
}

fn cellular(p: glam::Vec3, period: i32) -> f32 {
    let p = p * period as f32;
    let cell = p.floor().as_ivec3();
    let mut nearest = 4_f32;
    for z in -1..=1 {
        for y in -1..=1 {
            for x in -1..=1 {
                let c = cell + glam::IVec3::new(x, y, z);
                let a = c.to_array();
                let point = c.as_vec3()
                    + glam::Vec3::new(
                        hash(a, 17, period),
                        hash(a, 739, period),
                        hash(a, 2957, period),
                    );
                nearest = nearest.min(p.distance_squared(point));
            }
        }
    }
    (1. - nearest.sqrt()).clamp(0., 1.)
}

fn variation(p: glam::Vec3, period: i32) -> f32 {
    let p = p * period as f32;
    let cell = p.floor().as_ivec3();
    let f = p - p.floor();
    let w = f * f * f * (f * (f * 6. - glam::Vec3::splat(15.)) + glam::Vec3::splat(10.));
    let mut sum = 0.;
    for z in 0..2 {
        for y in 0..2 {
            for x in 0..2 {
                let weight = glam::Vec3::new(
                    if x == 0 { 1. - w.x } else { w.x },
                    if y == 0 { 1. - w.y } else { w.y },
                    if z == 0 { 1. - w.z } else { w.z },
                );
                sum += hash((cell + glam::IVec3::new(x, y, z)).to_array(), 991, period)
                    * weight.x
                    * weight.y
                    * weight.z;
            }
        }
    }
    sum
}

fn channels(p: glam::Vec3) -> [f32; 4] {
    [
        cellular(p, 4),
        cellular(p, 8),
        cellular(p, 16),
        variation(p, 4) * 0.7 + variation(p, 8) * 0.3,
    ]
}

pub(super) fn levels() -> Vec<Vec<u8>> {
    let mut levels = vec![
        (0..SIZE.pow(3))
            .flat_map(|i| {
                let p = glam::Vec3::new(
                    (i % SIZE) as f32 + 0.5,
                    ((i / SIZE) % SIZE) as f32 + 0.5,
                    (i / (SIZE * SIZE)) as f32 + 0.5,
                ) / SIZE as f32;
                channels(p).map(|v| (v * 255.).round() as u8)
            })
            .collect::<Vec<_>>(),
    ];
    let mut size = SIZE as usize;
    while size > 1 {
        let previous = &levels[levels.len() - 1];
        let half = size / 2;
        let mut next = Vec::with_capacity(half.pow(3) * 4);
        for z in 0..half {
            for y in 0..half {
                for x in 0..half {
                    for channel in 0..4 {
                        let mut sum = 0_u32;
                        for dz in 0..2 {
                            for dy in 0..2 {
                                for dx in 0..2 {
                                    let voxel =
                                        ((z * 2 + dz) * size + y * 2 + dy) * size + x * 2 + dx;
                                    sum += u32::from(previous[voxel * 4 + channel]);
                                }
                            }
                        }
                        next.push(((sum + 4) / 8) as u8);
                    }
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
            for (d, repeated) in channels(p)
                .into_iter()
                .zip(channels(p + glam::Vec3::new(1., -1., 2.)))
            {
                assert!((0.0..=1.0).contains(&d));
                assert!((d - repeated).abs() < 1e-5);
            }
        }
        let levels = levels();
        assert_eq!(levels, super::levels());
        assert_eq!(levels.iter().map(Vec::len).sum::<usize>(), 1198372);
        let mean = |data: &[u8], channel: usize| {
            data.chunks_exact(4)
                .map(|v| f64::from(v[channel]))
                .sum::<f64>()
                / (data.len() / 4) as f64
        };
        for (i, data) in levels.iter().enumerate() {
            assert_eq!(data.len(), ((SIZE >> i) as usize).pow(3) * 4);
            for channel in 0..4 {
                assert!((mean(data, channel) - mean(&levels[0], channel)).abs() < 2.);
            }
        }
        for channel in 0..4 {
            let min = levels[0].chunks_exact(4).map(|v| v[channel]).min().unwrap();
            let max = levels[0].chunks_exact(4).map(|v| v[channel]).max().unwrap();
            assert!(max - min > 100, "each channel must carry shape variation");
        }
    }
}
