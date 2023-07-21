use arbiter::agent::Agent;
// dynamic imports... generate with build.sh
use arbiter::{
    environment::contract::SimulationContract,
    manager,
    utils::{recast_address, unpack_execution},
};
use bindings::{actor, entrypoint, exchange, mock_erc20, portfolio, weth};
use ethers::{
    abi::{encode_packed, Token, Tokenize},
    prelude::Address,
    types::H160,
};
use revm::primitives::B160;

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
    admin.call(entrypoint_callable, "start", vec![Token::Bytes(encoded)])?;

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

    Ok(())
}
