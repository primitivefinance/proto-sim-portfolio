// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import "./ArbiterContract.sol";
import "./ExtendedNormalCurveLib.sol";
import "portfolio/interfaces/IPortfolio.sol";
import "portfolio/interfaces/IStrategy.sol";
import "portfolio/strategies/NormalStrategyLib.sol";

interface NormalStrategyLike {
    function configs(uint64 poolId)
        external
        view
        returns (PortfolioConfig memory);
}

contract Arbitrageur {
    using ExtendedNormalCurveLib for NormalCurve;
    using FixedPointMathLib for *;

    function computeArbSwapOrder(
        address portfolio,
        uint64 poolId,
        uint256 priceWad
    ) public view returns (Order memory order) {
        PortfolioPool memory pool = IPortfolioStruct(portfolio).pools(poolId);
        require(pool.liquidity > 0, "Pool has zero liquidity");
        require(pool.virtualX > 0, "Pool has zero virtualX");
        require(pool.virtualY > 0, "Pool has zero virtualY");

        IStrategy strategy = IStrategy(IPortfolio(portfolio).DEFAULT_STRATEGY()); // todo: fix with latest portfolio version

        PortfolioConfig memory config =
            NormalStrategyLike(address(strategy)).configs(poolId);

        NormalCurve memory curve = config.transform();
        curve.reserveXPerWad = pool.virtualX.divWadDown(pool.liquidity);
        curve.reserveYPerWad = pool.virtualY.divWadDown(pool.liquidity);

        uint256 gammaPctWad = (1e4 - pool.feeBasisPoints) * WAD / 1e4;

        uint256 input;
        bool sellAsset;

        uint256 xInput =
            curve.computeXInputGivenMarginalPrice(priceWad, gammaPctWad);

        // If xInput is 0, then we need to compute yInput, since we don't need to change x in a positive direction (sell it).
        if (xInput == 0) sellAsset = false;
        else input = xInput;

        uint256 yInput =
            curve.computeYInputGivenMarginalPrice(priceWad, gammaPctWad, 0);

        if (yInput == 0) sellAsset = true;
        else input = yInput;

        return _getOrder(portfolio, poolId, sellAsset, input);
    }

    function _getOrder(
        address portfolio,
        uint64 poolId,
        bool sellAsset,
        uint256 input
    ) internal view returns (Order memory order) {
        uint256 output = IPortfolio(portfolio).getAmountOut(
            poolId, sellAsset, input, msg.sender
        );

        order = Order({
            poolId: poolId,
            input: uint128(input), // todo: proper cast
            output: uint128(output),
            sellAsset: sellAsset,
            useMax: false
        });
    }
}

contract Actor is Arbitrageur, ArbiterContract {
    function start(bytes memory input)
        public
        override
        returns (bytes memory output)
    { }

    function execute(address target, bytes calldata data) external {
        (bool success, bytes memory returnData) = target.call(data);
        require(success, string(returnData));
    }
}
