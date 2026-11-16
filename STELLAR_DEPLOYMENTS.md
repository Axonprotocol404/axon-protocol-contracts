# Stellar Testnet Deployments — AXON Protocol

Network: Stellar Testnet · RPC: https://soroban-testnet.stellar.org

## Current deployment — v2 (2026-11-16, escrow → Registry reputation loop)

Deployer / admin: `GDT4KNR2MFOLIA3ENSV5MXIMRHOJX2VX22EC4QFZNKHNNQ532M5RJDZW` — dedicated agent-held deployer keypair created
2026-11-16 for AXON deployments (the v1 deployer account is retired; mainnet
keys remain a separate, future decision). Funded with testnet XLM via friendbot.

| Contract | Contract ID | WASM hash (sha256) | Deploy tx |
|----------|-------------|--------------------|-----------|
| AgentRegistry | `CBO6F574QSLONYNWHM5WG7U3ZE7R4ONZKKYLSTQYURB6VCOF7LM4OBMV` | `62dc604eb0b910ca5cee926ce16f2172cc72242084d807d697c81116ad465ec0` | `61cb985b91c73d60e044ff65832ceb7ecfc522530f96332e3e549e620674cad1` |
| AgentWallet | `CBNORTXZDKAULKEHSZCIZ7KSF3JUYI5LMPVFXX7FCDQQMFBFS22ZDI35` | `c44f248d83afef66b5d3efb4f659f514446956bb3c863af0467435f2f39b67cd` | `4079cba901e4620f12035f262c86cea0bb342e05324b032b2efc9f11e3790c33` |
| SimpleEscrow | `CCJKMU7BPP5LWMQJZNYCATFWCAWZ5ZJNJEUNSNROZC534EGSOVRAGS6E` | `1b84211062e63ffba7ae47deb2698ee6ec54c49ecd78451b7d6b7ce6a8c6ea35` | `c72a316d6ef8fb06f9765b8271e05342094965cc3f2a56123934e4964709257c` |
| TokenSale | `CCFRQY2IEBGROVPMK2MHKO35IJSSDYUEDTOHJJQPVFLZQFCQQPKARD5B` | `696b5259bde16a5365be29eea750c21631fb89046dbdc91ce331c074719c84d9` | `976cffa9c56b8af1465c8f8bcaf7b964806ca05cd4271b58b7a156c2b756300d` |
| BondingCurve | `CAQKKZ7YDAGJFOKZVYISDLFMA3QC2LIE2GSS3SEBDDGX3A5BAPTMJMMM` | `6fe8f4884f35dab51a4e428cd4443c8ca23dd7ddc5fa5e515dac5d1ce68f9455` | `5b0a887c11a870f9bf59d1e10ea54ef986ed68529770b17a4c68cb6d0cd9c606` |

### Initialization & wiring (all confirmed on-chain)

- Registry `init(admin = deployer)` — tx `73fb34214f36b2a477b3ab21ebe3c45219ec02961cf548768bd0ce4c03416118`
- Escrow `init(admin, default_fee_bps = 30, registry = AgentRegistry)` — tx `1cf4d1c610827ab0107d053efc1caf91c0a3e7df97bec734c7790c48d4212d8b`
- Registry `set_recorder(escrow, true)` — tx `1128f1ebfb2a731180726cb2c9ff0c8d6330dacf3c059ccf118b5418533ebfcb` (`is_recorder(escrow)` reads back `true`)
- TokenSale `init(admin, registry, native_token = XLM SAC, fee_bps = 150, fee_recipient = deployer)` — tx `c04773a8df56778a7cfafdc04cd46ae776593c36a50c2c724cc59f9adbe8ecd7`
- BondingCurve `init(admin, registry, native_token, fee_bps = 150, fee_recipient = deployer)` — tx `1afff4404eeacaa393a0b32f39288afc202b322ed46990811322e6689fb3fea6`
- AgentWallet `init(owner = deployer, operator = test provider, token = XLM SAC, per-tx cap 100 XLM, daily cap 500 XLM)` — tx `8238d2f3714cb8b18011ddfdb69f42e821549c7e9f2fe946cbe62aa04ec38746`

### On-chain smoke test (2026-11-16) — reputation write-back proven live

- Test agent registered: id `1`, owner = deployer, operator = `GATP3GPQPDX5CTJ44JGGXNLM2XUTUMCPR4JE4UXDEKGKHGLT4RJIEOUX` — tx `00929bc5f27a11c8b83736d6adce85030da024ff47883822e90b55ef390d843a`
- AgentWallet bound to agent 1 — tx `046714111ea40374d375096d328384ddbc709ec8fd1bad5bf0c078401158c73d`
- Escrow #1 opened: 5 XLM, client = deployer, provider = test agent — tx `0f96e8ab05cec36168cf9932bb3180b5e291107d5c276109d314a864e71bb48f`
- Escrow #1 released — tx `6eeade6a88d4fa141e1478391cb7de04fd0e9c160789671fa2a89a160884b951`: provider received 4.985 XLM (0.3% fee = 0.015 XLM to admin), and the Registry agent read back **reputation = 10, task_count = 1** — the escrow → Registry write-back works on-chain.

Unit tests at deploy time: agent-registry 5/5, simple-escrow 4/4, token-sale 2/2,
bonding-curve 2/2, agent-wallet 1/1 (14/14). Built with stellar-cli 30.0.1,
optimized WASM (target wasm32v1-none).

The website dashboard, AXON Pad pages and the JS SDK (`sdk/`) point at the v2 IDs.

---

## Previous deployment — v1 (2026-11-14, superseded)

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
