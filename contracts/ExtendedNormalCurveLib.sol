/// SPDX-LICENSE-IDENTIFIER: MIT
pragma solidity ^0.8.4;

import "portfolio/strategies/NormalStrategyLib.sol";

uint256 constant WAD = 1e18;

function toInt(uint256 x) pure returns (int256) {
    if (x > type(int256).max) revert("toInt: overflow");
    return int256(x);
}

function toUint(int256 x) pure returns (uint256) {
    return uint256(x);
}

library ExtendedNormalCurveLib {
    using Gaussian for uint256;
    using FixedPointMathLib for uint256;
    using { toInt } for uint256;
    using { toUint } for int256;

    error ExtendedNormalCurveLib_InvalidGammaPct(uint256);

    /// @dev ∆α = (1 − Rα − Φ( ln(m/γK) σ√τ + 1/2σ√τ)) / γ
    function computeXInputGivenMarginalPrice(
        NormalCurve memory self,
        uint256 marginalPriceWad,
        uint256 gammaPctWad
    ) internal pure returns (uint256 deltaX) {
        if (gammaPctWad == 0) {
            revert ExtendedNormalCurveLib_InvalidGammaPct(gammaPctWad);
        }
        // σ√τ
        uint256 stdDevSqrtTau = self.computeStdDevSqrtTau();

        // ln(m/γK)
        int256 logResult = marginalPriceWad.divWadDown(
            gammaPctWad.mul(self.strikePriceWad)
        ).toInt().lnWad();

        // ln(m/γK) σ√τ + 1/2σ√τ
        int256 cdfInput =
            (logResult.mulWadDown(stdDevSqrtTau) + stdDevSqrtTau / 2).toInt();

        // Φ( ln(m/γK) σ√τ + 1/2σ√τ)
        int256 result =
            WAD.toInt() - self.reserveXPerWad.toInt() - cdfInput.cdf();

        return result.toUint().divWadDown(gammaPctWad);
    }

    /// @dev ∆β = (KΦ( ln(m/K) σ√τ − 1/2 σ√τ) + k − Rβ) / γ
    function computeYInputGivenMarginalPrice(
        NormalCurve memory self,
        uint256 marginalPriceWad,
        uint256 gammaPctWad,
        int256 invariant // k
    ) internal pure returns (uint256 deltaY) {
        if (gammaPctWad == 0) {
            revert ExtendedNormalCurveLib_InvalidGammaPct(gammaPctWad);
        }

        // σ√τ
        uint256 stdDevSqrtTau = self.computeStdDevSqrtTau();

        // ln(m/K)
        int256 logResult =
            marginalPriceWad.divWadDown(self.strikePriceWad).toInt().lnWad();

        // ln(m/K) σ√τ − 1/2 σ√τ
        int256 cdfInput =
            (logResult.mulWadDown(stdDevSqrtTau) - stdDevSqrtTau / 2).toInt();

        // KΦ( ln(m/K) σ√τ − 1/2 σ√τ)
        int256 result = (
            self.strikePriceWad.mulWadDown(cdfInput.cdf()) + invariant
                - self.reserveYPerWad.toInt()
        ).toInt();

        return result.toUint().divWadDown(gammaPctWad);
    }
}
