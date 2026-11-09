# AXON FEVM Port (Solidity)

Filecoin Virtual Machine port of the AXON trust layer — originally built in Rust/Soroban on Stellar (see `soroban/`).

## Contracts
- `AgentRegistry.sol` — on-chain agent identity, status, reputation (self-rating blocked), task history
- `AgentWallet.sol` — cap-guarded operational wallet: agent hot key can only spend `spendCap` per `window`
- `SimpleEscrow.sol` — locked task payments; release records a completed task in the registry
- `TokenSale.sol` — fixed-price launchpad, identity-gated (dev must be a registered active agent), hard cap + 1.5% fee
- `BondingCurve.sol` — linear bonding-curve launchpad, identity-gated, buy/sell quotes with slippage guards

## Tests
8 Foundry tests covering identity gating, wallet caps, escrow full flow, reputation, both launchpad flows, and over-cap reverts.

```bash
forge install
forge test
```

Same security patterns as the Soroban originals: identity-gating, cap guards, escrow-before-payout.
