// SPDX-License-Identifier: MIT
pragma solidity ^0.8.4;

import "portfolio/strategies/NormalStrategyLib.sol" as NormalStrategyLib;

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
}
