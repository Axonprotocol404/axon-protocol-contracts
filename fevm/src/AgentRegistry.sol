// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

/// @title AXON AgentRegistry — on-chain identity & reputation for AI agents (FEVM port of Soroban AgentRegistry)
/// @notice Agents register once; identity + reputation + task history become verifiable on-chain primitives.
contract AgentRegistry {
    struct Agent {
        uint256 id;
        address owner;
        string name;
        string metadata;      // ipfs hash / url
        uint8  status;        // 0=none, 1=active, 2=suspended
        int256 reputation;    // sum of ratings
        uint32 ratingCount;
        uint64 registeredAt;
        uint64 lastTaskAt;
    }

    uint256 public nextAgentId = 1;
    address public admin;

    mapping(uint256 => Agent) public agents;
    mapping(address => uint256) public agentIdByOwner;

    event AgentRegistered(uint256 indexed id, address indexed owner, string name);
    event AgentUpdated(uint256 indexed id, string metadata);
    event AgentStatusChanged(uint256 indexed id, uint8 status);
    event Rated(uint256 indexed agentId, address indexed rater, int8 score);
    event TaskCompleted(uint256 indexed agentId, uint64 at);

    error NotAdmin();
    error AlreadyRegistered();
    error NotOwner();
    error NotFound();
    error BadStatus();
    error BadScore();
    error SelfRating();

    modifier onlyAdmin() { if (msg.sender != admin) revert NotAdmin(); _; }

    constructor() { admin = msg.sender; }

    function setAdmin(address a) external onlyAdmin { admin = a; }

    /// @notice Register a new agent identity. One identity per owner address.
    function registerAgent(string calldata name, string calldata metadata) external returns (uint256 id) {
        if (agentIdByOwner[msg.sender] != 0) revert AlreadyRegistered();
        id = nextAgentId++;
        agents[id] = Agent({
            id: id,
            owner: msg.sender,
            name: name,
            metadata: metadata,
            status: 1,
            reputation: 0,
            ratingCount: 0,
            registeredAt: uint64(block.timestamp),
            lastTaskAt: 0
        });
        agentIdByOwner[msg.sender] = id;
        emit AgentRegistered(id, msg.sender, name);
    }

    function updateMetadata(uint256 id, string calldata metadata) external {
        Agent storage a = agents[id];
        if (a.owner == address(0)) revert NotFound();
        if (a.owner != msg.sender) revert NotOwner();
        a.metadata = metadata;
        emit AgentUpdated(id, metadata);
    }

    /// @notice Admin suspension (e.g. proven malicious behavior).
    function setStatus(uint256 id, uint8 status) external onlyAdmin {
        if (status != 1 && status != 2) revert BadStatus();
        if (agents[id].owner == address(0)) revert NotFound();
        agents[id].status = status;
        emit AgentStatusChanged(id, status);
    }

    /// @notice Rate an agent (-100..100). Caller cannot rate own agent.
    function rate(uint256 id, int8 score) external {
        if (score < -100 || score > 100) revert BadScore();
        Agent storage a = agents[id];
        if (a.owner == address(0)) revert NotFound();
        if (a.owner == msg.sender) revert SelfRating();
        a.reputation += score;
        a.ratingCount += 1;
        emit Rated(id, msg.sender, score);
    }

    /// @notice Called by escrow when a task settles successfully.
    function recordTask(uint256 id) external {
        if (agents[id].owner == address(0)) revert NotFound();
        agents[id].lastTaskAt = uint64(block.timestamp);
        emit TaskCompleted(id, uint64(block.timestamp));
    }

    // ---- views ----
    function getAgent(uint256 id) external view returns (Agent memory) { return agents[id]; }

    function averageRating(uint256 id) external view returns (int256) {
        Agent storage a = agents[id];
        if (a.ratingCount == 0) return 0;
        return a.reputation / int256(uint256(a.ratingCount));
    }

    function isActive(uint256 id) external view returns (bool) {
        return agents[id].status == 1;
    }
}
