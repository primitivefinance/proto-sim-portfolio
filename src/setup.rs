use arbiter::{manager, environment::contract::SimulationContract};
use arbiter::agent::Agent;
// dynamic imports... generate with build.sh
use bindings::{entrypoint};

pub async fn run(manager: &mut manager::SimulationManager) -> Result<(), Box<dyn std::error::Error>> {
    let admin = manager.agents.get("admin").unwrap();
    let entrypoint = SimulationContract::new(entrypoint::ENTRYPOINT_ABI.clone(), entrypoint::ENTRYPOINT_BYTECODE.clone());
    let (entrypoint, result) = admin.deploy(entrypoint, Vec::new()).unwrap();

    manager.deployed_contracts.insert("entrypoint".to_string(), entrypoint);

    let called = admin.call(manager.deployed_contracts.get("entrypoint").unwrap(), "start", Vec::new()).unwrap();
    assert!(called.is_success()); 
    println!("Gas used: {:?}", called.gas_used());

    Ok(())
}