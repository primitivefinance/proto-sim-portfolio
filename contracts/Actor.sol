// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import "./ArbiterContract.sol";

contract Actor is ArbiterContract {
    function start(bytes memory input)
        public
        override
        returns (bytes memory output)
    { }

    function execute(address target, bytes calldata data) external {
        (bool success, bytes memory returnData) = target.call(data);
        require(success, string(returnData));
    }

    function step() public returns (bytes memory output) { }
}
