// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "./AgentWallet.sol";
import "./AgentRegistry.sol";

/**
 * @title WalletFactory
 * @notice AXON Protocol — Auto-deploys wallets for registered AI agents
 * @dev Called during agent registration or separately by agent owners
 */
contract WalletFactory {
    // Reference to AgentRegistry
    AgentRegistry public registry;

    // agentId => wallet address
    mapping(uint256 => address) public agentWallets;

    // Default limits
    uint256 public defaultDailyLimit = 0.1 ether;
    uint256 public defaultPerTxLimit = 0.05 ether;

    // Protocol owner
    address public owner;

    // Events
    event WalletCreated(
        uint256 indexed agentId,
        address indexed wallet,
        address indexed agentOwner
    );
    event DefaultLimitsUpdated(uint256 dailyLimit, uint256 perTxLimit);

    modifier onlyOwner() {
        require(msg.sender == owner, "Not owner");
        _;
    }

    constructor(address _registry) {
        registry = AgentRegistry(_registry);
        owner = msg.sender;
    }

    /**
     * @notice Create a wallet for a registered agent
     * @param agentId The agent's NFT ID from AgentRegistry
     */
    function createWallet(uint256 agentId) external returns (address) {
        require(agentWallets[agentId] == address(0), "Wallet already exists");

        // Verify caller owns this agent
        require(registry.ownerOf(agentId) == msg.sender, "Not agent owner");

        // Get agent's operator address
        AgentRegistry.AgentProfile memory agent = registry.getAgent(agentId);
        require(agent.active, "Agent not active");

        // Deploy new wallet
        AgentWallet wallet = new AgentWallet(
            msg.sender,          // owner = agent's NFT owner
            agent.operator,      // operator = agent's operator address
            agentId,
            defaultDailyLimit,
            defaultPerTxLimit
        );

        agentWallets[agentId] = address(wallet);

        emit WalletCreated(agentId, address(wallet), msg.sender);
        return address(wallet);
    }

    /**
     * @notice Create wallet with custom limits
     */
    function createWalletCustom(
        uint256 agentId,
        uint256 dailyLimit,
        uint256 perTxLimit
    ) external returns (address) {
        require(agentWallets[agentId] == address(0), "Wallet already exists");
        require(registry.ownerOf(agentId) == msg.sender, "Not agent owner");

        AgentRegistry.AgentProfile memory agent = registry.getAgent(agentId);
        require(agent.active, "Agent not active");

        AgentWallet wallet = new AgentWallet(
            msg.sender,
            agent.operator,
            agentId,
            dailyLimit,
            perTxLimit
        );

        agentWallets[agentId] = address(wallet);

        emit WalletCreated(agentId, address(wallet), msg.sender);
        return address(wallet);
    }

    /**
     * @notice Get wallet address for an agent
     */
    function getWallet(uint256 agentId) external view returns (address) {
        return agentWallets[agentId];
    }

    /**
     * @notice Update default limits for new wallets
     */
    function setDefaultLimits(uint256 dailyLimit, uint256 perTxLimit) external onlyOwner {
        defaultDailyLimit = dailyLimit;
        defaultPerTxLimit = perTxLimit;
        emit DefaultLimitsUpdated(dailyLimit, perTxLimit);
    }
}
