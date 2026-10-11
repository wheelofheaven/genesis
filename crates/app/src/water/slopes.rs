//! Smooth presentation normals shared across river ribbons at bends and joins.
//! Filters the presentation level and longitudinal reflection slope. The physical
//! channel bed and drainage graph stay in the simulation; this is not hydraulics.
use genesis_sim_core::{SimState, surface::channels, units::metres};
use glam::Vec2;
use std::collections::HashMap;

struct Segment {
    a: Vec2,
    b: Vec2,
    slope: Vec2,
    radius: f32,
    levels: [f32; 2],
}

pub(super) struct Slopes {
    segments: Vec<Segment>,
    bins: HashMap<(i32, i32), Vec<usize>>,
}

impl Slopes {
    pub fn new(sim: &SimState) -> Self {
        let mut field = Self {
            segments: Vec::new(),
            bins: HashMap::new(),
        };
        for river in &sim.geography.rivers {
            for (i, pair) in river.points.windows(2).enumerate() {
                let a = Vec2::new(pair[0].x, pair[0].z);
                let b = Vec2::new(pair[1].x, pair[1].z);
                let d = b - a;
                if d.length_squared() < 1e-10 {
                    continue;
                }
                let levels = [channels::level(sim, pair[0]), channels::level(sim, pair[1])];
                let rise = levels[1] - levels[0];
                let slope = (d * (rise / d.length_squared())).clamp_length_max(1.);
                let radius = channels::width(river, i, 0.).max(channels::width(river, i, 1.)) * 0.5
                    + channels::BANK
                    + metres(2.);
                // A coarse terrain corner may lie just beyond the ribbon. Keep
                // enough context to extrapolate its nearest section's level.
                let support = radius + metres(20.);
                let min = (a.min(b) - Vec2::splat(support)).floor().as_ivec2();
                let max = (a.max(b) + Vec2::splat(support)).floor().as_ivec2();
                for z in min.y..=max.y {
                    for x in min.x..=max.x {
                        field
                            .bins
                            .entry((x, z))
                            .or_default()
                            .push(field.segments.len());
                    }
                }
                field.segments.push(Segment {
                    a,
                    b,
                    slope,
                    radius,
                    levels,
                });
            }
        }
        field
    }

    pub fn at(&self, p: Vec2) -> Vec2 {
        self.sample(p).1
    }

    /// Filtered water presentation height and slope, independent of which
    /// ribbon covers the query. Does not change the simulation channel bed.
    pub fn sample(&self, p: Vec2) -> (Option<f32>, Vec2) {
        let mut slope = Vec2::ZERO;
        let mut weight = 0.;
        let mut level = 0.;
        let mut nearest = f32::INFINITY;
        let mut fallback = None;
        if let Some(indices) = self.bins.get(&(p.x.floor() as i32, p.y.floor() as i32)) {
            for &index in indices {
                let segment = &self.segments[index];
                let d = segment.b - segment.a;
                let t = ((p - segment.a).dot(d) / d.length_squared()).clamp(0., 1.);
                let distance = p.distance(segment.a + d * t);
                let height = segment.levels[0] + (segment.levels[1] - segment.levels[0]) * t;
                if distance < nearest {
                    nearest = distance;
                    fallback = Some(height);
                }
                // Compact, smooth support: branches blend instead of switching
                // to a different ribbon's triangle normal at a nearest-edge tie.
                let q = (1. - (distance / segment.radius).powi(2)).max(0.);
                let w = q * q * d.length().min(segment.radius);
                slope += segment.slope * w;
                level += height * w;
                weight += w;
            }
        }
        if weight > 1e-8 {
            (Some(level / weight), slope / weight)
        } else {
            (fallback, Vec2::ZERO)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use genesis_sim_core::{config::SimConfig, spatial::Position, worldgen::River};

    #[test]
    fn slopes_follow_world_axes_and_blend_continuously_at_junctions() {
        let mut sim = SimState::new(SimConfig {
            width: 16,
            height: 16,
            ..Default::default()
        });
        for z in 0..16 {
            for x in 0..16 {
                sim.grid.get_mut(x, z).elevation = 2. + x as f32 * 0.1 + z as f32 * 0.2;
            }
        }
        let river = |points| River {
            points,
            width: metres(6.),
            widths: Vec::new(),
            bounds: None,
        };
        sim.geography.rivers = vec![
            river(vec![Position::new(4., 8.), Position::new(12., 8.)]),
            river(vec![Position::new(8., 4.), Position::new(8., 8.)]),
        ];
        let field = Slopes::new(&sim);
        assert!(field.at(Vec2::new(5., 8.)).distance(Vec2::new(0.1, 0.)) < 1e-5);
        assert!(field.at(Vec2::new(8., 5.)).distance(Vec2::new(0., 0.2)) < 1e-5);
        let join = field.at(Vec2::splat(8.));
        assert!(join.x > 0. && join.y > 0. && join.is_finite());
        let mut last = field.at(Vec2::new(7.5, 8.));
        for i in 1..=1000 {
            let next = field.at(Vec2::new(7.5 + i as f32 * 0.001, 8.));
            assert!(next.is_finite() && next.distance(last) < 0.003);
            last = next;
        }
        sim.geography.rivers.reverse();
        assert!(Slopes::new(&sim).at(Vec2::splat(8.)).distance(join) < 1e-6);
        assert_eq!(field.at(Vec2::ZERO), Vec2::ZERO);
    }
}
