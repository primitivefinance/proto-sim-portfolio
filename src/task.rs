use arbiter::{
    agent::{
        simple_arbitrageur::{NextTx, SwapDirection},
        Agent,
    },
    manager::SimulationManager,
    utils::{float_to_wad, recast_address, unpack_execution},
};
use bindings::shared_types::Order;
use ethers::abi::Tokenize;

/// Runs the tasks for each actor in the environment
/// Requires the arbitrageur's next desired transaction
pub fn run(
    manager: &SimulationManager,
    price: f64,
    next_tx: NextTx,
) -> Result<(), Box<dyn std::error::Error>> {
    let actor = manager.deployed_contracts.get("actor").unwrap();

    let price_wad = float_to_wad(price);

    match next_tx {
        NextTx::Swap => {
            // do the arbitrage
            println!("Executing task.");
            let swap_order = get_swap_order(manager, 0, price_wad)?;
            println!("Swap order: {:#?}", swap_order);
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

fn get_swap_order(
    manager: &SimulationManager,
    pool_id: u64,
    price_wad: ethers::prelude::U256,
) -> Result<Order, Box<dyn std::error::Error>> {
    let arbitrageur = manager.agents.get("arbitrageur").unwrap();
    let actor = manager.deployed_contracts.get("actor").unwrap();
    let portfolio = manager.deployed_contracts.get("portfolio").unwrap();

    let result = arbitrageur.call(
        actor,
        "computeArbSwapOrder",
        (recast_address(portfolio.address), pool_id, price_wad).into_tokens(),
    )?;

    let swap_order: Order =
        actor.decode_output("computeArbSwapOrder", unpack_execution(result)?)?;

    Ok(swap_order)
}
