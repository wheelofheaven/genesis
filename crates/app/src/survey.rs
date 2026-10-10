//! Presentation-only layer choices; readings and interventions belong to sim_core.
use genesis_sim_core::{SimState, spatial::Position, subsurface::Mineral};
use glam::Vec3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Survey {
    pub open: bool,
    pub water: bool,
    pub minerals: bool,
    pub seafloor: bool,
    pub opacity: f32,
}
impl Default for Survey {
    fn default() -> Self {
        Self {
            open: false,
            water: false,
            minerals: false,
            seafloor: false,
            opacity: 0.8,
        }
    }
}
impl Survey {
    pub fn active(self) -> bool {
        self.water || self.minerals || self.seafloor
    }
    pub fn label(self) -> &'static str {
        if self.seafloor && !self.water && !self.minerals {
            return "Seafloor survey";
        }
        match (self.water, self.minerals) {
            (true, true) => "Hydrology + geology",
            (true, false) => "Hydrology survey",
            (false, true) => "Geological survey",
            _ => "Surface view",
        }
    }
}
pub fn mineral_color(mineral: Mineral) -> [f32; 3] {
    match mineral {
        Mineral::Copper => [0.92, 0.39, 0.14],
        Mineral::Iron => [0.57, 0.34, 0.69],
        Mineral::Clay => [0.88, 0.72, 0.28],
    }
}
pub fn water_color(stored: f32) -> Vec3 {
    let amount = (stored / 200.).clamp(0., 1.);
    Vec3::new(0.18, 0.23, 0.26).lerp(Vec3::new(0.04, 0.66, 0.91), amount.sqrt())
}
/// Hydrology is a volume wash; mineralogy stays distinct through body outlines.
pub fn terrain_color(sim: &SimState, p: Position, base: Vec3, layers: Survey) -> Vec3 {
    if !layers.water && !layers.minerals {
        return base;
    }
    let reading = sim.subsurface.reading(p);
    let mut color = Vec3::new(0.23, 0.28, 0.28);
    if layers.water {
        // Absolute stored volume, scaled to a documented 200-unit legend.
        color = water_color(reading.stored);
    }
    if layers.minerals && !layers.water {
        for d in &sim.subsurface.deposits {
            let strength = (d.concentration(p) / d.grade.max(0.001)).sqrt();
            color = color.lerp(Vec3::from_array(mineral_color(d.mineral)), strength);
        }
    }
    base.lerp(color, layers.opacity)
}
