// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

interface IERC20 {
    function transferFrom(
        address sender,
        address recipient,
        uint256 amount
    ) external returns (bool);

    function transfer(
        address recipient,
        uint256 amount
    ) external returns (bool);

    function decimals() external view returns (uint256);
}

/// @dev Simple infinitely liquid exchange!
contract Exchange {
    string public constant version = "v1.0.0";

    mapping(address token => uint256 price) public _prices;

    function getPrice(address token) public view returns (uint256) {
        return _prices[token]; // price = token / denomination
    }

    /// @dev Sets a price for a token in a denomination, in WAD units.
    function setPrice(address token, uint256 price) public {
        _prices[token] = price;
    }

    function trade(
        address asset,
        address quote,
        bool sellAsset,
        uint256 amountIn
    ) public {
        uint256 price = _prices[asset]; // selling asset ? price = quote / asset : price = asset / quote

        uint256 unit = 10 ** IERC20(sellAsset ? quote : asset).decimals();

        uint256 amountOut =
            sellAsset ? amountIn * price / unit : amountIn * 1e18 / price; // Cancel numerator units out.

        _debit(sellAsset ? asset : quote, amountIn);
        _credit(sellAsset ? quote : asset, amountOut);
    }

    function _debit(address token, uint256 amount) internal {
        IERC20(token).transferFrom(msg.sender, address(this), amount);
    }

    function _credit(address token, uint256 amount) internal {
        IERC20(token).transfer(msg.sender, amount);
    }
}
