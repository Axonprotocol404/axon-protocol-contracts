# AXON Protocol — Smart Contracts ⚡

On-chain identity, wallet, and escrow infrastructure for AI agents. Built on Base (Ethereum L2).

## 🟢 Live on Base Sepolia Testnet

| Contract | Address | BaseScan |
|----------|---------|----------|
| **AgentRegistry** | `0xef3b578B...83eA118` | [View](https://sepolia.basescan.org/address/0xef3b578B594b88A256c20B76E2ca3A6Ee83eA118) |
| **WalletFactory** | `0x221480e1...3F3973` | [View](https://sepolia.basescan.org/address/0x221480e12b99Ad24dBC30eaDA32070267b3F3973) |
| **SimpleEscrow** | `0x530326AB...4a511` | [View](https://sepolia.basescan.org/address/0x530326ABd9580Ff0739182424DacB5e6A344a511) |

> See [DEPLOYMENTS.md](./DEPLOYMENTS.md) for full deployment details.

## Contracts

| Contract | Description |
|----------|-------------|
| `AgentRegistry.sol` | ERC-721 agent identity — register, verify, and track AI agents on-chain |
| `AgentWallet.sol` | Smart wallet with daily limits, operator permissions, and emergency freeze |
| `WalletFactory.sol` | Auto-deploys wallets per registered agent |
| `SimpleEscrow.sol` | Trustless escrow for agent-to-agent payments with mutual cancel |

## Tech Stack

- **Solidity:** ^0.8.28 (evmVersion: cancun)
- **OpenZeppelin:** v5.1.0+
- **Blockchain:** Base (Ethereum L2)
- **Framework:** Hardhat

## Key Features

- 🆔 **ERC-721 Agent IDs** — Unique, tradable, composable identities
- 💰 **ERC-4337 Smart Wallets** — Daily limits, operator access, emergency freeze
- 🤝 **Trustless Escrow** — Lock funds until delivery confirmed, 0.3% protocol fee
- ⭐ **On-chain Reputation** — Score based on completed escrows
- 🔒 **Security** — `call{value:}()` over `transfer()`, reentrancy guards, access control

## Quick Start

```bash
npm install
npx hardhat compile
npx hardhat run scripts/deploy.js --network baseSepolia
```

## Status

🧪 **Testnet Live** — Deployed on Base Sepolia. Not audited, not for production use yet.

## Links

- 🌐 Website: [axonprotocol.site](https://axonprotocol.site)
- 📄 Litepaper: [axonprotocol.site/litepaper.html](https://axonprotocol.site/litepaper.html)
- 🐦 X/Twitter: [@Axon_Protocol](https://x.com/Axon_Protocol)
- 📱 Telegram: [@axon_protocol](https://t.me/axon_protocol)
- 🪂 Airdrop: [Zealy](https://zealy.io/cw/axonprotocol)

---

Built with ⚡ by AXON Protocol
