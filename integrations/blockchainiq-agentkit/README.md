# axon-blockchainiq-agentkit

A [Coinbase AgentKit](https://github.com/coinbase/agentkit) **action provider** for the [Blockchain IQ](https://www.blockchainiq.co/agents) x402 crypto data API. Your agent pays per call in **USDC on Base** with x402 v2, signed by the AgentKit wallet provider it already uses — no account, no API key. Built by [AXON Protocol](https://axonprotocol.site).

| Action | Endpoint | Price |
|--------|----------|-------|
| `blockchainiq_market_brief` | `GET /api/x402/v1/brief` | $0.01 |
| `blockchainiq_event_risk` | `GET /api/x402/v1/event-risk?days&minImportance` | $0.05 |
| `blockchainiq_free_sample` | `GET /api/x402/v1/sample` | free |

## Install

```bash
git clone https://github.com/Axonprotocol404/axon-protocol-contracts
cd axon-protocol-contracts/integrations/blockchainiq-agentkit
npm install
```

`.npmrc` sets `ignore-scripts=true`: AgentKit's optional native addons (`bufferutil`, `bigint-buffer`) fall back to pure JS, so install works on machines without a C toolchain.

## Quick check (no funds)

```bash
npm run check
```

Registers the provider in `AgentKit`, lists its three actions, runs the free sample, and confirms `/brief` answers HTTP 402 with x402 v2 terms on `eip155:8453`.

## One paid call ($0.01)

Use a **dedicated throwaway wallet** holding a few cents of USDC on Base. Settlement is gasless for the payer.

```bash
AGENT_PRIVATE_KEY=0x... npm run paid
```

Builds a `ViemWalletProvider` on Base, invokes `blockchainiq_market_brief` through `AgentKit.getActions()`, and prints the brief plus the settlement proof (payer, network, transaction hash) decoded from the `PAYMENT-RESPONSE` header.

## Use in your agent

```js
import { AgentKit } from "@coinbase/agentkit";
import { blockchainIQActionProvider } from "axon-blockchainiq-agentkit";

const agentkit = await AgentKit.from({
  walletProvider, // any EVM wallet provider on Base mainnet: CDP, Viem, Privy, ...
  actionProviders: [
    blockchainIQActionProvider({
      maxUsdPerCall: 0.06,
      onSettlement: (s, path) => console.log("paid", path, s.transaction),
    }),
  ],
});
// then hand agentkit to @coinbase/agentkit-langchain, -vercel-ai-sdk, or your own loop
```

`supportsNetwork` returns true only for Base mainnet (chain id 8453), where Blockchain IQ settles.

## Safety

- `maxUsdPerCall` (default $0.10) is enforced by x402 `spendControls` before anything is signed.
- Never use a main wallet; keep only a few cents in the agent wallet.
- The key never leaves your process; it only signs the EIP-3009 USDC authorization the server asks for.
- The examples ignore a known AgentKit quirk: its telemetry POST can reject unhandled and would otherwise crash the process.

Market data and analysis from Blockchain IQ, not investment advice. MIT licensed.
