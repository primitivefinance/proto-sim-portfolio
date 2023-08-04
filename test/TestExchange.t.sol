// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "forge-std/Test.sol";
import "../contracts/Exchange.sol";
import "solmate/test/utils/mocks/MockERC20.sol";

contract TestExchange is Test {
    Exchange subject;
    MockERC20 asset;
    MockERC20 quote;

    function setUp() public {
        subject = new Exchange();
        asset = new MockERC20("Asset", "ASSET", 18);
        quote = new MockERC20("Quote", "QUOTE", 18);
    }

    function test_trade() public {
        asset.mint(address(this), 1000e18);
        quote.mint(address(this), 1000e18);

        asset.mint(address(subject), 1000000e18);
        quote.mint(address(subject), 1000000e18);

        asset.approve(address(subject), 1000e18);
        quote.approve(address(subject), 1000e18);

        subject.setPrice(address(asset), 100e18);

        subject.trade(address(asset), address(quote), true, 1000e18);

        assertEq(asset.balanceOf(address(this)), 0);
        assertEq(
            quote.balanceOf(address(this)), 1000e18 + 1000e18 * 100e18 / 1e18
        );
    }

    function test_trade_y_to_x() public {
        asset.mint(address(this), 1000e18);
        quote.mint(address(this), 1000e18);

        asset.mint(address(subject), 1000000e18);
        quote.mint(address(subject), 1000000e18);

        asset.approve(address(subject), 1000e18);
        quote.approve(address(subject), 1000e18);

        subject.setPrice(address(asset), 100e18);

        subject.trade(address(asset), address(quote), false, 1000e18);

        assertEq(
            asset.balanceOf(address(this)), 1000e18 + 1000e18 * 1e18 / 100e18
        );
        assertEq(quote.balanceOf(address(this)), 0);
    }
}
