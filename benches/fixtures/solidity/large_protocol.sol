// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

interface IAsset {
    function balanceOf(address account) external view returns (uint256);
    function transfer(address recipient, uint256 amount) external returns (bool);
    function transferFrom(address sender, address recipient, uint256 amount)
        external
        returns (bool);
}

interface IPriceOracle {
    function price(address asset) external view returns (uint256);
}

library FixedPoint {
    uint256 internal constant SCALE = 1e18;

    function multiply(uint256 left, uint256 right) internal pure returns (uint256) {
        return (left * right) / SCALE;
    }

    function divide(uint256 left, uint256 right) internal pure returns (uint256) {
        require(right != 0, "division");
        return (left * SCALE) / right;
    }
}

contract AccessRegistry {
    address public owner;
    mapping(address => bool) public operators;
    mapping(address => bool) public guardians;

    modifier onlyOwner() {
        require(msg.sender == owner, "owner");
        _;
    }

    modifier onlyOperator() {
        require(operators[msg.sender], "operator");
        _;
    }

    constructor() {
        owner = msg.sender;
        operators[msg.sender] = true;
    }

    function setOperator(address account, bool enabled) external onlyOwner {
        operators[account] = enabled;
    }

    function setGuardian(address account, bool enabled) external onlyOwner {
        guardians[account] = enabled;
    }

    function transferOwnership(address nextOwner) external onlyOwner {
        require(nextOwner != address(0), "owner");
        owner = nextOwner;
    }
}

contract LargeProtocol is AccessRegistry {
    struct Position {
        uint256 collateral;
        uint256 debt;
        uint256 openedAt;
    }

    IPriceOracle public oracle;
    address public treasury;
    uint256 public minimumCollateralRatio;
    uint256 public liquidationRatio;
    uint256 public totalCollateral;
    uint256 public totalDebt;
    bool public paused;

    mapping(address => IAsset) public assets;
    mapping(address => mapping(address => Position)) public positions;
    mapping(address => uint256) public protocolFees;

    event PositionOpened(address indexed account, address indexed asset, uint256 collateral);
    event PositionAdjusted(address indexed account, address indexed asset, int256 delta);
    event PositionClosed(address indexed account, address indexed asset);
    event Liquidated(address indexed account, address indexed asset, uint256 repaid);

    modifier whenActive() {
        require(!paused, "paused");
        _;
    }

    constructor(IPriceOracle priceOracle, address feeTreasury) {
        oracle = priceOracle;
        treasury = feeTreasury;
        minimumCollateralRatio = 15e17;
        liquidationRatio = 12e17;
    }

    function registerAsset(address asset, IAsset token) external onlyOwner {
        require(asset != address(0), "asset");
        assets[asset] = token;
    }

    function open(address asset, uint256 collateral, uint256 debtAmount)
        external
        whenActive
    {
        require(collateral > 0, "collateral");
        require(assets[asset].transferFrom(msg.sender, address(this), collateral), "transfer");
        Position storage position = positions[msg.sender][asset];
        require(position.collateral == 0, "exists");
        position.collateral = collateral;
        position.debt = debtAmount;
        position.openedAt = block.timestamp;
        require(collateralRatio(msg.sender, asset) >= minimumCollateralRatio, "ratio");
        totalCollateral += collateral;
        totalDebt += debtAmount;
        emit PositionOpened(msg.sender, asset, collateral);
    }

    function addCollateral(address asset, uint256 amount) external whenActive {
        require(amount > 0, "amount");
        require(assets[asset].transferFrom(msg.sender, address(this), amount), "transfer");
        positions[msg.sender][asset].collateral += amount;
        totalCollateral += amount;
        emit PositionAdjusted(msg.sender, asset, int256(amount));
    }

    function removeCollateral(address asset, uint256 amount) external whenActive {
        Position storage position = positions[msg.sender][asset];
        require(position.collateral >= amount, "collateral");
        position.collateral -= amount;
        require(collateralRatio(msg.sender, asset) >= minimumCollateralRatio, "ratio");
        totalCollateral -= amount;
        require(assets[asset].transfer(msg.sender, amount), "transfer");
        emit PositionAdjusted(msg.sender, asset, -int256(amount));
    }

    function borrow(address asset, uint256 amount) external whenActive {
        Position storage position = positions[msg.sender][asset];
        position.debt += amount;
        require(collateralRatio(msg.sender, asset) >= minimumCollateralRatio, "ratio");
        totalDebt += amount;
    }

    function repay(address asset, uint256 amount) external {
        Position storage position = positions[msg.sender][asset];
        uint256 repaid = amount > position.debt ? position.debt : amount;
        position.debt -= repaid;
        totalDebt -= repaid;
    }

    function close(address asset) external {
        Position storage position = positions[msg.sender][asset];
        require(position.debt == 0, "debt");
        uint256 collateral = position.collateral;
        delete positions[msg.sender][asset];
        totalCollateral -= collateral;
        require(assets[asset].transfer(msg.sender, collateral), "transfer");
        emit PositionClosed(msg.sender, asset);
    }

    function liquidate(address account, address asset, uint256 repayment)
        external
        onlyOperator
    {
        require(collateralRatio(account, asset) < liquidationRatio, "healthy");
        Position storage position = positions[account][asset];
        uint256 repaid = repayment > position.debt ? position.debt : repayment;
        uint256 seized = collateralForDebt(asset, repaid);
        if (seized > position.collateral) {
            seized = position.collateral;
        }
        position.debt -= repaid;
        position.collateral -= seized;
        totalDebt -= repaid;
        totalCollateral -= seized;
        require(assets[asset].transfer(msg.sender, seized), "transfer");
        emit Liquidated(account, asset, repaid);
    }

    function collateralRatio(address account, address asset) public view returns (uint256) {
        Position storage position = positions[account][asset];
        if (position.debt == 0) {
            return type(uint256).max;
        }
        uint256 value = FixedPoint.multiply(position.collateral, oracle.price(asset));
        return FixedPoint.divide(value, position.debt);
    }

    function collateralForDebt(address asset, uint256 debtAmount)
        public
        view
        returns (uint256)
    {
        return FixedPoint.divide(debtAmount, oracle.price(asset));
    }

    function portfolioValue(address account, address[] calldata listedAssets)
        external
        view
        returns (uint256 total)
    {
        for (uint256 index = 0; index < listedAssets.length; index++) {
            address asset = listedAssets[index];
            Position storage position = positions[account][asset];
            if (position.collateral > 0) {
                total += FixedPoint.multiply(position.collateral, oracle.price(asset));
            }
        }
    }

    function collectFees(address[] calldata listedAssets) external onlyOperator {
        for (uint256 index = 0; index < listedAssets.length; index++) {
            address asset = listedAssets[index];
            uint256 fee = protocolFees[asset];
            if (fee > 0) {
                protocolFees[asset] = 0;
                require(assets[asset].transfer(treasury, fee), "transfer");
            }
        }
    }

    function setRatios(uint256 minimumRatio, uint256 newLiquidationRatio)
        external
        onlyOwner
    {
        require(minimumRatio > newLiquidationRatio, "ratios");
        minimumCollateralRatio = minimumRatio;
        liquidationRatio = newLiquidationRatio;
    }

    function setOracle(IPriceOracle nextOracle) external onlyOwner {
        oracle = nextOracle;
    }

    function setTreasury(address nextTreasury) external onlyOwner {
        require(nextTreasury != address(0), "treasury");
        treasury = nextTreasury;
    }

    function setPaused(bool nextPaused) external onlyOwner {
        paused = nextPaused;
    }
}
