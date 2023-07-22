use arbiter::agent::simple_arbitrageur::SimpleArbitrageur;
use arbiter::agent::{Agent, AgentType, SimulationEventFilter};
use arbiter::{
    environment::contract::SimulationContract,
    manager,
    utils::{recast_address, unpack_execution},
};
// dynamic imports... generate with build.sh
use bindings::{actor, entrypoint, exchange, mock_erc20, portfolio, weth};
use ethers::{
    abi::{encode_packed, Token, Tokenize},
    prelude::{Address, U256},
    types::H160,
};
use revm::primitives::B160;

use super::common;

pub fn run(manager: &mut manager::SimulationManager) -> Result<(), Box<dyn std::error::Error>> {
    let admin = manager.agents.get("admin").unwrap();

    // Deploy weth
    let weth = SimulationContract::new(weth::WETH_ABI.clone(), weth::WETH_BYTECODE.clone());
    let (weth_contract, _result) = admin.deploy(weth, vec![])?;

    // Deploy portfolio
    let portfolio = SimulationContract::new(
        portfolio::PORTFOLIO_ABI.clone(),
        portfolio::PORTFOLIO_BYTECODE.clone(),
    );
    let (portfolio_contract, _result) = admin.deploy(
        portfolio,
        (
            recast_address(weth_contract.address),
            Address::from(B160::from(0)),
        )
            .into_tokens(),
    )?;

    // Deploy Entrypoint
    let entrypoint = SimulationContract::new(
        entrypoint::ENTRYPOINT_ABI.clone(),
        entrypoint::ENTRYPOINT_BYTECODE.clone(),
    );
    let (entrypoint_contract, _result) = admin.deploy(
        entrypoint,
        (
            recast_address(portfolio_contract.address),
            recast_address(weth_contract.address),
        )
            .into_tokens(),
    )?;

    // Add deployed contracts to manager
    manager
        .deployed_contracts
        .insert("entrypoint".to_string(), entrypoint_contract);
    let entrypoint_callable = manager.deployed_contracts.get("entrypoint").unwrap();

    let encoded = encode_packed(
        &[
            recast_address(weth_contract.address),
            recast_address(portfolio_contract.address),
        ]
        .into_tokens(),
    )?;

    println!("Entrypoint encoded: {:?}", encoded);
    let start_call = admin.call(entrypoint_callable, "start", vec![Token::Bytes(encoded)])?;
    println!(
        "Entrypoint start call result: {:?}",
        unpack_execution(start_call)
    );

    let exchange = admin.call(entrypoint_callable, "exchange", vec![])?;
    let exchange_address: H160 =
        entrypoint_callable.decode_output("exchange", unpack_execution(exchange)?)?;
    let exchange_address_bytes = B160::from(exchange_address.as_fixed_bytes());
    let exchange_contract =
        SimulationContract::bind(exchange::EXCHANGE_ABI.clone(), exchange_address_bytes);

    let token0 = admin.call(entrypoint_callable, "token0", vec![])?;
    let token0_address: H160 =
        entrypoint_callable.decode_output("token0", unpack_execution(token0)?)?;
    let token0_address_bytes = B160::from(token0_address.as_fixed_bytes());
    let token0_contract =
        SimulationContract::bind(mock_erc20::MOCKERC20_ABI.clone(), token0_address_bytes);

    let token1 = admin.call(entrypoint_callable, "token1", vec![])?;
    let token1_address: H160 =
        entrypoint_callable.decode_output("token1", unpack_execution(token1)?)?;
    let token1_address_bytes = B160::from(token1_address.as_fixed_bytes());
    let token1_contract =
        SimulationContract::bind(mock_erc20::MOCKERC20_ABI.clone(), token1_address_bytes);

    let actor = admin.call(entrypoint_callable, "actor", vec![])?;
    let actor_address: H160 =
        entrypoint_callable.decode_output("actor", unpack_execution(actor)?)?;
    let actor_address_bytes = B160::from(actor_address.as_fixed_bytes());
    let actor_contract = SimulationContract::bind(actor::ACTOR_ABI.clone(), actor_address_bytes);

    manager
        .deployed_contracts
        .insert("weth".to_string(), weth_contract);
    manager
        .deployed_contracts
        .insert("portfolio".to_string(), portfolio_contract);
    manager
        .deployed_contracts
        .insert("exchange".to_string(), exchange_contract);
    manager
        .deployed_contracts
        .insert("token0".to_string(), token0_contract);
    manager
        .deployed_contracts
        .insert("token1".to_string(), token1_contract);
    manager
        .deployed_contracts
        .insert("actor".to_string(), actor_contract);

    let portfolio = manager.deployed_contracts.get("portfolio").unwrap();
    let get_pair_nonce = admin.call(portfolio, "getPairNonce", vec![])?;
    let get_pair_nonce_result: u64 =
        portfolio.decode_output("getPairNonce", unpack_execution(get_pair_nonce)?)?;
    println!(
        "portfolio get_pair_nonce result: {:?}",
        get_pair_nonce_result
    );

    setup_agent(manager);

    Ok(())
}

fn setup_agent(manager: &mut manager::SimulationManager) {
    let exchange = manager.deployed_contracts.get("exchange").unwrap();

    let event_filters = vec![SimulationEventFilter::new(exchange, "PriceChange")];

    let agent = SimpleArbitrageur::new(
        "arbitrageur",
        event_filters,
        revm::primitives::U256::from(common::WAD as u128) - revm::primitives::U256::from(100),
    );

    manager
        .activate_agent(
            AgentType::SimpleArbitrageur(agent),
            B160::from_low_u64_be(2),
        )
        .unwrap();
}
