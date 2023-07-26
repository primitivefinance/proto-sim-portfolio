use arbiter::{
    agent::Agent,
    manager::SimulationManager,
    utils::{float_to_wad, recast_address, unpack_execution},
};
use ethers::abi::{Tokenizable, Tokenize};
use std::error::Error;

// dynamic, generated with compile.sh
use bindings::{i_portfolio_actions::SwapReturn, portfolio::PoolsReturn, shared_types::Order};

/// Runs the tasks for each actor in the environment
/// Requires the arbitrageur's next desired transaction
pub fn run(manager: &SimulationManager, price: f64, pool_id: u64) -> Result<(), Box<dyn Error>> {
    let portfolio = manager.deployed_contracts.get("portfolio").unwrap();
    let price_wad = float_to_wad(price);

    let swap_order = get_swap_order(manager, pool_id, price_wad)?;
    println!("Swap order: {:#?}", swap_order);
    bisection(manager, price, pool_id);

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

/*function bisection(
    bytes memory args,
    uint256 lower,
    uint256 upper,
    uint256 epsilon,
    uint256 maxIterations,
    function(bytes memory,uint256) pure returns (int256) fx
) pure returns (uint256 root) {
    if (lower > upper) revert BisectionLib_InvalidBounds(lower, upper);
    // Passes the lower and upper bounds to the optimized function.
    // Reverts if the optimized function `fx` returns both negative or both positive values.
    // This means that the root is not between the bounds.
    // The root is between the bounds if the product of the two values is negative.
    int256 lowerOutput = fx(args, lower);
    int256 upperOutput = fx(args, upper);
    if (lowerOutput * upperOutput > 0) {
        revert BisectionLib_RootOutsideBounds(lower, upper);
    }

    // Distance is optimized to equal `epsilon`.
    uint256 distance = upper - lower;

    uint256 iterations; // Bounds the amount of loops to `maxIterations`.
    do {
        // Bisection uses the point between the lower and upper bounds.
        // The `distance` is halved each iteration.
        root = (lower + upper) / 2;

        int256 output = fx(args, root);

        // If the product is negative, the root is between the lower and root.
        // If the product is positive, the root is between the root and upper.
        if (output * lowerOutput <= 0) {
            upper = root; // Set the new upper bound to the root because we know its between the lower and root.
        } else {
            lower = root; // Set the new lower bound to the root because we know its between the upper and root.
            lowerOutput = output; // root function value becomes new lower output value
        }

        // Update the distance with the new bounds.
        distance = upper - lower;

        unchecked {
            iterations++; // Increment the iterator.
        }
    } while (distance > epsilon && iterations < maxIterations);
}*/
#[warn(unused_variables, dead_code)]
pub fn bisection(manager: &SimulationManager, price: f64, pool_id: u64) {
    let portfolio = manager.deployed_contracts.get("portfolio").unwrap();
    let strategy = manager.deployed_contracts.get("strategy").unwrap();
    let actor = manager.deployed_contracts.get("actor").unwrap();
    let arbitrageur = manager.agents.get("arbitrageur").unwrap();
    let price_wad = float_to_wad(price);

    let pool_data = arbitrageur
        .call(portfolio, "pools", vec![pool_id.into_token()])
        .unwrap();
    let pool: PoolsReturn = portfolio
        .decode_output("pools", unpack_execution(pool_data).unwrap())
        .unwrap();

    println!("pool: {:#?}", pool);
}
