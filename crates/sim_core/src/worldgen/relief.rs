//! Regional uplift, a continental shelf, and nested ridge systems. Metres are
//! kept consistent with settlement scale; noise does not enlarge the houses.
use super::{
    Region,
    noise::{fbm, noise, unit},
};
use crate::grid::Grid;

pub fn build(grid: &mut Grid, seed: u64, region: Region) {
    if region != Region::Island {
        for z in 0..grid.height {
            for x in 0..grid.width {
                let (height, rainfall) = regional_sample(seed, region, x as f32, z as f32);
                let cell = grid.get_mut(x as i32, z as i32);
                cell.elevation = height;
                cell.rainfall = rainfall;
            }
        }
        return;
    }
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

/// Continuous regional base relief in a fixed east/south frame. Inputs and returned
/// elevation use world units; the recipe is calculated in metres.
/// Also used outside the active simulation window for visual context. This
/// procedural source can later be replaced by an imported topobathymetric raster.
pub fn regional_sample(seed: u64, region: Region, x: f32, z: f32) -> (f32, f32) {
    let east = (x - super::WIDTH as f32 * 0.5) * crate::units::METRES_PER_UNIT;
    let south = (z - super::HEIGHT as f32 * 0.5) * crate::units::METRES_PER_UNIT;
    let coast = -950. + fbm(seed ^ 101, 7., south / 1900.) * 220. + (south / 1200.).sin() * 130.;
    let inland = east - coast;
    let warp = fbm(seed ^ 219, east / 1800., south / 1800.) * 0.55;
    let ridge = 1. - noise(seed ^ 383, east / 900. + warp, south / 1500.).abs();
    let fine = fbm(seed ^ 481, east / 420., south / 420.);
    let height = if region == Region::LevantHighlands {
        // A regional fall gives the valley an outlet; small local depressions
        // are filled during routing rather than enclosing the whole catchment.
        // Shape the valley before routing water through it. A single broad,
        // unseeded sine left kilometre-long grid-aligned reaches near its extrema.
        // Seeded bends at two scales and a smooth offset give the drainage graph
        // a continuous, varying corridor rather than decorating river polylines.
        let phase = unit(seed, 1103) * std::f32::consts::TAU;
        let bend_phase = unit(seed, 1109) * std::f32::consts::TAU;
        let axis = (south / 1700. + phase).sin() * 260.
            + (south / 320. + bend_phase).sin() * 100.
            + noise(seed ^ 1117, south / 700., 4.) * 70.;
        let distance = (east - axis).abs();
        // A rounded valley floor avoids a sharp V crease locking flow to one
        // column. Low relief varies the floor and tributary routes; the regional
        // downstream fall remains separate from this smaller-scale terrain.
        let valley = ((distance * distance + 90_f32.powi(2)).sqrt() - 90.) * 0.075;
        let drainage_relief = noise(seed ^ 1123, east / 160., south / 160.) * 8.
            + noise(seed ^ 1129, east / 55., south / 55.) * 1.4;
        160. + south * 0.018
            + valley
            + drainage_relief
            + (ridge.powi(2) * 120. + fine * 50.) * (distance / 1500.).clamp(0., 1.)
    } else if inland >= 0. {
        let foothill = ((inland - 950.) / 2800.).clamp(0., 1.);
        let plain = inland.min(950.) * 0.025;
        let drainage_relief = fbm(seed ^ 711, east / 330., south / 330.) * 22.
            + fbm(seed ^ 751, east / 850., south / 850.) * 12.;
        (plain + drainage_relief * (inland / 400.).clamp(0., 1.)).max(inland.min(950.) * 0.004)
            + foothill.powf(1.4) * (110. + ridge.powi(2) * 350.)
            + fine * foothill * 45.
    } else {
        let offshore = -inland;
        let shelf = -offshore.min(1800.) * 0.035;
        let slope = (offshore - 1800.).max(0.);
        let basin = -680. * (1. - (-slope / 3500.).exp());
        let canyon_axis = south - (offshore / 950.).sin() * 220. - 350.;
        let canyon = (-(canyon_axis / (85. + offshore * 0.07)).powi(2)).exp()
            * (1. - (-offshore / 1200.).exp())
            * 75.;
        shelf + basin - canyon + fine * (offshore / 1000.).clamp(0., 1.) * 12.
    };
    let rainfall = (0.62 + noise(seed ^ 991, east / 4000., south / 4000.) * 0.15
        - (east / 12000.).max(0.))
    .clamp(0.2, 0.85);
    (
        crate::units::metres(height)
            .clamp(crate::units::MIN_ELEVATION, crate::units::MAX_ELEVATION),
        rainfall,
    )
}
