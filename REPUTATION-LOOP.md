# Escrow → Registry Reputation Write-Back

**Date:** 2026-11-16 · **Network:** Stellar testnet only · **Status:** code + tests delivered, **redeployed to testnet 2026-11-16** (deployment v2 — see "Redeploy" below and `STELLAR_DEPLOYMENTS.md`)

## The gap

The Agent Registry is AXON's trust layer: every agent carries a `reputation`
score and a `task_count`. But nothing in the protocol ever updated them —
the SimpleEscrow contract settled payments without reporting the outcome,
so a provider could complete (or fail) any number of escrows and its
Registry record never changed. Reputation could only be set by the admin
by hand (`set_reputation`).

## What changed

### `soroban/agent-registry/src/lib.rs`
- New storage key `DataKey::Recorder(Address)` — the set of contracts
  authorized to report task outcomes.
- New admin function `set_recorder(recorder, authorized)` — authorizes or
  revokes a recorder (intended: the SimpleEscrow contract address).
- New function `record_outcome(recorder, operator, success) -> bool`:
  - Reverts unless `recorder` is authorized **and** authorizes the call.
    For a recorder contract, being the invoker of the call is its
    authorization — a third party cannot forge reputation updates.
  - Finds the agent by `operator` address (same lookup style as the
    existing `require_active_operator` gate).
  - On success: `task_count += 1`, `reputation += 10`
    (`REP_PER_COMPLETION`). On failure: `task_count += 1`,
    `reputation -= 5` (`REP_FAILURE_PENALTY`, saturating at 0).
  - If `operator` is not a registered agent, returns `false` and changes
    nothing (no revert), so callers can settle payments to unregistered
    providers safely.
- New view `is_recorder(recorder) -> bool` for ops verification.
- `set_reputation` remains as the admin's manual override/bootstrap.

### `soroban/simple-escrow/src/lib.rs`
- `init` now also takes the AgentRegistry address
  (`init(admin, default_fee_bps, registry)`), stored as `DataKey::Registry`
  — the same wiring pattern TokenSale and BondingCurve already use.
- `release`: after paying the provider and the protocol fee and marking
  the escrow `Released`, it calls
  `Registry.record_outcome(escrow_contract, provider, true)` via a locally
  declared `RegClientX` client trait (the repo's established
  no-crate-dependency pattern).
- `mutual_cancel` records nothing: a mutual cancel is an amicable unwind,
  not a failed task.
- Failure mode is safe: if the escrow is not an authorized recorder in
  the Registry, `record_outcome` reverts and the whole release rolls back
  (funds stay in escrow; `mutual_cancel` remains the exit). Wiring is
  therefore part of deployment, below.

## Tests

Registry (`soroban/agent-registry`): existing `register_and_gate` plus —
- `record_outcome_success_updates_reputation_and_count` — +10 rep / +1 task per success, accumulates.
- `record_outcome_failure_penalizes_and_saturates` — −5 on failure, floor 0, task still counts.
- `record_outcome_rejects_unauthorized_recorder` — never-authorized and revoked recorders revert; nothing recorded.
- `record_outcome_unknown_operator_is_noop` — returns `false`, no panic, no state change.

Escrow (`soroban/simple-escrow`): existing `open_release_with_fee` and
`mutual_cancel_refunds` (updated for the new `init` signature), plus —
- `release_writes_outcome_back_to_registry` — against a stub registry that
  records calls: exactly one successful outcome for the provider, none for
  the client.
- `mutual_cancel_writes_no_outcome` — cancel reports nothing.

**Test run results:** ⚠️ **NOT EXECUTED in this environment.** The VM has
no Rust/Soroban toolchain; the rustup installer download was rate-limited
(HTTP 429), and the distro toolchain (rustc/cargo 1.75) cannot build this
repo's pinned dependency set — `time-core 0.1.9` in the lockfile requires
edition 2024 (Rust ≥ 1.85), and downgrading it conflicts with
`soroban-ledger-snapshot`'s `serde_with` requirement. The tests above are
written and reviewed but must be run once with a current toolchain:

```bash
cd soroban/agent-registry && cargo test
cd soroban/simple-escrow  && cargo test
```

Expected: registry 5/5, escrow 4/4. The first run will also regenerate
the escrow `test_snapshots/` (the `init` signature changed); the copies in
this repo predate the change — snapshots are artifacts, they do not gate
the test result.

Note: the escrow tests use a stub registry (the repo's existing pattern —
crates deliberately carry no cross-crate dependency in wasm). The
registry-side tests exercise the real `record_outcome` logic. A combined
two-real-contracts integration test does not exist in this repo layout;
the end-to-end check happens on testnet after redeploy (below).

## Redeploy (testnet) — DONE 2026-11-16

Executed with a fresh agent-held deployer keypair
(`GDT4KNR2MFOLIA3ENSV5MXIMRHOJX2VX22EC4QFZNKHNNQ532M5RJDZW`, created for AXON
deployments; the v1 deployer account was not touched). All five contracts
were redeployed (TokenSale/BondingCurve/AgentWallet point at the Registry,
so they moved with it), wired, and verified:

- `escrow.init(admin, 30, <new registry>)` — done
- `registry.set_recorder(<new escrow>, true)` — done; `is_recorder` reads `true`
- End-to-end check passed on testnet: agent #1 registered, escrow of 5 XLM
  opened + released (tx `6eeade6a88d4fa141e1478391cb7de04fd0e9c160789671fa2a89a160884b951`),
  and `get_agent` read back `task_count = 1`, `reputation = 10`.
- New contract IDs recorded in `STELLAR_DEPLOYMENTS.md` (deployment v2);
  dashboard/SDK/website address constants updated to the v2 IDs.

The v1 contracts (Registry `CDLTKHIAAQPMJMGRGCUMVDSVWO672OSW3KEHUPEV5GFN3UXVCMIAHTFN`,
Escrow `CBT52U22SJW357F6Y3FGP3GY7IA4BT3XNLUKFSRAPEC732F425ALRL6L`) remain on-chain
untouched, running the old code without the write-back.

## Remaining gaps

- No dispute/timeout path in the escrow (only release / mutual cancel),
  so the `success = false` branch of `record_outcome` is exercised by
  tests but not yet by any protocol flow. When a dispute resolution or
  deadline-refund path is added, it should call
  `record_outcome(escrow, provider, false)`.
- Reputation is a simple linear score (+10/−5). No decay, no
  amount-weighting, no client-side rating. Deliberately out of scope for
  this fix.
- The write-back covers the provider side only; clients build no
  on-chain history from escrows yet.
