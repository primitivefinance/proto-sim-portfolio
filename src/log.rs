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

use super::math;

// dynamic... generated with build.sh
use bindings::{external_normal_strategy_lib, i_portfolio_getters::*};

pub static OUTPUT_DIRECTORY: &str = "out_data";
pub static OUTPUT_FILE_NAME: &str = "results";

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

pub fn deploy_external_normal_strategy_lib(
    manager: &mut SimulationManager,
) -> Result<&SimulationContract<IsDeployed>, Box<dyn std::error::Error>> {
    let admin = manager.agents.get("admin").unwrap();
    let library = SimulationContract::new(
        external_normal_strategy_lib::EXTERNALNORMALSTRATEGYLIB_ABI.clone(),
        external_normal_strategy_lib::EXTERNALNORMALSTRATEGYLIB_BYTECODE.clone(),
    );
    let (library_contract, _) = admin.deploy(library, vec![])?;
    manager
        .deployed_contracts
        .insert("library".to_string(), library_contract);

    let library = manager.deployed_contracts.get("library").unwrap();
    Ok(library)
}

pub fn approximate_y_given_x(
    admin: &AgentType<IsActive>,
    library: &SimulationContract<IsDeployed>,
    curve: math::NormalCurve,
) -> Result<ethers::types::U256, Box<dyn std::error::Error>> {
    let arguments: external_normal_strategy_lib::NormalCurve =
        external_normal_strategy_lib::NormalCurve {
            reserve_x_per_wad: float_to_wad(curve.reserve_x_per_wad),
            reserve_y_per_wad: float_to_wad(curve.reserve_y_per_wad),
            strike_price_wad: float_to_wad(curve.strike_price_f),
            standard_deviation_wad: float_to_wad(curve.std_dev_f),
            time_remaining_seconds: (curve.time_remaining_sec as u32).into(),
            invariant: (0).into(),
        };
    let result = admin.call(
        library,
        "approximateYGivenX",
        external_normal_strategy_lib::ApproximateYGivenXCall { self_: arguments }.into_tokens(),
    )?;
    let decoded: ethers::types::U256 =
        library.decode_output("approximateYGivenX", unpack_execution(result)?)?;
    Ok(decoded)
}

pub fn trading_function(
    admin: &AgentType<IsActive>,
    library: &SimulationContract<IsDeployed>,
    curve: math::NormalCurve,
) -> Result<ethers::types::I256, Box<dyn std::error::Error>> {
    let arguments: external_normal_strategy_lib::NormalCurve =
        external_normal_strategy_lib::NormalCurve {
            reserve_x_per_wad: float_to_wad(curve.reserve_x_per_wad).into(),
            reserve_y_per_wad: float_to_wad(curve.reserve_y_per_wad).into(),
            strike_price_wad: float_to_wad(curve.strike_price_f).into(),
            standard_deviation_wad: float_to_wad(curve.std_dev_f).into(),
            time_remaining_seconds: (curve.time_remaining_sec as u32).into(),
            invariant: (0).into(),
        };
    let result = admin.call(library, "tradingFunction", arguments.into_tokens())?;
    let decoded: ethers::types::I256 =
        library.decode_output("tradingFunction", unpack_execution(result)?)?;
    Ok(decoded)
}

pub fn get_configuration(
    admin: &AgentType<IsActive>,
    external_normal_strategy: &SimulationContract<IsDeployed>,
    pool_id: u64,
) -> Result<external_normal_strategy_lib::NormalCurve, Box<dyn std::error::Error>> {
    let result = admin.call(
        external_normal_strategy,
        "getCurveConfiguration",
        pool_id.into_tokens(),
    )?;
    let pool_return: external_normal_strategy_lib::NormalCurve = external_normal_strategy
        .decode_output("getCurveConfiguration", unpack_execution(result)?)?;
    Ok(pool_return)
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
        output_path: String::from(OUTPUT_DIRECTORY),
        output_file_names: String::from(OUTPUT_FILE_NAME),
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
            Some(format!("{}/reserves.html", OUTPUT_DIRECTORY.to_string())),
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
            Some(format!("{}/prices.html", OUTPUT_DIRECTORY.to_string())),
        );
    } else {
        println!("x coords are empty");
    }
}

pub fn plot_trading_curve(display: Display, curves: Vec<Curve>) {
    // plot the trading curve coordinates using transparent_plot
    let title: String = String::from("Trading Curve");

    if let Some(last_point) = curves[0].x_coordinates.clone().last() {
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
            bounds: (vec![0.0, last_point.clone()], vec![*min_y, *max_y]),
        };

        // Plot it.
        transparent_plot(
            Some(curves),
            None,
            axes,
            title,
            display,
            Some(format!(
                "{}/trading_curve.html",
                OUTPUT_DIRECTORY.to_string()
            )),
        );
    } else {
        println!("x coords are empty");
    }
}
