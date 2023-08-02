use arbiter::{
    agent::Agent,
    manager::SimulationManager,
    utils::{float_to_wad, recast_address},
};
use ethers::abi::Tokenize;

/// Moves the simulation forward a step.

pub fn run(manager: &SimulationManager, price: f64) -> Result<(), Box<dyn std::error::Error>> {
    let admin = manager.agents.get("admin").unwrap();
    let exchange = manager.deployed_contracts.get("exchange").unwrap();
    let token = manager.deployed_contracts.get("token0").unwrap();

    let wad_price = float_to_wad(price);

    // Triggers the "PriceChange" event, which agents might be awaiting.
    let new_price_call = admin.call(
        exchange,
        "setPrice",
        (recast_address(token.address), wad_price).into_tokens(),
    )?;

    match new_price_call.is_success() {
        true => {} // just be quiet... make noise if there's an error.,
        false => println!("New price failed to set: {}", price),
    }

    Ok(())
}
