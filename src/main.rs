use arbiter::environment::contract::{IsDeployed, SimulationContract};
use arbiter::stochastic::price_process::{PriceProcess, PriceProcessType, OU};
use arbiter::utils::wad_to_float;
use arbiter::{
    agent::{Agent, AgentType},
    manager::SimulationManager,
    utils::recast_address,
};
use clap::Parser;
use ethers::abi::Tokenize;
use serde::{Deserialize, Serialize};
use visualize::{design::*, plot::*};

// dynamic imports... generate with build.sh

pub static OUTPUT_DIRECTORY: &str = "out_data";
pub static OUTPUT_FILE_NAME: &str = "results";

mod analysis;
mod bisection;
mod calls;
mod cli;
mod common;
mod config;
mod log;
mod math;
mod plots;
mod raw_data;
mod setup;
mod sim;
mod spreadsheetorizer;
mod step;
mod task;

use log::*;
use plots::*;
use spreadsheetorizer::*;

// useful traits
use config::GenerateProcess;

#[tokio::main]

async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Simulation config defines the key parameters that are being used to generate data.
    let sim_config = config::SimConfig::default();
    // Create the evm god.
    let mut manager = SimulationManager::new();
    // Deploys initial contracts and agents.
    setup::run(&mut manager, &sim_config)?;

    // Grab the cli commands.
    let _ = cli::main(&manager).await?;

    /* let start_time = std::time::Instant::now();
    // All sim data is collected in the raw data container.
    let mut raw_data_container = raw_data::RawData::new();
    // Underlying price process that the sim will run on.
    let substrate = sim_config.generate();
    // Get the price vector to use for the simulation.
    let prices = substrate.generate_price_path().1;

    // Simulation setup:
    // - Deploy contracts
    // - Instantiate initial state of contracts, if any
    // - Create portfolio pool

    // Instantiates the arbitrageur agent.
    let arbitrageur = manager.agents.get("arbitrageur").unwrap();
    let arbitrageur = match arbitrageur {
        AgentType::SimpleArbitrageur(arbitrageur) => arbitrageur,
        _ => panic!("Arbitrageur not found! Was it initialized in setup.rs?"),
    };

    // Initialize the arbitrageur's start prices.
    setup::init_arbitrageur(arbitrageur, prices.clone()).await;

    // Approve portfolio to spend arbitrageur's tokens.
    let token0 = manager.deployed_contracts.get("token0").unwrap();
    let token1 = manager.deployed_contracts.get("token1").unwrap();
    let portfolio = manager.deployed_contracts.get("portfolio").unwrap();
    let mut arb_caller = calls::Caller::new(arbitrageur);
    let _ = arb_caller
        .approve(&token0, recast_address(portfolio.address), 0.0)
        .res()?;
    let _ = arb_caller
        .approve(&token1, recast_address(portfolio.address), 0.0)
        .res()?;

    // Simulation loop

    // Initialize the pool.
    let pool_id = setup::init_pool(&manager)?;

    // Add liquidity to the pool
    setup::allocate_liquidity(&manager, pool_id)?;

    // Run the first price update. This is important, as it triggers the arb detection.
    step::run(&manager, prices[0])?;

    // Logs initial simulation state.
    log::run(&manager, &mut raw_data_container, pool_id)?;

    for (i, price) in prices.iter().skip(1).enumerate() {
        println!("====== Sim step: {}, price: {} =========", i, price);

        // Run's the arbitrageur's task given the next desired tx.
        task::run(&manager, *price, pool_id)?;

        // Logs the simulation data.
        log::run(&manager, &mut raw_data_container, pool_id)?;

        // Increments the simulation forward.
        step::run(&manager, *price)?;
    }

    let output = OutputStorage {
        output_path: String::from(OUTPUT_DIRECTORY),
        output_file_names: String::from(OUTPUT_FILE_NAME),
    };

    let path = format!(
        "{}/{}_pool_id_{}.csv",
        output.output_path, output.output_file_names, pool_id
    );

    println!(
        "arb value {:?}",
        raw_data_container
            .derived_data
            .get(&pool_id)
            .unwrap()
            .arbitrageur_portfolio_value
    );

    // Write the sim data to a file.
    raw_data_container.write_to_disk(&path, pool_id)?;

    // Write some plots from the data.
    let plot = Plot::new(
        Display {
            transparent: false,
            mode: DisplayMode::Light,
            show: false,
        },
        raw_data_container.to_spreadsheet(pool_id),
    );
    plot.stacked_price_plot();
    plot.lp_pvf_plot();
    plot.arbitrageur_pvf_plot();

    // Simulation finish and log
    manager.shutdown();
    println!("Simulation finished.");

    let elapsed = start_time.elapsed();
    println!("Simulation took {} seconds to run.", elapsed.as_secs_f64()); */

    Ok(())
}
