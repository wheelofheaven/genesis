//! Regional uplift, a continental shelf, and nested ridge systems. Metres are
//! kept consistent with settlement scale; noise does not enlarge the houses.
use super::noise::{fbm, noise, unit};
use crate::grid::Grid;

pub fn build(grid: &mut Grid, seed: u64) {
    let (sin, cos) = (unit(seed, 1) * std::f32::consts::TAU).sin_cos();
    for z in 0..grid.height {
        for x in 0..grid.width {
            let px = (x as f32 / grid.width as f32 - 0.5) * 2.8;
            let pz = (z as f32 / grid.height as f32 - 0.5) * 2.8;
            let u = cos * px - sin * pz;
            let v = sin * px + cos * pz;
            let wx = u + fbm(seed ^ 91, u * 2.1 + 9., v * 2.1) * 0.25;
            let wz = v + fbm(seed ^ 173, u * 2.1, v * 2.1 + 8.) * 0.25;
            // Overlapping geological uplands yield peninsulas and saddles,
            // rather than one radial cone or a single extruded ridge.
            let lobe = |cx: f32, cz: f32, rx: f32, rz: f32| {
                1. - (((wx - cx) / rx).powi(2) + ((wz - cz) / rz).powi(2)).sqrt()
            };
            let continent = lobe(-0.27, 0.08, 0.87, 0.68)
                .max(lobe(0.36, -0.17, 0.70, 0.60))
                .max(lobe(0.08, 0.49, 0.40, 0.46));
            let bay = (-((wx - 0.65) / 0.32).powi(2) - ((wz - 0.32) / 0.27).powi(2)).exp();
            let coast = continent + fbm(seed ^ 301, wx * 3.8, wz * 3.8) * 0.30 - bay * 0.25;
            let inland = coast.max(0.);
            let ridge = 1.
                - noise(
                    seed ^ 691,
                    wx * 3.5 + fbm(seed ^ 16, wx * 5., wz * 5.) * 0.8,
                    wz * 3.5,
                )
                .abs();
            let foothills = fbm(seed ^ 331, wx * 9., wz * 9.);
            let secondary = 1. - noise(seed ^ 891, wx * 8. + ridge, wz * 8.).abs();
            let basin = (-((wx + 0.35) / 0.45).powi(2) - ((wz - 0.35) / 0.45).powi(2)).exp();
            // Concave coast profile leaves beaches and low coastal terraces.
            // Lowland basins interrupt uplift; nested ridges break up summits.
            let mut h = if coast < 0. {
                coast * 8. - (coast + 0.18).min(0.).powi(2) * 24.
            } else {
                inland * 3.
                    + inland.powf(1.35)
                        * (5. + ridge.powi(3) * 32. + secondary.powi(3) * 12.)
                        * (1. - basin * 0.75)
                    + foothills * inland * 4.
            };
            let edge = x.min(grid.width - 1 - x).min(z.min(grid.height - 1 - z)) as f32;
            h = -24. + (h + 24.) * (edge / 12.).clamp(0., 1.);
            let c = grid.get_mut(x as i32, z as i32);
            c.elevation = h.clamp(-30., 65.);
            c.rainfall = (0.56 + noise(seed ^ 991, px * 1.8, pz * 1.8) * 0.20
                - (wz - 0.2).max(0.) * 0.16)
                .clamp(0.18, 0.9);
        }
    }
}
