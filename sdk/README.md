# AXON SDK — JS client for Stellar (Soroban)

JS SDK for the AXON Protocol: agent identity, cap-guarded wallets & escrow on Stellar.

> ⚠️ Work in progress. Not audited. Testnet for now.

## Install

```bash
npm install @stellar/stellar-sdk   # peer dependency
# then import src/index.js directly (bundlers/ESM)
```

## Quick start

```js
import { AxonClient, TESTNET_CONTRACTS } from "./src/index.js";
import { Keypair } from "@stellar/stellar-sdk";

const axon = new AxonClient({ network: "testnet" });

// Read: verify an operator is an active registered agent (gate for your app)
const agentId = await axon.isActiveOperator("G...");
console.log("operator is agent #", agentId);

// Read: agent info
const agent = await axon.getAgent(1);

// Write: register a new agent (needs owner keypair)
const owner = Keypair.fromSecret("S...");
await axon.registerAgent({
  ownerKeypair: owner,
  operator: "G...",
  metadataUri: "ipfs://my-agent-meta",
});

// Escrow: open / release / cancel
await axon.openEscrow({ clientKeypair, provider: "G...", token: "CAS3J...", amount: 1_0000000n });
await axon.releaseEscrow({ clientKeypair, escrowId: 1 });
```

## Deployed contracts (testnet)

See [STELLAR_DEPLOYMENTS.md](../STELLAR_DEPLOYMENTS.md).
