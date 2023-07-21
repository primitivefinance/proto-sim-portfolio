use arbiter::{agent::Agent, manager::SimulationManager, utils::float_to_wad};
use ethers::abi::Tokenize;

pub fn step(manager: &mut SimulationManager, price: f64) -> Result<(), Box<dyn std::error::Error>> {
    let admin = manager.agents.get("admin").unwrap();
    let exchange = manager.deployed_contracts.get("exchange").unwrap();
    let wad_price = float_to_wad(price);
    let new_price_call = admin.call(exchange, "setPrice", wad_price.into_tokens())?;
    Ok(())
}
