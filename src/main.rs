use arbiter::{manager, environment::contract::SimulationContract};
use bindings::{exchange};
use arbiter::agent::Agent;
use m3_rs::models::{base_model::BaseModel, rmm_01::RMM01};


fn main() {
    let mut manager = manager::SimulationManager::new();
    println!("Hello, world! Simulation is running.");

    let admin = manager.agents.get("admin").unwrap();
    let exchange = SimulationContract::new(exchange::EXCHANGE_ABI.clone(), exchange::EXCHANGE_BYTECODE.clone());
    let(exchange, result) = admin.deploy(exchange, Vec::new()).unwrap();
    manager.deployed_contracts.insert("exchange".to_string(), exchange);
    
    assert!(result.is_success());
    println!("Gas used: {:?}", result.gas_used());
    println!("Exchange address: {:?}", manager.deployed_contracts.get("exchange").unwrap().address);
    println!("Success!");


    let mut strategy = BaseModel::new(
        "NormalStrategy".to_string(),
        "v1.4.0-beta".to_string(),
        "x".to_string(),
        "id".to_string()
    );

    strategy.set_objective(Box::new(RMM01{
        strike: 1_f64,
        volatility: 0.1_f64,
        time_to_maturity: 1.0_f64
    }));

    let price = strategy.objective.expect("No objective set!").get_reported_price();
    println!("Price: {:?}", price);
}
