// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/token/ERC721/ERC721.sol";
import "@openzeppelin/contracts/token/ERC721/extensions/ERC721Enumerable.sol";
import "@openzeppelin/contracts/access/Ownable.sol";
/**
 * @title AgentRegistry
 * @notice AXON Protocol — On-chain identity for AI agents
 * @dev Each AI agent gets a unique ERC-721 NFT as their identity
 */
contract AgentRegistry is ERC721, ERC721Enumerable, Ownable {
    uint256 private _nextAgentId = 1;

    // Registration fee (can be updated by owner)
    uint256 public registrationFee = 0.001 ether;

    // Agent types
    enum AgentType {
        General,
        Coding,
        Trading,
        Research,
        Content,
        Data,
        Custom
    }

    // Agent profile stored on-chain
    struct AgentProfile {
        string name;
        AgentType agentType;
        string metadataURI;      // IPFS link to full metadata
        address operator;         // Address that the agent uses to transact
        uint256 reputationScore;
        uint256 tasksCompleted;
        uint256 registeredAt;
        bool active;
    }

    // agentId => AgentProfile
    mapping(uint256 => AgentProfile) public agents;

    // operator address => agentId (reverse lookup)
    mapping(address => uint256) public operatorToAgent;

    // track if operator is assigned (since agentId 0 is default)
    mapping(address => bool) public hasOperator;

    // Events
    event AgentRegistered(
        uint256 indexed agentId,
        address indexed owner,
        address indexed operator,
        string name,
        AgentType agentType
    );

    event AgentUpdated(uint256 indexed agentId, string metadataURI);
    event AgentDeactivated(uint256 indexed agentId);
    event AgentReactivated(uint256 indexed agentId);
    event OperatorChanged(uint256 indexed agentId, address indexed newOperator);
    event ReputationUpdated(uint256 indexed agentId, uint256 newScore);
    event RegistrationFeeUpdated(uint256 newFee);

    constructor() ERC721("AXON Agent ID", "AXON-ID") Ownable(msg.sender) {}

    /**
     * @notice Register a new AI agent
     * @param name Agent display name
     * @param agentType Type/category of the agent
     * @param metadataURI IPFS URI to full metadata JSON
     * @param operator Address the agent will use to operate
     */
    function registerAgent(
        string calldata name,
        AgentType agentType,
        string calldata metadataURI,
        address operator
    ) external payable returns (uint256) {
        require(msg.value >= registrationFee, "Insufficient registration fee");
        require(bytes(name).length > 0, "Name cannot be empty");
        require(operator != address(0), "Invalid operator address");
        require(!hasOperator[operator], "Operator already assigned to an agent");

        uint256 newAgentId = _nextAgentId++;


        // Mint NFT to the caller (owner of the agent)
        _safeMint(msg.sender, newAgentId);

        // Store agent profile
        agents[newAgentId] = AgentProfile({
            name: name,
            agentType: agentType,
            metadataURI: metadataURI,
            operator: operator,
            reputationScore: 100, // Start with base score of 100
            tasksCompleted: 0,
            registeredAt: block.timestamp,
            active: true
        });

        // Map operator to agent
        operatorToAgent[operator] = newAgentId;
        hasOperator[operator] = true;

        // Refund excess payment
        if (msg.value > registrationFee) {
            (bool success, ) = payable(msg.sender).call{value: msg.value - registrationFee}("");
            require(success, "Refund failed");
        }

        emit AgentRegistered(newAgentId, msg.sender, operator, name, agentType);
        return newAgentId;
    }

    /**
     * @notice Update agent metadata (only agent owner)
     */
    function updateMetadata(uint256 agentId, string calldata metadataURI) external {
        require(ownerOf(agentId) == msg.sender, "Not agent owner");
        agents[agentId].metadataURI = metadataURI;
        emit AgentUpdated(agentId, metadataURI);
    }

    /**
     * @notice Change agent operator address (only agent owner)
     */
    function changeOperator(uint256 agentId, address newOperator) external {
        require(ownerOf(agentId) == msg.sender, "Not agent owner");
        require(newOperator != address(0), "Invalid operator");
        require(!hasOperator[newOperator], "Operator already assigned");

        // Remove old mapping
        address oldOperator = agents[agentId].operator;
        delete operatorToAgent[oldOperator];
        hasOperator[oldOperator] = false;

        // Set new operator
        agents[agentId].operator = newOperator;
        operatorToAgent[newOperator] = agentId;
        hasOperator[newOperator] = true;

        emit OperatorChanged(agentId, newOperator);
    }

    /**
     * @notice Deactivate an agent (only agent owner)
     */
    function deactivateAgent(uint256 agentId) external {
        require(ownerOf(agentId) == msg.sender, "Not agent owner");
        require(agents[agentId].active, "Already inactive");
        agents[agentId].active = false;
        emit AgentDeactivated(agentId);
    }

    /**
     * @notice Reactivate an agent (only agent owner)
     */
    function reactivateAgent(uint256 agentId) external {
        require(ownerOf(agentId) == msg.sender, "Not agent owner");
        require(!agents[agentId].active, "Already active");
        agents[agentId].active = true;
        emit AgentReactivated(agentId);
    }

    /**
     * @notice Update reputation score (only protocol owner or authorized contracts)
     * @dev In production, this should be called by the Escrow/Trust contracts
     */
    function updateReputation(uint256 agentId, uint256 newScore) external onlyOwner {
        require(_exists(agentId), "Agent does not exist");
        agents[agentId].reputationScore = newScore;
        emit ReputationUpdated(agentId, newScore);
    }

    /**
     * @notice Increment tasks completed (only protocol owner or authorized contracts)
     */
    function incrementTasks(uint256 agentId) external onlyOwner {
        require(_exists(agentId), "Agent does not exist");
        agents[agentId].tasksCompleted++;
    }

    /**
     * @notice Update registration fee (only protocol owner)
     */
    function setRegistrationFee(uint256 newFee) external onlyOwner {
        registrationFee = newFee;
        emit RegistrationFeeUpdated(newFee);
    }

    /**
     * @notice Withdraw collected fees (only protocol owner)
     */
    function withdrawFees() external onlyOwner {
        uint256 balance = address(this).balance;
        require(balance > 0, "No fees to withdraw");
        (bool success, ) = payable(owner()).call{value: balance}("");
        require(success, "Withdraw failed");
    }

    // ============ View Functions ============

    /**
     * @notice Get full agent profile
     */
    function getAgent(uint256 agentId) external view returns (AgentProfile memory) {
        require(_exists(agentId), "Agent does not exist");
        return agents[agentId];
    }

    /**
     * @notice Get agent ID by operator address
     */
    function getAgentByOperator(address operator) external view returns (uint256) {
        return operatorToAgent[operator];
    }

    /**
     * @notice Check if an agent is active
     */
    function isActive(uint256 agentId) external view returns (bool) {
        return agents[agentId].active;
    }

    /**
     * @notice Get total registered agents
     */
    function totalAgents() external view returns (uint256) {
        return _nextAgentId - 1;
    }

    /**
     * @notice Check if agent exists
     */
    function _exists(uint256 tokenId) internal view returns (bool) {
        return _ownerOf(tokenId) != address(0);
    }

    // ============ Required Overrides ============

    function _update(address to, uint256 tokenId, address auth)
        internal
        override(ERC721, ERC721Enumerable)
        returns (address)
    {
        return super._update(to, tokenId, auth);
    }

    function _increaseBalance(address account, uint128 value)
        internal
        override(ERC721, ERC721Enumerable)
    {
        super._increaseBalance(account, value);
    }

    function supportsInterface(bytes4 interfaceId)
        public
        view
        override(ERC721, ERC721Enumerable)
        returns (bool)
    {
        return super.supportsInterface(interfaceId);
    }
}
