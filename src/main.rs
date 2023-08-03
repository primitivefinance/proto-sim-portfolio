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
//use log::plot_trading_curve;

// dynamic imports... generate with build.sh

pub static OUTPUT_DIRECTORY: &str = "out_data";
pub static OUTPUT_FILE_NAME: &str = "results";

mod bisection;
mod calls;
mod common;
mod config;
mod log;
mod math;
mod plots;
mod raw_data;
mod setup;
mod spreadsheetorizer;
mod step;
mod task;

use log::*;
use plots::*;
use spreadsheetorizer::*;

// useful traits
use config::GenerateProcess;

/// todo: finish the arbitrageur task with the new rust bisection
/// todo: integrate the config into the pool creation process
/// todo: move the contract call functions into the calls.rs file
#[tokio::main]

async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // All sim data is collected in the raw data container.
    let mut raw_data_container = raw_data::RawData::new();
    // Simulation config defines the key parameters that are being used to generate data.
    let sim_config = config::SimConfig::default();
    // Underlying price process that the sim will run on.
    let substrate = sim_config.generate();
    // Get the price vector to use for the simulation.
    let prices = substrate.generate_price_path().1;

    // Simulation setup:
    // - Deploy contracts
    // - Instantiate initial state of contracts, if any
    // - Create portfolio pool
    let mut manager = SimulationManager::new();
    setup::run(&mut manager, &sim_config)?;

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
    arb_caller
        .approve(&token0, recast_address(portfolio.address), 0.0)
        .unwrap();
    arb_caller
        .approve(&token1, recast_address(portfolio.address), 0.0)
        .unwrap();

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

    Ok(())
}

fn get_config(manager: &SimulationManager, pool_id: u64) -> Result<(), Box<dyn std::error::Error>> {
    let library = manager.deployed_contracts.get("library").unwrap();
    let admin = manager.agents.get("admin").unwrap();
    let config = log::get_configuration(admin, library, pool_id)?;
    println!("config: {:?}", config);

    Ok(())
}

fn trading_curve_analysis(manager: &SimulationManager) {
    let library = manager.deployed_contracts.get("library").unwrap();
    let admin = manager.agents.get("admin").unwrap();

    let curve: math::NormalCurve = math::NormalCurve {
        reserve_x_per_wad: 0.308537538726,
        reserve_y_per_wad: 0.308537538726,
        strike_price_f: 1.0,
        std_dev_f: 1.0,
        time_remaining_sec: 31556953.0,
        invariant_f: 0.0,
    };

    let approx_amount_out = curve.approximate_amount_out(true, 0.1);
    println!("approx_amount_out: {}", approx_amount_out);

    let sol_y = log::approximate_y_given_x(admin, library, curve.clone()).unwrap();
    let rust_coordinates = curve.get_trading_function_coordinates();

    let mut sol_coordinates = Vec::new();
    let mut curve_copy = curve;

    let mut x = 0.0;
    let mut y = 0.0;
    while x < 1.0 {
        curve_copy.reserve_x_per_wad = x;
        let y_wad = log::approximate_y_given_x(admin, library, curve_copy.clone()).unwrap();
        y = wad_to_float(y_wad);
        sol_coordinates.push((x, y));
        x += 0.01;
    }

    let mut curves: Vec<Curve> = Vec::new();

    // difference between sol coords and rust coords
    let y_coords_error = rust_coordinates
        .clone()
        .into_iter()
        .zip(sol_coordinates.clone().into_iter())
        .map(|(x, y)| (x.1 - y.1).abs())
        .collect::<Vec<f64>>();

    /*  curves.push(Curve {
        x_coordinates: rust_coordinates
            .clone()
            .into_iter()
            .map(|x| x.0)
            .collect::<Vec<f64>>(),
        y_coordinates: rust_coordinates
            .clone()
            .into_iter()
            .map(|x| x.1)
            .collect::<Vec<f64>>(),
        design: CurveDesign {
            color: Color::Green,
            color_slot: 1,
            style: Style::Lines(LineEmphasis::Light),
        },
        name: Some(format!("{}", ".rs")),
    });

    curves.push(Curve {
        x_coordinates: sol_coordinates
            .clone()
            .into_iter()
            .map(|x| x.0)
            .collect::<Vec<f64>>(),
        y_coordinates: sol_coordinates
            .clone()
            .into_iter()
            .map(|x| x.1)
            .collect::<Vec<f64>>(),
        design: CurveDesign {
            color: Color::Blue,
            color_slot: 1,
            style: Style::Lines(LineEmphasis::Light),
        },
        name: Some(format!("{}", ".sol")),
    }); */

    curves.push(Curve {
        x_coordinates: rust_coordinates
            .clone()
            .into_iter()
            .map(|x| x.0)
            .collect::<Vec<f64>>(),
        y_coordinates: y_coords_error.clone(),
        design: CurveDesign {
            color: Color::Purple,
            color_slot: 1,
            style: Style::Lines(LineEmphasis::Light),
        },
        name: Some(format!("{}", ".rs")),
    });

    let display = Display {
        transparent: false,
        mode: DisplayMode::Light,
        show: false,
    };
    //plot_trading_curve(display, curves);
}
