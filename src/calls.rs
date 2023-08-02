/// Wrapper methods for calling common functions on the contracts.
use arbiter::{
    agent::{Agent, AgentType},
    environment::contract::{IsDeployed, SimulationContract},
    utils::recast_address,
};

use ethers::{
    abi::Tokenize,
    types::{Address, U256},
};

/// Wraps an agent that can calls the contracts.
pub struct Caller<'a> {
    pub caller: &'a dyn Agent,
}

/// Gives the agent access to simple and common calls to the smart contracts.
impl<'a> Caller<'a> {
    pub fn new(caller: &'a dyn Agent) -> Self {
        Caller { caller }
    }

    pub fn approve(
        &self,
        token: &SimulationContract<IsDeployed>,
        spender: Address,
        amount_f: f64,
    ) -> Result<(), String> {
        let amount = if amount_f == 0.0 {
            ethers::prelude::U256::MAX
        } else {
            ethers::utils::parse_ether(amount_f).unwrap()
        };

        let result = &self
            .caller
            .call(token, "approve", (spender, amount.clone()).into_tokens())
            .unwrap();

        if !result.is_success() {
            // todo: make this a special error type that is returned with the Result instead,
            // so its handled in the unwrap that has to be done on the call anyway, instead
            // of panicking here.
            // panic with a msg for the caller, token address and spender, along with result
            panic!(
                "Failed to approve token {} for spender {} with amount {}. Result: {:?}",
                token.address, spender, amount, result
            );
        }

        Ok(())
    }
}
