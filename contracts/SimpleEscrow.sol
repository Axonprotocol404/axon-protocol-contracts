// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import "@openzeppelin/contracts/access/Ownable.sol";

/**
 * @title SimpleEscrow
 * @notice AXON Protocol — Trustless escrow for agent-to-agent transactions
 * @dev Agent A deposits payment, Agent B completes task, payment releases
 */
contract SimpleEscrow is ReentrancyGuard, Ownable {
    using SafeERC20 for IERC20;

    // Protocol fee (basis points, 30 = 0.3%)
    uint256 public protocolFeeBps = 30;
    uint256 public constant MAX_FEE_BPS = 500; // Max 5%

    // Escrow states
    enum EscrowStatus {
        Created,      // Escrow created, awaiting funding
        Funded,       // Payment deposited
        Completed,    // Task done, payment released
        Disputed,     // Under dispute
        Refunded,     // Payment returned to client
        Cancelled     // Cancelled before funding
    }

    // Escrow deal
    struct Escrow {
        uint256 id;
        address client;           // Agent/person hiring (pays)
        address provider;         // Agent/person doing the work (gets paid)
        uint256 amount;           // Payment amount
        address token;            // ERC20 token (address(0) = ETH)
        string taskDescription;   // What needs to be done
        EscrowStatus status;
        uint256 createdAt;
        uint256 completedAt;
        uint256 deadline;         // Task deadline (0 = no deadline)
    }

    // All escrows
    uint256 public escrowCount;
    mapping(uint256 => Escrow) public escrows;

    // Track escrows per address
    mapping(address => uint256[]) public userEscrows;

    // Accumulated protocol fees
    mapping(address => uint256) public collectedFees; // token => amount

    // Events
    event EscrowCreated(
        uint256 indexed escrowId,
        address indexed client,
        address indexed provider,
        uint256 amount,
        address token
    );
    event EscrowFunded(uint256 indexed escrowId);
    event EscrowCompleted(uint256 indexed escrowId, uint256 protocolFee);
    event EscrowDisputed(uint256 indexed escrowId, address indexed by);
    event EscrowRefunded(uint256 indexed escrowId);
    event EscrowCancelled(uint256 indexed escrowId);
    event DisputeResolved(uint256 indexed escrowId, address indexed winner);
    event ProtocolFeeUpdated(uint256 newFeeBps);

    constructor() Ownable(msg.sender) {}

    // ============ Create & Fund ============

    /**
     * @notice Create a new escrow deal (ETH)
     * @param provider Address of the service provider (agent doing the work)
     * @param taskDescription Description of the task
     * @param deadline Task deadline timestamp (0 = no deadline)
     */
    function createEscrowETH(
        address provider,
        string calldata taskDescription,
        uint256 deadline
    ) external payable nonReentrant returns (uint256) {
        require(provider != address(0), "Invalid provider");
        require(provider != msg.sender, "Cannot escrow to yourself");
        require(msg.value > 0, "Must send ETH");
        require(bytes(taskDescription).length > 0, "Task description required");
        if (deadline > 0) {
            require(deadline > block.timestamp, "Deadline must be in the future");
        }

        escrowCount++;
        uint256 escrowId = escrowCount;

        escrows[escrowId] = Escrow({
            id: escrowId,
            client: msg.sender,
            provider: provider,
            amount: msg.value,
            token: address(0),
            taskDescription: taskDescription,
            status: EscrowStatus.Funded,
            createdAt: block.timestamp,
            completedAt: 0,
            deadline: deadline
        });

        userEscrows[msg.sender].push(escrowId);
        userEscrows[provider].push(escrowId);

        emit EscrowCreated(escrowId, msg.sender, provider, msg.value, address(0));
        emit EscrowFunded(escrowId);

        return escrowId;
    }

    /**
     * @notice Create a new escrow deal (ERC20)
     */
    function createEscrowToken(
        address provider,
        address token,
        uint256 amount,
        string calldata taskDescription,
        uint256 deadline
    ) external nonReentrant returns (uint256) {
        require(provider != address(0), "Invalid provider");
        require(provider != msg.sender, "Cannot escrow to yourself");
        require(token != address(0), "Use createEscrowETH for ETH");
        require(amount > 0, "Amount must be > 0");
        require(bytes(taskDescription).length > 0, "Task description required");
        if (deadline > 0) {
            require(deadline > block.timestamp, "Deadline must be in the future");
        }

        // Transfer tokens to escrow
        IERC20(token).safeTransferFrom(msg.sender, address(this), amount);

        escrowCount++;
        uint256 escrowId = escrowCount;

        escrows[escrowId] = Escrow({
            id: escrowId,
            client: msg.sender,
            provider: provider,
            amount: amount,
            token: token,
            taskDescription: taskDescription,
            status: EscrowStatus.Funded,
            createdAt: block.timestamp,
            completedAt: 0,
            deadline: deadline
        });

        userEscrows[msg.sender].push(escrowId);
        userEscrows[provider].push(escrowId);

        emit EscrowCreated(escrowId, msg.sender, provider, amount, token);
        emit EscrowFunded(escrowId);

        return escrowId;
    }

    // ============ Complete & Release ============

    /**
     * @notice Client approves task completion, releases payment to provider
     */
    function approveAndRelease(uint256 escrowId) external nonReentrant {
        Escrow storage e = escrows[escrowId];
        require(e.client == msg.sender, "Only client can approve");
        require(e.status == EscrowStatus.Funded, "Escrow not in funded state");

        // Calculate protocol fee
        uint256 fee = (e.amount * protocolFeeBps) / 10000;
        uint256 payout = e.amount - fee;

        e.status = EscrowStatus.Completed;
        e.completedAt = block.timestamp;

        // Track collected fees
        collectedFees[e.token] += fee;

        // Transfer payment to provider
        if (e.token == address(0)) {
            (bool success, ) = payable(e.provider).call{value: payout}("");
            require(success, "Transfer failed");
        } else {
            IERC20(e.token).safeTransfer(e.provider, payout);
        }

        emit EscrowCompleted(escrowId, fee);
    }

    // ============ Disputes ============

    /**
     * @notice Either party can raise a dispute
     */
    function raiseDispute(uint256 escrowId) external {
        Escrow storage e = escrows[escrowId];
        require(
            msg.sender == e.client || msg.sender == e.provider,
            "Not a party to this escrow"
        );
        require(e.status == EscrowStatus.Funded, "Cannot dispute in current state");

        e.status = EscrowStatus.Disputed;
        emit EscrowDisputed(escrowId, msg.sender);
    }

    /**
     * @notice Protocol owner resolves dispute (temporary — will be DAO later)
     * @param escrowId The escrow in dispute
     * @param winner Address to receive the funds (client or provider)
     */
    function resolveDispute(uint256 escrowId, address winner) external onlyOwner nonReentrant {
        Escrow storage e = escrows[escrowId];
        require(e.status == EscrowStatus.Disputed, "Not in disputed state");
        require(
            winner == e.client || winner == e.provider,
            "Winner must be client or provider"
        );

        if (winner == e.provider) {
            // Provider wins — release payment (with fee)
            uint256 fee = (e.amount * protocolFeeBps) / 10000;
            uint256 payout = e.amount - fee;
            collectedFees[e.token] += fee;

            e.status = EscrowStatus.Completed;
            e.completedAt = block.timestamp;

            if (e.token == address(0)) {
                (bool success, ) = payable(e.provider).call{value: payout}("");
            require(success, "Transfer failed");
            } else {
                IERC20(e.token).safeTransfer(e.provider, payout);
            }
        } else {
            // Client wins — refund
            e.status = EscrowStatus.Refunded;

            if (e.token == address(0)) {
                (bool success2, ) = payable(e.client).call{value: e.amount}("");
            require(success2, "Transfer failed");
            } else {
                IERC20(e.token).safeTransfer(e.client, e.amount);
            }
        }

        emit DisputeResolved(escrowId, winner);
    }

    // ============ Mutual Cancel ============

    // Track cancel requests: escrowId => address => agreed
    mapping(uint256 => mapping(address => bool)) public cancelAgreed;

    event MutualCancelRequested(uint256 indexed escrowId, address indexed by);
    event MutualCancelCompleted(uint256 indexed escrowId);

    /**
     * @notice Request mutual cancellation — both parties must agree
     */
    function requestMutualCancel(uint256 escrowId) external nonReentrant {
        Escrow storage e = escrows[escrowId];
        require(
            msg.sender == e.client || msg.sender == e.provider,
            "Not a party to this escrow"
        );
        require(e.status == EscrowStatus.Funded, "Escrow not in funded state");

        cancelAgreed[escrowId][msg.sender] = true;
        emit MutualCancelRequested(escrowId, msg.sender);

        // If both parties agreed, refund client
        if (cancelAgreed[escrowId][e.client] && cancelAgreed[escrowId][e.provider]) {
            e.status = EscrowStatus.Cancelled;

            if (e.token == address(0)) {
                (bool success, ) = payable(e.client).call{value: e.amount}("");
                require(success, "Refund failed");
            } else {
                IERC20(e.token).safeTransfer(e.client, e.amount);
            }

            emit MutualCancelCompleted(escrowId);
        }
    }

    // ============ Refund & Cancel ============

    /**
     * @notice Client requests refund (only if deadline passed and not completed)
     */
    function requestRefund(uint256 escrowId) external nonReentrant {
        Escrow storage e = escrows[escrowId];
        require(e.client == msg.sender, "Only client can request refund");
        require(e.status == EscrowStatus.Funded, "Escrow not in funded state");
        require(e.deadline > 0 && block.timestamp > e.deadline, "Deadline not passed");

        e.status = EscrowStatus.Refunded;

        if (e.token == address(0)) {
            (bool success2, ) = payable(e.client).call{value: e.amount}("");
            require(success2, "Transfer failed");
        } else {
            IERC20(e.token).safeTransfer(e.client, e.amount);
        }

        emit EscrowRefunded(escrowId);
    }

    // ============ Protocol Admin ============

    /**
     * @notice Update protocol fee
     */
    function setProtocolFee(uint256 newFeeBps) external onlyOwner {
        require(newFeeBps <= MAX_FEE_BPS, "Fee too high");
        protocolFeeBps = newFeeBps;
        emit ProtocolFeeUpdated(newFeeBps);
    }

    /**
     * @notice Withdraw collected protocol fees
     */
    function withdrawFees(address token) external onlyOwner nonReentrant {
        uint256 amount = collectedFees[token];
        require(amount > 0, "No fees to withdraw");
        collectedFees[token] = 0;

        if (token == address(0)) {
            (bool success, ) = payable(owner()).call{value: amount}("");
            require(success, "Transfer failed");
        } else {
            IERC20(token).safeTransfer(owner(), amount);
        }
    }

    // ============ View Functions ============

    /**
     * @notice Get escrow details
     */
    function getEscrow(uint256 escrowId) external view returns (Escrow memory) {
        return escrows[escrowId];
    }

    /**
     * @notice Get all escrow IDs for a user
     */
    function getUserEscrows(address user) external view returns (uint256[] memory) {
        return userEscrows[user];
    }

    /**
     * @notice Get total number of escrows
     */
    function totalEscrows() external view returns (uint256) {
        return escrowCount;
    }
}
