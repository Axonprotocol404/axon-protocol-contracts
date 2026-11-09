// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {AgentRegistry} from "./AgentRegistry.sol";

/// @title AXON SimpleEscrow — escrow-secured agent tasks (FEVM port)
/// @notice Client locks payment; agent delivers; client confirms → release + reputation trace.
///         Disputes resolved by client release/refund; admin can arbitrate after timeout.
contract SimpleEscrow {
    enum State { None, Open, Released, Refunded }

    struct Escrow {
        uint256 id;
        address client;
        address agent;
        uint256 agentId;      // registry id of agent
        uint256 amount;
        uint64  timeout;
        State   state;
    }

    uint256 public nextEscrowId = 1;
    address public admin;
    AgentRegistry public registry;

    mapping(uint256 => Escrow) public escrows;

    event EscrowOpened(uint256 indexed id, address indexed client, address indexed agent, uint256 amount, uint64 timeout);
    event Released(uint256 indexed id, uint256 amount);
    event Refunded(uint256 indexed id, uint256 amount);
    event Arbitrated(uint256 indexed id, address to, uint256 amount);

    error NotParty();
    error NotAdmin();
    error BadState();
    error TooEarly();
    error ZeroValue();
    error NotRegistered();

    modifier onlyAdmin() { if (msg.sender != admin) revert NotAdmin(); _; }

    constructor(address _registry) { admin = msg.sender; registry = AgentRegistry(_registry); }

    function setAdmin(address a) external onlyAdmin { admin = a; }

    /// @notice Open escrow: lock payment for a task with the given agent.
    function open(address agent, uint256 agentId, uint64 timeout) external payable returns (uint256 id) {
        if (msg.value == 0) revert ZeroValue();
        if (!registry.isActive(agentId)) revert NotRegistered();
        id = nextEscrowId++;
        escrows[id] = Escrow({
            id: id,
            client: msg.sender,
            agent: agent,
            agentId: agentId,
            amount: msg.value,
            timeout: uint64(block.timestamp) + timeout,
            state: State.Open
        });
        emit EscrowOpened(id, msg.sender, agent, msg.value, uint64(block.timestamp) + timeout);
    }

    /// @notice Client confirms delivery → pay agent + record successful task in registry.
    function release(uint256 id) external {
        Escrow storage e = escrows[id];
        if (e.state != State.Open) revert BadState();
        if (msg.sender != e.client && msg.sender != admin) revert NotParty();
        e.state = State.Released;
        (bool ok, ) = payable(e.agent).call{value: e.amount}("");
        require(ok, "release failed");
        registry.recordTask(e.agentId);
        emit Released(id, e.amount);
    }

    /// @notice Client refunds itself (agent failed to deliver) — allowed after timeout, or anytime by agreement? Client can refund only after timeout OR admin anytime.
    function refund(uint256 id) external {
        Escrow storage e = escrows[id];
        if (e.state != State.Open) revert BadState();
        if (msg.sender != e.client && msg.sender != admin) revert NotParty();
        if (msg.sender == e.client && block.timestamp < e.timeout) revert TooEarly();
        e.state = State.Refunded;
        (bool ok, ) = payable(e.client).call{value: e.amount}("");
        require(ok, "refund failed");
        emit Refunded(id, e.amount);
    }

    /// @notice Admin arbitration: send to either party after timeout.
    function arbitrate(uint256 id, address to) external onlyAdmin {
        Escrow storage e = escrows[id];
        if (e.state != State.Open) revert BadState();
        if (block.timestamp < e.timeout) revert TooEarly();
        if (to != e.client && to != e.agent) revert NotParty();
        e.state = (to == e.agent) ? State.Released : State.Refunded;
        (bool ok, ) = payable(to).call{value: e.amount}("");
        require(ok, "arb failed");
        if (to == e.agent) registry.recordTask(e.agentId);
        emit Arbitrated(id, to, e.amount);
    }

    function getEscrow(uint256 id) external view returns (Escrow memory) { return escrows[id]; }
}
