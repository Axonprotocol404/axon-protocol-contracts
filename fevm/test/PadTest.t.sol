// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../src/mocks/MockERC20.sol";
import {AgentRegistry} from "../src/AgentRegistry.sol";
import {SimpleEscrow} from "../src/SimpleEscrow.sol";
import {TokenSale} from "../src/TokenSale.sol";
import {BondingCurve} from "../src/BondingCurve.sol";

contract PadTest is Test {
    AgentRegistry registry;
    SimpleEscrow escrow;
    TokenSale sale;
    BondingCurve curve;
    MockERC20 token;

    address dev = makeAddr("dev");
    address buyer1 = makeAddr("buyer1");
    address buyer2 = makeAddr("buyer2");

    function setUp() public {
        registry = new AgentRegistry();
        escrow = new SimpleEscrow(address(registry));
        sale = new TokenSale(address(registry));
        sale.setAdmin(makeAddr("admin"));
        curve = new BondingCurve(address(registry), address(escrow));
        curve.setAdmin(makeAddr("admin"));
        token = new MockERC20("AXON Demo", "DEMO");
        vm.prank(dev);
        registry.registerAgent("axon-dev", "ipfs://dev");
        // dev funds tokens to be sold
        vm.prank(dev);
        token.mint(dev, 1_000_000 ether);
        vm.prank(dev);
        token.approve(address(sale), type(uint256).max);
        vm.prank(dev);
        token.approve(address(curve), type(uint256).max);
    }

    function test_fixed_price_sale_flow() public {
        uint256 agentId = registry.agentIdByOwner(dev);
        vm.prank(dev);
        uint256 s = sale.createSale(address(token), agentId, 0.001 ether, 10 ether, 7 days);
        vm.deal(buyer1, 5 ether);
        vm.prank(buyer1);
        sale.buy{value: 2 ether}(s, 2 ether);
        assertEq(token.balanceOf(buyer1), 2000 ether);
        TokenSale.Sale memory saleData = sale.getSale(s);
        assertEq(saleData.raised, 2 ether);
        // finalize
        vm.prank(dev);
        sale.finalize(s);
        assertEq(dev.balance, 2 ether - ((2 ether * 150) / 10000));
    }

    function test_sale_identity_gate_and_cap() public {
        // unregistered dev blocked
        vm.prank(address(0xBAD));
        vm.expectRevert(TokenSale.NotRegistered.selector);
        sale.createSale(address(token), 42, 0.001 ether, 10 ether, 7 days);
        // over hard cap blocked
        uint256 agentId = registry.agentIdByOwner(dev);
        vm.prank(dev);
        uint256 s = sale.createSale(address(token), agentId, 0.001 ether, 1 ether, 7 days);
        vm.deal(buyer1, 10 ether);
        vm.prank(buyer1);
        vm.expectRevert(TokenSale.OverCap.selector);
        sale.buy{value: 2 ether}(s, 2 ether);
    }

    function test_bonding_curve_flow() public {
        uint256 agentId = registry.agentIdByOwner(dev);
        vm.prank(dev);
        uint256 c = curve.createCurve(address(token), agentId, 0.0001 ether, 0.000001 ether, 1000 ether, 150);
        vm.deal(buyer1, 10 ether);
        uint256 cost = curve.buyQuote(c, 100 ether);
        vm.prank(buyer1);
        curve.buy{value: cost}(c, 100 ether, cost);
        assertEq(token.balanceOf(buyer1), 100 ether);
        // price should have increased
        assertTrue(curve.currentPrice(c) > 0.0001 ether);
        // sell back
        vm.prank(buyer1);
        token.approve(address(curve), type(uint256).max);
        vm.deal(address(curve), 10 ether); // fund contract float
        uint256 refund0 = curve.sellQuote(c, 50 ether);
        vm.prank(buyer1);
        curve.sell(c, 50 ether, 0);
        assertTrue(buyer1.balance > 0);
        assertEq(token.balanceOf(buyer1), 50 ether);
    }

    function test_curve_identity_gate() public {
        vm.prank(address(0xBAD));
        vm.expectRevert(BondingCurve.NotRegistered.selector);
        curve.createCurve(address(token), 99, 0.0001 ether, 0.000001 ether, 1000 ether, 150);
    }
}
