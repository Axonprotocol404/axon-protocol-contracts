// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";

/**
 * @title AgentWallet
 * @notice AXON Protocol — Smart wallet for AI agents with spending controls
 * @dev Owner sets limits, agent (operator) executes transactions within those limits
 */
contract AgentWallet is ReentrancyGuard {
    using SafeERC20 for IERC20;

    // Wallet owner (human who controls the agent)
    address public owner;

    // Agent operator (address the AI agent uses)
    address public operator;

    // Agent ID in AgentRegistry
    uint256 public agentId;

    // Spending limits
    uint256 public dailyLimit;           // Max ETH per day
    uint256 public perTransactionLimit;  // Max ETH per single transaction
    uint256 public dailySpent;           // ETH spent today
    uint256 public lastResetTimestamp;   // When daily counter was last reset

    // ERC20 spending limits
    mapping(address => uint256) public tokenDailyLimit;
    mapping(address => uint256) public tokenDailySpent;
    mapping(address => uint256) public tokenLastReset;

    // Whitelisted addresses (agent can send to these without extra approval)
    mapping(address => bool) public whitelisted;

    // Emergency freeze
    bool public frozen;

    // Events
    event Deposited(address indexed from, uint256 amount);
    event Withdrawn(address indexed to, uint256 amount, string reason);
    event TokenWithdrawn(address indexed token, address indexed to, uint256 amount);
    event DailyLimitSet(uint256 newLimit);
    event PerTransactionLimitSet(uint256 newLimit);
    event TokenDailyLimitSet(address indexed token, uint256 newLimit);
    event AddressWhitelisted(address indexed addr, bool status);
    event OperatorChanged(address indexed newOperator);
    event WalletFrozen();
    event WalletUnfrozen();
    event OwnerChanged(address indexed newOwner);

    modifier onlyOwner() {
        require(msg.sender == owner, "Not wallet owner");
        _;
    }

    modifier onlyOperator() {
        require(msg.sender == operator, "Not operator");
        _;
    }

    modifier onlyOwnerOrOperator() {
        require(msg.sender == owner || msg.sender == operator, "Not authorized");
        _;
    }

    modifier notFrozen() {
        require(!frozen, "Wallet is frozen");
        _;
    }

    /**
     * @param _owner Wallet owner (human)
     * @param _operator Agent operator address
     * @param _agentId Agent ID from AgentRegistry
     * @param _dailyLimit Daily spending limit in wei
     * @param _perTxLimit Per-transaction limit in wei
     */
    constructor(
        address _owner,
        address _operator,
        uint256 _agentId,
        uint256 _dailyLimit,
        uint256 _perTxLimit
    ) {
        require(_owner != address(0), "Invalid owner");
        require(_operator != address(0), "Invalid operator");

        owner = _owner;
        operator = _operator;
        agentId = _agentId;
        dailyLimit = _dailyLimit;
        perTransactionLimit = _perTxLimit;
        lastResetTimestamp = block.timestamp;
    }

    // ============ Receive ETH ============

    receive() external payable {
        emit Deposited(msg.sender, msg.value);
    }

    // ============ Operator Functions (Agent) ============

    /**
     * @notice Agent sends ETH to a whitelisted address
     * @param to Recipient address (must be whitelisted)
     * @param amount Amount in wei
     * @param reason Description of the transaction
     */
    function send(address payable to, uint256 amount, string calldata reason)
        external
        onlyOperator
        notFrozen
        nonReentrant
    {
        require(whitelisted[to], "Recipient not whitelisted");
        require(amount <= perTransactionLimit, "Exceeds per-transaction limit");

        // Reset daily counter if new day
        _resetDailyIfNeeded();

        require(dailySpent + amount <= dailyLimit, "Exceeds daily limit");

        dailySpent += amount;
        (bool success, ) = to.call{value: amount}("");
        require(success, "Transfer failed");

        emit Withdrawn(to, amount, reason);
    }

    /**
     * @notice Agent sends ERC20 tokens to a whitelisted address
     */
    function sendToken(
        address token,
        address to,
        uint256 amount
    ) external onlyOperator notFrozen nonReentrant {
        require(whitelisted[to], "Recipient not whitelisted");

        // Reset token daily counter if new day
        _resetTokenDailyIfNeeded(token);

        require(
            tokenDailySpent[token] + amount <= tokenDailyLimit[token],
            "Exceeds token daily limit"
        );

        tokenDailySpent[token] += amount;
        IERC20(token).safeTransfer(to, amount);

        emit TokenWithdrawn(token, to, amount);
    }

    // ============ Owner Functions ============

    /**
     * @notice Owner withdraws ETH (no limits, emergency or management)
     */
    function ownerWithdraw(address payable to, uint256 amount)
        external
        onlyOwner
        nonReentrant
    {
        require(address(this).balance >= amount, "Insufficient balance");
        (bool success, ) = to.call{value: amount}("");
        require(success, "Transfer failed");
        emit Withdrawn(to, amount, "owner_withdraw");
    }

    /**
     * @notice Owner withdraws ERC20 tokens
     */
    function ownerWithdrawToken(address token, address to, uint256 amount)
        external
        onlyOwner
        nonReentrant
    {
        IERC20(token).safeTransfer(to, amount);
        emit TokenWithdrawn(token, to, amount);
    }

    /**
     * @notice Set daily spending limit for ETH
     */
    function setDailyLimit(uint256 newLimit) external onlyOwner {
        dailyLimit = newLimit;
        emit DailyLimitSet(newLimit);
    }

    /**
     * @notice Set per-transaction limit for ETH
     */
    function setPerTransactionLimit(uint256 newLimit) external onlyOwner {
        perTransactionLimit = newLimit;
        emit PerTransactionLimitSet(newLimit);
    }

    /**
     * @notice Set daily spending limit for an ERC20 token
     */
    function setTokenDailyLimit(address token, uint256 newLimit) external onlyOwner {
        tokenDailyLimit[token] = newLimit;
        emit TokenDailyLimitSet(token, newLimit);
    }

    /**
     * @notice Whitelist or remove an address
     */
    function setWhitelist(address addr, bool status) external onlyOwner {
        whitelisted[addr] = status;
        emit AddressWhitelisted(addr, status);
    }

    /**
     * @notice Batch whitelist addresses
     */
    function batchWhitelist(address[] calldata addrs, bool status) external onlyOwner {
        for (uint256 i = 0; i < addrs.length; i++) {
            whitelisted[addrs[i]] = status;
            emit AddressWhitelisted(addrs[i], status);
        }
    }

    /**
     * @notice Change the operator (agent address)
     */
    function setOperator(address newOperator) external onlyOwner {
        require(newOperator != address(0), "Invalid operator");
        operator = newOperator;
        emit OperatorChanged(newOperator);
    }

    /**
     * @notice Emergency freeze — stops all agent transactions
     */
    function freeze() external onlyOwner {
        frozen = true;
        emit WalletFrozen();
    }

    /**
     * @notice Unfreeze wallet
     */
    function unfreeze() external onlyOwner {
        frozen = false;
        emit WalletUnfrozen();
    }

    /**
     * @notice Transfer ownership
     */
    function transferOwnership(address newOwner) external onlyOwner {
        require(newOwner != address(0), "Invalid owner");
        owner = newOwner;
        emit OwnerChanged(newOwner);
    }

    // ============ View Functions ============

    /**
     * @notice Get remaining daily allowance for ETH
     */
    function remainingDailyAllowance() external view returns (uint256) {
        if (block.timestamp >= lastResetTimestamp + 1 days) {
            return dailyLimit;
        }
        if (dailySpent >= dailyLimit) return 0;
        return dailyLimit - dailySpent;
    }

    /**
     * @notice Get remaining daily allowance for a token
     */
    function remainingTokenDailyAllowance(address token) external view returns (uint256) {
        if (block.timestamp >= tokenLastReset[token] + 1 days) {
            return tokenDailyLimit[token];
        }
        if (tokenDailySpent[token] >= tokenDailyLimit[token]) return 0;
        return tokenDailyLimit[token] - tokenDailySpent[token];
    }

    /**
     * @notice Get wallet ETH balance
     */
    function balance() external view returns (uint256) {
        return address(this).balance;
    }

    /**
     * @notice Get wallet ERC20 token balance
     */
    function tokenBalance(address token) external view returns (uint256) {
        return IERC20(token).balanceOf(address(this));
    }

    // ============ Internal ============

    function _resetDailyIfNeeded() internal {
        if (block.timestamp >= lastResetTimestamp + 1 days) {
            dailySpent = 0;
            lastResetTimestamp = block.timestamp;
        }
    }

    function _resetTokenDailyIfNeeded(address token) internal {
        if (block.timestamp >= tokenLastReset[token] + 1 days) {
            tokenDailySpent[token] = 0;
            tokenLastReset[token] = block.timestamp;
        }
    }
}
