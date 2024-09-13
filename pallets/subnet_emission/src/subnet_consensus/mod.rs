use frame_support::{traits::Get, weights::Weight};
use frame_system::Config;

pub mod linear;
pub mod treasury;
pub mod yuma;

#[derive(Debug)]
pub struct WeightCounter {
    reads: usize,
    writes: usize,
}

impl WeightCounter {
    pub fn new() -> Self {
        Self {
            reads: 0,
            writes: 0,
        }
    }

    pub fn read(&mut self, amount: usize) {
        self.reads = self.reads.saturating_add(amount);
    }

    pub fn wrote(&mut self, amount: usize) {
        self.writes = self.writes.saturating_add(amount);
    }

    pub fn to_weights<T: Config>(self) -> Weight {
        T::DbWeight::get().reads_writes(
            self.reads.try_into().unwrap_or(0),
            self.writes.try_into().unwrap_or(0),
        )
    }
}

impl Default for WeightCounter {
    fn default() -> Self {
        Self::new()
    }
}
