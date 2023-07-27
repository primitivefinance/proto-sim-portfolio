use arbiter::utils::{float_to_wad, wad_to_float};
use ethers::{
    prelude::{I256, U256},
    utils::{parse_ether, parse_units},
};
/// Implements the storage of raw simulation data.
use std::collections::HashMap;

use bindings::i_portfolio::*;

/// # RawData
/// ==================
/// This is the storage of raw simulation data. All direct
/// calls to the underlying db will have their results in this struct.
///
/// # Arguments
/// ==================
/// * `pool_data` - A hashmap of pool data, keyed by pool id.
/// * `exchange_prices_wad` - A vector of exchange prices, in wad format.
/// * `arbitrageur_balances_wad` - A hashmap of arbitrageur balances, keyed by token name.
/// * `reported_price_wad_sol` - A vector of reported prices, in wad format.
/// * `invariant_wad_sol` - A vector of invariant values, in wad format.
/// * `portfolio_value_wad_sol` - Portfolio value function is the sum of the value of tokens, in wad format.
pub struct RawData {
    pub pool_data: HashMap<u64, PoolsReturn>,
    pub exchange_prices_wad: Vec<U256>,
    pub arbitrageur_balances_wad: HashMap<String, Vec<U256>>,
    pub reported_price_wad_sol: Vec<U256>,
    pub invariant_wad_sol: Vec<I256>,
    pub portfolio_value_wad_sol: Vec<U256>,
}

/// Implements the raw data type with
/// methods to easily handle the data.
impl RawData {
    pub fn new() -> Self {
        Self {
            pool_data: HashMap::new(),
            exchange_prices_wad: Vec::new(),
            arbitrageur_balances_wad: HashMap::new(),
            reported_price_wad_sol: Vec::new(),
            invariant_wad_sol: Vec::new(),
            portfolio_value_wad_sol: Vec::new(),
        }
    }
}

/// # WadToFloat
/// Converts wad integers into floats.
pub trait WadToFloat {
    fn wad_to_float(&self) -> Vec<f64>;
}

/// # FloatToWad
/// Converts floats into wad integers.
pub trait FloatToWad {
    fn float_to_wad(&self) -> Vec<U256>;
}

impl WadToFloat for Vec<U256> {
    fn wad_to_float(&self) -> Vec<f64> {
        self.clone().into_iter().map(wad_to_float).collect()
    }
}

impl FloatToWad for Vec<f64> {
    fn float_to_wad(&self) -> Vec<U256> {
        self.clone().into_iter().map(float_to_wad).collect()
    }
}

pub trait PoolTransformers {
    fn map_x_total(&self) -> Vec<U256>;
    fn map_y_total(&self) -> Vec<U256>;
    fn map_x_per_lq(&self) -> Vec<U256>;
    fn map_y_per_lq(&self) -> Vec<U256>;
}

impl PoolTransformers for Vec<PoolsReturn> {
    fn map_x_total(&self) -> Vec<U256> {
        self.clone()
            .into_iter()
            .map(|p: PoolsReturn| U256::from(p.virtual_x))
            .into_iter()
            .collect()
    }

    fn map_y_total(&self) -> Vec<U256> {
        self.clone()
            .into_iter()
            .map(|p: PoolsReturn| U256::from(p.virtual_y))
            .into_iter()
            .collect()
    }

    fn map_x_per_lq(&self) -> Vec<U256> {
        // gets the x total mapping, then gets the self.liquidity, then multiplies each x by 1e18 and divides by liquidity.
        let x_total = self.map_x_total();
        let liquidity = self
            .clone()
            .into_iter()
            .map(|p: PoolsReturn| U256::from(p.liquidity));

        x_total
            .into_iter()
            .zip(liquidity)
            .map(|(x, lq)| {
                x.checked_mul(parse_ether(1.0).unwrap())
                    .unwrap()
                    .checked_div(lq)
                    .unwrap()
            })
            .into_iter()
            .collect()
    }

    fn map_y_per_lq(&self) -> Vec<U256> {
        // gets the y total mapping, then gets the self.liquidity, then multiplies each y by 1e18 and divides by liquidity.
        let y_total = self.map_y_total();
        let liquidity = self
            .clone()
            .into_iter()
            .map(|p: PoolsReturn| U256::from(p.liquidity));

        y_total
            .into_iter()
            .zip(liquidity)
            .map(|(y, lq)| {
                y.checked_mul(parse_ether(1.0).unwrap())
                    .unwrap()
                    .checked_div(lq)
                    .unwrap()
            })
            .into_iter()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::PoolTransformers;
    use super::*;
    use bindings::i_portfolio::PoolsReturn;

    #[test]
    fn raw_data_to_floats() {
        let mut RAW_: RawData = RawData::new();
        // insert raw pool data with virtual x = 1 wad and liquidity = 1 wad
        // then call the map x per lq to get the x per l vector

        let pool_data = PoolsReturn {
            virtual_x: 1,
            virtual_y: 1,
            liquidity: 1,
            fee_basis_points: 0,
            priority_fee_basis_points: 0,
            last_timestamp: 0,
            controller: ethers::types::H160::zero(),
            strategy: ethers::types::H160::zero(),
        };

        // insert the pool data into the raw data storage
        RAW_.pool_data.insert(0, pool_data);
        // get x per lq
        let x_per_lq = vec![RAW_.pool_data.get(&0).unwrap().clone()];
        let x_per_lq = x_per_lq.map_x_per_lq();
        // convert to floats
        let x_per_lq_float = x_per_lq.wad_to_float();
        assert_eq!(x_per_lq_float, vec![1.0]);
    }
}
