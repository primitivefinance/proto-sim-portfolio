// SPDX-License-Identifier: MIT
pragma solidity ^0.8.4;

import "portfolio/strategies/NormalStrategyLib.sol" as NormalStrategyLib;
import "portfolio/strategies/INormalStrategy.sol";
import "portfolio/interfaces/IPortfolio.sol";

interface GetConfigs {
    function configs(uint64)
        external
        view
        returns (NormalStrategyLib.PortfolioConfig memory);
}

/// @dev Exposes the normal strategy lib functions via a public interface.
contract ExternalNormalStrategyLib {
    function approximateYGivenX(NormalStrategyLib.NormalCurve memory self)
        external
        pure
        returns (uint256)
    {
        return NormalStrategyLib.approximateYGivenX(self);
    }

    function tradingFunction(NormalStrategyLib.NormalCurve memory self)
        external
        pure
        returns (int256)
    {
        return NormalStrategyLib.tradingFunction(self);
    }

    function getCurveConfiguration(
        address portfolio,
        uint64 poolId
    ) external view returns (NormalStrategyLib.NormalCurve memory) {
        NormalStrategyLib.PortfolioConfig memory config =
            GetConfigs(portfolio).configs(poolId);
        PortfolioPool memory pool = IPortfolioStruct(portfolio).pools(poolId);
        NormalStrategyLib.NormalCurve memory curve = config.transform();
        curve.reserveXPerWad = pool.virtualX * 1e18 / (pool.liquidity);
        curve.reserveYPerWad = pool.virtualY * 1e18 / (pool.liquidity);
        curve.invariant = curve.tradingFunction();
        return curve;
    }
}
