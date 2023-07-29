/// SPDX-LICENSE-IDENTIFIER: MIT
pragma solidity ^0.8.4;

import "portfolio/strategies/NormalStrategyLib.sol";
import { console2 as logger } from "forge-std/console2.sol";

function toInt(uint256 x) pure returns (int256) {
    if (x > uint256(type(int256).max)) revert("toInt: overflow");
    return int256(x);
}

function toUint(int256 x) pure returns (uint256) {
    if (x < 0) return 0;

    return uint256(x);
}

library ExtendedNormalCurveLib {
    using Gaussian for *;
    using FixedPointMathLib for *;
    using { toInt } for uint256;
    using { toUint } for int256;

    error ExtendedNormalCurveLib_InvalidGammaPct(uint256);

    /// @dev Δ1 = 𝛾−1(1 − 𝑅1 − Φ(Φ−1 (1 − 𝑅1) + ln(1 + 𝜖)/𝜎√𝜏)).
    function computeXInToMatchReportedPrice(
        NormalCurve memory self,
        uint currentPriceWad,
        uint256 desiredPriceWad,
        uint256 gammaPctWad
    ) internal pure returns (uint256 deltaX) {
        uint epsilonScalar = desiredPriceWad.mulWadDown(gammaPctWad).divWadDown(currentPriceWad);
        // 1 - R1
        int256 oneMinusR1 = WAD.toInt() - self.reserveXPerWad.toInt();

        // Φ−1 (1 − 𝑅1)
        int256 cdfInvOneMinusR1 = oneMinusR1.ppf();

        // ln(1 + 𝜖)
        int256 logOnePlusEpsilon =
            epsilonScalar.toInt().lnWad();

        // ln(1 + 𝜖)/𝜎√𝜏
        int256 logOnePlusEpsilonStdDevSqrtTau = (
            logOnePlusEpsilon * WAD.toInt() / self.computeStdDevSqrtTau().toInt()
        );

        // Φ−1 (1 − 𝑅1) + ln(1 + 𝜖)/𝜎√𝜏
        int256 cdfInvOneMinusR1PlusLogOnePlusEpsilonStdDevSqrtTau =
            (cdfInvOneMinusR1 + logOnePlusEpsilonStdDevSqrtTau);

    
        int256 result = oneMinusR1
            - cdfInvOneMinusR1PlusLogOnePlusEpsilonStdDevSqrtTau.cdf();
        return uint256(result).mulWadDown(gammaPctWad);
    }

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
            gammaPctWad.mulWadDown(self.strikePriceWad)
        ).toInt().lnWad();

        // ln(m/γK) σ√τ + 1/2σ√τ
        int256 cdfInput = (
            logResult * stdDevSqrtTau.toInt() / WAD.toInt()
                + stdDevSqrtTau.toInt() / 2
        );

        // 1 - Rα − Φ( ln(m/γK) σ√τ + 1/2σ√τ)
        logger.logInt(logResult);
        logger.logInt(cdfInput);
        logger.logInt(cdfInput.cdf());
        int256 result =
            WAD.toInt() - self.reserveXPerWad.toInt() - cdfInput.cdf();

        logger.logInt(result);

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
        int256 cdfInput = (
            logResult * stdDevSqrtTau.toInt() / WAD.toInt()
                - stdDevSqrtTau.toInt() / 2
        );

        // KΦ( ln(m/K) σ√τ − 1/2 σ√τ)
        int256 result = (
            self.strikePriceWad.toInt() * cdfInput.cdf() / WAD.toInt()
                + invariant - self.reserveYPerWad.toInt()
        );

        return result.toUint().divWadDown(gammaPctWad);
    }
}
