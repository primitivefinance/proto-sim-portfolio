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

    event PriceChange(uint256 price);

    mapping(address token => uint256 price) public _prices;

    function getPrice(address token) public view returns (uint256) {
        return _prices[token]; // price = token / denomination
    }

    /// @dev Sets a price for a token in a denomination, in WAD units.
    function setPrice(address token, uint256 price) public {
        _prices[token] = price;
        emit PriceChange(price);
    }

    function trade(
        address asset,
        address quote,
        bool sellAsset,
        uint256 amountIn
    ) public returns (bool) {
        uint256 price = _prices[asset]; // selling asset ? price = quote / asset : price = asset / quote

        uint256 decimals = IERC20(sellAsset ? quote : asset).decimals();
        uint256 scalar = 10 ** (18 - decimals);

        uint256 amountOut;
        if (sellAsset) {
            amountOut = amountIn * price / 1e18 / scalar;
        } else {
            amountOut = amountIn * scalar * 1e18 / price;
        }

        _debit(sellAsset ? asset : quote, amountIn);
        _credit(sellAsset ? quote : asset, amountOut);

        return true;
    }

    function _debit(address token, uint256 amount) internal {
        IERC20(token).transferFrom(msg.sender, address(this), amount);
    }

    function _credit(address token, uint256 amount) internal {
        IERC20(token).transfer(msg.sender, amount);
    }
}
