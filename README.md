# proto-sim

Prototype simulation using Arbiter as the simulation & agent engine.

## Build & Run
```bash
build.sh

cargo run
```

## Arbiter config

The `arbiter.toml` config does not do anything right now, but it is there to show what the config file will look like.


## Example

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