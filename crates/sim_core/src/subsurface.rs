//! Synthetic subsurface scenario model. Depths and volumes use game units, not SI.
//! Independent storage columns model an unconfined aquifer; no lateral flow,
//! pressure, salinity, or coupled surface-water solver is claimed.
use crate::{
    grid::Grid,
    settlement::Building,
    spatial::{self, Position},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Rock {
    Sandstone,
    Limestone,
    Basalt,
}
impl Rock {
    pub fn name(self) -> &'static str {
        match self {
            Self::Sandstone => "Sandstone",
            Self::Limestone => "Limestone",
            Self::Basalt => "Basalt",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mineral {
    Copper,
    Iron,
    Clay,
}
impl Mineral {
    pub fn name(self) -> &'static str {
        match self {
            Self::Copper => "Copper",
            Self::Iron => "Iron",
            Self::Clay => "Clay",
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Column {
    pub rock: Rock,
    /// Fixed elevations relative to world datum, unaffected by terraforming.
    pub aquifer_top: f32,
    pub aquifer_base: f32,
    pub capacity: f32,
    pub stored: f32,
    /// Prescribed scenario recharge, volume units per second on dry land.
    pub recharge: f32,
    pub permeability: f32,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Deposit {
    pub id: u64,
    pub mineral: Mineral,
    pub center: Position,
    pub radii: [f32; 2],
    #[serde(default)]
    pub rotation: f32,
    #[serde(default)]
    pub irregularity: f32,
    pub roof: f32,
    pub thickness: f32,
    /// Peak concentration proxy, not an economic ore reserve estimate.
    pub grade: f32,
}
impl Deposit {
    fn shape(&self, angle: f32) -> f32 {
        1. + self.irregularity
            * ((angle * 3. + self.id as f32).sin() * 0.16 + (angle * 5.).cos() * 0.09)
    }
    pub fn outline(&self, angle: f32, radius: f32) -> Position {
        let r = self.shape(angle) * radius;
        let x = angle.cos() * self.radii[0] * r;
        let z = angle.sin() * self.radii[1] * r;
        let (s, c) = self.rotation.sin_cos();
        Position::new(self.center.x + c * x - s * z, self.center.z + s * x + c * z)
    }
    pub fn concentration(&self, p: Position) -> f32 {
        let (s, c) = self.rotation.sin_cos();
        let dx = p.x - self.center.x;
        let dz = p.z - self.center.z;
        let x = (c * dx + s * dz) / self.radii[0];
        let z = (-s * dx + c * dz) / self.radii[1];
        let r = (x * x + z * z) / self.shape(z.atan2(x)).powi(2);
        self.grade * (1. - r).max(0.).powi(2)
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Well {
    pub id: u64,
    pub position: Position,
    pub active: bool,
    pub recovered: f64,
    pub last_yield: f32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WellError {
    OutsideMap,
    Submerged,
    NoAquifer,
    Occupied,
    UnknownWell,
}
impl std::fmt::Display for WellError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::OutsideMap => "Choose a point inside the surveyed region",
            Self::Submerged => "Boreholes need dry ground",
            Self::NoAquifer => "No usable aquifer at this location",
            Self::Occupied => "Too close to a dwelling or another borehole",
            Self::UnknownWell => "Borehole not found",
        })
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subsurface {
    width: u32,
    height: u32,
    pub columns: Vec<Column>,
    pub deposits: Vec<Deposit>,
    pub wells: Vec<Well>,
}
#[derive(Debug, Clone, Copy)]
pub struct Reading {
    pub rock: Rock,
    pub top: f32,
    pub base: f32,
    pub water_level: f32,
    pub capacity: f32,
    pub stored: f32,
    pub permeability: f32,
    pub recharge: f32,
}
impl Reading {
    pub fn saturation(self) -> f32 {
        if self.capacity > 0. {
            (self.stored / self.capacity).clamp(0., 1.)
        } else {
            0.
        }
    }
}
impl Subsurface {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            columns: vec![
                Column {
                    rock: Rock::Basalt,
                    aquifer_top: -3.,
                    aquifer_base: -10.,
                    capacity: 0.,
                    stored: 0.,
                    recharge: 0.,
                    permeability: 0.
                };
                (width * height) as usize
            ],
            deposits: Vec::new(),
            wells: Vec::new(),
        }
    }
    /// Deterministic synthetic formations initialized after the terrain fixture.
    pub fn coastal(grid: &Grid, sea: f32) -> Self {
        let mut result = Self::new(grid.width, grid.height);
        for z in 0..grid.height {
            for x in 0..grid.width {
                let p = Position::new(x as f32, z as f32);
                let ground = spatial::elevation(grid, p);
                let basin = (-((p.x - 19.) / 8.).powi(2) - ((p.z - 21.) / 9.).powi(2)).exp();
                let ridge = ((p.x * 0.21 + p.z * 0.1).sin() + 1.) * 0.5;
                let rock = if p.x > 30. + (p.z * 0.3).sin() * 2. {
                    Rock::Basalt
                } else if p.z > 22. + (p.x * 0.24).sin() * 3. {
                    Rock::Limestone
                } else {
                    Rock::Sandstone
                };
                let permeability = match rock {
                    Rock::Sandstone => 0.7,
                    Rock::Limestone => 0.45,
                    Rock::Basalt => 0.12,
                };
                let capacity = if ground > sea { 30. + basin * 190. } else { 0. };
                let top = ground - 2. - ridge * 3.;
                result.columns[(z * grid.width + x) as usize] = Column {
                    rock,
                    aquifer_top: top,
                    aquifer_base: top - 5. - basin * 7.,
                    capacity,
                    stored: capacity * (0.22 + basin * 0.68),
                    recharge: capacity * 0.0006,
                    permeability,
                };
            }
        }
        for (id, mineral, center, radii, depth, thickness, grade) in [
            (
                1,
                Mineral::Copper,
                Position::new(29., 16.),
                [5.5, 3.2],
                8.,
                5.,
                0.72,
            ),
            (
                2,
                Mineral::Iron,
                Position::new(17., 16.),
                [3.8, 5.5],
                12.,
                7.,
                0.85,
            ),
            (
                3,
                Mineral::Clay,
                Position::new(23., 27.),
                [6., 2.8],
                1.5,
                2.5,
                0.9,
            ),
        ] {
            result.deposits.push(Deposit {
                id,
                mineral,
                center,
                radii,
                rotation: 0.,
                irregularity: 0.,
                roof: spatial::elevation(grid, center) - depth,
                thickness,
                grade,
            });
        }
        result
    }
    fn taps(&self, p: Position) -> [(usize, f32); 4] {
        let x = p.x.clamp(0., self.width as f32 - 1.);
        let z = p.z.clamp(0., self.height as f32 - 1.);
        let ix = x.floor() as u32;
        let iz = z.floor() as u32;
        let tx = x.fract();
        let tz = z.fract();
        [
            (ix, iz, (1. - tx) * (1. - tz)),
            ((ix + 1).min(self.width - 1), iz, tx * (1. - tz)),
            (ix, (iz + 1).min(self.height - 1), (1. - tx) * tz),
            (
                (ix + 1).min(self.width - 1),
                (iz + 1).min(self.height - 1),
                tx * tz,
            ),
        ]
        .map(|(x, z, w)| ((z * self.width + x) as usize, w))
    }
    pub fn reading(&self, p: Position) -> Reading {
        let taps = self.taps(p);
        let sample = |f: fn(&Column) -> f32| {
            taps.iter()
                .map(|(i, w)| f(&self.columns[*i]) * w)
                .sum::<f32>()
        };
        let nearest = taps
            .iter()
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .map_or(0, |t| t.0);
        let top = sample(|c| c.aquifer_top);
        let base = sample(|c| c.aquifer_base);
        let capacity = sample(|c| c.capacity);
        let stored = sample(|c| c.stored);
        Reading {
            rock: self.columns[nearest].rock,
            top,
            base,
            water_level: base + (top - base) * if capacity > 0. { stored / capacity } else { 0. },
            capacity,
            stored,
            permeability: sample(|c| c.permeability),
            recharge: sample(|c| c.recharge),
        }
    }
    pub fn validate_well(
        &self,
        grid: &Grid,
        sea: f32,
        buildings: &[Building],
        p: Position,
    ) -> Result<(), WellError> {
        if !p.x.is_finite()
            || !p.z.is_finite()
            || p.x < 0.
            || p.z < 0.
            || p.x >= self.width as f32
            || p.z >= self.height as f32
        {
            return Err(WellError::OutsideMap);
        }
        if spatial::elevation(grid, p) <= sea + 0.08 {
            return Err(WellError::Submerged);
        }
        if self.reading(p).capacity < 5. {
            return Err(WellError::NoAquifer);
        }
        let near = |q: Position| {
            (p.x - q.x).powi(2) + (p.z - q.z).powi(2) < Building::WELL_CLEARANCE.powi(2)
        };
        if self.wells.iter().any(|w| near(w.position)) || buildings.iter().any(|b| near(b.position))
        {
            return Err(WellError::Occupied);
        }
        Ok(())
    }
    /// Caller validates with validate_well in the action handler.
    pub(crate) fn drill(&mut self, p: Position) -> u64 {
        let id = self.wells.len() as u64 + 1;
        self.wells.push(Well {
            id,
            position: p,
            active: true,
            recovered: 0.,
            last_yield: 0.,
        });
        id
    }
    /// Fixed-order conservative withdrawals from the sampled column footprint.
    fn withdraw(&mut self, p: Position, amount: f32) -> f32 {
        let mut extracted = 0.;
        for (i, w) in self.taps(p) {
            let c = &mut self.columns[i];
            let take = (amount * w).min(c.stored);
            c.stored -= take;
            extracted += take;
        }
        extracted
    }
    pub fn step(&mut self, grid: &Grid, sea: f32, dt: f32) {
        if !dt.is_finite() || dt <= 0. {
            return;
        }
        for (i, c) in self.columns.iter_mut().enumerate() {
            let environment = &grid.cells()[i];
            if environment.elevation > sea {
                let recharge = c.recharge * dt;
                c.stored = (c.stored + recharge).min(c.capacity);
            }
        }
        for i in 0..self.wells.len() {
            let p = self.wells[i].position;
            let rate = if self.wells[i].active && spatial::elevation(grid, p) > sea + 0.08 {
                self.reading(p).permeability * 4.
            } else {
                0.
            };
            let amount = (rate as f64 * dt as f64).min(f32::MAX as f64) as f32;
            let extracted = self.withdraw(p, amount);
            self.wells[i].last_yield = extracted / dt;
            self.wells[i].recovered += extracted as f64;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{actions::Action, events::SimEvent, scenarios};
    #[test]
    fn withdrawals_conserve_water_and_stop_at_empty() {
        let mut sim = scenarios::coastal_outpost();
        for c in &mut sim.subsurface.columns {
            c.recharge = 0.;
        }
        let site = Position::new(18.25, 21.5);
        assert!(
            sim.subsurface
                .validate_well(&sim.grid, sim.sea_level, &sim.buildings, site)
                .is_ok()
        );
        let stock = |s: &Subsurface| s.columns.iter().map(|c| c.stored as f64).sum::<f64>();
        let before = stock(&sim.subsurface);
        sim.step(0., &[Action::DrillWell { position: site }]);
        for _ in 0..100 {
            sim.step(0.1, &[]);
        }
        let recovered = sim.subsurface.wells[0].recovered;
        assert!(recovered > 0.);
        assert!((before - stock(&sim.subsurface) - recovered).abs() < 0.01);
        sim.subsurface.step(&sim.grid, sim.sea_level, 1e6);
        sim.subsurface.step(&sim.grid, sim.sea_level, 1.);
        assert_eq!(sim.subsurface.wells[0].last_yield, 0.);
        assert!(
            sim.subsurface
                .columns
                .iter()
                .all(|c| c.stored.is_finite() && c.stored >= 0. && c.stored <= c.capacity)
        );
        assert!((before - stock(&sim.subsurface) - sim.subsurface.wells[0].recovered).abs() < 0.01);
    }
    #[test]
    fn stopping_allows_recharge_and_deposits_do_not_move_with_terrain() {
        let mut sim = scenarios::coastal_outpost();
        let p = Position::new(18., 21.);
        sim.step(0., &[Action::DrillWell { position: p }]);
        sim.subsurface.step(&sim.grid, sim.sea_level, 1000.);
        sim.step(
            0.,
            &[Action::SetWellActive {
                id: 1,
                active: false,
            }],
        );
        let stock = sim.subsurface.reading(p).stored;
        let recovered = sim.subsurface.wells[0].recovered;
        sim.step(10., &[]);
        assert!(sim.subsurface.reading(p).stored > stock);
        assert_eq!(sim.subsurface.wells[0].recovered, recovered);
        assert_eq!(sim.subsurface.wells[0].last_yield, 0.);
        let deposits = sim.subsurface.deposits.clone();
        let top = sim.subsurface.reading(p).top;
        sim.step(
            0.,
            &[Action::Terraform {
                x: 18,
                y: 21,
                radius: 3,
                strength: 1.,
            }],
        );
        assert_eq!(sim.subsurface.deposits, deposits);
        assert_eq!(sim.subsurface.reading(p).top, top);
    }
    #[test]
    fn invalid_and_duplicate_wells_are_rejected_and_replay_is_identical() {
        let mut a = scenarios::coastal_outpost();
        let mut b = a.clone();
        let actions = [
            Action::DrillWell {
                position: Position::new(18.25, 21.5),
            },
            Action::DrillWell {
                position: Position::new(18.25, 21.5),
            },
            Action::DrillWell {
                position: Position::new(f32::NAN, 3.),
            },
            Action::DrillWell {
                position: Position::new(1., 1.),
            },
            Action::DrillWell {
                position: a.buildings[0].position,
            },
        ];
        let events = a.step(0.1, &actions);
        b.step(0.1, &actions);
        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(e, SimEvent::WellDrilled { .. }))
                .count(),
            1
        );
        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(e, SimEvent::WellRejected { .. }))
                .count(),
            4
        );
        for _ in 0..200 {
            a.step(0.1, &[]);
            b.step(0.1, &[]);
        }
        assert_eq!(a.subsurface.wells, b.subsurface.wells);
        for (a, b) in a.subsurface.columns.iter().zip(&b.subsurface.columns) {
            assert_eq!(a.stored, b.stored);
        }
        let candidate = Building {
            id: 100,
            position: Position::new(18.25, 21.5),
            rotation: 0.,
        };
        assert_eq!(
            a.validate_building(&candidate),
            Err(crate::settlement::PlacementError::Occupied)
        );
        let e = a.step(
            0.,
            &[Action::SetWellActive {
                id: 999,
                active: false,
            }],
        );
        assert!(e.iter().any(|e| matches!(
            e,
            SimEvent::WellRejected {
                reason: WellError::UnknownWell
            }
        )));
    }
}
