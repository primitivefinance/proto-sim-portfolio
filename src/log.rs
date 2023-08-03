use clap::Parser;
use ethers::types::I256;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::{error::Error, fs::File};

use arbiter::{
    agent::*, environment::contract::*, manager::SimulationManager,
    stochastic::price_process::PriceProcess, utils::*,
};
use ethers::abi::Tokenize;
use ethers::core::utils;
use ethers::prelude::U256;
use polars::prelude::*;
use revm::primitives::Address;
use visualize::{design::*, plot::*};

use super::{
    calls::{Caller, DecodedReturns},
    math,
    raw_data::*,
    spreadsheetorizer::*,
};

// dynamic... generated with build.sh
use bindings::{external_normal_strategy_lib, i_portfolio::*};

pub static OUTPUT_DIRECTORY: &str = "out_data";
pub static OUTPUT_FILE_NAME: &str = "results";

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
    let arbitrageur_balance_0 = graceful_arber.balance_of(token0).decoded(&token0).unwrap();
    let arbitrageur_balance_1 = graceful_arber.balance_of(token1).decoded(&token1).unwrap();
    raw_data_container.add_arbitrageur_balance(token_key_0, arbitrageur_balance_0);
    raw_data_container.add_arbitrageur_balance(token_key_1, arbitrageur_balance_1);

    // 2. Edit the exchange price
    let exchange = manager.deployed_contracts.get("exchange").unwrap();
    let exchange_price = get_reference_price(admin, exchange, token0.address)?;
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
    let pool_data = get_pool(admin, portfolio, pool_id)?;

    let pool_reserve_x = utils::format_units(pool_data.virtual_x, "ether")?.parse::<f64>()?;
    let pool_reserve_y = utils::format_units(pool_data.virtual_y, "ether")?.parse::<f64>()?;

    let pool_value = pool_reserve_x * price_token0 + pool_reserve_y * price_token1;

    raw_data_container.add_pool_portfolio_value(pool_id, pool_value);
    raw_data_container.add_pool_data(pool_id, pool_data);

    // 3b. Edit portfolio reported price
    let portfolio_prices = get_portfolio_prices(admin, portfolio, pool_id)?;
    raw_data_container.add_reported_price(pool_id, portfolio_prices);

    // 3c. Edit portfolio invariant
    let portfolio_invariant: I256 = I256::zero(); // todo: get actual invariant
    raw_data_container.add_invariant(pool_id, portfolio_invariant);

    // 3d. Edit portfolio value
    let portfolio_value = U256::zero(); // todo: get actual portfolio value
    raw_data_container.add_portfolio_value(pool_id, portfolio_value);

    Ok(())
}

pub fn approximate_y_given_x(
    admin: &AgentType<IsActive>,
    library: &SimulationContract<IsDeployed>,
    curve: math::NormalCurve,
) -> Result<ethers::types::U256, Box<dyn std::error::Error>> {
    let arguments: external_normal_strategy_lib::NormalCurve =
        external_normal_strategy_lib::NormalCurve {
            reserve_x_per_wad: float_to_wad(curve.reserve_x_per_wad),
            reserve_y_per_wad: float_to_wad(curve.reserve_y_per_wad),
            strike_price_wad: float_to_wad(curve.strike_price_f),
            standard_deviation_wad: float_to_wad(curve.std_dev_f),
            time_remaining_seconds: (curve.time_remaining_sec as u32).into(),
            invariant: (0).into(),
        };
    let result = admin.call(
        library,
        "approximateYGivenX",
        external_normal_strategy_lib::ApproximateYGivenXCall { self_: arguments }.into_tokens(),
    )?;
    let decoded: ethers::types::U256 =
        library.decode_output("approximateYGivenX", unpack_execution(result)?)?;
    Ok(decoded)
}

pub fn trading_function(
    admin: &AgentType<IsActive>,
    library: &SimulationContract<IsDeployed>,
    curve: math::NormalCurve,
) -> Result<ethers::types::I256, Box<dyn std::error::Error>> {
    let arguments: external_normal_strategy_lib::NormalCurve =
        external_normal_strategy_lib::NormalCurve {
            reserve_x_per_wad: float_to_wad(curve.reserve_x_per_wad).into(),
            reserve_y_per_wad: float_to_wad(curve.reserve_y_per_wad).into(),
            strike_price_wad: float_to_wad(curve.strike_price_f).into(),
            standard_deviation_wad: float_to_wad(curve.std_dev_f).into(),
            time_remaining_seconds: (curve.time_remaining_sec as u32).into(),
            invariant: (0).into(),
        };
    let result = admin.call(library, "tradingFunction", arguments.into_tokens())?;
    let decoded: ethers::types::I256 =
        library.decode_output("tradingFunction", unpack_execution(result)?)?;
    Ok(decoded)
}

pub fn get_configuration(
    admin: &AgentType<IsActive>,
    external_normal_strategy: &SimulationContract<IsDeployed>,
    pool_id: u64,
) -> Result<external_normal_strategy_lib::NormalCurve, Box<dyn std::error::Error>> {
    let result = admin.call(
        external_normal_strategy,
        "getCurveConfiguration",
        pool_id.into_tokens(),
    )?;
    let pool_return: external_normal_strategy_lib::NormalCurve = external_normal_strategy
        .decode_output("getCurveConfiguration", unpack_execution(result)?)?;
    Ok(pool_return)
}

/// Calls portfolio.pools
fn get_pool(
    admin: &AgentType<IsActive>,
    portfolio: &SimulationContract<IsDeployed>,
    pool_id: u64,
) -> Result<PoolsReturn, Box<dyn std::error::Error>> {
    let result = admin.call(portfolio, "pools", pool_id.into_tokens())?;
    let pool_return: PoolsReturn = portfolio.decode_output("pools", unpack_execution(result)?)?;
    Ok(pool_return)
}

fn get_portfolio_prices(
    admin: &AgentType<IsActive>,
    portfolio: &SimulationContract<IsDeployed>,
    pool_id: u64,
) -> Result<U256, Box<dyn std::error::Error>> {
    let result = admin.call(portfolio, "getSpotPrice", pool_id.into_tokens())?;
    let portfolio_price: U256 =
        portfolio.decode_output("getSpotPrice", unpack_execution(result)?)?;
    Ok(portfolio_price)
}

/// Calls token.balanceOf
fn get_balance(
    admin: &AgentType<IsActive>,
    token: &SimulationContract<IsDeployed>,
    address: Address,
) -> Result<U256, Box<dyn std::error::Error>> {
    let result = admin.call(token, "balanceOf", recast_address(address).into_tokens())?;
    let balance: U256 = token.decode_output("balanceOf", unpack_execution(result)?)?;
    Ok(balance)
}

/// Calls exchange.getPrice
fn get_reference_price(
    admin: &AgentType<IsActive>,
    exchange: &SimulationContract<IsDeployed>,
    token: Address,
) -> Result<U256, Box<dyn std::error::Error>> {
    let result = admin.call(exchange, "getPrice", recast_address(token).into_tokens())?;
    let reference_price: U256 = exchange.decode_output("getPrice", unpack_execution(result)?)?;
    Ok(reference_price)
}

/// Defines the output file directory and name for the plots and csv data.
#[derive(Clone, Parser, Serialize, Deserialize, Debug)]
pub struct OutputStorage {
    pub output_path: String,
    pub output_file_names: String,
}

pub fn plot_trading_curve(display: Display, curves: Vec<Curve>) {
    // plot the trading curve coordinates using transparent_plot
    let title: String = String::from("Trading Curve");

    if let Some(last_point) = curves[0].x_coordinates.clone().last() {
        let min_y = curves
            .iter()
            .flat_map(|curve| &curve.y_coordinates)
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap(); // assumes no NANs
        let max_y = curves
            .iter()
            .flat_map(|curve| &curve.y_coordinates)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap(); // assumes no NANs

        println!("min_y: {}", min_y);
        println!("max_y: {}", max_y);

        let axes = Axes {
            x_label: String::from("X"),
            y_label: String::from("Y"), // todo: add better y label
            bounds: (vec![0.0, last_point.clone()], vec![*min_y, *max_y]),
        };

        // Plot it.
        transparent_plot(
            Some(curves),
            None,
            axes,
            title,
            display,
            Some(format!(
                "{}/trading_curve.html",
                OUTPUT_DIRECTORY.to_string()
            )),
        );
    } else {
        println!("x coords are empty");
    }
}
