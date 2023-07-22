// SPDX-License-Identifier: GPL-3.0-only
pragma solidity ^0.8.4;

import "forge-std/Test.sol";
import "contracts/ExtendedNormalCurveLib.sol";

contract TestExtendedNormalCurveLib is Test {
    using ExtendedNormalCurveLib for *;

    function test_input_x_given_mp() public {
        uint256 priceWad = 1e18;
        uint256 gammaPctWad = 1e4 - 100; // 1% fee

        PortfolioConfig memory config = PortfolioConfig({
            strikePriceWad: 1e18,
            volatilityBasisPoints: 1000,
            durationSeconds: uint32(SECONDS_PER_YEAR),
            creationTimestamp: 0, // creationTimestamp isnt set, its set to block.timestamp
            isPerpetual: true
        });

        NormalCurve memory curve = config.transform();
        (curve.reserveXPerWad, curve.reserveYPerWad) =
            curve.approximateReservesGivenPrice(priceWad);

        console.log(curve.reserveXPerWad, curve.reserveYPerWad);

        uint256 xInput =
            curve.computeXInputGivenMarginalPrice(priceWad, gammaPctWad);

        uint256 yInput =
            curve.computeYInputGivenMarginalPrice(priceWad, gammaPctWad, 0);

        console.log("xInput", xInput);
        console.log("yInput", yInput);
    }

    function test_input_y_given_mp() public { }
}
