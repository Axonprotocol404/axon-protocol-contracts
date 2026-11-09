// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {AgentRegistry} from "./AgentRegistry.sol";
import {SimpleEscrow} from "./SimpleEscrow.sol";

/// @title AXON BondingCurve — curve-based launchpad, identity-gated dev (FEVM port)
/// @notice Linear curve price = basePrice + slope*(supply/1e18). Dev must be registered.
///         Buy with native currency mints/pulls tokens from dev float; sell returns native.
contract BondingCurve {
    struct Curve {
        uint256 id;
        address dev;
        uint256 agentId;
        address token;
        uint256 basePrice;     // wei at supply=0
        uint256 slope;         // wei added per 1e18 supply
        uint256 supply;        // tokens currently sold (18d)
        uint256 maxSupply;
        uint256 feeBps;
        bool    finalized;
    }

    uint256 public nextCurveId = 1;
    address public admin;
    AgentRegistry public registry;
    SimpleEscrow public escrow;

    mapping(uint256 => Curve) public curves;

    event CurveCreated(uint256 indexed id, address indexed dev, address token, uint256 basePrice, uint256 slope);
    event Bought(uint256 indexed id, address indexed buyer, uint256 tokens, uint256 cost);
    event Sold(uint256 indexed id, address indexed seller, uint256 tokens, uint256 refund);
    event Finalized(uint256 indexed id, uint256 devAmount, uint256 feeAmount);

    error NotRegistered();
    error NotDev();
    error BadState();
    error BadAmount();
    error TooSmall();
    error NotAdmin();
    error Slippage();
    error BelowMin();

    modifier onlyAdmin() { if (msg.sender != admin) revert NotAdmin(); _; }

    constructor(address _registry, address _escrow) { admin = msg.sender; registry = AgentRegistry(_registry); escrow = SimpleEscrow(_escrow); }

    function setAdmin(address a) external onlyAdmin { admin = a; }

    function currentPrice(uint256 id) public view returns (uint256) {
        Curve storage c = curves[id];
        return c.basePrice + (c.slope * (c.supply / 1e18)) / 1;
    }

    function buyQuote(uint256 id, uint256 tokenAmount) public view returns (uint256 cost) {
        Curve storage c = curves[id];
        // integral of linear price over [supply, supply+tokenAmount]
        // price(s) = basePrice + slope * (s / 1e18)
        // cost = tokenAmount*basePrice + slope * ((s2^2 - s1^2) / 2e18)  [in wei, tokenAmount in 18d]
        uint256 s1 = c.supply;
        uint256 s2 = s1 + tokenAmount;
        cost = (tokenAmount * c.basePrice) / 1e18;
        cost += (c.slope * ((s2 * s2 - s1 * s1) / 1e18)) / 2e18;
    }

    function sellQuote(uint256 id, uint256 tokenAmount) public view returns (uint256 refundAmount) {
        Curve storage c = curves[id];
        uint256 s1 = c.supply;
        uint256 s2 = s1 - tokenAmount; // selling decreases supply
        refundAmount = (tokenAmount * c.basePrice) / 1e18;
        refundAmount += (c.slope * ((s1 * s1 - s2 * s2) / 1e18)) / 2e18;
    }

    function createCurve(address token, uint256 agentId, uint256 basePrice, uint256 slope, uint256 maxSupply, uint256 feeBps) external returns (uint256 id) {
        if (!registry.isActive(agentId)) revert NotRegistered();
        id = nextCurveId++;
        curves[id] = Curve({ id:id, dev:msg.sender, agentId:agentId, token:token, basePrice:basePrice, slope:slope, supply:0, maxSupply:maxSupply, feeBps:feeBps, finalized:false });
        emit CurveCreated(id, msg.sender, token, basePrice, slope);
    }

    /// @notice Buy: pay native; dev must have pre-approved/approved tokens to this contract (float).
    function buy(uint256 id, uint256 tokenAmount, uint256 maxCost) external payable {
        Curve storage c = curves[id];
        if (c.dev == address(0) || c.finalized) revert BadState();
        if (c.supply + tokenAmount > c.maxSupply) revert BadAmount();
        uint256 cost = buyQuote(id, tokenAmount);
        if (cost > maxCost) revert Slippage();
        if (msg.value < cost) revert BelowMin();
        if (msg.value > cost) {
            (bool back, ) = msg.sender.call{value: msg.value - cost}("");
            require(back, "refund failed");
        }
        c.supply += tokenAmount;
        (bool okT, bytes memory data) = c.token.call(abi.encodeWithSignature("transferFrom(address,address,uint256)", c.dev, msg.sender, tokenAmount));
        require(okT && (data.length == 0 || abi.decode(data, (bool))), "token transfer failed");
        emit Bought(id, msg.sender, tokenAmount, cost);
    }

    /// @notice Sell tokens back: send tokens first (approve), then sell; refund = quote minus fee.
    function sell(uint256 id, uint256 tokenAmount, uint256 minRefund) external {
        Curve storage c = curves[id];
        if (c.dev == address(0) || c.finalized) revert BadState();
        if (c.supply < tokenAmount) revert BadAmount();
        uint256 gross = sellQuote(id, tokenAmount);
        uint256 fee = (gross * c.feeBps) / 10_000;
        uint256 net = gross - fee;
        if (net < minRefund) revert Slippage();
        // pull tokens from seller & return to dev float
        (bool okT, bytes memory data) = c.token.call(abi.encodeWithSignature("transferFrom(address,address,uint256)", msg.sender, c.dev, tokenAmount));
        require(okT && (data.length == 0 || abi.decode(data, (bool))), "token transfer failed");
        c.supply -= tokenAmount;
        (bool ok, ) = msg.sender.call{value: net}("");
        require(ok, "sell failed");
        (bool okD, ) = admin.call{value: fee}("");
        require(okD, "fee failed");
        emit Sold(id, msg.sender, tokenAmount, net);
    }

    function finalize(uint256 id) external {
        Curve storage c = curves[id];
        if (c.dev == address(0)) revert BadState();
        if (c.finalized) revert BadState();
        if (msg.sender != c.dev && msg.sender != admin) revert NotDev();
        c.finalized = true;
        // remaining native balance (float from buys) minus fee to dev
        uint256 bal = address(this).balance;
        uint256 fee = (bal * c.feeBps) / 10_000;
        (bool okD, ) = admin.call{value: fee}("");
        require(okD, "fee failed");
        (bool ok, ) = payable(c.dev).call{value: bal - fee}("");
        require(ok, "dev withdraw failed");
        emit Finalized(id, bal - fee, fee);
    }

    function getCurve(uint256 id) external view returns (Curve memory) { return curves[id]; }
}
