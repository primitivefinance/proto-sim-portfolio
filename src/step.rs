use super::calls::Caller;
use arbiter::{
    manager::SimulationManager,
    utils::{float_to_wad, recast_address},
};
use ethers::abi::Tokenize;

/// Moves the simulation forward a step by calling `setPrice` triggering the `PriceChange` event.
pub fn run(manager: &SimulationManager, price: f64) -> Result<(), Box<dyn std::error::Error>> {
    let exchange = manager.deployed_contracts.get("exchange").unwrap();
    let token = manager.deployed_contracts.get("token0").unwrap();
    let admin = manager.agents.get("admin").unwrap();
    let mut caller = Caller::new(admin);

    let wad_price = float_to_wad(price);

    // Triggers the "PriceChange" event, which agents might be awaiting.
    // Calls the `res()` at the end with a `?` to propagate any errors.
    let _ = caller
        .call(
            exchange,
            "setPrice",
            (recast_address(token.address), wad_price).into_tokens(),
        )?
        .res()?;

    Ok(())
}
