//! Stateless, version-stable randomness and smooth multi-scale value noise.
pub fn hash(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e3779b97f4a7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
    value ^ (value >> 31)
}
pub fn unit(seed: u64, index: u64) -> f32 {
    (hash(seed ^ index.wrapping_mul(0x9e3779b97f4a7c15)) >> 40) as f32 / 16_777_216.
}
pub fn noise(seed: u64, x: f32, z: f32) -> f32 {
    let ix = x.floor() as i32;
    let iz = z.floor() as i32;
    let smooth = |t: f32| t * t * t * (t * (t * 6. - 15.) + 10.);
    let tx = smooth(x.fract().rem_euclid(1.));
    let tz = smooth(z.fract().rem_euclid(1.));
    let at = |x: i32, z: i32| unit(seed, (x as u32 as u64) << 32 | z as u32 as u64);
    let a = at(ix, iz) * (1. - tx) + at(ix + 1, iz) * tx;
    let b = at(ix, iz + 1) * (1. - tx) + at(ix + 1, iz + 1) * tx;
    (a * (1. - tz) + b * tz) * 2. - 1.
}
pub fn fbm(seed: u64, x: f32, z: f32) -> f32 {
    let mut total = 0.;
    let mut scale = 1.;
    let mut amplitude = 0.55;
    for octave in 0..4 {
        total += noise(seed.wrapping_add(octave * 31), x * scale, z * scale) * amplitude;
        scale *= 2.03;
        amplitude *= 0.48;
    }
    total
}
