# AXON Protocol — Smart Contracts ⚡

On-chain identity, wallet, and escrow contracts for AI agents. Deployed on testnets only (Base Sepolia and BNB Smart Chain Testnet).

> ⚠️ **Testnet only. Not audited. Do not use with real funds.**

## 🟡 BNB Smart Chain Testnet (Chain ID 97)

| Contract | Address | Source (Sourcify, exact match) | BscScan |
|----------|---------|--------------------------------|---------|
| **AgentRegistry** | `0x4499d13b881dB471E81539FaE733cFAC766dcB31` | [Verified](https://repo.sourcify.dev/97/0x4499d13b881dB471E81539FaE733cFAC766dcB31) | [View](https://testnet.bscscan.com/address/0x4499d13b881dB471E81539FaE733cFAC766dcB31) |
| **WalletFactory** | `0x9259894EfD2C5876f9f33CDB3320bb07bc7f0EAD` | [Verified](https://repo.sourcify.dev/97/0x9259894EfD2C5876f9f33CDB3320bb07bc7f0EAD) | [View](https://testnet.bscscan.com/address/0x9259894EfD2C5876f9f33CDB3320bb07bc7f0EAD) |
| **SimpleEscrow** | `0x60cF39523B2988af21bD5E878c22dB765926B936` | [Verified](https://repo.sourcify.dev/97/0x60cF39523B2988af21bD5E878c22dB765926B936) | [View](https://testnet.bscscan.com/address/0x60cF39523B2988af21bD5E878c22dB765926B936) |

## 🟢 Base Sepolia Testnet (Chain ID 84532)

| Contract | Address | BaseScan |
|----------|---------|----------|
| **AgentRegistry** | `0xef3b578B594b88A256c20B76E2ca3A6Ee83eA118` | [View](https://sepolia.basescan.org/address/0xef3b578B594b88A256c20B76E2ca3A6Ee83eA118) |
| **WalletFactory** | `0x221480e12b99Ad24dBC30eaDA32070267b3F3973` | [View](https://sepolia.basescan.org/address/0x221480e12b99Ad24dBC30eaDA32070267b3F3973) |
| **SimpleEscrow** | `0x530326ABd9580Ff0739182424DacB5e6A344a511` | [View](https://sepolia.basescan.org/address/0x530326ABd9580Ff0739182424DacB5e6A344a511) |

`AgentWallet` has no standalone deployment: one instance is deployed per agent by `WalletFactory`.

> See [DEPLOYMENTS.md](./DEPLOYMENTS.md) for full deployment details.

## Contracts

| Contract | Description |
|----------|-------------|
| `AgentRegistry.sol` | ERC-721 (Enumerable) agent identity: register agents, set an operator address, update metadata, deactivate/reactivate. Reputation score and task count are set by the registry owner. |
| `AgentWallet.sol` | Owner-controlled smart contract wallet for an agent. The owner (human) sets limits; the operator (agent key) can send native coin or ERC-20 tokens only to whitelisted addresses, within per-transaction and daily limits. The owner can withdraw freely, change the operator, and freeze/unfreeze the wallet. |
| `WalletFactory.sol` | Deploys one `AgentWallet` per registered, active agent; callable only by the agent NFT owner. Default or custom limits. |
| `SimpleEscrow.sol` | Escrow for agent-to-agent payments (native coin or ERC-20). Client releases funds to the provider; 0.3% protocol fee on release (owner-adjustable, capped at 5%); mutual cancel; disputes resolved by the protocol owner for now. |

## What `AgentWallet` is (and isn't)

`AgentWallet` is a plain owner/operator contract wallet with spending controls. It is **not** an ERC-4337 account: there is no `EntryPoint` integration, no `validateUserOp`, no bundler/paymaster support. Transactions are sent directly by the operator or owner EOA, which pays its own gas.

Spending controls:
- Per-transaction limit and daily limit for native coin (daily counter resets 24h after the last reset)
- Per-token daily limits for ERC-20s
- Recipient whitelist for all operator sends
- Emergency freeze (blocks operator sends; owner withdrawals still work)

## Tech Stack

- **Solidity:** 0.8.28 (evmVersion: cancun, optimizer enabled, 200 runs)
- **OpenZeppelin:** v5.1.0+
- **Framework:** Hardhat
- **Networks:** Base Sepolia, BNB Smart Chain Testnet

## Key Features

- 🆔 **ERC-721 Agent IDs** — one NFT identity per agent, with an operator address and metadata URI
- 💰 **Agent wallets with spending limits** — owner-controlled, whitelist + per-tx/daily limits, emergency freeze
- 🤝 **Escrow** — funds locked until the client releases; mutual cancel; owner-resolved disputes (planned to move to governance)
- ⭐ **Reputation field** — on-chain score per agent, currently updated by the registry owner
- 🔒 **Safety basics** — `call{value:}()` instead of `transfer()`, OpenZeppelin `ReentrancyGuard` and `SafeERC20`, access control

## Quick Start

```bash
npm install
npx hardhat compile
# create .env with PRIVATE_KEY=<testnet key>  (gitignored — never commit it)
npx hardhat run scripts/deploy.js --network bscTestnet   # or baseSepolia
```

## Repository Layout

- `contracts/` — Solidity sources
- `scripts/` — deployment script
- `integrations/` — agent framework integrations

## Status

🧪 **Testnet only** — deployed on Base Sepolia and BSC Testnet. Not audited, not for production use. Admin roles (registry owner, escrow owner) are currently a single key.

## Links

- 🌐 Website: [axonprotocol.site](https://axonprotocol.site)
- 📄 Litepaper: [axonprotocol.site/litepaper.html](https://axonprotocol.site/litepaper.html)
- 🐦 X/Twitter: [@Axon_Protocol](https://x.com/Axon_Protocol)
- 📱 Telegram: [@axon_protocol](https://t.me/axon_protocol)
- 🪂 Airdrop: [Zealy](https://zealy.io/cw/axonprotocol)

---

Built with ⚡ by AXON Protocol
