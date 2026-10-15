//! Collectable water; cumulative extraction remains an accounting measurement.
use serde::{Deserialize, Serialize};
pub const CAPACITY: f64 = 20.;
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Supply {
    pub stored: f64,
    pub collected: f64,
    /// External outflow, including pre-schema-4 historical extraction.
    pub released: f64,
}
impl Supply {
    pub(crate) fn receive(&mut self, amount: f64) {
        let retained = amount.min((CAPACITY - self.stored).max(0.));
        self.stored += retained;
        self.released += amount - retained;
    }
    pub(crate) fn collect(&mut self, amount: f64) -> bool {
        if !amount.is_finite() || amount <= 0. || amount > self.stored {
            return false;
        }
        self.stored -= amount;
        self.collected += amount;
        true
    }
    pub(crate) fn valid(&self, recovered: f64) -> bool {
        [self.stored, self.collected, self.released]
            .into_iter()
            .all(|x| x.is_finite() && x >= 0.)
            && self.stored <= CAPACITY
            && (self.stored + self.collected + self.released - recovered).abs()
                <= 1e-7 * (1. + recovered)
    }
}
