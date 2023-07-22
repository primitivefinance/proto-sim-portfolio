use arbiter::{agent::Agent, manager::SimulationManager, utils::{float_to_wad, recast_address, unpack_execution}};
use ethers::abi::Tokenize;

/// Runs the tasks for each actor in the environment
pub fn run(manager: &mut SimulationManager) -> Result<(), Box<dyn std::error::Error>> {
 let actor = manager.deployed_contracts.get("actor").unwrap();
 Ok(())
}