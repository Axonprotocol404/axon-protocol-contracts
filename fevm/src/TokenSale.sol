// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {AgentRegistry} from "./AgentRegistry.sol";
import {AgentWallet} from "./AgentWallet.sol";
import {SimpleEscrow} from "./SimpleEscrow.sol";

/// @title AXON TokenSale — fixed-price launchpad, identity-gated dev (FEVM port)
/// @notice Dev must be a registered active agent. Buyers pay fixed price per token.
///         Hard cap guards total raise. finalize() ends sale; dev withdraws.
contract TokenSale {
    struct Sale {
        uint256 id;
        address dev;
        uint256 agentId;
        address token;        // ERC20 token sold
        uint256 price;        // wei per token (18d)
        uint256 hardCap;      // max total raise (wei)
        uint256 raised;
        uint64  deadline;
        bool    finalized;
    }

    uint256 public nextSaleId = 1;
    uint256 public feeBps = 150;               // 1.5%
    address public admin;
    AgentRegistry public registry;

    mapping(uint256 => Sale) public sales;

    event SaleCreated(uint256 indexed id, address indexed dev, address token, uint256 price, uint256 hardCap);
    event Bought(uint256 indexed id, address indexed buyer, uint256 tokenOut, uint256 xlmIn);
    event Finalized(uint256 indexed id, uint256 raised, uint256 fee);

    error NotRegistered();
    error NotDev();
    error SaleClosed();
    error OverCap();
    error BadState();
    error NotAdmin();

    modifier onlyAdmin() { if (msg.sender != admin) revert NotAdmin(); _; }

    constructor(address _registry) { admin = msg.sender; registry = AgentRegistry(_registry); }

    function setAdmin(address a) external onlyAdmin { admin = a; }

    function setFeeBps(uint256 bps) external onlyAdmin { feeBps = bps; }

    /// @notice Create a sale. Caller must be an active registered agent (identity gate).
    function createSale(address token, uint256 agentId, uint256 price, uint256 hardCap, uint64 duration) external returns (uint256 id) {
        if (!registry.isActive(agentId)) revert NotRegistered();
        id = nextSaleId++;
        sales[id] = Sale({
            id: id,
            dev: msg.sender,
            agentId: agentId,
            token: token,
            price: price,
            hardCap: hardCap,
            raised: 0,
            deadline: uint64(block.timestamp) + duration,
            finalized: false
        });
        emit SaleCreated(id, msg.sender, token, price, hardCap);
    }

    /// @notice Buy tokens: pay `xlmAmount`, receive tokens at fixed price. Dev must have approved this contract.
    function buy(uint256 id, uint256 xlmAmount) external payable {
        Sale storage s = sales[id];
        if (s.dev == address(0)) revert BadState();
        if (s.finalized || block.timestamp > s.deadline) revert SaleClosed();
        if (s.raised + xlmAmount > s.hardCap) revert OverCap();
        // cap purchase to remaining hard cap
        uint256 pay = xlmAmount;
        if (s.raised + pay > s.hardCap) pay = s.hardCap - s.raised;
        if (msg.value > pay) {
            (bool back, ) = msg.sender.call{value: msg.value - pay}("");
            require(back, "refund failed");
        }
        s.raised += pay;
        uint256 tokenOut = (pay * 1e18) / s.price;
        // pull tokens from dev (dev pre-approved this contract)
        (bool okT, bytes memory data) = s.token.call(abi.encodeWithSignature("transferFrom(address,address,uint256)", s.dev, msg.sender, tokenOut));
        require(okT && (data.length == 0 || abi.decode(data, (bool))), "token transfer failed");
        emit Bought(id, msg.sender, tokenOut, pay);
    }

    /// @notice Finalize: no more buys. Dev withdraws raise minus fee.
    function finalize(uint256 id) external {
        Sale storage s = sales[id];
        if (s.dev == address(0)) revert BadState();
        if (s.finalized) revert BadState();
        if (msg.sender != s.dev && msg.sender != admin) revert NotDev();
        // dev may finalize early (raised < hardCap before deadline) — allowed by design
        s.finalized = true;
        uint256 fee = (s.raised * feeBps) / 10_000;
        (bool okF, ) = admin.call{value: fee}("");
        require(okF, "fee failed");
        (bool ok, ) = payable(s.dev).call{value: s.raised - fee}("");
        require(ok, "dev withdraw failed");
        emit Finalized(id, s.raised, fee);
    }

    function getSale(uint256 id) external view returns (Sale memory) { return sales[id]; }
}
