use clap::Parser;
use ethers::types::I256;
use serde::{Deserialize, Serialize};

use arbiter::{manager::SimulationManager, utils::*};
use ethers::abi::Tokenize;
use ethers::core::utils;
use ethers::prelude::U256;

use super::{
    calls::{Caller, DecodedReturns},
    raw_data::*,
};

// dynamic, must be built wth ./build.sh or forge bind.
use bindings::i_portfolio::PoolsReturn;

/// Defines the output file directory and name for the plots and csv data.
#[derive(Clone, Parser, Serialize, Deserialize, Debug)]
pub struct OutputStorage {
    pub output_path: String,
    pub output_file_names: String,
}

/// # Log::Run
/// Fetches the raw simulation data and records
/// it to the raw_data container.
///
/// # Data collected
/// - Arbitrageur balances for each token
/// - Portfolio pool data
/// - Portfolio reported price
/// - Exchange price
///
/// # Notes
/// - Must log an entry for each series point so all vectors are equal in length!
pub fn run(
    manager: &SimulationManager,
    raw_data_container: &mut RawData,
    pool_id: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let admin = manager.agents.get("admin").unwrap();
    let arbitrageur = manager.agents.get("arbitrageur").unwrap();
    let token0 = manager.deployed_contracts.get("token0").unwrap();
    let token1 = manager.deployed_contracts.get("token1").unwrap();

    // Gracefully handles REVM calls for us.
    let mut graceful = Caller::new(admin);
    let mut graceful_arber = Caller::new(arbitrageur);

    // 1. Edit the arb balances
    let token_key_0 = "token0".to_string();
    let token_key_1 = "token1".to_string();
    let arbitrageur_balance_0 = graceful_arber.balance_of(token0).decoded(&token0)?;
    let arbitrageur_balance_1 = graceful_arber.balance_of(token1).decoded(&token1)?;
    raw_data_container.add_arbitrageur_balance(token_key_0, arbitrageur_balance_0);
    raw_data_container.add_arbitrageur_balance(token_key_1, arbitrageur_balance_1);

    // 2. Edit the exchange price
    let exchange = manager.deployed_contracts.get("exchange").unwrap();
    let exchange_price = graceful
        .call(
            exchange,
            "getPrice",
            recast_address(token0.address).into_tokens(),
        )?
        .decoded(exchange)?;
    raw_data_container.add_exchange_price(pool_id, exchange_price);

    let price_token0 = utils::format_units(exchange_price, "ether")?.parse::<f64>()?;
    let price_token1 = 1.0 / price_token0;

    let arb_balance_token0_float =
        utils::format_units(arbitrageur_balance_0, "ether")?.parse::<f64>()?;
    let arb_balance_token1_float =
        utils::format_units(arbitrageur_balance_1, "ether")?.parse::<f64>()?;

    let portfolio_value =
        arb_balance_token0_float * price_token0 + arb_balance_token1_float * price_token1;

    raw_data_container.add_arbitrageur_portfolio_value(pool_id, portfolio_value);

    // 3a. Edit portfolio pool data
    let portfolio = manager.deployed_contracts.get("portfolio").unwrap();
    let pool_data: PoolsReturn = graceful
        .call(portfolio, "pools", pool_id.into_tokens())?
        .decoded(portfolio)?;

    let pool_reserve_x = utils::format_units(pool_data.virtual_x, "ether")?.parse::<f64>()?;
    let pool_reserve_y = utils::format_units(pool_data.virtual_y, "ether")?.parse::<f64>()?;

    let pool_value = pool_reserve_x * price_token0 + pool_reserve_y * price_token1;

    raw_data_container.add_pool_portfolio_value(pool_id, pool_value);
    raw_data_container.add_pool_data(pool_id, pool_data);

    // 3b. Edit portfolio reported price
    let portfolio_prices = graceful
        .call(portfolio, "getSpotPrice", pool_id.into_tokens())?
        .decoded(portfolio)?;
    raw_data_container.add_reported_price(pool_id, portfolio_prices);

    // 3c. Edit portfolio invariant
    let portfolio_invariant: I256 = I256::zero(); // todo: get actual invariant
    raw_data_container.add_invariant(pool_id, portfolio_invariant);

    // 3d. Edit portfolio value
    let portfolio_value = U256::zero(); // todo: get actual portfolio value
    raw_data_container.add_portfolio_value(pool_id, portfolio_value);

    Ok(())
}
