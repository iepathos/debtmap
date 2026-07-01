// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

interface IERC20 {
    function transfer(address to, uint256 amount) external returns (bool);
}

library SafeTransfer {
    function safeTransfer(IERC20 token, address to, uint256 amount) internal {
        token.transfer(to, amount);
    }
}
