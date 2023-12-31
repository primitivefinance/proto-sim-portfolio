/// Command line interface for the sim.
use super::analysis;
use anyhow::anyhow;
use clap::{Parser, Subcommand};
use colored::*;

use super::sim;

/// CLI arguments.
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

/// CLI commands that can be run.
/// `analyze` - Runs an analysis specified with --name and --subtype.
/// `sim` - Runs the simulation.
#[derive(Subcommand)]
enum Commands {
    /// Runs an analysis.
    Analyze {
        #[arg(short, long)]
        /// REQUIRED: The analysis to run.
        name: String,

        /// OPTIONAL: The subtype analysis to run
        #[arg(short, long)]
        subtype: Option<String>,
    },
    Sim {},
}

/// Handles the cli commands argument parsing to run the sim or a specific analysis.
pub async fn main() -> anyhow::Result<(), anyhow::Error> {
    let cli = Cli::parse();

    let start_time = std::time::Instant::now();

    match &cli.command {
        Some(Commands::Analyze { name, subtype }) => {
            println!("\n{}", "Running analysis!".blue());

            match name.as_str() {
                "trading_function" => {
                    let mut subtype_to_run = analysis::TradingFunctionSubtype::default();

                    if let Some(subtype) = subtype {
                        match subtype.as_str() {
                            "error" => {
                                subtype_to_run = analysis::TradingFunctionSubtype::Error;
                            }
                            "curve" => {
                                subtype_to_run = analysis::TradingFunctionSubtype::Curve;
                            }
                            _ => {
                                return Err(anyhow!("Analysis subtype not found: {}", subtype));
                            }
                        }
                    }

                    analysis::trading_function::main(subtype_to_run)?;
                }
                _ => {
                    return Err(anyhow!("Analysis not found: {}", name));
                }
            };

            // Print the time to run.
            let elapsed = start_time.elapsed();
            println!(
                "\n{} {} {}",
                "Trading Function Error Analysis took".bright_cyan(),
                elapsed.as_secs_f64().to_string().purple().bold(),
                "seconds to run.".bright_cyan(),
            );
        }
        Some(Commands::Sim {}) => {
            println!("\n{}", "Starting simulation!".blue());

            // Run the simulation.
            match sim::main().await {
                Ok(_) => {
                    println!("{}", "Simulation complete!".green());
                }
                Err(e) => {
                    return Err(anyhow!("Error running simulation: {}", e));
                }
            }

            let elapsed = start_time.elapsed();
            println!(
                "{} {} {}",
                "Simulation took".bright_black(),
                elapsed.as_secs_f64().to_string().purple().bold(),
                "seconds to run.".bright_black(),
            );
        }
        None => {
            println!("\n{}", "Running simulation!".blue());

            // Run the simulation.
            match sim::main().await {
                Ok(_) => {
                    println!("{}", "Simulation complete!".green());
                }
                Err(e) => {
                    return Err(anyhow!("Error running simulation: {}", e));
                }
            }

            let elapsed = start_time.elapsed();
            println!(
                "{} {} {}",
                "Simulation took".bright_blue(),
                elapsed.as_secs_f64().to_string().purple().bold(),
                "seconds to run.".bright_blue(),
            );
        }
    }

    Ok(())
}
