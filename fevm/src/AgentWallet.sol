// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {AgentRegistry} from "./AgentRegistry.sol";

/// @title AXON AgentWallet — cap-guarded operational wallet for AI agents (FEVM port)
/// @notice A deployed wallet owned by an agent owner. Holds funds for the agent but
///         structurally enforces a spending cap per window. The agent (hot key) can
///         spend up to `spendCap` per `window`; the owner can withdraw anything.
contract AgentWallet {
    address public owner;         // human owner (cold key)
    address public agent;         // agent hot key allowed to spend within cap
    uint256 public spendCap;      // max spend per window (wei)
    uint256 public windowSeconds;
    uint256 private windowStart;
    uint256 private spentThisWindow;
    AgentRegistry public registry;
    uint256 public agentId;

    event AgentSet(address indexed agent);
    event CapSet(uint256 cap, uint256 windowSeconds);
    event Deposited(address indexed from, uint256 amount);
    event Spent(address indexed to, uint256 amount, uint256 spentThisWindow);
    event Withdrawn(address indexed to, uint256 amount);

    error NotOwner();
    error NotAgent();
    error CapExceeded();
    error ZeroAddress();

    modifier onlyOwner() { if (msg.sender != owner) revert NotOwner(); _; }
    modifier onlyAgentOrOwner() { if (msg.sender != agent && msg.sender != owner) revert NotAgent(); _; }

    constructor(address _registry, uint256 _agentId, address _agent, uint256 _spendCap, uint256 _windowSeconds) {
        owner = msg.sender;
        registry = AgentRegistry(_registry);
        agentId = _agentId;
        agent = _agent;
        spendCap = _spendCap;
        windowSeconds = _windowSeconds;
        windowStart = block.timestamp;
    }

    function setAgent(address a) external onlyOwner {
        if (a == address(0)) revert ZeroAddress();
        agent = a;
        emit AgentSet(a);
    }

    function setCap(uint256 cap, uint256 window) external onlyOwner {
        spendCap = cap;
        windowSeconds = window;
        emit CapSet(cap, window);
    }

    function deposit() external payable { emit Deposited(msg.sender, msg.value); }
    receive() external payable { emit Deposited(msg.sender, msg.value); }

    /// @notice Agent hot key spends native currency within the per-window cap.
    function spend(address payable to, uint256 amount) external onlyAgentOrOwner {
        if (block.timestamp >= windowStart + windowSeconds) {
            windowStart = block.timestamp;
            spentThisWindow = 0;
        }
        if (msg.sender != owner) {
            if (spentThisWindow + amount > spendCap) revert CapExceeded();
        }
        spentThisWindow += amount;
        (bool ok, ) = to.call{value: amount}("");
        require(ok, "spend failed");
        if (registry.isActive(agentId)) {
            // task usage leaves a trace via registry is handled by escrow; wallet stays minimal
        }
        emit Spent(to, amount, spentThisWindow);
    }

    function withdraw(address payable to, uint256 amount) external onlyOwner {
        (bool ok, ) = to.call{value: amount}("");
        require(ok, "withdraw failed");
        emit Withdrawn(to, amount);
    }

    function balance() external view returns (uint256) { return address(this).balance; }

    function remainingThisWindow() external view returns (uint256) {
        if (block.timestamp >= windowStart + windowSeconds) return spendCap;
        return spendCap - spentThisWindow;
    }
}
