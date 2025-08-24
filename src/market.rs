use strum::{EnumCount, EnumIter};

pub trait Market {
    type Outcome;
    type Error;

    fn price(&self, outcome: Self::Outcome) -> Result<f64, Self::Error>;

    fn buy(&mut self, outcome: Self::Outcome, amount: u64) -> Result<f64, Self::Error>;
    fn sell(&mut self, outcome: Self::Outcome, amount: u64) -> Result<f64, Self::Error>;

    /// Permanently resolves the market with the correct outcome
    fn resolve(&mut self, winning_outcome: Self::Outcome) -> Result<(), Self::Error>;

    /// Returns the payout per sahre for a given outcome after resolution
    fn payout_per_share(&self, outcome: Self::Outcome) -> Result<f64, Self::Error>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumCount, EnumIter)]
pub enum BinaryOutcome {
    Yes,
    No,
}

impl From<bool> for BinaryOutcome {
    fn from(value: bool) -> Self {
        if value {
            BinaryOutcome::Yes
        } else {
            BinaryOutcome::No
        }
    }
}

impl From<BinaryOutcome> for bool {
    fn from(value: BinaryOutcome) -> Self {
        value == BinaryOutcome::Yes
    }
}
