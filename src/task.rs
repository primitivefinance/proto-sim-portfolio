use arbiter::{
    agent::Agent,
    manager::SimulationManager,
    utils::{float_to_wad, recast_address, unpack_execution},
};
use ethers::{
    abi::{Tokenizable, Tokenize},
    types::*,
    utils::parse_ether,
};
use std::error::Error;

// dynamic, generated with compile.sh
use bindings::{i_portfolio_actions::SwapReturn, portfolio::PoolsReturn, shared_types::Order};

use super::calls::{Caller, DecodedReturns};
use super::common;

#[allow(unused)]
enum SwapDirection {
    SwapXToY,
    SwapYToX,
    None,
}

#[allow(unused)]
fn check_no_arb_bounds(
    current_price: U256,
    target_price: U256,
    fee: U256,
) -> Option<SwapDirection> {
    // Check the no-arbitrage bounds
    let upper_arb_bound = current_price
        .checked_mul(parse_ether(1.0).unwrap())
        .unwrap()
        .checked_div(fee)
        .unwrap();
    let lower_arb_bound = current_price
        .checked_mul(fee)
        .unwrap()
        .checked_div(parse_ether(1.0).unwrap())
        .unwrap();

    //println!("Current price: {:?}", current_price);
    //println!("Target price: {:?}", target_price);
    //println!("Upper bound: {:?}", upper_arb_bound);
    //println!("Lower bound: {:?}", lower_arb_bound);

    if (target_price > upper_arb_bound) | (target_price < lower_arb_bound) {
        // If the prices are outside of the no-arbitrage bounds, then we can arbitrage.
        let price_difference = current_price.checked_sub(target_price);
        if price_difference.is_none() {
            // If this difference is `None`, then the subtraction overflowed so current_price<target_price.
            Some(SwapDirection::SwapXToY)
        } else {
            // If the price difference is still nonzero, then we must swap with price[0]>price[1].
            Some(SwapDirection::SwapYToX)
        }
    } else {
        // Prices are within the no-arbitrage bounds, so we don't have an arbitrage.
        Some(SwapDirection::None)
    }
}

/// Runs the tasks for each actor in the environment
/// Requires the arbitrageur's next desired transaction
pub fn run(manager: &SimulationManager, price: f64, pool_id: u64) -> Result<(), Box<dyn Error>> {
    // Get the instances we need.
    let admin = manager.agents.get("admin").unwrap();
    let portfolio = manager.deployed_contracts.get("portfolio").unwrap();
    let mut caller = Caller::new(admin);

    // Collect the key variables to check for arbitrage.
    let target_price_wad = float_to_wad(price);

    // Check if we are within the no-arb bounds.
    let current_price_wad: U256 = caller
        .call(portfolio, "getSpotPrice", pool_id.into_tokens())?
        .decoded(portfolio)?;

    //println!(
    //    "Reported price: {:?}, Reference price: {:?}",
    //    current_price_wad, target_price_wad
    //);

    let fee = U256::from(
        (common::BASIS_POINT_DIVISOR as u128 - common::FEE_BPS as u128) * 1e18 as u128
            / common::BASIS_POINT_DIVISOR as u128,
    );
    let direction: Option<SwapDirection> =
        check_no_arb_bounds(current_price_wad, target_price_wad, fee);

    let verbose = std::env::var("VERBOSE");

    match direction {
        Some(SwapDirection::SwapXToY) => {
            if verbose.is_ok() {
                println!("Swap X to Y");
            }
        }
        Some(SwapDirection::SwapYToX) => {
            if verbose.is_ok() {
                println!("Swap Y to X");
            }
        }
        Some(SwapDirection::None) => {
            if verbose.is_ok() {
                println!("No swap required.");
            }
            return Ok(());
        }
        None => {
            if verbose.is_ok() {
                println!("No swap required.");
            }
            return Ok(());
        }
    }

    let swap_order = get_swap_order(manager, pool_id, target_price_wad)?;
    if verbose.is_ok() {
        println!("Swap order: {:#?}", swap_order);
    }

    if swap_order.input == 0 {
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
                if verbose.is_ok() {
                    let swap_return: SwapReturn = portfolio.decode_output("swap", unpacked)?;
                    println!(
                        "Swap successful call returned: poolId {}, input {}, output {}, starting output: {}",
                        swap_return.pool_id,
                        swap_return.input,
                        swap_return.output,
                        swap_order.output
                    );
                }

                swap_success = true;
            }
            Err(_) => {
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

/// Computes the swap order required to move the portfolio pool's reported price to `target_price_wad`.
fn get_swap_order(
    manager: &SimulationManager,
    pool_id: u64,
    target_price_wad: ethers::prelude::U256,
) -> Result<Order, Box<dyn std::error::Error>> {
    //println!("Pool id: {}", pool_id);
    let arbitrageur = manager.agents.get("arbitrageur").unwrap();
    let actor = manager.deployed_contracts.get("actor").unwrap();
    let portfolio = manager.deployed_contracts.get("portfolio").unwrap();

    //println!("here");
    let result = arbitrageur.call(
        actor,
        "computeArbInput",
        (recast_address(portfolio.address), pool_id, target_price_wad).into_tokens(),
    )?;

    let mut swap_x_in: bool = false;
    let mut order_input_wad_per_liq: U256 = U256::from(0);

    match unpack_execution(result) {
        Ok(unpacked) => {
            (swap_x_in, order_input_wad_per_liq) =
                actor.decode_output("computeArbInput", unpacked)?;
            //println!(
            //    "decoded computeArbInput: swapInX {:?} order input {:?}",
            //    swap_x_in, order_input_wad_per_liq
            //);
        }
        Err(e) => {
            println!("Error: {:?}", e);
        }
    }

    //println!("there");

    //println!("swap_x_in: {}", order_input_wad_per_liq);

    //println!("swap_x_in: {}", swap_x_in);
    //println!("order_input_wad_per_liq: {}", order_input_wad_per_liq);

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
    let arbitrageur = manager.agents.get("arbitrageur").unwrap();

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
