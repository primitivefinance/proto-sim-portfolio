use arbiter::stochastic::price_process::{PriceProcess, PriceProcessType, OU};
use arbiter::utils::wad_to_float;
use arbiter::{
    agent::{Agent, AgentType},
    manager::SimulationManager,
    utils::recast_address,
};
use ethers::abi::Tokenize;
use log::plot_trading_curve;
use visualize::{design::*, plot::*};

// dynamic imports... generate with build.sh

mod common;
mod log;
mod math;
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

    // Write the sim data to a file.
    log::write_to_file(price_process, &mut sim_data)?;

    let display = Display {
        transparent: false,
        mode: DisplayMode::Light,
        show: false,
    };

    log::plot_reserves(display.clone(), &sim_data);
    log::plot_prices(display.clone(), &sim_data);

    // uncomment to plot the trading curve error
    //let library = log::deploy_external_normal_strategy_lib(&mut manager).unwrap();
    //trading_curve_analysis(&manager);

    // Simulation finish and log
    manager.shutdown();
    println!("Simulation finished.");

    Ok(())
}

fn trading_curve_analysis(manager: &SimulationManager) {
    let library = manager.deployed_contracts.get("library").unwrap();
    let admin = manager.agents.get("admin").unwrap();

    let mut curve: math::NormalCurve = math::NormalCurve {
        reserve_x_per_wad: 0.308537538726,
        reserve_y_per_wad: 0.308537538726,
        strike_price_f: 1.0,
        std_dev_f: 1.0,
        time_remaining_sec: 31556953.0,
        invariant_f: 0.0,
    };

    let sol_y = log::approximate_y_given_x(admin, library, curve.clone()).unwrap();
    let rust_coordinates = math::get_trading_function_coordinates(curve.clone());

    let mut sol_coordinates = Vec::new();
    let mut curve_copy = curve.clone();

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
    plot_trading_curve(display, curves);
}
