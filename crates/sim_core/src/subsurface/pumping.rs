//! Buffer-limited pumping keeps unused water in the aquifer.
use super::{Subsurface, Well, supply};
use crate::{grid::Grid, spatial, spatial::Position};
use serde::{Deserialize, Serialize};

/// The operator's policy; `Well::active` remains the independent master switch.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PumpMode {
    #[default]
    Continuous,
    FillBuffer,
}
impl PumpMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Continuous => "Continuous",
            Self::FillBuffer => "Fill buffer",
        }
    }
}
// Column stocks are f32; leave sub-display-precision room instead of repeatedly
// attempting withdrawals smaller than a source column can represent.
const FULL_MARGIN: f64 = 0.0001;

impl Well {
    pub(super) fn buffer_room(&self) -> f64 {
        let room = self
            .supply
            .as_ref()
            .map_or(0., |s| (supply::CAPACITY - s.stored).max(0.));
        if room <= FULL_MARGIN { 0. } else { room }
    }
    /// Read-only operational explanation, including automatic pauses. Pausing at
    /// capacity never changes `active` and never cancels collection reservations.
    pub fn pumping_status(&self, grid: &Grid, sea: f32) -> &'static str {
        if !self.active {
            "Pump stopped"
        } else if spatial::elevation(grid, self.position) <= sea + 0.08 {
            "Submerged · pumping blocked"
        } else if self.pump_mode == PumpMode::FillBuffer && self.buffer_room() == 0. {
            "Buffer full · waiting for collection"
        } else if self.last_yield > 0. {
            "Pumping"
        } else {
            "No current yield · waiting for water"
        }
    }
}

impl Subsurface {
    /// Debit actual representable column losses, never more than the receiving
    /// buffer can hold. Continuous mode retains its historical arithmetic.
    pub(super) fn withdraw_to_buffer(&mut self, p: Position, amount: f32, room: f64) -> f64 {
        let mut extracted = 0.;
        for (i, weight) in self.taps(p) {
            let remaining = room - extracted;
            if remaining <= 0. {
                break;
            }
            let column = &mut self.columns[i];
            let requested = f64::from(amount * weight)
                .min(f64::from(column.stored))
                .min(remaining);
            let target = f64::from(column.stored) - requested;
            let mut next = target as f32;
            if f64::from(next) < target {
                next = next.next_up();
            }
            let taken = f64::from(column.stored) - f64::from(next);
            column.stored = next;
            extracted += taken;
        }
        extracted
    }
}

#[cfg(test)]
mod tests;
