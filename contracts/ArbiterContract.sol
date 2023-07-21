// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

using { get } for bytes32;

function get(bytes32 slot) view returns (bytes32 value) {
    assembly ("memory-safe") {
        value := sload(slot)
    }
}

abstract contract ArbiterContract {
    function start(bytes memory input)
        public
        virtual
        returns (bytes memory output);

    function step(bytes memory input)
        public
        virtual
        returns (bytes memory output)
    {
        // optional
    }

    function end(bytes memory input)
        public
        virtual
        returns (bytes memory output)
    {
        // optional
    }

    function extsload(bytes32 slot) external view returns (bytes32 value) {
        return slot.get();
    }
}
