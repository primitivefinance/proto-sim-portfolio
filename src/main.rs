use arbiter::stochastic::price_process::{PriceProcess, PriceProcessType, OU};
use arbiter::{manager, utils::unpack_execution};
use m3_rs::models::{base_model::BaseModel, rmm_01::RMM01};

// dynamic imports... generate with build.sh

mod log;
mod setup;
mod step;
mod task;

#[tokio::main]

async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Simulation setup

    let mut manager = manager::SimulationManager::new();

    setup::run(&mut manager)?;

    let mut sim_data = log::SimData {
        pool_data: Vec::new(),
        actor_balances: Vec::new(),
        reference_prices: Vec::new(),
    };

    log::run(&mut manager, &mut sim_data)?;
    let weth = manager.deployed_contracts.get("weth");
    let portfolio = manager.deployed_contracts.get("portfolio");
    let exchange = manager.deployed_contracts.get("exchange");
    let token0 = manager.deployed_contracts.get("token0");
    let token1 = manager.deployed_contracts.get("token1");
    let actor = manager.deployed_contracts.get("actor");

    // Base model is struct for informational data, set objective for parameters and determining a
    // model, objective trait has methods like get_reported_price

    // 1. Generate price process
    // 2. Setup agents
    // 3. Create pool
    // 4. Allocate liquidity
    // 5. Create step.rs -> update exchange with next price
    // 6. Create task.rs -> read exchange state, determine actor response

    // Generate price process
    let ou = OU::new(0.01, 10.0, 1.0);
    let price_path = PriceProcess::new(
        PriceProcessType::OU(ou),
        0.01,
        "trade".to_string(),
        500,
        1.0,
        1,
    )
    .generate_price_path()
    .1;

    // Simulation loop

    for price in price_path {
        // Adjusts the reference market price
        step::step(&mut manager, price)?;

        // Runs the actor tasks
        task::run(&mut manager)?;
    }

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

    // Simulation finish and log

    Ok(())
}
