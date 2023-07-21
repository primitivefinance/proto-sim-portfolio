// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import "solmate/tokens/ERC20.sol";
import "solmate/test/utils/mocks/MockERC20.sol";

import "./ArbiterContract.sol";
import { Actor } from "./Actor.sol";

contract Entrypoint is ArbiterContract {
    uint256 public constant startBalance = 4_809e18;

    address public _actor;
    address public _subject;
    address public token0;
    address public token1;

    /// @dev Called by the Simulation manager in the setup.rs function.
    function start(bytes memory input)
        public
        override
        returns (bytes memory output)
    {
        _actor = address(new Actor());
        token0 = address(new MockERC20("Mock0", "X", 18));
        token1 = address(new MockERC20("Mock1", "Y", 18));

        // token approvals
        Actor(_actor).execute(
            token0,
            abi.encodeWithSignature(
                ERC20.approve.selector, address(this), type(uint256).max
            )
        );

        Actor(_actor).execute(
            token1,
            abi.encodeWithSignature(
                ERC20.approve.selector, address(this), type(uint256).max
            )
        );

        // token minting
        ERC20(token0).mint(_actor, startBalance);
        ERC20(token1).mint(_actor, startBalance);
    }

    function actor() public view returns (address) {
        return _actor;
    }
}
