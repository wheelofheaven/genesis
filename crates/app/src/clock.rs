//! Wall-time scheduling only; simulation rules remain in sim_core.
use std::time::Duration;

pub const TICK: Duration = Duration::from_millis(100);
pub const MAX_TICKS_PER_FRAME: usize = 8;

#[derive(Debug)]
pub struct Clock {
    pub paused: bool,
    speed: u32,
    remainder: Duration,
    pub discarded_ticks: u64,
}

impl Default for Clock {
    fn default() -> Self {
        Self {
            paused: false,
            speed: 1,
            remainder: Duration::ZERO,
            discarded_ticks: 0,
        }
    }
}

impl Clock {
    pub fn speed(&self) -> u32 {
        self.speed
    }
    pub fn set_speed(&mut self, speed: u32) {
        if matches!(speed, 1 | 4 | 16) {
            self.speed = speed;
        }
    }
    /// Overload policy: discard elapsed-time debt, never skip simulation steps.
    /// Paused elapsed time is ignored; the fractional tick is retained.
    pub fn advance(&mut self, elapsed: Duration) -> usize {
        if self.paused {
            return 0;
        }
        self.remainder += elapsed.min(Duration::from_millis(250)) * self.speed;
        let due = (self.remainder.as_nanos() / TICK.as_nanos()) as usize;
        self.remainder = Duration::from_nanos((self.remainder.as_nanos() % TICK.as_nanos()) as u64);
        self.discarded_ticks += due.saturating_sub(MAX_TICKS_PER_FRAME) as u64;
        due.min(MAX_TICKS_PER_FRAME)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn frame_partition_does_not_change_tick_count() {
        let mut a = Clock::default();
        let mut b = Clock::default();
        let fine: usize = (0..100).map(|_| a.advance(Duration::from_millis(10))).sum();
        let coarse: usize = (0..10).map(|_| b.advance(Duration::from_millis(100))).sum();
        assert_eq!(fine, 10);
        assert_eq!(fine, coarse);
    }
    #[test]
    fn pause_keeps_fraction_without_banking_wall_time() {
        let mut clock = Clock::default();
        assert_eq!(clock.advance(Duration::from_millis(50)), 0);
        clock.paused = true;
        assert_eq!(clock.advance(Duration::from_secs(10)), 0);
        clock.paused = false;
        assert_eq!(clock.advance(Duration::from_millis(50)), 1);
    }
    #[test]
    fn overload_is_bounded_and_speed_changes_preserve_fraction() {
        let mut clock = Clock::default();
        clock.advance(Duration::from_millis(50));
        clock.set_speed(4);
        assert_eq!(clock.advance(Duration::from_millis(25)), 1);
        clock.set_speed(16);
        assert_eq!(
            clock.advance(Duration::from_secs(3600)),
            MAX_TICKS_PER_FRAME
        );
        assert_eq!(clock.discarded_ticks, 32);
        assert_eq!(clock.advance(Duration::ZERO), 0);
    }
}
