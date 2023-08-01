use arbiter::{
    agent::Agent,
    manager::SimulationManager,
    utils::{float_to_wad, recast_address, unpack_execution, wad_to_float},
};
use ethers::{
    abi::{AbiDecode, AbiEncode, Tokenizable, Tokenize},
    types::*,
    utils::parse_ether,
};
use std::error::Error;

// dynamic, generated with compile.sh
use bindings::{
    i_portfolio_actions::SwapReturn,
    portfolio::{PoolsReturn, PortfolioErrors},
    shared_types::{Order, PortfolioConfig},
};

use crate::math::NormalCurve;

/// Runs the tasks for each actor in the environment
/// Requires the arbitrageur's next desired transaction
pub fn run(manager: &SimulationManager, price: f64, pool_id: u64) -> Result<(), Box<dyn Error>> {
    let portfolio = manager.deployed_contracts.get("portfolio").unwrap();
    let price_wad = float_to_wad(price);

    println!("Price: {}, price_wad: {}", price, price_wad);
    let swap_order = get_swap_order(manager, pool_id, price_wad)?;
    println!("Swap order: {:#?}", swap_order);

    if swap_order.input == 0 {
        println!("No swap order required.");
        return Ok(());
    }

    let mut swap_success = false;

    let mut order = swap_order.clone();

    while !swap_success {
        let swap_call_result = manager.agents.get("arbitrageur").unwrap().call(
            portfolio,
            "swap",
            vec![order.clone().into_token()],
        )?;

        match unpack_execution(swap_call_result) {
            Ok(unpacked) => {
                let swap_return: SwapReturn = portfolio.decode_output("swap", unpacked)?;
                println!(
                    "Swap return: poolId {}, input {}, output {}, starting output: {}",
                    swap_return.pool_id, swap_return.input, swap_return.output, swap_order.output
                );

                swap_success = true;
            }
            Err(e) => {
                // This `InvalidInvariant` can pop up in multiple ways. Best to check for this.
                println!("Invalid invariant error: {:?}", e);
                let value = e.output.unwrap();
                println!("Value: {:?}", value.clone().encode_hex());

                // reduce output by a small amount until we are successful in swapping
                order.output = order
                    .output
                    .checked_mul(999_u128)
                    .unwrap()
                    .checked_div(1000_u128)
                    .unwrap();
            }
        };
    }

    Ok(())
}

/// Computes the swap order required to move the portfolio pool's reported price to `price_wad`.
fn get_swap_order(
    manager: &SimulationManager,
    pool_id: u64,
    price_wad: ethers::prelude::U256,
) -> Result<Order, Box<dyn std::error::Error>> {
    println!("Pool id: {}", pool_id);
    let arbitrageur = manager.agents.get("arbitrageur").unwrap();
    let actor = manager.deployed_contracts.get("actor").unwrap();
    let portfolio = manager.deployed_contracts.get("portfolio").unwrap();

    println!("here");
    let result = arbitrageur
        .call(
            actor,
            "computeArbInput",
            (recast_address(portfolio.address), pool_id, price_wad).into_tokens(),
        )
        .expect("Failed to call computeArbInput");

    let (swap_x_in, order_input_wad_per_liq): (bool, U256) =
        actor.decode_output("computeArbInput", unpack_execution(result)?)?;
    println!("there");

    println!("swap_x_in: {}", order_input_wad_per_liq);

    println!("swap_x_in: {}", swap_x_in);
    println!("order_input_wad_per_liq: {}", order_input_wad_per_liq);

    let order_output_wad_per_liq =
        get_amount_out(manager, pool_id, swap_x_in, order_input_wad_per_liq).unwrap();

    let pool_data = arbitrageur
        .call(portfolio, "pools", vec![pool_id.into_token()])
        .unwrap();
    let pool: PoolsReturn = portfolio
        .decode_output("pools", unpack_execution(pool_data).unwrap())
        .unwrap();

    let order_input_total_wad = order_input_wad_per_liq
        .checked_mul(U256::from(pool.liquidity))
        .unwrap()
        .checked_div(parse_ether(1.0).unwrap())
        .unwrap();
    let order_output_total_wad = order_output_wad_per_liq
        .checked_mul(U256::from(pool.liquidity))
        .unwrap()
        .checked_div(parse_ether(1.0).unwrap())
        .unwrap();

    let order: Order = Order {
        use_max: false,
        pool_id: pool_id.into(),
        input: order_input_total_wad.as_u128(),
        output: order_output_total_wad.as_u128(),
        sell_asset: swap_x_in,
    };

    Ok(order)
}

pub fn get_amount_out(
    manager: &SimulationManager,
    pool_id: u64,
    sell_asset: bool,
    amount_in: U256,
) -> Result<U256, Box<dyn Error>> {
    let portfolio = manager.deployed_contracts.get("portfolio").unwrap();
    let actor = manager.deployed_contracts.get("actor").unwrap();
    let arbitrageur = manager.agents.get("arbitrageur").unwrap();

    let pool_data = arbitrageur
        .call(portfolio, "pools", vec![pool_id.into_token()])
        .unwrap();
    let pool: PoolsReturn = portfolio
        .decode_output("pools", unpack_execution(pool_data).unwrap())
        .unwrap();

    let config = arbitrageur.call(
        actor,
        "getConfig",
        (recast_address(portfolio.address), pool_id).into_tokens(),
    );
    let config_return: PortfolioConfig = actor
        .decode_output("getConfig", unpack_execution(config.unwrap()).unwrap())
        .unwrap();

    println!("config: {:#?}", config_return);
    println!("pool: {:#?}", pool);

    /*
    let _rust_curve = NormalCurve::new_from_portfolio(&pool, &config_return);
    let amount_out = _rust_curve.approximate_amount_out(sell_asset, wad_to_float(amount_in));
    let amount_out = float_to_wad(amount_out);
    */
    if amount_in == U256::from(0) {
        return Ok(0.into());
    }

    let amount_out_call = arbitrageur.call(
        portfolio,
        "getAmountOut",
        (
            pool_id,
            sell_asset,
            amount_in,
            recast_address(arbitrageur.address()),
        )
            .into_tokens(),
    );

    let amount_out: U256 = portfolio
        .decode_output("getAmountOut", unpack_execution(amount_out_call?)?)
        .unwrap();

    Ok(amount_out)
}
