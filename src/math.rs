/// Implements the portfolio "Normal Strategy" math functions in rust.
use arbiter::utils::wad_to_float;
use statrs::distribution::{ContinuousCDF, Normal};

use super::bisection;
use bindings::{portfolio::PoolsReturn, shared_types::PortfolioConfig};

/// Amount of seconds per year used in the smart contracts.
pub static SECONDS_PER_YEAR: f64 = 31556953.0;

/// Normal curve contains the parameters for the normal distribution trading function
/// reserve_x_per_wad - x reserves per liquidity, scaled from wad to float.
/// reserve_y_per_wad - y reserves per liquidity, scaled from wad to float.
/// strike_price_f - strike price, scaled from wad to float.
/// std_dev_f - standard deviation, scaled from wad to float.
/// time_remaining_sec - time remaining in seconds, same units.
/// invariant_f - invariant, scaled from wad to float.
#[derive(Clone)]
pub struct NormalCurve {
    pub reserve_x_per_wad: f64,
    pub reserve_y_per_wad: f64,
    pub strike_price_f: f64,
    pub std_dev_f: f64,
    pub time_remaining_sec: f64,
    pub invariant_f: f64,
}

/// Math functions of the trading function,
/// adjusted trading function, and related
/// expressions.
///
/// Original trading function
/// { 0    KΦ(Φ⁻¹(1-x) - σ√τ) >= y
/// { -∞   otherwise
///
/// k = y - KΦ(Φ⁻¹(1-x) - σ√τ)
/// y = KΦ(Φ⁻¹(1-x) - σ√τ) + k
/// x = 1 - Φ(Φ⁻¹((y - k)/K) + σ√τ)
///
/// Adjusted trading function
/// { 0    Φ⁻¹(1-x) - σ√τ >= Φ⁻¹(y/K)
/// { -∞   otherwise
///
/// k = Φ⁻¹(y/K) - Φ⁻¹(1-x) + σ√τ
///  -> Φ⁻¹(y/K) = Φ⁻¹(1-x) - σ√τ + k
///      -> y/K = Φ(Φ⁻¹(1-x) - σ√τ + k)
///          -> y = KΦ(Φ⁻¹(1-x) - σ√τ + k)
///  -> Φ⁻¹(1-x) = Φ⁻¹(y/K) + σ√τ - k
///      -> 1-x = Φ(Φ⁻¹(y/K) + σ√τ - k)
///          -> x = 1 - Φ(Φ⁻¹(y/K) + σ√τ - k)
/// todo: fixed point arithmetic?
///
/// note: uses floating point math and depends on
/// floating point math libraries, which introduce some error.
/// Therefore, these functions are nice to use as sanity checks,
/// to validate the behavior is consistent with fixed point counterparts.
impl NormalCurve {
    /// constructor
    pub fn new(
        reserve_x_per_wad: f64,
        reserve_y_per_wad: f64,
        strike_price_f: f64,
        std_dev_f: f64,
        time_remaining_sec: f64,
        invariant_f: f64,
    ) -> Self {
        Self {
            reserve_x_per_wad,
            reserve_y_per_wad,
            strike_price_f,
            std_dev_f,
            time_remaining_sec,
            invariant_f,
        }
    }

    /// constructor from portfolio pool
    /// pool_return - Return from calling the portfolio contract's `pools(uint64 poolId)` function.
    /// portfolio_config - Return from calling the pool's __strategy__ contract's `configs(uint64 poolId)` function.
    pub fn new_from_portfolio(
        pool_return: &PoolsReturn,
        portfolio_config: &PortfolioConfig,
    ) -> Self {
        Self {
            reserve_x_per_wad: wad_to_float(pool_return.virtual_x.into()),
            reserve_y_per_wad: wad_to_float(pool_return.virtual_y.into()),
            strike_price_f: wad_to_float(portfolio_config.strike_price_wad.into()),
            std_dev_f: (portfolio_config.volatility_basis_points as f64) / 10000.0,
            time_remaining_sec: portfolio_config.duration_seconds as f64,
            invariant_f: 0.0,
        }
    }

    /// computes the adjusted trading function invariant
    /// invariant = Φ⁻¹(y/K) - Φ⁻¹(1-x) + σ√τ
    pub fn trading_function_floating(&self) -> f64 {
        // standard normal distribution...
        let n = Normal::new(0.0, 1.0).unwrap();
        // σ√τ
        let std_dev_sqrt_tau =
            self.std_dev_f * f64::sqrt(self.time_remaining_sec / SECONDS_PER_YEAR);
        // Φ⁻¹(1 - x)
        let invariant_term_x = n.inverse_cdf(1.0 - self.reserve_x_per_wad);
        // Φ⁻¹(y/K)
        let invariant_term_y = n.inverse_cdf(self.reserve_y_per_wad / self.strike_price_f);
        println!("invariant_term_x: {}", invariant_term_x);
        println!("invariant_term_y: {}", invariant_term_y);
        println!("std_dev_sqrt_tau: {}", std_dev_sqrt_tau);
        // k = Φ⁻¹(y/K) - Φ⁻¹(1-x) + σ√τ
        let k = invariant_term_y - invariant_term_x + std_dev_sqrt_tau;

        k
    }

    /// computes the adjusted trading function y variable.
    /// y = KΦ(Φ⁻¹(1-x) - σ√τ + k)
    pub fn approximate_y_given_x_floating(&self) -> f64 {
        // standard normal distribution...
        let n = Normal::new(0.0, 1.0).unwrap();
        // σ√τ
        let std_dev_sqrt_tau =
            self.std_dev_f * f64::sqrt(self.time_remaining_sec / SECONDS_PER_YEAR);
        // Φ⁻¹(1 - x)
        let invariant_term_x = n.inverse_cdf(1.0 - self.reserve_x_per_wad);
        // y = KΦ(Φ⁻¹(1-x) - σ√τ + k)
        let k = 0.0; // if we are solving for y, k = 0.0
        let y = self.strike_price_f * n.cdf(invariant_term_x - std_dev_sqrt_tau + k);

        y
    }

    /// computes the adjusted trading function x variable.
    /// x = 1 - Φ(Φ⁻¹(y/K) + σ√τ - k)
    pub fn approximate_x_given_y_floating(&self) -> f64 {
        // standard normal distribution...
        let n = Normal::new(0.0, 1.0).unwrap();
        // σ√τ
        let std_dev_sqrt_tau =
            self.std_dev_f * f64::sqrt(self.time_remaining_sec / SECONDS_PER_YEAR);
        // Φ⁻¹(y/K)
        let invariant_term_y = n.inverse_cdf(self.reserve_y_per_wad / self.strike_price_f);
        // x = 1 - Φ(Φ⁻¹(y/K) + σ√τ - k)
        let k = self.trading_function_floating();
        let x = 1.0 - n.cdf(invariant_term_y + std_dev_sqrt_tau - k);

        x
    }

    /// gets the y coordinates for the trading function across the range (0, 1)
    pub fn get_trading_function_coordinates(&self) -> Vec<(f64, f64)> {
        let mut points = Vec::new();

        let mut x = 0.0;
        let mut y = 0.0;

        // can probably clean this up to not need clone
        // maybe needs getters
        let mut copy = self.clone();

        while x < 1.0 {
            copy.reserve_x_per_wad = x;
            y = self.approximate_y_given_x_floating();
            points.push((x, y));
            x += 0.01;
        }

        points
    }

    /// approximates the maximum amount out of a given trade.
    pub fn approximate_amount_out(&self, sell_asset: bool, amount_in: f64) -> f64 {
        if sell_asset {
            let reserve_in = self.reserve_x_per_wad + amount_in;
            let reserve_out = self.approximate_other_reserve(true, reserve_in);
            self.reserve_y_per_wad - reserve_out // current reserve - new reserve
        } else {
            let reserve_in = self.reserve_y_per_wad + amount_in;
            let reserve_out = self.approximate_other_reserve(false, reserve_in);
            self.reserve_x_per_wad - reserve_out // current reserve - new reserve
        }
    }

    /// finds the root such that the invariant is 1e-18 more than the current invariant.
    /// sell_asset - if true, we are increasing the x reserve, else we are increasing the y reserve
    /// amount_in_f - the known x or y reserve value
    pub fn approximate_other_reserve(&self, sell_asset: bool, reserve_in: f64) -> f64 {
        // if sell asset, use the find root swapping x, else use the find root swapping y in the bisection's fx argument

        let mut data = bisection::Bisection::new(0.0, 1.0, 0.0001, 1000.0);

        let mut copy = self.clone();

        if sell_asset {
            copy.reserve_x_per_wad = reserve_in;
            return data.bisection(|x| copy.find_root_swapping_x(x));
        } else {
            copy.reserve_y_per_wad = reserve_in;
            return data.bisection(|x| copy.find_root_swapping_y(x));
        }
    }

    /// finds the root such that the invariant is 1e-18 more than the current invariant.
    /// value - the known x reserve value
    /// returns the y value that would result in the invariant being 1e-18 more than the current invariant.
    pub fn find_root_swapping_x(&self, value: f64) -> f64 {
        let mut copy = self.clone();
        copy.reserve_y_per_wad = value;
        return copy.trading_function_floating() - (self.invariant_f + 1e-18);
    }

    /// finds the root such that the invariant is 1e-18 less than the current invariant.
    /// value - the known y reserve value
    /// returns the x value that would result in the invariant being 1e-18 less than the current invariant.
    pub fn find_root_swapping_y(&self, value: f64) -> f64 {
        let mut copy = self.clone();
        copy.reserve_x_per_wad = value;
        return copy.trading_function_floating() - (self.invariant_f - 1e-18);
    }
}

/// Exposes nice methods to easily graph whatever data!
pub trait Graphable {
    fn y_equals(&self, x: f64) -> f64;
    fn x_equals(&self, y: f64) -> f64;
    fn range_inclusive(&self) -> (f64, f64);
    fn domain_inclusive(&self) -> (f64, f64);
}

/// Allows us to graph the trading function.
impl Graphable for NormalCurve {
    fn y_equals(&self, x: f64) -> f64 {
        let mut copy = self.clone();
        copy.reserve_x_per_wad = x;
        copy.approximate_y_given_x_floating()
    }

    fn x_equals(&self, y: f64) -> f64 {
        let mut copy = self.clone();
        copy.reserve_y_per_wad = y;
        copy.approximate_x_given_y_floating()
    }

    fn range_inclusive(&self) -> (f64, f64) {
        let min = 0.0;
        let max = 1.0;
        (min, max)
    }

    fn domain_inclusive(&self) -> (f64, f64) {
        let min = 0.0;
        let max = self.strike_price_f.clone();
        (min, max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CURVE: NormalCurve = NormalCurve {
        reserve_x_per_wad: 0.308537538726,
        reserve_y_per_wad: 0.308537538726,
        strike_price_f: 1.0,
        std_dev_f: 1.0,
        time_remaining_sec: 31556953.0,
        invariant_f: 0.0,
    };

    #[test]
    fn math_trading_function_floating() {
        let k = CURVE.clone().trading_function_floating();
        assert_eq!(k, 0.00000000000007427392034742297);
    }

    #[test]
    fn math_approximate_amount_out() {
        let amount_in = 0.1;
        let sell_asset = true;
        let amount_out = CURVE.clone().approximate_amount_out(sell_asset, amount_in);
        assert!(amount_out < 1.0); // price should go down...
    }
}
