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

    function getStrategyData(
        uint256 strikePriceWad,
        uint256 volatilityBasisPoints,
        uint256 durationSeconds,
        bool isPerpetual,
        uint256 priceWad
    )
        external
        view
        returns (bytes memory strategyData, uint256 initialX, uint256 initialY);
}

contract Arbitrageur {
    using ExtendedNormalCurveLib for NormalCurve;
    using FixedPointMathLib for *;

    function computeArbInput(
        address portfolio,
        uint64 poolId,
        uint256 priceWad
    ) public view returns (bool swapXIn, uint256 inputWad) {
        PortfolioPool memory pool = IPortfolioStruct(portfolio).pools(poolId);
        require(pool.liquidity > 0, "Pool has zero liquidity");
        require(pool.virtualX > 0, "Pool has zero virtualX");
        require(pool.virtualY > 0, "Pool has zero virtualY");

        IStrategy strategy = IStrategy(IPortfolio(portfolio).DEFAULT_STRATEGY()); // todo: fix with latest portfolio version

        PortfolioConfig memory config =
            NormalStrategyLike(address(strategy)).configs(poolId);

        uint256 spotPrice = IPortfolio(portfolio).getSpotPrice(poolId);
        require(spotPrice > 0, "Spot price is zero");

        NormalCurve memory curve = config.transform();
        require(curve.standardDeviationWad > 0, "Standard deviation is zero");
        require(curve.strikePriceWad > 0, "Strike price is zero");
        require(curve.timeRemainingSeconds > 0, "Time remaining is zero");
        if (config.isPerpetual) curve.timeRemainingSeconds = SECONDS_PER_YEAR;
        curve.reserveXPerWad = pool.virtualX.divWadDown(pool.liquidity);
        curve.reserveYPerWad = pool.virtualY.divWadDown(pool.liquidity);
        uint256 gammaPctWad = ((1e4 - pool.feeBasisPoints) * WAD) / 1e4;

        // If xInput is 0, then we need to compute yInput, since we don't need to change x in a positive direction (sell it).
        uint256 xInput = curve.computeXInToMatchReportedPrice(
            spotPrice, priceWad, gammaPctWad
        );
        if (xInput > 0) {
            inputWad = xInput;
            swapXIn = true;
            return (true, xInput);
        }

        int256 invariant = curve.tradingFunction();

        uint256 yInput = curve.computeYInToMatchReportedPrice(
            spotPrice, priceWad, gammaPctWad
        );
        if (yInput > 0) {
            inputWad = yInput;
        }

        require(yInput > 0 || xInput > 0, "Input is zero");
    }

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
        if (config.isPerpetual) curve.timeRemainingSeconds = SECONDS_PER_YEAR;
        curve.reserveXPerWad = pool.virtualX.divWadDown(pool.liquidity);
        curve.reserveYPerWad = pool.virtualY.divWadDown(pool.liquidity);

        uint256 gammaPctWad = (1e4 - pool.feeBasisPoints) * WAD / 1e4;

        uint256 input;
        bool sellAsset;

        uint256 xInput =
            curve.computeXInputGivenMarginalPrice(priceWad, gammaPctWad);

        // If xInput is 0, then we need to compute yInput, since we don't need to change x in a positive direction (sell it).
        if (xInput > 0) {
            return _getOrder(portfolio, poolId, true, xInput, pool.liquidity);
        }

        uint256 yInput =
            curve.computeYInputGivenMarginalPrice(priceWad, gammaPctWad, 0);
        if (yInput > 0) {
            return _getOrder(portfolio, poolId, false, yInput, pool.liquidity);
        }
    }

    function getConfig(
        address portfolio,
        uint64 poolId
    ) public view returns (PortfolioConfig memory config) {
        IStrategy strategy = IStrategy(IPortfolio(portfolio).DEFAULT_STRATEGY()); // todo: fix with latest portfolio version
        config = NormalStrategyLike(address(strategy)).configs(poolId);
    }

    function _getOrder(
        address portfolio,
        uint64 poolId,
        bool sellAsset,
        uint256 input,
        uint256 liquidity
    ) internal view returns (Order memory order) {
        require(input > 0, "Input is zero");

        // The input amount must be multiplied by the liquidity...
        // this is because the arbitrage math computes the input/output amounts
        // on a per liquidity basis, due to the constraints with the trading function.
        input = input.mulWadDown(liquidity);
        uint256 output = IPortfolio(portfolio).getAmountOut(
            poolId, sellAsset, input, msg.sender
        );
        require(output > 0, "Output is zero");

        order = Order({
            poolId: poolId,
            input: uint128(input), // todo: proper cast
            output: uint128(output),
            sellAsset: sellAsset,
            useMax: false
        });
    }

    /// very temporary
    /// wraps the strategy interface so we can get the initial reserves and encoded strategy args
    function getCreatePoolComputedArgs(
        address portfolio,
        uint256 strikePriceWad,
        uint256 volatilityBasisPoints,
        uint256 durationSeconds,
        bool isPerpetual,
        uint256 priceWad
    )
        public
        view
        returns (bytes memory strategyData, uint256 initialX, uint256 initialY)
    {
        return NormalStrategyLike(
            address(IStrategy(IPortfolio(portfolio).DEFAULT_STRATEGY()))
        ).getStrategyData(
            strikePriceWad,
            volatilityBasisPoints,
            durationSeconds,
            isPerpetual,
            priceWad
        );
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
