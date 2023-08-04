# proto-sim

Prototype simulation using Arbiter as the simulation & agent engine.

## Build & Run
```bash
build.sh

cargo run
```

## Arbiter config

The `arbiter.toml` config defines the initial parameterization of the underlying price process and pool parameters.


## Project Structure
- analysis - Contains analysis scripts for the simulation results.
    - trading_function - Contains scripts for analyzing the trading function.
- bisection - Implements the bisection algorithm in rust.
- calls - Agent wrapper abstraction for gracefully handling EVM transactions and calls.
- cli - Handles the cli parser and matching commands.
- common - Static variables used across the sim as default values.
- config - Loads the arbiter.toml config into a deserialized struct type.
- log - Fetches EVM state and loads it into a DataFrame type that can be written to a csv.
- main - Main entry point for the cli.
- math - Implements the Portfolio Strategy math in rust.
- plots - Implements utility functions for plotting simulation csv or other data.
- raw_data - Handles the storage of the raw EVM state that is processed by log.
- setup - Handles the simulation environment setup, including contract and agents deployment.
- sim - Implements the simulation loop and agent interaction.
- spreadsheetorizer - Converts the DataFrame raw data type to a csv which can be written to a file.
- step - Handles a "simulation step" in the simulation loop in sim.rs.
- task - Handles a specific agent task in the simulation loop in sim.rs.



### Basic Sim Example

```rust
use arbiter::prelude::*;
use bindings::weth;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Simulation manager is the controller of the arbiter simulation environment.
    let mut manager = SimulationManager::new();

    // Get a deployer account from the manager.
    let admin = manager.agents.get("admin").unwrap();

    // Deploy a contract to the EVM.
    let contract = SimulationContract::new(weth::WETH_ABI.clone(), weth::WETH_BYTECODE.clone());
    let (weth, tx_deploy) = admin.deploy(weth, vec![])?;

    // Add it to the manager's deployed contracts list.
    manager.deployed_contracts.insert("weth".to_string(), weth);

    // Execute EVM transactions, log the data, and repeat.
    admin.call(weth, "deposit", vec![1000]).await?;
}
```