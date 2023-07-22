use arbiter::agent::simple_arbitrageur::SimpleArbitrageur;
use arbiter::agent::IsActive;
use arbiter::stochastic::price_process::{PriceProcess, PriceProcessType, OU};
use arbiter::{
    agent::{Agent, AgentType},
    manager::SimulationManager,
    utils::unpack_execution,
};
use m3_rs::models::{base_model::BaseModel, rmm_01::RMM01};
use revm::primitives::U256;

// dynamic imports... generate with build.sh

mod common;
mod log;
mod setup;
mod step;
mod task;

#[tokio::main]

async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Simulation setup

    let mut manager = SimulationManager::new();

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
    let ou = OU::new(1.0, 10.0, 1.0);
    let price_path = PriceProcess::new(
        PriceProcessType::OU(ou),
        0.01,
        "trade".to_string(),
        10, // temp: 500,
        1.0,
        1,
    )
    .generate_price_path()
    .1;

    // Simulation loop

    let arbitrageur = manager.agents.get("arbitrageur").unwrap();
    let arbitrageur = match arbitrageur {
        AgentType::SimpleArbitrageur(arbitrageur) => arbitrageur,
        _ => panic!("Arbitrageur not found! Was it initialized in setup.rs?"),
    };

    // Initialize the arbitrageur's start prices.
    setup::init_arbitrageur(arbitrageur, price_path.clone()).await;

    // Initialize the pool.
    let pool_id = setup::init_pool(&manager)?;

    // Add liquidity to the pool
    setup::allocate_liquidity(&manager, pool_id)?;

    // Run the first price update. This is important, as it triggers the arb detection.
    step::run(&manager, price_path[0])?;

    // note: arbitrageur borrows manager so it can't be used in the loop...
    let mut index: usize = 1;
    while let Ok((next_tx, _sell_asset)) = arbitrageur.detect_price_change().await {
        if index >= price_path.len() {
            // end sim
            println!("Ending sim loop at index: {}", index);
            break;
        }

        println!(
            "====== Sim step: {}, price: {} =========",
            index, price_path[index]
        );

        let price_f64 = price_path[index];

        // Run's the arbitrageur's task given the next desired tx.
        task::run(&manager, price_f64, next_tx, pool_id)?;

        // Logs the simulation data.
        log::run(&manager, &mut sim_data)?;

        // Increments the simulation forward.
        step::run(&manager, price_f64)?;

        // Increments the simulation loop.
        index += 1;
    }

    // Simulation finish and log
    manager.shutdown();

    println!("Simulation finished.");

    Ok(())
}
