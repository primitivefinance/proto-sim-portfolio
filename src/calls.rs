/// Wrapper methods for calling common functions on the contracts.
use arbiter::{
    agent::Agent,
    environment::contract::{IsDeployed, SimulationContract},
    utils::{recast_address, unpack_execution},
};

use bindings::i_portfolio_actions::{AllocateCall, Order, SwapCall};
use ethers::{
    abi::{Tokenizable, Tokenize},
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
/// Provides additional context when calls fail.
#[derive(Debug, Clone)]
#[allow(unused)]
pub struct Call {
    from: Address,
    function_name: String,
    target: Address,
    args: Vec<ethers::abi::Token>,
    result: Option<ExecutionResult>,
}

/// Uses zero addresses and empty strings as defaults.
impl Default for Call {
    fn default() -> Self {
        Call {
            from: Address::zero(),
            function_name: "".to_string(),
            target: Address::zero(),
            args: vec![],
            result: None,
        }
    }
}

/// Gives the agent access to simple and common calls to the smart contracts.
#[allow(unused)]
impl<'a> Caller<'a> {
    /// Creates a new caller!
    pub fn new(caller: &'a dyn Agent) -> Self {
        Caller {
            caller,
            last_call: Call::default(),
        }
    }

    /// Updates the last_call field, based on the last call made
    fn set_last_call(&mut self, last_call: Call) {
        self.last_call = last_call;
    }

    /// Updates the last_call field, based on the last call made
    fn set_last_call_result(&mut self, result: ExecutionResult) {
        self.last_call.result = Some(result);
    }

    /// Call `res()` to get the result and error.
    /// Call `decoded()` to get the decoded result.
    /// These are terminal methods for the caller.
    pub fn res(&mut self) -> Result<ExecutionResult, Error> {
        self.last_call.result.clone().ok_or(anyhow!(
            "calls.rs: {:?} call result is None",
            self.last_call
        ))
    }

    /// Wraps the raw REVM call to gracefully handle errors and log more context using anyhow errors.
    pub fn call(
        &mut self,
        contract: &SimulationContract<IsDeployed>,
        function_name: &str,
        args: Vec<ethers::abi::Token>,
    ) -> Result<&mut Self, Error> {
        self.set_last_call(Call {
            from: recast_address(self.caller.address()),
            function_name: function_name.to_string(),
            target: recast_address(contract.address),
            args: args.clone(),
            result: None,
        });

        let result = self.caller.call(contract, function_name, args.clone());

        // Wraps the dynamic error into the anyhow error with some context for the last call.
        // Return type of this function must be a result so we can propagate the error with `?`.
        let _ = self.handle_error_gracefully(result)?;
        Ok(self)
    }

    pub fn balance_of(&mut self, token: &SimulationContract<IsDeployed>) -> &mut Self {
        let owner = recast_address(self.caller.address().clone()).clone();
        self.set_last_call(Call {
            from: owner,
            function_name: "balanceOf".to_string(),
            target: recast_address(token.address),
            args: (owner).into_tokens(),
            result: None,
        });

        let result = self.caller.call(token, "balanceOf", (owner).into_tokens());

        // Wraps the dynamic error into the anyhow error with some context for the last call.
        let _ = self.handle_error_gracefully(result);
        self
    }

    pub fn approve(
        &mut self,
        token: &SimulationContract<IsDeployed>,
        spender: Address,
        amount_f: f64,
    ) -> &mut Self {
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
            result: None,
        });

        let result = self
            .caller
            .call(token, "approve", (spender, amount.clone()).into_tokens());

        // Wraps the dynamic error into the anyhow error with some context for the last call.
        let _ = self.handle_error_gracefully(result);
        self
    }

    pub fn transfer_from(
        &mut self,
        token: &SimulationContract<IsDeployed>,
        to: Address,
        amount_f: f64,
    ) -> &mut Self {
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
            result: None,
        });

        let result = self
            .caller
            .call(token, "transferFrom", (to, amount.clone()).into_tokens());

        // Wraps the dynamic error into the anyhow error with some context for the last call.
        let _ = self.handle_error_gracefully(result);
        self
    }

    /// For allocating on portfolio
    pub fn allocate(
        &mut self,
        portfolio: &SimulationContract<IsDeployed>,
        pool_id: u64,
        amount_f: f64,
    ) -> &mut Self {
        let amount = if amount_f == 0.0 {
            U256::MAX
        } else {
            ethers::utils::parse_ether(amount_f).unwrap()
        };

        let from = recast_address(self.caller.address());

        let args: AllocateCall = AllocateCall {
            use_max: false,
            recipient: from.clone(),
            pool_id: pool_id.into(),
            delta_liquidity: amount.as_u128(),
            max_delta_asset: u128::MAX,
            max_delta_quote: u128::MAX,
        };

        self.set_last_call(Call {
            from: from.clone(),
            function_name: "allocate".to_string(),
            target: recast_address(portfolio.address),
            args: args.clone().into_tokens(),
            result: None,
        });

        let result = self
            .caller
            .call(portfolio, "allocate", args.clone().into_tokens());

        // Wraps the dynamic error into the anyhow error with some context for the last call.
        let _ = self.handle_error_gracefully(result);
        self
    }

    /// For swapping on portfolio
    pub fn swap(
        &mut self,
        portfolio: &SimulationContract<IsDeployed>,
        swap_order: Order,
    ) -> Result<&mut Self, Error> {
        let args: SwapCall = SwapCall {
            args: swap_order.clone(),
        };

        self.set_last_call(Call {
            from: recast_address(self.caller.address()),
            function_name: "swap".to_string(),
            target: recast_address(portfolio.address),
            args: args.clone().into_tokens(),
            result: None,
        });

        let result = self
            .caller
            .call(portfolio, "swap", args.clone().into_tokens());

        // Wraps the dynamic error into the anyhow error with some context for the last call.
        let _ = self.handle_error_gracefully(result)?;
        Ok(self)
    }

    /// Wraps the arbiter call with anyhow's error context, using the last call details.
    fn handle_error_gracefully(
        &mut self,
        tx_result: Result<ExecutionResult, Box<dyn std::error::Error>>,
    ) -> Result<ExecutionResult, Error> {
        match tx_result {
            Ok(res) => {
                if res.is_success() {
                    /*let return_bytes = unpack_execution(res.clone()).unwrap();

                    // todo: do we need this check?
                     if return_bytes.len() == 0 {
                        return Err(anyhow!(
                            "calls.rs: {:?} call returned empty bytes: {:?}",
                            self.last_call,
                            res
                        ));
                    } */

                    // Sets the result of the last call.
                    self.set_last_call_result(res.clone());

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
                    "calls.rs: failed to call {:?}: msg: {:?}",
                    self.last_call,
                    msg
                ));
            }
        }
    }
}

/// Decodes the last call's result into a tokenizable type.
pub trait DecodedReturns {
    fn decoded<T: Tokenizable>(
        &self,
        contract: &SimulationContract<IsDeployed>,
    ) -> Result<T, Error>;
}

/// Enables the `decoded` method for the Caller struct.
impl DecodedReturns for Caller<'_> {
    fn decoded<T: Tokenizable>(
        &self,
        contract: &SimulationContract<IsDeployed>,
    ) -> Result<T, Error> {
        let result = self.last_call.result.clone();
        let result = match result {
            Some(result) => result,
            None => {
                return Err(anyhow!(
                "calls.rs: {:?} call result is None when attempting to decode, was there a result?",
                self.last_call
            ))
            }
        };
        let return_bytes = unpack_execution(result.clone())?;

        if return_bytes.len() == 0 {
            return Err(anyhow!(
                "calls.rs: {:?} call returned empty bytes: {:?}",
                self.last_call,
                result
            ));
        }

        let decoded: Result<T, ethers::prelude::AbiError> =
            contract.decode_output(&self.last_call.function_name, return_bytes);

        match decoded {
            Ok(decoded) => Ok(decoded as T),
            Err(e) => Err(anyhow!(
                "calls.rs: failed to decode output: {:?}",
                e.to_string()
            )),
        }
    }
}

#[cfg(test)]
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
        let approve_tx = caller.approve(&bad_contract, Address::zero(), 0.0).res();

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
        let approve_tx = caller.approve(&contract, Address::zero(), 0.0).res();

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
        let tx = caller.transfer_from(&contract, Address::zero(), 0.0).res();

        match tx {
            Ok(res) => assert!(false),
            Err(e) => assert!(true),
        }
    }
}
