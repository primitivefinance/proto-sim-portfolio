/// Wrapper methods for calling common functions on the contracts.
use arbiter::{
    agent::Agent,
    environment::contract::{IsDeployed, SimulationContract},
    utils::{recast_address, unpack_execution},
};

use ethers::{
    abi::Tokenize,
    types::{Address, U256},
};

use anyhow::{anyhow, Error, Result};
use revm::primitives::ExecutionResult;

/// Wraps an agent that can calls the contracts.
pub struct Caller<'a> {
    pub caller: &'a dyn Agent,
    pub last_call: Call,
}

/// Represents a call to a contract.
#[derive(Debug, Clone)]
pub struct Call {
    from: Address,
    function_name: String,
    target: Address,
    args: Vec<ethers::abi::Token>,
}

impl Default for Call {
    fn default() -> Self {
        Call {
            from: Address::zero(),
            function_name: "".to_string(),
            target: Address::zero(),
            args: vec![],
        }
    }
}

/// Gives the agent access to simple and common calls to the smart contracts.
impl<'a> Caller<'a> {
    pub fn new(caller: &'a dyn Agent) -> Self {
        Caller {
            caller,
            last_call: Call::default(),
        }
    }

    pub fn set_last_call(&mut self, last_call: Call) {
        self.last_call = last_call;
    }

    pub fn approve(
        &mut self,
        token: &SimulationContract<IsDeployed>,
        spender: Address,
        amount_f: f64,
    ) -> anyhow::Result<ExecutionResult, Error> {
        let amount = if amount_f == 0.0 {
            U256::MAX
        } else {
            ethers::utils::parse_ether(amount_f).unwrap()
        };

        self.set_last_call(Call {
            from: recast_address(self.caller.address()),
            function_name: "approve".to_string(),
            target: recast_address(token.address),
            args: (spender, amount.clone()).into_tokens(),
        });

        let result = self
            .caller
            .call(token, "approve", (spender, amount.clone()).into_tokens());

        // Wraps the dynamic error into the anyhow error with some context for the last call.
        self.handle_error_gracefully(result)
    }

    pub fn transfer_from(
        &mut self,
        token: &SimulationContract<IsDeployed>,
        to: Address,
        amount_f: f64,
    ) -> anyhow::Result<ExecutionResult, Error> {
        let amount = if amount_f == 0.0 {
            U256::MAX
        } else {
            ethers::utils::parse_ether(amount_f).unwrap()
        };

        self.set_last_call(Call {
            from: recast_address(self.caller.address()),
            function_name: "transferFrom".to_string(),
            target: recast_address(token.address),
            args: (to, amount.clone()).into_tokens(),
        });

        let result = self
            .caller
            .call(token, "transferFrom", (to, amount.clone()).into_tokens());

        // Wraps the dynamic error into the anyhow error with some context for the last call.
        self.handle_error_gracefully(result)
    }

    /// Wraps the arbiter call with anyhow's error context, using the last call details.
    fn handle_error_gracefully(
        &self,
        tx_result: Result<ExecutionResult, Box<dyn std::error::Error>>,
    ) -> Result<ExecutionResult, Error> {
        match tx_result {
            Ok(res) => {
                if res.is_success() {
                    let return_bytes = unpack_execution(res.clone()).unwrap();

                    if return_bytes.len() == 0 {
                        return Err(anyhow!(
                            "calls.rs: {:?} call returned empty bytes: {:?}",
                            self.last_call,
                            res
                        ));
                    }

                    return Ok(res);
                } else {
                    return Err(anyhow!(
                        "calls.rs: {:?} call failed: {:?}",
                        self.last_call,
                        res
                    ));
                }
            }
            Err(e) => {
                let msg = e.to_string();
                return Err(anyhow!(
                    "calls.rs: failed to call {:?}: {:?}",
                    self.last_call,
                    msg
                ));
            }
        }
    }
}

mod tests {

    use super::*;
    use arbiter::*;
    use bindings::weth;

    #[test]
    fn approve_bad_contract_fails() {
        let mut manager = manager::SimulationManager::new();

        let admin = manager.agents.get("admin").unwrap();

        let bad_contract =
            SimulationContract::<IsDeployed>::bind(weth::WETH_ABI.clone(), admin.address());

        let mut caller = Caller::new(admin);
        let approve_tx = caller.approve(&bad_contract, Address::zero(), 0.0);

        match approve_tx {
            Ok(res) => {
                println!("Successful call {:?} {:?}", caller.last_call.clone(), res);
                assert!(false)
            }
            Err(e) => assert!(true),
        }
    }

    #[test]
    fn approve_good_contract_succeeds() {
        let mut manager = manager::SimulationManager::new();

        let admin = manager.agents.get("admin").unwrap();

        let contract = SimulationContract::new(weth::WETH_ABI.clone(), weth::WETH_BYTECODE.clone());
        let (contract, _) = admin.deploy(contract, vec![]).unwrap();

        let mut caller = Caller::new(admin);
        let approve_tx = caller.approve(&contract, Address::zero(), 0.0);

        match approve_tx {
            Ok(res) => assert!(true),
            Err(e) => {
                println!("Failed call {:?} {:?}", caller.last_call.clone(), e);
                assert!(false);
            }
        }
    }

    #[test]
    fn transfer_from_fail() {
        let mut manager = manager::SimulationManager::new();

        let admin = manager.agents.get("admin").unwrap();

        let contract = SimulationContract::new(weth::WETH_ABI.clone(), weth::WETH_BYTECODE.clone());
        let (contract, _) = admin.deploy(contract, vec![]).unwrap();

        let mut caller = Caller::new(admin);
        let tx = caller.transfer_from(&contract, Address::zero(), 0.0);

        match tx {
            Ok(res) => assert!(false),
            Err(e) => assert!(true),
        }
    }
}
