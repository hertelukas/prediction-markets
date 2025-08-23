use strum::{EnumCount, IntoEnumIterator};

use crate::market::Market;

#[derive(Debug, PartialEq, Eq)]
pub enum LmsrError {
    InsufficientShares,
    Resolved,
    NegativeMarketCapitalization,
}

/// Used for serialization
pub struct LmsrMarketDTO<T: EnumCount + IntoEnumIterator + Copy + Eq> {
    pub shares: Vec<u64>,
    pub liquidity: f64,
    pub resolved: Option<T>,
    pub market_volume: f64,
}

impl<T> From<LmsrMarket<T>> for LmsrMarketDTO<T>
where
    T: EnumCount + IntoEnumIterator + Copy + Eq,
{
    fn from(value: LmsrMarket<T>) -> Self {
        Self {
            shares: value.shares,
            liquidity: value.liquidity,
            resolved: value.resolved,
            market_volume: value.market_volume,
        }
    }
}

impl<T> From<LmsrMarketDTO<T>> for LmsrMarket<T>
where
    T: EnumCount + IntoEnumIterator + Copy + Eq,
{
    fn from(value: LmsrMarketDTO<T>) -> Self {
        Self {
            shares: value.shares,
            liquidity: value.liquidity,
            resolved: value.resolved,
            market_volume: value.market_volume,
        }
    }
}
pub struct LmsrMarket<T: EnumCount + IntoEnumIterator + Copy + Eq> {
    shares: Vec<u64>,
    liquidity: f64,
    resolved: Option<T>,
    market_volume: f64,
}

impl<T> LmsrMarket<T>
where
    T: EnumCount + IntoEnumIterator + Copy + Eq,
{
    pub fn new(liquidity: f64) -> Self {
        Self {
            shares: vec![0; T::COUNT],
            liquidity,
            resolved: None,
            market_volume: 0.0,
        }
    }

    pub fn outcome_index(outcome: T) -> usize {
        T::iter()
            .position(|o| o == outcome)
            .expect("Invalid outcome")
    }

    fn cost(&self, shares: &[u64]) -> f64 {
        let sum: f64 = shares
            .iter()
            .map(|&q| (q as f64 / self.liquidity).exp())
            .sum();

        self.liquidity * sum.ln()
    }

    pub fn serialize(self) -> LmsrMarketDTO<T> {
        self.into()
    }
}

impl<T> Market for LmsrMarket<T>
where
    T: EnumCount + IntoEnumIterator + Copy + Eq,
{
    type Outcome = T;
    type Error = LmsrError;

    fn price(&self, outcome: Self::Outcome) -> Result<f64, Self::Error> {
        let i = Self::outcome_index(outcome);
        let q_i = self.shares[i] as f64;

        let exp_qi = (q_i / self.liquidity).exp();
        let denom: f64 = self
            .shares
            .iter()
            .map(|&q| (q as f64 / self.liquidity).exp())
            .sum();

        Ok(exp_qi / denom)
    }

    fn buy(&mut self, outcome: Self::Outcome, amount: u64) -> Result<f64, Self::Error> {
        if self.resolved.is_some() {
            return Err(LmsrError::Resolved);
        }
        let current_cost = self.cost(&self.shares);

        let mut new_shares = self.shares.clone();
        let i = Self::outcome_index(outcome);
        new_shares[i] += amount;

        let new_cost = self.cost(&new_shares);
        self.shares = new_shares;
        self.market_volume += new_cost - current_cost;
        Ok(new_cost - current_cost)
    }

    fn sell(&mut self, outcome: Self::Outcome, amount: u64) -> Result<f64, Self::Error> {
        if self.resolved.is_some() {
            return Err(LmsrError::Resolved);
        }

        let i = Self::outcome_index(outcome);
        if amount > self.shares[i] {
            return Err(LmsrError::InsufficientShares);
        }

        let current_cost = self.cost(&self.shares);

        let mut new_shares = self.shares.clone();
        new_shares[i] -= amount;

        let new_cost = self.cost(&new_shares);
        if self.market_volume - (current_cost - new_cost) < 0.0 {
            return Err(LmsrError::NegativeMarketCapitalization);
        }
        self.market_volume -= current_cost - new_cost;
        self.shares = new_shares;
        Ok(current_cost - new_cost)
    }

    fn resolve(&mut self, winning_outcome: Self::Outcome) -> Result<(), Self::Error> {
        self.resolved = Some(winning_outcome);
        Ok(())
    }

    fn payout_per_share(&self, outcome: Self::Outcome) -> Result<f64, Self::Error> {
        let i = Self::outcome_index(outcome);
        let amount = self.shares[i];
        if amount == 0 {
            Err(LmsrError::InsufficientShares)
        } else {
            Ok(self.market_volume / amount as f64)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::market::BinaryOutcome;

    use super::*;

    #[test]
    fn test_price_equal() {
        let market = LmsrMarket::<BinaryOutcome>::new(10.0);

        assert_eq!(
            market.price(BinaryOutcome::Yes),
            market.price(BinaryOutcome::No)
        );
    }

    #[test]
    fn test_buy_raises_price() {
        let mut market = LmsrMarket::<BinaryOutcome>::new(10.0);

        market.buy(BinaryOutcome::Yes, 1).expect("could not buy");

        assert!(
            market
                .price(BinaryOutcome::Yes)
                .expect("could not determine yes price")
                > market
                    .price(BinaryOutcome::No)
                    .expect("could not determine no price")
        );
    }

    #[test]
    fn test_buy_sell_no_impact_on_market() {
        let mut market = LmsrMarket::<BinaryOutcome>::new(10.0);

        market.buy(BinaryOutcome::Yes, 1).expect("could not buy");
        market.sell(BinaryOutcome::Yes, 1).expect("could not sell");

        assert_eq!(
            market.price(BinaryOutcome::Yes),
            market.price(BinaryOutcome::No)
        );
    }

    #[test]
    fn test_buy_sell_no_impact_on_trader() {
        let mut market = LmsrMarket::<BinaryOutcome>::new(10.0);

        let cost = market.buy(BinaryOutcome::Yes, 1).expect("could not buy");
        let wins = market.sell(BinaryOutcome::Yes, 1).expect("could not sell");

        assert_eq!(cost, wins);
    }

    #[test]
    fn test_market_payout_same_one() {
        let mut market = LmsrMarket::<BinaryOutcome>::new(10.0);

        let cost = market.buy(BinaryOutcome::Yes, 1).expect("could not buy");
        market
            .resolve(BinaryOutcome::Yes)
            .expect("could not resolve market");

        assert_eq!(
            cost,
            market
                .payout_per_share(BinaryOutcome::Yes)
                .expect("could not calculate payout")
        );
    }

    #[test]
    fn test_market_payout_same_multiple() {
        let shares = 8;
        let mut market = LmsrMarket::<BinaryOutcome>::new(10.0);

        let cost = market
            .buy(BinaryOutcome::Yes, shares)
            .expect("could not buy");
        market
            .resolve(BinaryOutcome::Yes)
            .expect("could not resolve market");

        assert_eq!(
            cost,
            market
                .payout_per_share(BinaryOutcome::Yes)
                .expect("could not calculate payout")
                * shares as f64
        );
    }

    #[test]
    fn test_market_payout_different_multiple() {
        let shares: u64 = 8;
        let mut market = LmsrMarket::<BinaryOutcome>::new(10.0);

        let mut cost = market
            .buy(BinaryOutcome::Yes, shares.div_ceil(2))
            .expect("could not buy");

        cost += market
            .buy(BinaryOutcome::No, (shares + 1).div_ceil(2))
            .expect("could not buy");

        market
            .resolve(BinaryOutcome::Yes)
            .expect("could not resolve market");

        assert_eq!(
            cost,
            market
                .payout_per_share(BinaryOutcome::Yes)
                .expect("could not calculate payout")
                * (shares / 2) as f64
        );
    }

    #[test]
    fn test_market_payout_different_multiple_with_sell() {
        let shares: u64 = 8;
        let mut market = LmsrMarket::<BinaryOutcome>::new(10.0);

        let mut cost = market
            .buy(BinaryOutcome::Yes, shares.div_ceil(2))
            .expect("could not buy");

        cost += market
            .buy(BinaryOutcome::No, (shares + 1).div_ceil(2))
            .expect("could not buy");

        cost -= market.sell(BinaryOutcome::No, 1).expect("coult not sell");

        market
            .resolve(BinaryOutcome::Yes)
            .expect("could not resolve market");

        assert_eq!(
            cost,
            market
                .payout_per_share(BinaryOutcome::Yes)
                .expect("could not calculate payout")
                * (shares / 2) as f64
        );
    }
}
