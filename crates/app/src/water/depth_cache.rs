//! Bounded memoization for shared river polygon vertices within one extraction.
//! Exact float bits identify queries; collisions replace entries and affect only
//! work, never depth. No values survive a world or terrain revision.
use genesis_sim_core::{SimState, spatial::Position};

const CAPACITY: usize = 8192;
#[derive(Clone, Copy, Default)]
struct Entry {
    key: u64,
    value: Option<f32>,
}
pub(super) struct DepthCache {
    entries: Vec<Entry>,
}
impl DepthCache {
    pub fn new() -> Self {
        Self {
            entries: vec![Entry::default(); CAPACITY],
        }
    }
    pub fn at(&mut self, sim: &SimState, p: Position) -> f32 {
        self.sample(p, || super::surface::channel_depth(sim, p))
    }
    fn sample(&mut self, p: Position, calculate: impl FnOnce() -> f32) -> f32 {
        let key = (u64::from(p.x.to_bits()) << 32) | u64::from(p.z.to_bits());
        // Mix exponent and mantissa bits; dyadic terrain coordinates often have
        // long runs of zero low bits. This table is never iterated for output.
        let mut hash = key;
        hash = (hash ^ (hash >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        hash = (hash ^ (hash >> 27)).wrapping_mul(0x94d049bb133111eb);
        hash ^= hash >> 31;
        let entry = &mut self.entries[hash as usize & (CAPACITY - 1)];
        if entry.key == key
            && let Some(value) = entry.value
        {
            return value;
        }
        let value = calculate();
        *entry = Entry {
            key,
            value: Some(value),
        };
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn collisions_and_signed_zero_keep_exact_results_with_bounded_storage() {
        let mut cache = DepthCache::new();
        let capacity = cache.entries.capacity();
        let mut calls = 0;
        let p = Position::new(1.25, 2.5);
        for _ in 0..3 {
            assert_eq!(
                cache.sample(p, || {
                    calls += 1;
                    0.75
                }),
                0.75
            );
        }
        assert_eq!(calls, 1);
        // Far more queries than slots forces replacement. Repeat in reverse to
        // cover both retained and evicted entries without relying on hash layout.
        for reverse in [false, true] {
            for n in 0..CAPACITY * 4 {
                let i = if reverse { CAPACITY * 4 - 1 - n } else { n };
                let p = Position::new(i as f32 * 0.03125 - 512., i as f32 * 0.017);
                let value = p.x.mul_add(0.17, p.z);
                assert_eq!(cache.sample(p, || value).to_bits(), value.to_bits());
            }
        }
        for x in [-0.0_f32, 0.0, -0.0] {
            assert_eq!(
                cache.sample(Position::new(x, 0.), || x).to_bits(),
                x.to_bits()
            );
        }
        assert_eq!(cache.entries.len(), CAPACITY);
        assert_eq!(cache.entries.capacity(), capacity);
    }
}
