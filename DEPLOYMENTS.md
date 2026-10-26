# AXON Protocol — Deployed Contracts

## Base Sepolia Testnet (Chain ID: 84532)

Deployed: March 14, 2026

| Contract | Address | BaseScan |
|----------|---------|----------|
| **AgentRegistry** | `0xef3b578B594b88A256c20B76E2ca3A6Ee83eA118` | [View](https://sepolia.basescan.org/address/0xef3b578B594b88A256c20B76E2ca3A6Ee83eA118) |
| **WalletFactory** | `0x221480e12b99Ad24dBC30eaDA32070267b3F3973` | [View](https://sepolia.basescan.org/address/0x221480e12b99Ad24dBC30eaDA32070267b3F3973) |
| **SimpleEscrow** | `0x530326ABd9580Ff0739182424DacB5e6A344a511` | [View](https://sepolia.basescan.org/address/0x530326ABd9580Ff0739182424DacB5e6A344a511) |

### Deployment Details

- **Network:** Base Sepolia (Ethereum L2 Testnet)
- **RPC:** `https://sepolia.base.org`
- **Solidity:** `0.8.28` (evmVersion: cancun)
- **OpenZeppelin:** `5.1.0`
- **Deployer:** `0x211da85c9Bc01c132F81F0AF24D1a56eA0cA9e4b`
- **Gas used:** ~0.00004 ETH

## BSC Testnet (Chain ID 97)

Deployed: October 26, 2026

| Contract | Address | BscScan |
|----------|---------|---------|
| **AgentRegistry** | `0x4499d13b881dB471E81539FaE733cFAC766dcB31` | [View](https://testnet.bscscan.com/address/0x4499d13b881dB471E81539FaE733cFAC766dcB31) |
| **WalletFactory** | `0x9259894EfD2C5876f9f33CDB3320bb07bc7f0EAD` | [View](https://testnet.bscscan.com/address/0x9259894EfD2C5876f9f33CDB3320bb07bc7f0EAD) |
| **SimpleEscrow** | `0x60cF39523B2988af21bD5E878c22dB765926B936` | [View](https://testnet.bscscan.com/address/0x60cF39523B2988af21bD5E878c22dB765926B936) |

`AgentWallet` instances are deployed per agent by `WalletFactory` (no standalone deployment).

### Deployment Details

- **Network:** BNB Smart Chain Testnet
- **RPC:** `https://bsc-testnet-rpc.publicnode.com` (official alt: `https://data-seed-prebsc-1-s1.bnbchain.org:8545`)
- **Solidity:** `0.8.28` (evmVersion: cancun — Cancun opcodes TSTORE/TLOAD/MCOPY/BLOBBASEFEE confirmed on chain 97)
- **OpenZeppelin:** `5.1.0`
- **Deployer / owner:** `0x7DAf9a161c5cD7319249A081c5AeFdD62FbB6AfC` (testnet-only key)
- **Gas used:** ~0.0026 tBNB (gas price 0.1 gwei)
- **Sanity check:** `AgentRegistry.name()` = "AXON Agent ID", `WalletFactory.registry()` = AgentRegistry address, `totalAgents()` = 0, `totalEscrows()` = 0
- **Compiler settings:** solc `0.8.28+commit.7893614a`, optimizer enabled (200 runs), evmVersion `cancun`
- **Constructor args:** `AgentRegistry()` none · `WalletFactory(0x4499d13b881dB471E81539FaE733cFAC766dcB31)` · `SimpleEscrow()` none
- **Source verification:** Sourcify **exact match** (creation + runtime bytecode) for all three:
  - [AgentRegistry](https://repo.sourcify.dev/97/0x4499d13b881dB471E81539FaE733cFAC766dcB31)
  - [WalletFactory](https://repo.sourcify.dev/97/0x9259894EfD2C5876f9f33CDB3320bb07bc7f0EAD)
  - [SimpleEscrow](https://repo.sourcify.dev/97/0x60cF39523B2988af21bD5E878c22dB765926B936)
- **BscScan verification:** not done (requires an API key)

## Mainnet

Not deployed. Will only be deployed after a security audit.
