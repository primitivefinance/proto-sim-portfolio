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

    mapping(address denomination => mapping(address token => uint256 price))
        public _prices;

    function getPrice(
        address token,
        address denomination
    ) public view returns (uint256) {
        return _prices[denomination][token]; // price = token / denomination
    }

    /// @dev Sets a price for a token in a denomination, in WAD units.
    function setPrice(
        address token,
        address denomination,
        uint256 price
    ) public {
        _prices[denomination][token] = price;
    }

    function trade(
        address asset,
        address quote,
        bool sellAsset,
        uint256 amount
    ) public {
        uint256 price =
            sellAsset ? _prices[quote][asset] : _prices[asset][quote]; // selling asset ? price = quote / asset : price = asset / quote

        uint256 unit = 10 ** IERC20(sellAsset ? quote : asset).decimals();

        _debit(sellAsset ? asset : quote, amount);
        _credit(sellAsset ? quote : asset, amount * price / unit);
    }

    function _debit(address token, uint256 amount) internal {
        IERC20(token).transferFrom(msg.sender, address(this), amount);
    }

    function _credit(address token, uint256 amount) internal {
        IERC20(token).transfer(msg.sender, amount);
    }
}
