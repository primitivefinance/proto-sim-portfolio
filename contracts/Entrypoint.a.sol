// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import "solmate/tokens/ERC20.sol";
import "solmate/test/utils/mocks/MockERC20.sol";

import "./ArbiterContract.sol";
import { Actor } from "./Actor.sol";
import { Exchange } from "./Exchange.sol";

import "portfolio/interfaces/IPortfolio.sol";

contract Entrypoint is ArbiterContract {
    string public constant PORTFOLIO_VERSION = "v1.4.0-beta";
    uint256 public constant startBalance = 4_809e18;

    address public _actor;
    address public _subject;

    address public exchange;
    address public weth;
    address public token0;
    address public token1;

    /**
     * @notice
     * Called by SimulationManager to initialize the simulation in setup.rs.
     *
     * @param input abi.encode(weth, portfolio)
     */
    function start(bytes memory input)
        public
        override
        returns (bytes memory output)
    {
        (address weth_, address portfolio) =
            abi.decode(input, (address, address));

        // actor
        _actor = address(new Actor());

        // exchange
        exchange = address(new Exchange());

        // tokens
        token0 = address(new MockERC20("Mock0", "X", 18));
        token1 = address(new MockERC20("Mock1", "Y", 18));

        // token minting
        MockERC20(token0).mint(_actor, startBalance);
        MockERC20(token1).mint(_actor, startBalance);
        MockERC20(token0).mint(msg.sender, startBalance);
        MockERC20(token1).mint(msg.sender, startBalance);

        // weth
        weth = weth_;

        // subject
        _subject = portfolio;

        // token approvals
        address[] memory spenders = new address[](2);
        spenders[0] = exchange; // can transferFrom actor
        spenders[1] = _subject; // can transferFrom actor

        _approve(token0, spenders);
        _approve(token1, spenders);

        // Initialize state for portfolio.
        _init(input);
    }

    function _approve(address token, address spender) internal {
        Actor(actor()).execute(
            token,
            abi.encodeWithSelector(
                ERC20.approve.selector, spender, type(uint256).max
            )
        );
    }

    function _approve(address token, address[] memory spenders) internal {
        for (uint256 i = 0; i < spenders.length; i++) {
            _approve(token, spenders[i]);
        }
    }

    function _init(bytes memory input) internal {
        input; // dont use

        // Create the pair.
        IPortfolio(payable(subject())).createPair(token0, token1);

        // Verify the version!
        require(
            keccak256(
                abi.encodePacked(IPortfolio(payable(subject())).VERSION())
            ) == keccak256(abi.encodePacked(PORTFOLIO_VERSION)),
            "version mismatch"
        );
    }

    function actor() public view returns (address) {
        return _actor;
    }

    function subject() public view returns (address) {
        return _subject;
    }
}
