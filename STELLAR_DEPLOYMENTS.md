# AXON Protocol — Stellar Testnet Deployments (Soroban)

Network: Stellar Testnet | Deployer: `GDI3YMIP6AD2AWOYFPBREESK4EL77DODIGDVO4SMVT2M5PIOEDSSUXGY`
Date: 2026-11-03 | stellar-cli 30.0.1 | soroban-sdk 22

| Contract | Address (testnet C...) | Wasm hash |
|---|---|---|
| **AgentRegistry** | `CDLTKHIAAQPMJMGRGCUMVDSVWO672OSW3KEHUPEV5GFN3UXVCMIAHTFN` | `243a4e9e...` |
| **AgentWallet** | `CB3RBWCQIC3WTEDMDAWQMVZGUBP7SP53HXF5RKI55USVCA4PB765WHMM` | `49e31081...` |
| **SimpleEscrow** | `CBT52U22SJW357F6Y3FGP3GY7IA4BT3XNLUKFSRAPEC732F425ALRL6L` | `85699d2c...` |

## Verified on-chain
- Registry initialized (admin = deployer)
- Escrow initialized (admin = deployer, fee 30 bps = 0.3%)
- Demo agent registered: agent id 1, active, tx `444bfa6dbbfbf90d3e459732ac59e60039ae40ba0d3196de2c52030282a5344b`
- `require_active_operator` gate verified on-chain (returns agent id 1)

Explorer: https://stellar.expert/explorer/testnet
Tests: 4 passing (registry 1, wallet 1, escrow 2)
