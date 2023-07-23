use clap::Parser;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::{error::Error, fs::File};

use arbiter::{
    agent::*,
    environment::contract::*,
    manager::SimulationManager,
    stochastic::price_process::{PriceProcess, PriceProcessType, GBM, OU},
    utils::*,
};
use ethers::abi::Tokenize;
use ethers::prelude::k256::sha2::digest::Output;
use ethers::prelude::U256;
use polars::prelude::*;
use revm::primitives::Address;

// dynamic... generated with build.sh
use bindings::i_portfolio_getters::*;

/// Struct for storing simulation data
pub struct SimData {
    pub pool_data: Vec<PoolsReturn>,
    pub actor_balances: Vec<HashMap<u64, U256>>, // maps token index (0 or 1) => balance
    pub reference_prices: Vec<U256>,
    pub portfolio_prices: Vec<U256>,
}

// Path: src/log.rs
//
// @notice
// Data collection is handled before a step in the simulation, so it starts at zero.
//
// @dev
// Collected data:
// 1. Pool data
// 2. Actor token balances
// 3. Reference market prices
pub fn run(
    manager: &SimulationManager,
    sim_data: &mut SimData,
    pool_id: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let admin = manager.agents.get("admin").unwrap();
    let actor = manager.deployed_contracts.get("actor").unwrap();
    let token0 = manager.deployed_contracts.get("token0").unwrap();
    let token1 = manager.deployed_contracts.get("token1").unwrap();

    let actor_balance_0 = get_balance(admin, token0, actor.address)?;
    let actor_balance_1 = get_balance(admin, token1, actor.address)?;
    let mut actor_balance = HashMap::new();
    actor_balance.insert(0, actor_balance_0);
    actor_balance.insert(1, actor_balance_1);
    sim_data.actor_balances.push(actor_balance);

    let portfolio = manager.deployed_contracts.get("portfolio").unwrap();
    let pool_data = get_pool(admin, portfolio, pool_id)?;
    sim_data.pool_data.push(pool_data);

    let portfolio_prices = get_portfolio_prices(admin, portfolio, pool_id)?;
    sim_data.portfolio_prices.push(portfolio_prices);

    let exchange = manager.deployed_contracts.get("exchange").unwrap();
    let exchange_price = get_reference_price(admin, exchange, token0.address)?;
    sim_data.reference_prices.push(exchange_price);

    Ok(())
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

#[derive(Clone, Parser, Serialize, Deserialize, Debug)]
pub struct OutputStorage {
    pub output_path: String,
    pub output_file_names: String,
}

pub fn write_to_file(
    price_process: PriceProcess,
    data: &mut SimData,
) -> Result<(), Box<dyn Error>> {
    let output = OutputStorage {
        output_path: String::from("output"),
        output_file_names: String::from("portfolio"),
    };

    let series_length = data.pool_data.len();
    let seed = Series::new("seed", vec![price_process.seed; series_length]);
    let timestep = Series::new("timestep", vec![price_process.timestep; series_length]);

    let mut dataframe = make_series(data).unwrap();

    match price_process.process_type {
        PriceProcessType::OU(OU {
            volatility,
            mean_reversion_speed,
            mean_price,
        }) => {
            let volatility = Series::new("drift", vec![volatility; series_length]);
            let mean_reversion_speed = Series::new(
                "mean_reversion_speed",
                vec![mean_reversion_speed; series_length],
            );
            let mean_price = Series::new("mean_price", vec![mean_price; series_length]);

            dataframe.hstack_mut(&[
                volatility,
                timestep,
                seed,
                mean_reversion_speed,
                mean_price,
            ])?;

            println!("Dataframe: {:#?}", dataframe);
            let volatility = match price_process.process_type {
                PriceProcessType::GBM(GBM { volatility, .. }) => volatility,
                PriceProcessType::OU(OU { volatility, .. }) => volatility,
            };
            let file = File::create(format!(
                "{}/{}_{}_{}.csv",
                output.output_path, output.output_file_names, volatility, 0
            ))?;
            let mut writer = CsvWriter::new(file);
            writer.finish(&mut dataframe)?;
        }
        _ => {
            //na
        }
    };

    Ok(())
}

fn make_series(data: &mut SimData) -> Result<DataFrame, Box<dyn std::error::Error>> {
    // converts data.reference_prices to a float in a vector

    let exchange_prices = data
        .reference_prices
        .clone()
        .into_iter()
        .map(wad_to_float)
        .collect::<Vec<f64>>();

    // converts data.portfolio_prices to a float in a vector
    let portfolio_prices = data
        .portfolio_prices
        .clone()
        .into_iter()
        .map(wad_to_float)
        .collect::<Vec<f64>>();

    // converts each data.pool_data.virtualX to a float in a vector
    let reserve_x = data
        .pool_data
        .clone()
        .into_iter()
        .map(|x| U256::from(x.virtual_x))
        .into_iter()
        .map(wad_to_float)
        .collect::<Vec<f64>>();

    // converts each data.pool_data.virtualY to a float in a vector
    let reserve_y = data
        .pool_data
        .clone()
        .into_iter()
        .map(|y| U256::from(y.virtual_y))
        .map(wad_to_float)
        .collect::<Vec<f64>>();

    // converts data.actor_balances.get(token0.address) to a float in a vector
    let arb_x = data
        .actor_balances
        .clone()
        .into_iter()
        .map(|x| *x.get(&0).unwrap())
        .map(wad_to_float)
        .collect::<Vec<f64>>();

    // converts data.actor_balances.get(token1.address) to a float in a vector
    let arb_y = data
        .actor_balances
        .clone()
        .into_iter()
        .map(|x| *x.get(&1).unwrap())
        .map(wad_to_float)
        .collect::<Vec<f64>>();

    let data = DataFrame::new(vec![
        Series::new("portfolio_y_reserves", reserve_y),
        Series::new("portfolio_x_reserves", reserve_x),
        Series::new("portfolio_prices", portfolio_prices),
        Series::new("exchange_prices", exchange_prices),
        Series::new("arbitrageur_balance_x", arb_x),
        Series::new("arbitrageur_balance_y", arb_y),
    ])?;
    Ok(data)
}
