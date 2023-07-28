// SPDX-License-Identifier: GPL-3.0-only
pragma solidity ^0.8.4;

import "forge-std/Test.sol";
import "contracts/ExtendedNormalCurveLib.sol";

contract TestExtendedNormalCurveLib is Test {
    using ExtendedNormalCurveLib for *;

    function test_input_x_given_mp() public {
        uint256 gammaPctWad = (1e4 - 100) * 1e14; // 1% fee

        PortfolioConfig memory config = PortfolioConfig({
            strikePriceWad: 1e18,
            volatilityBasisPoints: 1000,
            durationSeconds: uint32(SECONDS_PER_YEAR),
            creationTimestamp: 0, // creationTimestamp isnt set, its set to block.timestamp
            isPerpetual: true
        });

        uint256 currentPriceWad = 1e18;
        NormalCurve memory curve = config.transform();
        (curve.reserveXPerWad, curve.reserveYPerWad) =
            curve.approximateReservesGivenPrice(currentPriceWad);

        // our desire to increase the x reserve, which decreases the price
        // decrease price by our volatility parameter so we know the liquidity
        // distribtion at the target price is > 0.
        uint256 targetPriceWad =
            config.strikePriceWad * (1e4 - config.volatilityBasisPoints) / 1e4; // x0.75

        int256 invariant = curve.tradingFunction();
        uint256 xInput =
            curve.computeXInputGivenMarginalPrice(targetPriceWad, gammaPctWad);
        uint256 yInput = curve.computeYInputGivenMarginalPrice(
            targetPriceWad, gammaPctWad, invariant
        );

        console.log("invariant");
        console.logInt(invariant);
        console.log("x", curve.reserveXPerWad);
        console.log("y", curve.reserveYPerWad);
        console.log("xInput", xInput);
        console.log("yInput", yInput);

        assertTrue(xInput > 0, "xInput should be non-zero");
        assertEq(yInput, 0, "should not increase y");
    }

    function test_input_y_given_mp() public {
        // need to increase the price!
        uint256 gammaPctWad = (1e4 - 100) * 1e14; // 1% fee
        PortfolioConfig memory config = PortfolioConfig({
            strikePriceWad: 1e18,
            volatilityBasisPoints: 1000,
            durationSeconds: uint32(SECONDS_PER_YEAR),
            creationTimestamp: 0, // creationTimestamp isnt set, its set to block.timestamp
            isPerpetual: true
        });

        uint256 currentPriceWad = 1e18;
        NormalCurve memory curve = config.transform();
        (curve.reserveXPerWad, curve.reserveYPerWad) =
            curve.approximateReservesGivenPrice(currentPriceWad);

        uint256 targetPriceWad =
            config.strikePriceWad * (1e4 + config.volatilityBasisPoints) / 1e4; // x1.25

        int256 invariant = curve.tradingFunction();
        uint256 xInput =
            curve.computeXInputGivenMarginalPrice(targetPriceWad, gammaPctWad);
        uint256 yInput = curve.computeYInputGivenMarginalPrice(
            targetPriceWad, gammaPctWad, invariant
        );

        console.log("invariant");
        console.logInt(invariant);
        console.log("x", curve.reserveXPerWad);
        console.log("y", curve.reserveYPerWad);
        console.log("xInput", xInput);
        console.log("yInput", yInput);

        assertEq(xInput, 0, "should not increase x");
        assertTrue(yInput > 0, "yInput should be non-zero");
    }

    error Portfolio_InvalidInvariant(int256, int256);

    function test_cast() public {
        int256 value;
        unchecked {
            value = -int256(
                uint256(
                    83787297544085766118857358201295311741309103974045547390490541163298689124680
                )
            );
        }

        revert Portfolio_InvalidInvariant(
            int256(value),
            int256(
                type(uint256).max
                    - uint256(
                        83787297544085766118857358201295311741309103974045547390490541163298689124680
                    )
            )
        );
    }
}
