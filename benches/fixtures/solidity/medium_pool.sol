// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

interface IPoolToken {
    function transfer(address recipient, uint256 amount) external returns (bool);
    function transferFrom(address sender, address recipient, uint256 amount)
        external
        returns (bool);
}

library PoolMath {
    function min(uint256 left, uint256 right) internal pure returns (uint256) {
        return left < right ? left : right;
    }
}

contract MediumPool {
    IPoolToken public immutable token;
    address public owner;
    uint256 public totalShares;
    mapping(address => uint256) public shares;
    mapping(address => uint256) public lastDeposit;

    event Deposited(address indexed account, uint256 amount, uint256 sharesMinted);
    event Withdrawn(address indexed account, uint256 amount, uint256 sharesBurned);

    modifier onlyOwner() {
        require(msg.sender == owner, "owner");
        _;
    }

    constructor(IPoolToken poolToken) {
        token = poolToken;
        owner = msg.sender;
    }

    function deposit(uint256 amount) external returns (uint256 minted) {
        require(amount > 0, "amount");
        require(token.transferFrom(msg.sender, address(this), amount), "transfer");
        minted = totalShares == 0 ? amount : quoteShares(amount);
        shares[msg.sender] += minted;
        totalShares += minted;
        lastDeposit[msg.sender] = block.timestamp;
        emit Deposited(msg.sender, amount, minted);
    }

    function withdraw(uint256 requested) external returns (uint256 amount) {
        uint256 available = shares[msg.sender];
        uint256 burned = PoolMath.min(requested, available);
        require(burned > 0, "shares");
        if (totalShares > 0) {
            amount = (burned * tokenBalance()) / totalShares;
        }
        shares[msg.sender] -= burned;
        totalShares -= burned;
        require(token.transfer(msg.sender, amount), "transfer");
        emit Withdrawn(msg.sender, amount, burned);
    }

    function quoteShares(uint256 amount) public view returns (uint256) {
        uint256 balance = tokenBalance();
        if (balance == 0 || totalShares == 0) {
            return amount;
        }
        return (amount * totalShares) / balance;
    }

    function tokenBalance() public view returns (uint256) {
        return shares[address(this)] + totalShares;
    }

    function rebalance(uint256[] calldata weights) external onlyOwner {
        uint256 total;
        for (uint256 index = 0; index < weights.length; index++) {
            if (weights[index] > 0) {
                total += weights[index];
            }
        }
        require(total > 0, "weights");
    }

    function transferOwnership(address nextOwner) external onlyOwner {
        require(nextOwner != address(0), "owner");
        owner = nextOwner;
    }
}
