// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import "./ArbiterContract.sol";
import { Actor } from "./Actor.sol";

contract Entrypoint is ArbiterContract {
    address public actor;
    address public token0;
    address public token1;

    /// @dev Called by the Simulation manager in the setup.rs function.
    function start(bytes memory input)
        public
        override
        returns (bytes memory output)
    {
        actor = address(new Actor());
    }
}
