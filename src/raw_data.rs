use arbiter::utils::{float_to_wad, wad_to_float};
use ethers::{
    prelude::{I256, U256},
    utils::parse_ether,
};
/// Implements the storage of raw simulation data.
use std::collections::HashMap;

use bindings::{i_portfolio::*, normal_strategy::ConfigsReturn};

/// # RawData
/// ==================
/// This is the storage of raw simulation data. All direct
/// calls to the underlying db will have their results in this struct.
///
/// # Arguments
/// ==================
/// * keys - Stores the series time keys, which are pool ids.
/// * arbitrageur_balances_wad - Stores the arbitrageur's balances in wad format.
/// * exchange_prices_wad - Stores the series exchange prices in wad format, indexed by the pool id.
/// * pools - Stores the series pool data, indexed by the pool id.
pub struct RawData {
    pub keys: Vec<u64>,
    pub arbitrageur_balances_wad: HashMap<String, Vec<U256>>,
    pub exchange_prices_wad: HashMap<u64, Vec<U256>>,
    pub pools: HashMap<u64, PoolSeries>,
    pub derived_data: HashMap<u64, DerivedData>,
    pub configs: HashMap<u64, PoolConfig>,
}

pub struct DerivedData {
    pub arbitrageur_portfolio_value: Vec<f64>,
    pub pool_portfolio_value: Vec<f64>,
}

impl Default for DerivedData {
    fn default() -> Self {
        Self {
            arbitrageur_portfolio_value: Vec::new(),
            pool_portfolio_value: Vec::new(),
        }
    }
}

/// Aliased type from the actual config stored in the pool's strategy contract.
/// source: normal_strategy.rs
pub type PoolConfig = ConfigsReturn;

/// # PoolSeries
/// Stores the timeseries data for an individual pool.
///
/// # Fields
/// * `pool_data` - Return value from calling `pools(uint64 poolId)` on portfolio.
/// * `reported_price_wad_sol` - Reported price of the pool, in wad format.
/// * `invariant_wad_sol` - Invariant value of the pool, in wad format.
/// * `portfolio_value_wad_sol` - Portfolio value function is the sum of the value of tokens, in wad format.
pub struct PoolSeries {
    pub pool_data: Vec<PoolsReturn>,
    pub reported_price_wad_sol: Vec<U256>,
    pub invariant_wad_sol: Vec<I256>,
    pub portfolio_value_wad_sol: Vec<U256>,
}

impl Default for PoolSeries {
    fn default() -> Self {
        Self {
            pool_data: Vec::new(),
            reported_price_wad_sol: Vec::new(),
            invariant_wad_sol: Vec::new(),
            portfolio_value_wad_sol: Vec::new(),
        }
    }
}

/// Implements the raw data type with
/// methods to easily handle the data.
#[allow(unused)]
impl RawData {
    pub fn new() -> Self {
        RawData {
            keys: Vec::new(),
            arbitrageur_balances_wad: HashMap::new(),
            exchange_prices_wad: HashMap::new(),
            pools: HashMap::new(),
            derived_data: HashMap::new(),
            configs: HashMap::new(),
        }
    }

    pub fn add_config(&mut self, key: u64, config: PoolConfig) {
        self.configs.insert(key, config);
    }

    pub fn add_key(&mut self, key: u64) {
        self.keys.push(key);
    }

    pub fn add_arbitrageur_balance(&mut self, key: String, balance: U256) {
        self.arbitrageur_balances_wad
            .entry(key)
            .or_insert_with(Vec::new)
            .push(balance);
    }

    pub fn add_exchange_price(&mut self, key: u64, price: U256) {
        self.exchange_prices_wad
            .entry(key)
            .or_insert_with(Vec::new)
            .push(price);
    }

    pub fn add_pool_data(&mut self, key: u64, pool_data: PoolsReturn) {
        self.pools
            .entry(key)
            .or_insert_with(PoolSeries::default)
            .pool_data
            .push(pool_data);
    }

    pub fn add_reported_price(&mut self, key: u64, price: U256) {
        self.pools
            .entry(key)
            .or_insert_with(PoolSeries::default)
            .reported_price_wad_sol
            .push(price);
    }

    pub fn add_invariant(&mut self, key: u64, invariant: I256) {
        self.pools
            .entry(key)
            .or_insert_with(PoolSeries::default)
            .invariant_wad_sol
            .push(invariant);
    }

    pub fn add_portfolio_value(&mut self, key: u64, value: U256) {
        self.pools
            .entry(key)
            .or_insert_with(PoolSeries::default)
            .portfolio_value_wad_sol
            .push(value);
    }

    pub fn add_arbitrageur_portfolio_value(&mut self, key: u64, value: f64) {
        self.derived_data
            .entry(key)
            .or_insert_with(DerivedData::default)
            .arbitrageur_portfolio_value
            .push(value);
    }

    pub fn add_pool_portfolio_value(&mut self, key: u64, value: f64) {
        self.derived_data
            .entry(key)
            .or_insert_with(DerivedData::default)
            .pool_portfolio_value
            .push(value);
    }

    pub fn get_arbitrageur_balance(&self, key: &str) -> Vec<U256> {
        self.arbitrageur_balances_wad.get(key).unwrap().clone()
    }

    pub fn get_exchange_price(&self, key: u64) -> Vec<U256> {
        self.exchange_prices_wad.get(&key).unwrap().clone()
    }

    pub fn get_pool_data(&self, key: u64) -> Vec<PoolsReturn> {
        self.pools.get(&key).unwrap().pool_data.clone()
    }

    pub fn get_pool_x_per_lq_float(&self, key: u64) -> Vec<f64> {
        self.get_pool_data(key).map_x_per_lq().vec_wad_to_float()
    }

    pub fn get_pool_y_per_lq_float(&self, key: u64) -> Vec<f64> {
        self.get_pool_data(key).map_y_per_lq().vec_wad_to_float()
    }

    pub fn get_reported_price(&self, key: u64) -> Vec<U256> {
        self.pools.get(&key).unwrap().reported_price_wad_sol.clone()
    }

    pub fn get_invariant(&self, key: u64) -> Vec<I256> {
        self.pools.get(&key).unwrap().invariant_wad_sol.clone()
    }

    // note: portfolio value is translated to float in the data collection stage
    // @kinrezc be careful about where you are converting data. in general,
    // we store raw EVM data in the raw data and do the conversions outside.
    pub fn get_portfolio_value(&self, key: u64) -> Vec<f64> {
        self.derived_data
            .get(&key)
            .unwrap()
            .pool_portfolio_value
            .clone()
    }

    pub fn get_arbitrageur_balance_float(&self, key: &str) -> Vec<f64> {
        self.get_arbitrageur_balance(key).vec_wad_to_float()
    }

    pub fn get_exchange_price_float(&self, key: u64) -> Vec<f64> {
        self.get_exchange_price(key).vec_wad_to_float()
    }

    pub fn get_reported_price_float(&self, key: u64) -> Vec<f64> {
        self.get_reported_price(key).vec_wad_to_float()
    }

    pub fn get_invariant_float(&self, key: u64) -> Vec<f64> {
        self.get_invariant(key).vec_wad_to_float()
    }

    pub fn get_portfolio_value_float(&self, key: u64) -> Vec<f64> {
        self.get_portfolio_value(key)
    }

    /// Balance of arbitrageur's "token0", or x, tokens.
    pub fn get_arber_reserve_x_float(&self) -> Vec<f64> {
        // todo: fix token0 getter so we know its the right x token for a given pool...
        self.get_arbitrageur_balance_float("token0")
    }

    /// Balance of arbitrageur's "token1", or y, tokens.
    pub fn get_arber_reserve_y_float(&self) -> Vec<f64> {
        self.get_arbitrageur_balance_float("token1")
    }

    /// Gets the portfolio value of the arbitrageur, which is the sum of its value of token reserves.
    pub fn get_arber_portfolio_value_float(&self, pool_id: u64) -> Vec<f64> {
        self.derived_data
            .get(&pool_id)
            .unwrap()
            .arbitrageur_portfolio_value
            .clone()
    }
}

impl Default for RawData {
    fn default() -> Self {
        Self::new()
    }
}

/// # WadToFloat
/// Converts wad integers into floats.
pub trait WadToFloat {
    fn vec_wad_to_float(&self) -> Vec<f64>;
}

/// # FloatToWad
/// Converts floats into wad integers.
pub trait FloatToWad {
    fn vec_float_to_wad(&self) -> Vec<U256>;
}

impl WadToFloat for Vec<U256> {
    fn vec_wad_to_float(&self) -> Vec<f64> {
        self.clone().into_iter().map(wad_to_float).collect()
    }
}

impl FloatToWad for Vec<f64> {
    fn vec_float_to_wad(&self) -> Vec<U256> {
        self.clone().into_iter().map(float_to_wad).collect()
    }
}

impl WadToFloat for Vec<I256> {
    fn vec_wad_to_float(&self) -> Vec<f64> {
        self.clone()
            .into_iter()
            .map(|x| x.as_i128() as f64)
            .collect()
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
        RAW_.add_pool_data(0, pool_data.clone());
        // get x per lq
        let x_per_lq = RAW_.pools.get(&0_u64).unwrap().pool_data.clone();
        let x_per_lq = x_per_lq.map_x_per_lq();
        // convert to floats
        let x_per_lq_float = x_per_lq.vec_wad_to_float();
        assert_eq!(x_per_lq_float, vec![1.0]);
    }
}
