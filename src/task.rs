use arbiter::{
    agent::{
        simple_arbitrageur::{NextTx, SwapDirection},
        Agent,
    },
    manager::SimulationManager,
    utils::{float_to_wad, recast_address, unpack_execution},
};
use ethers::abi::Tokenize;

/// Runs the tasks for each actor in the environment
/// Requires the arbitrageur's next desired transaction
pub fn run(
    manager: &mut SimulationManager,
    price: f64,
    next_tx: NextTx,
) -> Result<(), Box<dyn std::error::Error>> {
    let actor = manager.deployed_contracts.get("actor").unwrap();

    let price_wad = float_to_wad(price);

    match next_tx {
        NextTx::Swap => {
            // do the arbitrage
        }
        NextTx::UpdatePrice => {
            // do nothing... this case should be removed
        }
        NextTx::None => {
            // do nothing regularly...
        }
    }

    Ok(())
}
