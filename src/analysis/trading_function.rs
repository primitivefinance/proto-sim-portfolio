/// Analyzes the trading function solidity against the rust implementation.
use crate::calls::{Caller, DecodedReturns};
use crate::math::NormalCurve as RustInput;
use crate::plots::get_coordinate_bounds;
use itertools_num::linspace;
use visualize::{
    design::{Color, CurveDesign, DisplayMode},
    plot::{transparent_plot, Axes, Curve, Display},
};

use super::TradingFunctionSubtype;
use crate::config;
use crate::setup;
use anyhow::anyhow;
use arbiter::{
    manager::SimulationManager,
    utils::{float_to_wad, wad_to_float},
};
use bindings::external_normal_strategy_lib::NormalCurve as SolidityInput;
use chrono::Local;
use ethers::abi::Tokenizable;

/// Input for the data.
#[derive(Clone, Debug)]
struct Input(SolidityInput);

/// Output format of the data.
#[derive(Clone, Debug)]
#[allow(unused)]
struct Output {
    pub output_sol: f64,
    pub output_rs: f64,
}

/// Each data point.
#[derive(Clone, Debug)]
#[allow(unused)]
struct DataPoint {
    pub input: Input,
    pub output: Output,
}

/// Collection of data.
#[derive(Clone, Debug)]
#[allow(unused)]
struct Results {
    pub data: Vec<DataPoint>,
}

static STEP: f64 = 0.001;
static DIR: &str = "./out_data";
static FILE: &str = "trading_function_analysis";

/// Plots the trading function error.
pub fn main(subtype: TradingFunctionSubtype) -> anyhow::Result<(), anyhow::Error> {
    // Simulation config defines the key parameters that are being used to generate data.
    let sim_config = config::main();
    // Create the evm god.
    let mut manager = SimulationManager::new();
    // Deploys initial contracts and agents.
    let init = setup::run(&mut manager, &sim_config);
    match init {
        Ok(_) => {}
        Err(e) => {
            return Err(anyhow!(
                "Error in analyze trading function, setup.rs step: {}",
                e
            ));
        }
    }

    let timestamp = Local::now();

    let library = manager.deployed_contracts.get("library").unwrap();
    let admin = manager.agents.get("admin").unwrap();
    let mut caller = Caller::new(admin);

    let mut input_rs = RustInput {
        reserve_x_per_wad: 0.308537538726,
        reserve_y_per_wad: 0.308537538726,
        strike_price_f: 1.0,
        std_dev_f: 1.0,
        time_remaining_sec: 31556953.0,
        invariant_f: 0.0,
    };

    let mut input_sol = Input(SolidityInput {
        reserve_x_per_wad: float_to_wad(input_rs.reserve_x_per_wad),
        reserve_y_per_wad: float_to_wad(input_rs.reserve_y_per_wad),
        strike_price_wad: float_to_wad(input_rs.strike_price_f),
        standard_deviation_wad: float_to_wad(input_rs.std_dev_f),
        time_remaining_seconds: 31556953.into(),
        invariant: 0.into(),
    });

    let mut inputs = Vec::<Input>::new();
    let mut sol = Vec::<f64>::new();
    let mut rs = Vec::<f64>::new();

    let mut x = 0.0;
    let mut y = 0.0;

    // Collect y coordinates from sol & rust at x coordinates with a distance of STEP.
    // Important that x != 1.0, as that is outside the domain of the functions.
    while x <= 1.0 {
        let _ = y; // does nothing. Just to silence the compiler warning.

        // First step cannot be zero! Undefined input for the math functions.
        x += STEP;

        // Edit the rust input.
        input_rs.reserve_x_per_wad = x;

        // Compute the rust output.
        y = input_rs.approximate_y_given_x_floating();

        // Edit the rust output.
        rs.push(y);

        // Edit the rust input with the new y value.
        input_rs.reserve_y_per_wad = y;

        // Edit the solidity input.
        input_sol.0.reserve_x_per_wad = float_to_wad(x);

        // Compute the solidity output and edit the input.
        input_sol.0.reserve_y_per_wad = caller
            .call(
                library,
                "approximateYGivenX",
                vec![input_sol.0.clone().into_token()],
            )?
            .decoded(library)?;

        // Edit the solidity output.
        sol.push(wad_to_float(input_sol.0.reserve_y_per_wad));

        // Add the input to the inputs vector.
        inputs.push(input_sol.clone());
    }

    // Assert both y coordinates are the same length
    if sol.len() != rs.len() {
        return Err(anyhow!("sol.len() != rs.len()"));
    }

    // Compute the error solidity - rust.
    let error = sol
        .clone()
        .into_iter()
        .zip(rs.clone().into_iter())
        .map(|(x, y)| x - y)
        .collect::<Vec<f64>>();

    // Format the data into the Results struct.
    let mut data = Vec::<DataPoint>::new();

    for i in 0..sol.len() {
        data.push(DataPoint {
            input: inputs[i].clone(),
            output: Output {
                output_sol: sol[i],
                output_rs: rs[i],
            },
        });
    }

    // Plot the data.
    let len = rs.len();
    let x_coordinates = linspace(0.0, len as f64, len).collect::<Vec<f64>>();

    let mut last_x = 0.0;
    let _ = last_x; // does nothing. Just to silence the compiler warning.
    if let Some(last_point) = x_coordinates.clone().last() {
        last_x = *last_point;
    } else {
        return Err(anyhow!("last_point is None"));
    }

    let curve_err = Curve {
        x_coordinates: x_coordinates.clone(),
        y_coordinates: error.clone(),
        design: CurveDesign {
            color: Color::Purple,
            color_slot: 1,
            style: visualize::design::Style::Lines(visualize::design::LineEmphasis::Light),
        },
        name: Some("error".to_string()),
    };

    let curve_rs = Curve {
        x_coordinates: x_coordinates.clone(),
        y_coordinates: rs.clone(),
        design: CurveDesign {
            color: Color::Green,
            color_slot: 1,
            style: visualize::design::Style::Lines(visualize::design::LineEmphasis::Light),
        },
        name: Some("rust".to_string()),
    };

    let curve_sol = Curve {
        x_coordinates: x_coordinates.clone(),
        y_coordinates: sol.clone(),
        design: CurveDesign {
            color: Color::Blue,
            color_slot: 1,
            style: visualize::design::Style::Lines(visualize::design::LineEmphasis::Light),
        },
        name: Some("solidity".to_string()),
    };

    let display = Display {
        transparent: false,
        mode: DisplayMode::Light,
        show: false,
    };

    match subtype {
        TradingFunctionSubtype::Error => {
            let curves: Vec<Curve> = vec![curve_err];

            let (min_y, max_y) = get_coordinate_bounds(
                curves
                    .iter()
                    .map(|x| x.y_coordinates.clone())
                    .collect::<Vec<Vec<f64>>>(),
            );

            let axes = Axes {
                x_label: String::from("X"),
                y_label: String::from("Y"), // todo: add better y label
                bounds: (vec![0.0, last_x], vec![min_y, max_y]),
            };

            transparent_plot(
                Some(curves),
                None,
                axes,
                "Trading Function Error".to_string(),
                display,
                Some(format!("{}/{}.html", DIR.to_string(), FILE.to_string())),
            );
        }
        TradingFunctionSubtype::Curve => {
            let curves: Vec<Curve> = vec![curve_sol, curve_rs];

            let (min_y, max_y) = get_coordinate_bounds(
                curves
                    .iter()
                    .map(|x| x.y_coordinates.clone())
                    .collect::<Vec<Vec<f64>>>(),
            );

            let axes = Axes {
                x_label: String::from("X"),
                y_label: String::from("Y"), // todo: add better y label
                bounds: (vec![0.0, last_x], vec![min_y, max_y]),
            };

            transparent_plot(
                Some(curves),
                None,
                axes,
                "Trading Function Error".to_string(),
                display,
                Some(format!(
                    "{}/{}_{}.html",
                    DIR.to_string(),
                    FILE.to_string(),
                    timestamp.to_string()
                )),
            );
        }
    }

    Ok(())
}
