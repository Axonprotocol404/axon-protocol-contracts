# AXON Protocol — Stellar Testnet Deployments (Soroban)

Network: Stellar Testnet | Deployer: `GDI3YMIP6AD2AWOYFPBREESK4EL77DODIGDVO4SMVT2M5PIOEDSSUXGY`
Date: 2026-11-03 | stellar-cli 30.0.1 | soroban-sdk 22


## 5. BondingCurve (AXON Pad v1)

**Testnet:** `CAILFEYRVKYS4OPWALJMV5H2F5NBYNIYJSMH3EXOQRB25O4KIZIAMYXB`

Linear bonding curve for Pad launches: price = base_price + slope × (supply / 1M). Features:
- Identity gate: launcher must be a registered, active Agent (cross-contract Registry call)
- Full token allocation escrowed in-contract at curve creation
- `buy` with max_cost slippage protection, `sell` with min_refund protection
- Protocol fee 1.5% (collected on sell and at finalize)
- Finalize returns unsold tokens to dev, raised XLM to dev minus fee

Demo Curve #0 live: 50B DEMO tokens, base 1000 stroops, slope 1000/1M tokens.
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

## TokenSale (AXON Pad v0) — deployed 3 Nov 2026
- **Contract**: `CAR2SSLRJVOWLCGHFUMSUAIX5IWVD43PNJUMRSP4K53RIIIRYT4F75LE`
- Wasm hash sha256: (see build output) — 44.6KB
- **Features**: fixed-price token sale; dev must be a registered & active Agent in AgentRegistry (anti-rug gate); full token allocation escrowed in contract at create; hard cap; per-sale min buy; 1.5% protocol fee (fee_bps 150) to fee recipient on finalize; buyers claim tokens after sale ends.
- **On-chain demo**: Sale #0 created by agent #1 (dev = axon-testnet admin), token DEMO:admin (CDGK5WOWHL5RSFKZVOA65XYWGIXLV2GDKWKCGFALOQC7MP6NRAQI7SBG), 500k tokens offered, price 2000 stroops/token, hard cap 100 XLM, starts +60s ends +24h from creation.
- Tests: 2/2 pass (full sale flow: create→buy x2→finalize with fee→claims; unregistered dev rejected).
- Note: native XLM wrapper = CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC (testnet SAC for native).
