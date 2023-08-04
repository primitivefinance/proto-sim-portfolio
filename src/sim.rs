/// Runs a simulation using the config.
use arbiter::{agent::AgentType, manager::SimulationManager, utils::recast_address};
use colored::*;
use visualize;

pub static OUTPUT_DIRECTORY: &str = "out_data";
pub static OUTPUT_FILE_NAME: &str = "results";

// useful traits
use crate::calls;
use crate::config::SimConfig;
use crate::log;
use crate::plots;
use crate::raw_data;
use crate::setup;
use crate::spreadsheetorizer::{DiskWritable, Spreadsheet};
use crate::step;
use crate::task;

/// Runs the simulation using the config and logs the data to `out_data`.
///
/// # Errors
/// - The `out_data` directory does not exist.
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Simulation config defines the key parameters that are being used to generate data.
    let sim_config = SimConfig::new().unwrap_or(SimConfig::default());
    // Create the evm god.
    let mut manager = SimulationManager::new();
    // Deploys initial contracts and agents.
    setup::run(&mut manager, &sim_config)?;
    // All sim data is collected in the raw data container.
    let mut raw_data_container = raw_data::RawData::new();
    // Underlying price process that the sim will run on.
    let substrate = &sim_config.process;
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
    let exchange = manager.deployed_contracts.get("exchange").unwrap();
    let token0 = manager.deployed_contracts.get("token0").unwrap();
    let token1 = manager.deployed_contracts.get("token1").unwrap();
    let portfolio = manager.deployed_contracts.get("portfolio").unwrap();

    // Arbitrageur approvals...
    let mut arb_caller = calls::Caller::new(arbitrageur);
    arb_caller
        .approve(&token0, recast_address(portfolio.address), 0.0)
        .res()?;
    arb_caller
        .approve(&token1, recast_address(portfolio.address), 0.0)
        .res()?;
    arb_caller
        .approve(&token0, recast_address(exchange.address), 0.0)
        .res()?;
    arb_caller
        .approve(&token1, recast_address(exchange.address), 0.0)
        .res()?;

    // Simulation loop

    // Initialize the pool.
    let pool_id = setup::init_pool(&manager, &sim_config)?;

    // Add liquidity to the pool
    setup::allocate_liquidity(&manager, pool_id)?;

    // Run the first price update. This is important, as it triggers the arb detection.
    step::run(&manager, prices[0])?;

    // Logs initial simulation state.
    log::run(&manager, &mut raw_data_container, pool_id)?;

    println!("{}", "Running...".bright_yellow());
    for (i, price) in prices.iter().skip(1).enumerate() {
        if std::env::var("VERBOSE").is_ok() {
            println!("====== Sim step: {}, price: {} =========", i, price);
        }

        // Run's the arbitrageur's task given the next desired tx.
        task::run(&manager, &mut raw_data_container, *price, pool_id)?;

        // Logs the simulation data.
        log::run(&manager, &mut raw_data_container, pool_id)?;

        // Increments the simulation forward.
        step::run(&manager, *price)?;
    }

    let output = log::OutputStorage {
        output_path: String::from(OUTPUT_DIRECTORY),
        output_file_names: String::from(OUTPUT_FILE_NAME),
    };

    let path = format!(
        "{}/{}_pool_id_{}.csv",
        output.output_path, output.output_file_names, pool_id
    );

    // Write the sim data to a file.
    raw_data_container.write_to_disk(&path, pool_id)?;

    // Write some plots from the data.
    let plot = plots::Plot::new(
        visualize::plot::Display {
            transparent: false,
            mode: visualize::design::DisplayMode::Light,
            show: false,
        },
        raw_data_container.to_spreadsheet(pool_id),
    );
    plot.stacked_price_plot();
    plot.lp_pvf_plot();
    plot.arbitrageur_pvf_plot();
    plot.portfolio_volume_plot();
    plot.portfolio_volume_cumulative_plot();

    // Simulation finish and log
    manager.shutdown();

    Ok(())
}
