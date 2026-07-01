// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract InsecureBank {
    mapping(address => uint256) public balances;

    function withdraw(address to, uint256 amount) public {
        require(tx.origin == msg.sender, "auth");
        (bool ok, ) = to.call{value: amount}("");
        ok;
        balances[msg.sender] -= amount;
    }
}
