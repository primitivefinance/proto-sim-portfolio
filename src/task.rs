use arbiter::{
    agent::{
        simple_arbitrageur::{NextTx, SwapDirection},
        Agent,
    },
    manager::SimulationManager,
    utils::{float_to_wad, recast_address, unpack_execution},
};
use ethers::abi::{Tokenizable, Tokenize};

// dynamic, generated with compile.sh
use bindings::shared_types::Order;

/// Runs the tasks for each actor in the environment
/// Requires the arbitrageur's next desired transaction
pub fn run(
    manager: &SimulationManager,
    price: f64,
    next_tx: NextTx,
    pool_id: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let actor = manager.deployed_contracts.get("actor").unwrap();
    let portfolio = manager.deployed_contracts.get("portfolio").unwrap();

    let price_wad = float_to_wad(price);

    // todo: we should be aware of the liquidity distribution at this price...
    // right now we get 0 arb amounts so we fail to get a swap order
    match next_tx {
        NextTx::Swap => {
            // do the arbitrage
            println!("Executing task.");
            let swap_order = get_swap_order(manager, pool_id, price_wad)?;
            println!("Swap order: {:#?}", swap_order);
            let swap_call_result = manager
                .agents
                .get("arbitrageur")
                .unwrap()
                .call(portfolio, "swap", vec![swap_order.into_token()])
                .unwrap();

            match swap_call_result.is_success() {
                true => println!(
                    "Swap call success: {:#?}",
                    portfolio.decode_output("swap", unpack_execution(swap_call_result)?)?
                ),
                false => println!("Swap call failed: {:#?}", swap_call_result),
            }
        }
        NextTx::UpdatePrice => {
            // do nothing... this case should be removed
            println!("Updating price case");
        }
        NextTx::None => {
            // do nothing regularly...
            println!("No watched events triggered.");
        }
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