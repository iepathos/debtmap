// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

contract SmallToken {
    mapping(address => uint256) private balances;

    function mint(address account, uint256 amount) external {
        balances[account] += amount;
    }

    function balanceOf(address account) external view returns (uint256) {
        return balances[account];
    }
}
