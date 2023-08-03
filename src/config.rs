use arbiter::stochastic::price_process::{PriceProcess, PriceProcessType, OU};

/// Configuration for the simulation.
/// Includes all the key parameters used to generate
/// the price process, the agent parameters,
/// and the pool parameters.

/// # SimConfig
/// Data structure to hold the parameters for the sim.
#[derive(Clone, Debug)]
pub struct SimConfig {
    pub process: Process,
    pub timeline: Timeline,
    pub economic: Economic,
}

pub trait GenerateProcess {
    fn generate(&self) -> PriceProcess;
}

/// # GenerateProcess
/// Trait for generating a price process using the sim config.
impl GenerateProcess for SimConfig {
    /// Generates an OU process using the configuration parameters.
    fn generate(&self) -> PriceProcess {
        let ou = OU::new(
            self.process.volatility,
            self.process.mean_reversion_speed,
            self.process.mean_price,
        );
        PriceProcess::new(
            PriceProcessType::OU(ou),
            self.timeline.timestep,
            "OU".to_string(),
            self.timeline.num_steps,
            self.economic.initial_price,
            self.timeline.seed,
        )
    }
}

impl SimConfig {
    /// constructor
    pub fn new(
        volatility: f64,
        mean_reversion_speed: f64,
        mean_price: f64,
        seed: u64,
        timestep: f64,
        num_steps: usize,
        initial_price: f64,
        pool_volatility_f: f64,
        pool_strike_price_f: f64,
        pool_time_remaining_years_f: f64,
        pool_is_perpetual: bool,
    ) -> Self {
        SimConfig {
            process: Process {
                volatility,
                mean_reversion_speed,
                mean_price,
            },
            timeline: Timeline {
                seed,
                timestep,
                num_steps,
            },
            economic: Economic {
                initial_price,
                pool_volatility_f,
                pool_strike_price_f,
                pool_time_remaining_years_f,
                pool_is_perpetual,
            },
        }
    }
}

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
            process: Process {
                volatility: 0.01,
                mean_reversion_speed: 10.0,
                mean_price: 1.0,
            },
            timeline: Timeline {
                seed: 0,
                timestep: 0.01,
                num_steps: 100,
            },
            economic: Economic {
                initial_price: 1.0,
                pool_volatility_f: 0.1,
                pool_strike_price_f: 1.0,
                pool_time_remaining_years_f: 1.0,
                pool_is_perpetual: true,
            },
        }
    }
}

/// Defines the arguments for use in generating the underlying price process.
#[derive(Clone, Debug)]
pub struct Process {
    volatility: f64,
    mean_reversion_speed: f64,
    mean_price: f64,
}

/// # Timeline
/// A simulation tracks data at some point in team for a period,
/// this struct defines the universal time scale for the simulation.
///
/// # Fields
/// * `seed` - Generates randomness in the price process. (u64)
/// * `timestep` - Distance between points in time. (f64)
/// * `num_steps` - Number of steps in the simulation. (usize)
#[derive(Clone, Debug)]
pub struct Timeline {
    seed: u64,
    timestep: f64,
    num_steps: usize,
}

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
#[derive(Clone, Debug)]
pub struct Economic {
    initial_price: f64,
    pool_volatility_f: f64,
    pool_strike_price_f: f64,
    pool_time_remaining_years_f: f64,
    pool_is_perpetual: bool,
}
