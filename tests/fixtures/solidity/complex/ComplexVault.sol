// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract ComplexVault {
    function decide(bool a, bool b, bool c) public pure returns (uint256) {
        if (a) {
            if (b) {
                return 1;
            }
        }

        for (uint256 i = 0; i < 10; i++) {
            if (c) {
                return i;
            }
        }

        return 0;
    }
}
