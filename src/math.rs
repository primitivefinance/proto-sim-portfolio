/// Implements the portfolio "Normal Strategy" math functions in rust.
use ethers::prelude::*;
use statrs::distribution::{ContinuousCDF, Normal};
use statrs::statistics::Distribution;
use std::num;

pub static SECONDS_PER_YEAR: f64 = 31556953.0;

#[derive(Clone)]
pub struct NormalCurve {
    pub reserve_x_per_wad: f64,
    pub reserve_y_per_wad: f64,
    pub strike_price_f: f64,
    pub std_dev_f: f64,
    pub time_remaining_sec: f64,
    pub invariant_f: f64,
}

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
/// fixed point arithmetic?
pub fn trading_function_floating(curve: NormalCurve) -> f64 {
    // standard normal distribution...
    let n = Normal::new(0.0, 1.0).unwrap();
    // σ√τ
    let std_dev_sqrt_tau = curve.std_dev_f * f64::sqrt(curve.time_remaining_sec / SECONDS_PER_YEAR);
    // Φ⁻¹(1 - x)
    let invariant_term_x = n.inverse_cdf(1.0 - curve.reserve_x_per_wad);
    // Φ⁻¹(y/K)
    let invariant_term_y = n.inverse_cdf(curve.reserve_y_per_wad / curve.strike_price_f);
    println!("invariant_term_x: {}", invariant_term_x);
    println!("invariant_term_y: {}", invariant_term_y);
    println!("std_dev_sqrt_tau: {}", std_dev_sqrt_tau);
    // k = Φ⁻¹(y/K) - Φ⁻¹(1-x) + σ√τ
    let k = invariant_term_y - invariant_term_x + std_dev_sqrt_tau;

    k
}

/// y = KΦ(Φ⁻¹(1-x) - σ√τ + k)
pub fn approximate_y_given_x_floating(curve: NormalCurve) -> f64 {
    // standard normal distribution...
    let n = Normal::new(0.0, 1.0).unwrap();
    // σ√τ
    let std_dev_sqrt_tau = curve.std_dev_f * f64::sqrt(curve.time_remaining_sec / SECONDS_PER_YEAR);
    // Φ⁻¹(1 - x)
    let invariant_term_x = n.inverse_cdf(1.0 - curve.reserve_x_per_wad);
    // y = KΦ(Φ⁻¹(1-x) - σ√τ + k)
    let k = 0.0; // if we are solving for y, k = 0.0
    let y = curve.strike_price_f * n.cdf(invariant_term_x - std_dev_sqrt_tau + k);

    y
}

pub fn get_trading_function_coordinates(curve: NormalCurve) -> Vec<(f64, f64)> {
    let mut points = Vec::new();

    let mut x = 0.0;
    let mut y = 0.0;

    let mut copy = curve.clone();

    while x < 1.0 {
        copy.reserve_x_per_wad = x;
        y = approximate_y_given_x_floating(copy.clone());
        points.push((x, y));
        x += 0.01;
    }

    points
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn math_trading_function_floating() {
        let curve = NormalCurve {
            reserve_x_per_wad: 0.308537538726,
            reserve_y_per_wad: 0.308537538726,
            strike_price_f: 1.0,
            std_dev_f: 1.0,
            time_remaining_sec: 31556953.0,
            invariant_f: 0.0,
        };

        let k = super::trading_function_floating(curve);
        assert_eq!(k, 0.00000000000007427392034742297);
    }
}
