/// Implements the storage of raw simulation data.
use std::collections::HashMap;
use ethers::prelude::{U256, I256};
use arbiter::utils::{wad_to_float, float_to_wad};

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

/// # RawTransformers
/// ==================
/// This trait is used to transform raw data into
/// a more usable format. We use f64 types
/// for most of our graphing and analysis.
pub trait RawTransformers {
    fn wad_to_float(&self) -> Vec<f64>;
    fn float_to_wad(&self) -> Vec<U256>;
}

impl RawTransformers for Vec<U256> {
    // todo: dont think this should impl this method
    fn float_to_wad(&self) -> Vec<U256> {
        self.clone()
    }
    fn wad_to_float(&self) -> Vec<f64> {
        self.clone().into_iter().map(wad_to_float).collect()
    }
}

impl RawTransformers for Vec<f64> {
    fn float_to_wad(&self) -> Vec<U256> {
        self.clone().into_iter().map(float_to_wad).collect()
    }
    // todo: dont think this should impl this method
    fn wad_to_float(&self) -> Vec<f64> {
        self.clone()
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
        let mut arr = Vec::<U256>::new();
        self.clone().into_iter().map(|p: PoolsReturn| p.virtual_x).map(|x: u128| arr.push(U256::from(x)));
        arr
    }

    fn map_y_total(&self) -> Vec<U256> {
        let mut arr = Vec::<U256>::new();
        self.clone().into_iter().map(|p: PoolsReturn| p.virtual_y).map(|y: u128| arr.push(U256::from(y)));
        arr
    }

    fn map_x_per_lq(&self) -> Vec<U256> {
        // gets the x total mapping, then gets the self.liquidity, then multiplies each x by 1e18 and divides by liquidity.
        let mut arr = Vec::<U256>::new();
        let x_total = self.map_x_total();
        let liquidity = self.clone().into_iter().map(|p: PoolsReturn| p.liquidity);
        x_total.into_iter().zip(liquidity).map(|(x, lq)| arr.push(x * (float_to_wad(1.0)) / (lq)));
        arr
    }

    fn map_y_per_lq(&self) -> Vec<U256> {
        // multiplies by 1E18 then divides by liquidity.
        let mut arr = Vec::<U256>::new();
        let y_total = self.map_y_total();
        let liquidity = self.clone().into_iter().map(|p: PoolsReturn| p.liquidity);
        y_total.into_iter().zip(liquidity).map(|(y, lq)| arr.push(y * (float_to_wad(1.0)) / (lq)));
        arr
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use super::PoolTransformers;
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