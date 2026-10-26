# axon-blockchainiq-langchain

LangChain tools for the [Blockchain IQ](https://www.blockchainiq.co/agents) x402 crypto data API. Your agent pays per call in **USDC on Base** using x402 v2 — no account, no API key. Built by [AXON Protocol](https://axonprotocol.site).

| Tool | Endpoint | Price |
|------|----------|-------|
| `blockchainiq_market_brief` | `GET /api/x402/v1/brief` | $0.01 |
| `blockchainiq_event_risk` | `GET /api/x402/v1/event-risk?days&minImportance` | $0.05 |
| `blockchainiq_free_sample` | `GET /api/x402/v1/sample` | free |

## Install

```bash
git clone https://github.com/Axonprotocol404/axon-blockchainiq-langchain && cd axon-blockchainiq-langchain
npm install
```

## Quick check (no wallet)

```bash
npm run check   # free sample + confirms /brief answers HTTP 402 with x402 v2 terms
```

## One paid call ($0.01)

Use a **dedicated throwaway wallet** holding a few cents of USDC on Base. Settlement is gasless for the payer.

```bash
AGENT_PRIVATE_KEY=0x... npm run paid
```

It prints the market brief and the settlement proof (payer, network, transaction hash) decoded from the `PAYMENT-RESPONSE` header. A per-call spend cap (`maxUsdPerCall`, default $0.10) refuses anything dearer.

## Use in your agent

```js
import { createBlockchainIQTools } from "axon-blockchainiq-langchain";

const tools = createBlockchainIQTools({
  privateKey: process.env.AGENT_PRIVATE_KEY,
  maxUsdPerCall: 0.06,
  onSettlement: (s, path) => console.log("paid", path, s.transaction),
});

const llm = new ChatOpenAI({ model: "gpt-4o-mini" }).bindTools(tools);
```

See `examples/agent.js` for a full tool-calling loop.

## Security

- Never use a main wallet. Keep only a few cents in the agent wallet.
- The private key never leaves your process; it only signs the EIP-3009 USDC authorization the x402 server asks for.

Market data and analysis from Blockchain IQ, not investment advice. MIT licensed.
