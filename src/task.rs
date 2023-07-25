use arbiter::{
    agent::Agent,
    manager::SimulationManager,
    utils::{float_to_wad, recast_address, unpack_execution},
};
use ethers::abi::{Tokenizable, Tokenize};
use std::error::Error;

// dynamic, generated with compile.sh
use bindings::{i_portfolio_actions::SwapReturn, shared_types::Order};

/// Runs the tasks for each actor in the environment
/// Requires the arbitrageur's next desired transaction
pub fn run(manager: &SimulationManager, price: f64, pool_id: u64) -> Result<(), Box<dyn Error>> {
    let portfolio = manager.deployed_contracts.get("portfolio").unwrap();
    let price_wad = float_to_wad(price);

    let swap_order = get_swap_order(manager, pool_id, price_wad)?;
    println!("Swap order: {:#?}", swap_order);

    if swap_order.input == 0 {
        println!("No swap order required.");
        return Ok(());
    }

    let swap_call_result = manager
        .agents
        .get("arbitrageur")
        .unwrap()
        .call(portfolio, "swap", vec![swap_order.into_token()])
        .unwrap();

    let swap_result: SwapReturn =
        portfolio.decode_output("swap", unpack_execution(swap_call_result.clone())?)?;

    match swap_call_result.is_success() {
        true => println!("Swap call success: {:#?}", swap_result),
        false => println!("Swap call failed: {:#?}", swap_call_result),
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

    let result = arbitrageur
        .call(
            actor,
            "computeArbSwapOrder",
            (recast_address(portfolio.address), pool_id, price_wad).into_tokens(),
        )
        .expect("Failed to call computeArbSwapOrder");

    let swap_order: Order =
        actor.decode_output("computeArbSwapOrder", unpack_execution(result)?)?;

    Ok(swap_order)
}
