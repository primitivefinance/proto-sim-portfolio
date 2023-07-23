use clap::Parser;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::{error::Error, fs::File};

use arbiter::{
    agent::*,
    environment::contract::*,
    manager::SimulationManager,
    stochastic::price_process::{PriceProcess, PriceProcessType, GBM, OU},
    utils::*,
};
use ethers::abi::Tokenize;
use ethers::prelude::U256;
use polars::prelude::*;
use revm::primitives::Address;
use visualize::{design::*, plot::*};

// dynamic... generated with build.sh
use bindings::i_portfolio_getters::*;

/// Struct for storing simulation data
pub struct SimData {
    pub pool_data: Vec<PoolsReturn>,
    pub arbitrageur_balances: Vec<HashMap<u64, U256>>, // maps token index (0 or 1) => balance
    pub reference_prices: Vec<U256>,
    pub portfolio_prices: Vec<U256>,
}

// Path: src/log.rs
//
// @notice
// Data collection is handled before a step in the simulation, so it starts at zero.
//
// @dev
// Collected data:
// 1. Pool data
// 2. Actor token balances
// 3. Reference market prices
pub fn run(
    manager: &SimulationManager,
    sim_data: &mut SimData,
    pool_id: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let admin = manager.agents.get("admin").unwrap();
    let arbitrageur = manager.agents.get("arbitrageur").unwrap();
    let token0 = manager.deployed_contracts.get("token0").unwrap();
    let token1 = manager.deployed_contracts.get("token1").unwrap();

    let arbitrageur_balance_0 = get_balance(admin, token0, arbitrageur.address())?;
    let arbitrageur_balance_1 = get_balance(admin, token1, arbitrageur.address())?;
    let mut arbitrageur_balance = HashMap::new();
    arbitrageur_balance.insert(0, arbitrageur_balance_0);
    arbitrageur_balance.insert(1, arbitrageur_balance_1);
    sim_data.arbitrageur_balances.push(arbitrageur_balance);

    let portfolio = manager.deployed_contracts.get("portfolio").unwrap();
    let pool_data = get_pool(admin, portfolio, pool_id)?;
    sim_data.pool_data.push(pool_data);

    let portfolio_prices = get_portfolio_prices(admin, portfolio, pool_id)?;
    sim_data.portfolio_prices.push(portfolio_prices);

    let exchange = manager.deployed_contracts.get("exchange").unwrap();
    let exchange_price = get_reference_price(admin, exchange, token0.address)?;
    sim_data.reference_prices.push(exchange_price);

    Ok(())
}

/// Calls portfolio.pools
fn get_pool(
    admin: &AgentType<IsActive>,
    portfolio: &SimulationContract<IsDeployed>,
    pool_id: u64,
) -> Result<PoolsReturn, Box<dyn std::error::Error>> {
    let result = admin.call(portfolio, "pools", pool_id.into_tokens())?;
    let pool_return: PoolsReturn = portfolio.decode_output("pools", unpack_execution(result)?)?;
    Ok(pool_return)
}

fn get_portfolio_prices(
    admin: &AgentType<IsActive>,
    portfolio: &SimulationContract<IsDeployed>,
    pool_id: u64,
) -> Result<U256, Box<dyn std::error::Error>> {
    let result = admin.call(portfolio, "getSpotPrice", pool_id.into_tokens())?;
    let portfolio_price: U256 =
        portfolio.decode_output("getSpotPrice", unpack_execution(result)?)?;
    Ok(portfolio_price)
}

/// Calls token.balanceOf
fn get_balance(
    admin: &AgentType<IsActive>,
    token: &SimulationContract<IsDeployed>,
    address: Address,
) -> Result<U256, Box<dyn std::error::Error>> {
    let result = admin.call(token, "balanceOf", recast_address(address).into_tokens())?;
    let balance: U256 = token.decode_output("balanceOf", unpack_execution(result)?)?;
    Ok(balance)
}

/// Calls exchange.getPrice
fn get_reference_price(
    admin: &AgentType<IsActive>,
    exchange: &SimulationContract<IsDeployed>,
    token: Address,
) -> Result<U256, Box<dyn std::error::Error>> {
    let result = admin.call(exchange, "getPrice", recast_address(token).into_tokens())?;
    let reference_price: U256 = exchange.decode_output("getPrice", unpack_execution(result)?)?;
    Ok(reference_price)
}

#[derive(Clone, Parser, Serialize, Deserialize, Debug)]
pub struct OutputStorage {
    pub output_path: String,
    pub output_file_names: String,
}

pub fn write_to_file(
    price_process: PriceProcess,
    data: &mut SimData,
) -> Result<(), Box<dyn Error>> {
    let output = OutputStorage {
        output_path: String::from("output"),
        output_file_names: String::from("portfolio"),
    };

    let series_length = data.pool_data.len();
    let seed = Series::new("seed", vec![price_process.seed; series_length]);
    let timestep = Series::new("timestep", vec![price_process.timestep; series_length]);

    let mut dataframe = make_series(data).unwrap();

    match price_process.process_type {
        PriceProcessType::OU(OU {
            volatility,
            mean_reversion_speed,
            mean_price,
        }) => {
            let volatility = Series::new("drift", vec![volatility; series_length]);
            let mean_reversion_speed = Series::new(
                "mean_reversion_speed",
                vec![mean_reversion_speed; series_length],
            );
            let mean_price = Series::new("mean_price", vec![mean_price; series_length]);

            dataframe.hstack_mut(&[
                volatility,
                timestep,
                seed,
                mean_reversion_speed,
                mean_price,
            ])?;

            println!("Dataframe: {:#?}", dataframe);
            let volatility = match price_process.process_type {
                PriceProcessType::GBM(GBM { volatility, .. }) => volatility,
                PriceProcessType::OU(OU { volatility, .. }) => volatility,
            };
            let file = File::create(format!(
                "{}/{}_{}_{}.csv",
                output.output_path, output.output_file_names, volatility, 0
            ))?;
            let mut writer = CsvWriter::new(file);
            writer.finish(&mut dataframe)?;
        }
        _ => {
            //na
        }
    };

    Ok(())
}

fn make_series(data: &mut SimData) -> Result<DataFrame, Box<dyn std::error::Error>> {
    // converts data.reference_prices to a float in a vector

    let exchange_prices = data
        .reference_prices
        .clone()
        .into_iter()
        .map(wad_to_float)
        .collect::<Vec<f64>>();

    // converts data.portfolio_prices to a float in a vector
    let portfolio_prices = data
        .portfolio_prices
        .clone()
        .into_iter()
        .map(wad_to_float)
        .collect::<Vec<f64>>();

    // converts each data.pool_data.virtualX to a float in a vector
    let reserve_x = data
        .pool_data
        .clone()
        .into_iter()
        .map(|x| U256::from(x.virtual_x))
        .into_iter()
        .map(wad_to_float)
        .collect::<Vec<f64>>();

    // converts each data.pool_data.virtualY to a float in a vector
    let reserve_y = data
        .pool_data
        .clone()
        .into_iter()
        .map(|y| U256::from(y.virtual_y))
        .map(wad_to_float)
        .collect::<Vec<f64>>();

    // converts data.arbitrageur_balances.get(token0.address) to a float in a vector
    let arb_x = data
        .arbitrageur_balances
        .clone()
        .into_iter()
        .map(|x| *x.get(&0).unwrap())
        .map(wad_to_float)
        .collect::<Vec<f64>>();

    // converts data.arbitrageur_balances.get(token1.address) to a float in a vector
    let arb_y = data
        .arbitrageur_balances
        .clone()
        .into_iter()
        .map(|x| *x.get(&1).unwrap())
        .map(wad_to_float)
        .collect::<Vec<f64>>();

    let data = DataFrame::new(vec![
        Series::new("portfolio_y_reserves", reserve_y),
        Series::new("portfolio_x_reserves", reserve_x),
        Series::new("portfolio_prices", portfolio_prices),
        Series::new("exchange_prices", exchange_prices),
        Series::new("arbitrageur_balance_x", arb_x),
        Series::new("arbitrageur_balance_y", arb_y),
    ])?;
    Ok(data)
}

pub fn plot_reserves(display: Display, data: &SimData) {
    let title: String = String::from("Reserves");

    let mut curves: Vec<Curve> = Vec::new();

    let reserve_x = data
        .pool_data
        .clone()
        .into_iter()
        .map(|x| ethers::prelude::types::U256::from(x.virtual_x) / 100)
        .into_iter()
        .map(wad_to_float)
        .collect::<Vec<f64>>();

    let reserve_y = data
        .pool_data
        .clone()
        .into_iter()
        .map(|y| ethers::prelude::types::U256::from(y.virtual_y) / 100)
        .map(wad_to_float)
        .collect::<Vec<f64>>();

    let length = data.pool_data.clone().len();
    let x_coordinates = itertools_num::linspace(0.0, length as f64, length).collect::<Vec<f64>>();

    curves.push(Curve {
        x_coordinates: x_coordinates.clone(),
        y_coordinates: reserve_x.clone(),
        design: CurveDesign {
            color: Color::Green,
            color_slot: 1,
            style: Style::Lines(LineEmphasis::Light),
        },
        name: Some(format!("{}", "Rx")),
    });

    curves.push(Curve {
        x_coordinates: x_coordinates.clone(),
        y_coordinates: reserve_y.clone(),
        design: CurveDesign {
            color: Color::Blue,
            color_slot: 1,
            style: Style::Lines(LineEmphasis::Light),
        },
        name: Some(format!("{}", "Ry")),
    });

    if let Some(last_point) = x_coordinates.last() {
        let min_y = curves[0]
            .y_coordinates
            .iter()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();
        let max_y = curves[0]
            .y_coordinates
            .iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();

        println!("min_y: {}", min_y);
        println!("max_y: {}", max_y);

        let axes = Axes {
            x_label: String::from("X"),
            y_label: String::from("Y"), // todo: add better y label
            bounds: (vec![x_coordinates[0], *last_point], vec![*min_y, *max_y]),
        };

        // Plot it.
        transparent_plot(
            Some(curves),
            None,
            axes,
            title,
            display,
            Some("reserves.html".to_string()),
        );
    } else {
        println!("x coords are empty");
    }
}

pub fn plot_prices(display: Display, data: &SimData) {
    let title: String = String::from("Prices");

    let mut curves: Vec<Curve> = Vec::new();

    let portfolio_prices = data
        .portfolio_prices
        .clone()
        .into_iter()
        .map(wad_to_float)
        .collect::<Vec<f64>>();

    let reference_prices = data
        .reference_prices
        .clone()
        .into_iter()
        .map(wad_to_float)
        .collect::<Vec<f64>>();
    let length = data.portfolio_prices.clone().len();
    let x_coordinates = itertools_num::linspace(0.0, length as f64, length).collect::<Vec<f64>>();

    curves.push(Curve {
        x_coordinates: x_coordinates.clone(),
        y_coordinates: portfolio_prices.clone(),
        design: CurveDesign {
            color: Color::Green,
            color_slot: 1,
            style: Style::Lines(LineEmphasis::Light),
        },
        name: Some(format!("{}", "Spot")),
    });

    curves.push(Curve {
        x_coordinates: x_coordinates.clone(),
        y_coordinates: reference_prices.clone(),
        design: CurveDesign {
            color: Color::Blue,
            color_slot: 1,
            style: Style::Lines(LineEmphasis::Light),
        },
        name: Some(format!("{}", "Ref")),
    });

    if let Some(last_point) = x_coordinates.last() {
        let min_y = curves
            .iter()
            .flat_map(|curve| &curve.y_coordinates)
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap(); // assumes no NANs
        let max_y = curves
            .iter()
            .flat_map(|curve| &curve.y_coordinates)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap(); // assumes no NANs

        println!("min_y: {}", min_y);
        println!("max_y: {}", max_y);

        let axes = Axes {
            x_label: String::from("X"),
            y_label: String::from("Y"), // todo: add better y label
            bounds: (vec![x_coordinates[0], *last_point], vec![*min_y, *max_y]),
        };

        // Plot it.
        transparent_plot(
            Some(curves),
            None,
            axes,
            title,
            display,
            Some("prices.html".to_string()),
        );
    } else {
        println!("x coords are empty");
    }
}

pub fn plot_vs_price(
    display: Display,
    plot_name: String,
    x_coordinates: Vec<f64>,
    y_coordinates: Vec<Vec<f64>>,
) {
    let title: String = plot_name;

    let lines = y_coordinates.len();
    // for each line, a vec of f64 in the y_coords arg, make a curve and push to curves
    let mut curves = Vec::new();
    for i in 0..lines {
        // get a random color given i
        let color = match i {
            0 => Color::Blue,
            1 => Color::Green,
            2 => Color::White,
            3 => Color::Black,
            4 => Color::Purple,
            _ => Color::Green,
        };

        // get a random color slot given i
        let color_slot = i % 8;

        let curve = Curve {
            x_coordinates: x_coordinates.clone(),
            y_coordinates: y_coordinates[i].clone(),
            design: CurveDesign {
                color: color,
                color_slot: color_slot,
                style: Style::Lines(LineEmphasis::Light),
            },
            name: Some(format!("{} {}", "\\tau=", 1.0)),
        };
        curves.push(curve);
    }

    /* let curve = Curve {
        x_coordinates: x_coordinates.clone(),
        y_coordinates: y_coordinates.clone(),
        design: CurveDesign {
            color: Color::Green,
            color_slot: 1,
            style: Style::Lines(LineEmphasis::Light),
        },
        name: Some(format!("{} {}", "\\tau=", 1.0)),
    };

    // Capable of graphing multiple liquidity distributions, edit this code to do so.
    let curves = vec![curve]; */

    // Build the plot's axes
    if let Some(last_point) = x_coordinates.last() {
        let min_y = curves[0]
            .y_coordinates
            .iter()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();
        let max_y = curves[0]
            .y_coordinates
            .iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();

        println!("min_y: {}", min_y);
        println!("max_y: {}", max_y);

        let axes = Axes {
            x_label: String::from("X"),
            y_label: String::from("Y"), // todo: add better y label
            bounds: (vec![x_coordinates[0], *last_point], vec![0.4, 1.2]),
        };

        // Plot it.
        transparent_plot(Some(curves), None, axes, title, display, None);
    } else {
        println!("prices is empty");
    }
}
