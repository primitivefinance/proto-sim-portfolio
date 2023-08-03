use arbiter::agent::simple_arbitrageur::SimpleArbitrageur;
use arbiter::agent::{Agent, AgentType, SimulationEventFilter};
use arbiter::{
    environment::contract::{IsDeployed, SimulationContract},
    manager::SimulationManager,
    utils::{float_to_wad, recast_address, unpack_execution},
};
use bindings::{external_normal_strategy_lib, i_portfolio_actions::CreatePoolCall};
// dynamic imports... generate with build.sh
use bindings::{actor, entrypoint, exchange, mock_erc20, portfolio, weth};
use ethers::{
    abi::{encode_packed, Token, Tokenize},
    prelude::{Address, U128, U256},
    types::H160,
};
use revm::primitives::B160;

use super::common;
use super::config;

pub fn run(
    manager: &mut SimulationManager,
    config: &config::SimConfig,
) -> Result<(), Box<dyn std::error::Error>> {
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
            Address::from(B160::zero()),
            Address::from(B160::zero()),
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

    let _ = admin.call(entrypoint_callable, "start", vec![Token::Bytes(encoded)])?;

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

    admin
        .call(
            &token0_contract,
            "approve",
            (recast_address(portfolio_contract.address), U256::MAX).into_tokens(),
        )
        .unwrap();

    admin
        .call(
            &token1_contract,
            "approve",
            (recast_address(portfolio_contract.address), U256::MAX).into_tokens(),
        )
        .unwrap();

    admin
        .call(
            &token0_contract,
            "mint",
            (
                recast_address(B160::from_low_u64_be(common::ARBITRAGEUR_ADDRESS_BASE)),
                float_to_wad(4809.0),
            )
                .into_tokens(),
        )
        .unwrap();

    admin
        .call(
            &token1_contract,
            "mint",
            (
                recast_address(B160::from_low_u64_be(common::ARBITRAGEUR_ADDRESS_BASE)),
                float_to_wad(4809.0),
            )
                .into_tokens(),
        )
        .unwrap();

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

    deploy_external_normal_strategy_lib(manager)?;

    setup_agent(manager);

    Ok(())
}

fn setup_agent(manager: &mut SimulationManager) {
    let exchange = manager.deployed_contracts.get("exchange").unwrap();

    let event_filters = vec![SimulationEventFilter::new(exchange, "PriceChange")];

    let agent = SimpleArbitrageur::new(
        "arbitrageur",
        event_filters,
        revm::primitives::U256::from(common::WAD as u128)
            - revm::primitives::U256::from(common::FEE_BPS as f64 * 1e18),
    );

    manager
        .activate_agent(
            AgentType::SimpleArbitrageur(agent),
            B160::from_low_u64_be(common::ARBITRAGEUR_ADDRESS_BASE),
        )
        .unwrap();
}

pub async fn init_arbitrageur(
    arbitrageur: &SimpleArbitrageur<arbiter::agent::IsActive>,
    initial_prices: Vec<f64>,
) {
    // Arbitrageur needs two prices to arb between which are initialized to the initial price in the price path.
    let mut prices = arbitrageur.prices.lock().await;
    prices[0] = revm::primitives::U256::from(initial_prices[0]).into();
    prices[1] = revm::primitives::U256::from(initial_prices[0]).into();
    drop(prices);
}

pub fn init_pool(manager: &SimulationManager) -> Result<u64, Box<dyn std::error::Error>> {
    let admin = manager.agents.get("admin").unwrap();
    let portfolio = manager.deployed_contracts.get("portfolio").unwrap();

    let create_pool_args: CreatePoolCall = build_create_pool_call(manager);
    let result = admin
        .call(
            portfolio,
            "createPool",
            (
                create_pool_args.pair_id,
                create_pool_args.reserve_x_per_wad,
                create_pool_args.reserve_y_per_wad,
                create_pool_args.fee_basis_points,
                create_pool_args.priority_fee_basis_points,
                create_pool_args.controller,
                create_pool_args.strategy,
                create_pool_args.strategy_args,
            )
                .into_tokens(),
        )
        .unwrap();

    if !result.is_success() {
        panic!("createPool failed");
    }

    let pool_id: u64 = portfolio
        .decode_output("createPool", unpack_execution(result).unwrap())
        .unwrap();

    Ok(pool_id)
}

fn build_create_pool_call(manager: &SimulationManager) -> CreatePoolCall {
    let admin = manager.agents.get("admin").unwrap();
    let actor = manager.deployed_contracts.get("actor").unwrap();
    let portfolio = manager.deployed_contracts.get("portfolio").unwrap();

    let computed_args = admin
        .call(
            actor,
            "getCreatePoolComputedArgs",
            (
                recast_address(portfolio.address),
                float_to_wad(1.0),                  // strike price wad
                U256::from(common::VOLATILITY_BPS), // vol bps
                U256::from(31556953_u32),           // 1 year duration in seconds
                true,                               // is perpetual
                float_to_wad(1.0),                  // initial price wad
            )
                .into_tokens(),
        )
        .expect("getCreatePoolComputedArgs failed");

    if !computed_args.is_success() {
        panic!("getCreatePoolComputedArgs failed");
    }

    let computed_args: bindings::actor::GetCreatePoolComputedArgsReturn = actor
        .decode_output(
            "getCreatePoolComputedArgs",
            unpack_execution(computed_args).unwrap(),
        )
        .unwrap();

    CreatePoolCall {
        pair_id: 1_u32,                             // pairId
        reserve_x_per_wad: computed_args.initial_x, // reserveXPerWad
        reserve_y_per_wad: computed_args.initial_y, // reserveYPerWad
        fee_basis_points: common::FEE_BPS,          // feeBips
        priority_fee_basis_points: 0_u16,           // priorityFeeBips
        controller: H160::zero(),                   // controller,
        strategy: H160::zero(),                     // address(0) == default strategy
        strategy_args: computed_args.strategy_data, // strategyArgs
    }
}

pub fn allocate_liquidity(
    manager: &SimulationManager,
    pool_id: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let admin = manager.agents.get("admin").unwrap();
    let portfolio = manager.deployed_contracts.get("portfolio").unwrap();

    let recipient = recast_address(admin.address());

    // note: this can fail automatically if block.timestamp is 0.
    // note: this can fail if maxDeltaAsset/maxDeltaQuote is larger than uint128
    let result = admin
        .call(
            portfolio,
            "allocate",
            (
                false, // use max
                recipient,
                pool_id,                   // poolId
                float_to_wad(100.0),       // 100e18 liquidity
                U128::MAX / U128::from(2), // tries scaling to wad by multiplying beyond word size, div to avoid.
                U128::MAX / U128::from(2),
            )
                .into_tokens(),
        )
        .unwrap();

    if !result.is_success() {
        panic!("allocate for pool id {} failed {:#?}", pool_id, result);
    }

    Ok(())
}

pub fn deploy_external_normal_strategy_lib(
    manager: &mut SimulationManager,
) -> Result<&SimulationContract<IsDeployed>, Box<dyn std::error::Error>> {
    let admin = manager.agents.get("admin").unwrap();
    let library = SimulationContract::new(
        external_normal_strategy_lib::EXTERNALNORMALSTRATEGYLIB_ABI.clone(),
        external_normal_strategy_lib::EXTERNALNORMALSTRATEGYLIB_BYTECODE.clone(),
    );
    let (library_contract, _) = admin.deploy(library, vec![])?;
    manager
        .deployed_contracts
        .insert("library".to_string(), library_contract);

    let library = manager.deployed_contracts.get("library").unwrap();
    Ok(library)
}
