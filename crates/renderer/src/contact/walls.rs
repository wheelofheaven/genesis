//! Short-range ground contact along wall segments, in metres.
use super::{SIZE, TEXEL_METRES};
use glam::Vec2;

#[derive(Clone, Copy)]
pub struct Wall {
    pub a: Vec2,
    pub b: Vec2,
    pub reach: f32,
    pub strength: f32,
}

pub(super) fn stamp(values: &mut [u8], origin: Vec2, wall: Wall) {
    if !wall.a.is_finite()
        || !wall.b.is_finite()
        || !wall.reach.is_finite()
        || !wall.strength.is_finite()
        || wall.reach <= 0.
    {
        return;
    }
    let reach = wall.reach.min(4.);
    let lo = ((wall.a.min(wall.b) - Vec2::splat(reach) - origin) / TEXEL_METRES)
        .floor()
        .clamp(Vec2::ZERO, Vec2::splat(SIZE as f32))
        .as_uvec2();
    let hi = ((wall.a.max(wall.b) + Vec2::splat(reach) - origin) / TEXEL_METRES)
        .ceil()
        .clamp(Vec2::ZERO, Vec2::splat(SIZE as f32))
        .as_uvec2();
    let delta = wall.b - wall.a;
    let length = delta.length_squared().max(1e-8);
    for y in lo.y..hi.y {
        for x in lo.x..hi.x {
            let p = origin + (Vec2::new(x as f32, y as f32) + 0.5) * TEXEL_METRES;
            let nearest = wall.a + delta * ((p - wall.a).dot(delta) / length).clamp(0., 1.);
            let t = (p.distance(nearest) / reach).clamp(0., 1.);
            let amount =
                (wall.strength.clamp(0., 0.6) * (1. - t * t * (3. - 2. * t)) * 255.).round() as u8;
            let pixel = &mut values[(y * SIZE + x) as usize];
            *pixel = (*pixel).max(amount);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contact::Field;
    #[test]
    fn wall_contacts_follow_segments_fade_and_survive_field_shifts() {
        let wall = Wall {
            a: Vec2::new(-2., -2.),
            b: Vec2::new(2., 2.),
            reach: 0.75,
            strength: 0.35,
        };
        let field = Field::bake_with_walls(Vec2::ZERO, [], [wall]);
        let reverse = Field::bake_with_walls(
            Vec2::new(20., -20.),
            [],
            [Wall {
                a: wall.b,
                b: wall.a,
                ..wall
            }],
        );
        let near = field.at(Vec2::splat(0.125));
        assert!(near > 50 && near <= 90);
        assert!(field.at(Vec2::new(0.125, 0.625)) < near);
        assert_eq!(field.at(Vec2::new(0., 2.)), 0);
        assert_eq!(field.at(Vec2::splat(4.)), 0);
        for y in -16..16 {
            for x in -16..16 {
                let p = (Vec2::new(x as f32, y as f32) + 0.5) * TEXEL_METRES;
                assert_eq!(field.at(p), reverse.at(p));
            }
        }
    }
}
