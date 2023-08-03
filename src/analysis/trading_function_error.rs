/// Analyzes the trading function solidity against the rust implementation.
use crate::calls::{Caller, DecodedReturns};
use crate::math::NormalCurve as RustInput;
use itertools_num::linspace;
use visualize::{
    design::{Color, CurveDesign, DisplayMode},
    plot::{transparent_plot, Axes, Curve, Display},
};

use anyhow::anyhow;
use arbiter::{
    manager::SimulationManager,
    utils::{float_to_wad, wad_to_float},
};
use bindings::external_normal_strategy_lib::{
    ApproximateYGivenXCall, NormalCurve as SolidityInput,
};
use ethers::abi::{Tokenizable, Tokenize};

/// Input for the data.
#[derive(Clone, Debug)]
struct Input(SolidityInput);

/// Output format of the data.
#[derive(Clone, Debug)]
struct Output {
    pub output_sol: f64,
    pub output_rs: f64,
}

/// Each data point.
#[derive(Clone, Debug)]
struct DataPoint {
    pub input: Input,
    pub output: Output,
}

/// Collection of data.
#[derive(Clone, Debug)]
struct Results {
    pub data: Vec<DataPoint>,
}

static STEP: f64 = 0.001;
static DIR: &str = "./out_data";
static FILE: &str = "trading_function_error";

/// Plots the trading function error.
pub fn main(manager: &SimulationManager) -> anyhow::Result<(), anyhow::Error> {
    let start_time = std::time::Instant::now();

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

    // Print the time to run.
    let elapsed = start_time.elapsed();
    println!(
        "\nTrading Function Error Analysis took {} seconds to run.",
        elapsed.as_secs_f64()
    );

    // Print the data.
    println!(
        "data.output: {:?}",
        data.into_iter()
            .map(|x| x.output.output_sol.clone())
            .collect::<Vec<f64>>()
    );

    // Plot the data.
    let len = 1.0 / STEP;
    let curve_rs = Curve {
        x_coordinates: linspace(0.0, len, len as usize).collect::<Vec<f64>>(),
        y_coordinates: rs.clone(),
        design: CurveDesign {
            color: Color::Green,
            color_slot: 1,
            style: visualize::design::Style::Lines(visualize::design::LineEmphasis::Light),
        },
        name: Some("rust".to_string()),
    };

    let curve_sol = Curve {
        x_coordinates: linspace(0.0, len, len as usize).collect::<Vec<f64>>(),
        y_coordinates: sol.clone(),
        design: CurveDesign {
            color: Color::Blue,
            color_slot: 1,
            style: visualize::design::Style::Lines(visualize::design::LineEmphasis::Light),
        },
        name: Some("solidity".to_string()),
    };

    let curves: Vec<Curve> = vec![curve_rs, curve_sol];

    let y_coordinates_flat = curves
        .iter()
        .flat_map(|curve| curve.y_coordinates.clone())
        .collect::<Vec<f64>>();

    let min_y = y_coordinates_flat
        .iter()
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max_y = y_coordinates_flat
        .iter()
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();

    let axes = Axes {
        x_label: String::from("X"),
        y_label: String::from("Y"), // todo: add better y label
        bounds: (vec![0.0, 1.0], vec![*min_y, *max_y]),
    };

    let display = Display {
        transparent: false,
        mode: DisplayMode::Light,
        show: false,
    };

    transparent_plot(
        Some(curves),
        None,
        axes,
        "Trading Function Error".to_string(),
        display,
        Some(format!("{}/{}.html", DIR.to_string(), FILE.to_string())),
    );

    Ok(())
}
