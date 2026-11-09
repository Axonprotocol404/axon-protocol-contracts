// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import {AgentRegistry} from "../src/AgentRegistry.sol";
import {AgentWallet} from "../src/AgentWallet.sol";
import {SimpleEscrow} from "../src/SimpleEscrow.sol";

contract CoreTest is Test {
    AgentRegistry registry;
    AgentWallet wallet;
    SimpleEscrow escrow;

    address owner = makeAddr("alice");
    address agent = makeAddr("agent");
    address client = makeAddr("client");

    function setUp() public {
        registry = new AgentRegistry();
        // register agent (owner registers, agent hot key separate)
        vm.prank(owner);
        uint256 id = registry.registerAgent("axon-demo", "ipfs://demo");
        vm.prank(owner);
        wallet = new AgentWallet(address(registry), id, agent, 1 ether, 1 days);
        escrow = new SimpleEscrow(address(registry));
    }

    function test_register_and_identity_gate() public {
        AgentRegistry.Agent memory a = registry.getAgent(1);
        assertEq(a.id, 1);
        assertEq(a.owner, owner);
        assertEq(a.status, 1);
        assertTrue(registry.isActive(1));

        vm.expectRevert(SimpleEscrow.NotRegistered.selector);
        escrow.open{value: 1 ether}(agent, 99, 1 days); // unknown agent id
    }

    function test_wallet_cap_enforced() public {
        vm.deal(address(wallet), 10 ether);
        // agent can spend within cap
        vm.prank(agent);
        wallet.spend(payable(address(0xD1)), 1 ether);
        // agent cannot exceed cap in same window
        vm.prank(agent);
        vm.expectRevert(AgentWallet.CapExceeded.selector);
        wallet.spend(payable(address(0xD1)), 0.1 ether);
        // owner can always spend
        vm.prank(owner);
        wallet.spend(payable(address(0xD1)), 5 ether);
        // new window resets
        vm.warp(block.timestamp + 2 days);
        vm.prank(agent);
        wallet.spend(payable(address(0xD1)), 1 ether);
    }

    function test_escrow_full_flow() public {
        uint256 id = registry.agentIdByOwner(owner);
        vm.deal(client, 2 ether);
        vm.prank(client);
        uint256 e = escrow.open{value: 1 ether}(agent, id, 1 days);
        assertEq(address(escrow).balance, 1 ether);

        // release pays agent + records task
        vm.prank(client);
        escrow.release(e);
        assertEq(agent.balance, 1 ether);
        AgentRegistry.Agent memory a2 = registry.getAgent(id);
        assertTrue(a2.lastTaskAt > 0);
        assertEq(uint256(escrow.getEscrow(e).state), uint256(SimpleEscrow.State.Released));

        // second escrow: refund after timeout
        vm.prank(client);
        uint256 e2 = escrow.open{value: 0.5 ether}(agent, id, 1 days);
        vm.warp(block.timestamp + 2 days);
        vm.prank(client);
        escrow.refund(e2);
        assertEq(uint256(escrow.getEscrow(e2).state), uint256(SimpleEscrow.State.Refunded));
    }

    function test_reputation() public {
        uint256 id = registry.agentIdByOwner(owner);
        vm.prank(address(0xE1));
        registry.rate(id, 80);
        vm.prank(address(0xE2));
        registry.rate(id, -20);
        assertEq(registry.averageRating(id), 30);
        // self-rating blocked
        vm.prank(owner);
        vm.expectRevert(AgentRegistry.SelfRating.selector);
        registry.rate(id, 100);
    }
}
