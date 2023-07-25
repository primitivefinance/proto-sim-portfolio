use arbiter::stochastic::price_process::{PriceProcess, PriceProcessType, OU};
use arbiter::{
    agent::{Agent, AgentType},
    manager::SimulationManager,
    utils::recast_address,
};
use ethers::abi::Tokenize;
use visualize::{design::*, plot::*};

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
        arbitrageur_balances: Vec::new(),
        reference_prices: Vec::new(),
        portfolio_prices: Vec::new(),
    };

    let portfolio = manager.deployed_contracts.get("portfolio").unwrap();
    let token0 = manager.deployed_contracts.get("token0").unwrap();
    let token1 = manager.deployed_contracts.get("token1").unwrap();

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
    let price_process = PriceProcess::new(
        PriceProcessType::OU(ou),
        0.01,
        "trade".to_string(),
        500, // temp: 500,
        1.0,
        1,
    );

    let prices = price_process.generate_price_path().1;

    // Simulation loop

    let arbitrageur = manager.agents.get("arbitrageur").unwrap();
    let arbitrageur = match arbitrageur {
        AgentType::SimpleArbitrageur(arbitrageur) => arbitrageur,
        _ => panic!("Arbitrageur not found! Was it initialized in setup.rs?"),
    };

    // Initialize the arbitrageur's start prices.
    setup::init_arbitrageur(arbitrageur, prices.clone()).await;

    arbitrageur
        .call(
            &token0,
            "approve",
            (
                recast_address(portfolio.address),
                ethers::prelude::U256::MAX,
            )
                .into_tokens(),
        )
        .unwrap();

    arbitrageur
        .call(
            &token1,
            "approve",
            (
                recast_address(portfolio.address),
                ethers::prelude::U256::MAX,
            )
                .into_tokens(),
        )
        .unwrap();

    // Initialize the pool.
    let pool_id = setup::init_pool(&manager)?;

    // Add liquidity to the pool
    setup::allocate_liquidity(&manager, pool_id)?;

    // Run the first price update. This is important, as it triggers the arb detection.
    step::run(&manager, prices[0])?;

    // Logs initial simulation state.
    log::run(&manager, &mut sim_data, pool_id)?;

    for (i, price) in prices.iter().skip(1).enumerate() {
        println!("====== Sim step: {}, price: {} =========", i, price);

        // Run's the arbitrageur's task given the next desired tx.
        task::run(&manager, *price, pool_id)?;

        // Logs the simulation data.
        log::run(&manager, &mut sim_data, pool_id)?;

        // Increments the simulation forward.
        step::run(&manager, *price)?;
    }

    // Simulation finish and log
    manager.shutdown();

    // Write the sim data to a file.
    log::write_to_file(price_process, &mut sim_data)?;

    let display = Display {
        transparent: false,
        mode: DisplayMode::Light,
        show: false,
    };

    log::plot_reserves(display.clone(), &sim_data);
    log::plot_prices(display.clone(), &sim_data);

    println!("Simulation finished.");

    Ok(())
}
