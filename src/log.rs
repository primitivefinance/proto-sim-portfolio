use std::collections::HashMap;

use arbiter::{agent::*, environment::contract::*, manager::SimulationManager, utils::*};
use ethers::abi::Tokenize;
use ethers::prelude::U256;
use revm::primitives::Address;

// dynamic... generated with build.sh
use bindings::i_portfolio_getters::*;

/// Struct for storing simulation data
pub struct SimData {
    pub pool_data: Vec<PoolsReturn>,
    pub actor_balances: Vec<HashMap<Address, U256>>,
    pub reference_prices: Vec<U256>,
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
    manager: &mut SimulationManager,
    sim_data: &mut SimData,
) -> Result<(), Box<dyn std::error::Error>> {
    let admin = manager.agents.get("admin").unwrap();
    let actor = manager.deployed_contracts.get("actor").unwrap();
    let token0 = manager.deployed_contracts.get("token0").unwrap();
    let token1 = manager.deployed_contracts.get("token1").unwrap();

    let actor_balance_0 = get_balance(admin, token0, actor.address)?;
    let actor_balance_1 = get_balance(admin, token1, actor.address)?;
    let mut actor_balance = HashMap::new();
    actor_balance.insert(token0.address, actor_balance_0);
    actor_balance.insert(token1.address, actor_balance_1);
    sim_data.actor_balances.push(actor_balance);

    let portfolio = manager.deployed_contracts.get("portfolio").unwrap();
    let pool_data = get_pool(admin, portfolio, 0)?;
    sim_data.pool_data.push(pool_data);

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
