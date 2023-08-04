/// Configuration for the simulation.
/// Includes all the key parameters used to generate
/// the price process, the agent parameters,
/// and the pool parameters.
use arbiter::stochastic::price_process::{PriceProcess, PriceProcessType, OU};
use colored::*;
use config::{Config, ConfigError};
use serde_derive::Deserialize;

/// # Economic
/// Defines the parameters of a pool and
/// the initial economic state of the underlying price process.
///
/// # Fields
/// * `initial_price` - Initial price process and pool price. (f64)
/// * `pool_volatility_f` - Normal strategy pool's volatility parameter. (f64)
/// * `pool_strike_price_f` - Normal strategy pool's strike price parameter. (f64)
/// * `pool_time_remaining_years_f` - Normal strategy pool's time remaining seconds parameter. Note: not supported yet. (f64)
/// * `pool_is_perpetual` - Normal strategy pool's is perpetual parameter. Sets tau to be constant. (bool)
#[derive(Clone, Debug, Deserialize)]
#[allow(unused)] // todo: use
pub struct Economic {
    pool_volatility_f: f64,
    pool_strike_price_f: f64,
    pool_time_remaining_years_f: f64,
    pool_is_perpetual: bool,
}

/// # SimConfig
/// Data structure to hold the parameters for the sim.
#[derive(Clone, Debug, Deserialize)]
pub struct SimConfig {
    pub process: PriceProcess,
    pub economic: Economic,
}

impl SimConfig {
    /// Loads the `arbiter.toml` configuration file and attempts to deserialize it into a `SimConfig`.
    pub fn new() -> Result<Self, ConfigError> {
        let settings = Config::builder()
            .add_source(config::File::with_name("arbiter"))
            .add_source(config::Environment::with_prefix("ARBITER"))
            .build()?;

        settings.try_deserialize()
    }
}

pub fn main() -> SimConfig {
    let settings = SimConfig::new().unwrap();
    println!(
        "{}\n{}\n{:#?}\n{}",
        "Configuration:".bright_yellow(),
        "------------------".bright_yellow(),
        settings,
        "------------------".bright_yellow()
    );
    settings
}

/// # Default Parameterization
impl Default for SimConfig {
    /// Default parameters are:
    /// initial price: 1
    /// process volatility: 1%
    /// process mean reversion speed: 10
    /// process mean price: 1
    /// process timestep: 0.01
    /// process num_steps: 10
    /// pool volatility: 10%
    /// pool strike price: 1.0
    /// pool time remaining years: 1.0
    /// pool is perpetual: true
    fn default() -> Self {
        SimConfig {
            process: PriceProcess {
                process_type: PriceProcessType::OU(OU::new(0.01, 10.0, 1.0)),
                timestep: 0.01,
                timescale: "steps".to_string(),
                num_steps: 10,
                initial_price: 1.0,
                seed: 1,
            },

            economic: Economic {
                pool_volatility_f: 0.1,
                pool_strike_price_f: 1.0,
                pool_time_remaining_years_f: 1.0,
                pool_is_perpetual: true,
            },
        }
    }
}
