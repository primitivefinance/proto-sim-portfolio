use arbiter::agent::Agent;
use arbiter::{manager, utils::unpack_execution};
use m3_rs::models::{base_model::BaseModel, rmm_01::RMM01};
use setup::run;

// dynamic imports... generate with build.sh

mod setup;

#[tokio::main]

async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut manager = manager::SimulationManager::new();
    run(&mut manager)?;
    let weth = manager.deployed_contracts.get("weth");
    let portfolio = manager.deployed_contracts.get("portfolio");
    let exchange = manager.deployed_contracts.get("exchange");
    let token0 = manager.deployed_contracts.get("token0");
    let token1 = manager.deployed_contracts.get("token1");
    let actor = manager.deployed_contracts.get("actor");

    // Base model is struct for informational data, set objective for parameters and determining a
    // model, objective trait has methods like get_reported_price

    let mut strategy = BaseModel::new(
        "NormalStrategy".to_string(),
        "v1.4.0-beta".to_string(),
        "x".to_string(),
        "id".to_string(),
    );

    strategy.set_objective(Box::new(RMM01 {
        strike: 1_f64,
        volatility: 0.1_f64,
        time_to_maturity: 1.0_f64,
    }));

    let price = strategy
        .objective
        .expect("No objective set!")
        .get_reported_price();
    println!("Price: {:?}", price);

    //setup::run(&mut manager).await.unwrap();
    println!("Simulation ran setup");

    Ok(())
}
