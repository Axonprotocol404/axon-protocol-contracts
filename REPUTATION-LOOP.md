# Escrow → Registry Reputation Write-Back

**Date:** 2026-11-16 · **Network:** Stellar testnet only · **Status:** code + tests delivered, NOT yet redeployed (see "Redeploy" below)

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

## Redeploy (testnet) — NOT done yet

Not executed: this environment has no Rust/Soroban toolchain preinstalled
and no deployer secret key or stellar-cli identity in the workspace, so
there was nothing to deploy with. Needed to redeploy:

1. Rust toolchain + `stellar-cli` (deployments were last made with
   stellar-cli 30.0.1, soroban-sdk 22).
2. The deployer secret key for
   `GDI3YMIP6AD2AWOYFPBREESK4EL77DODIGDVO4SMVT2M5PIOEDSSUXGY`
   (testnet account, already funded).
3. Build & deploy both contracts (Registry first, then Escrow), then wire:
   - `escrow.init(admin, 30, <new registry address>)`
   - `registry.set_recorder(<new escrow address>, true)`
   - verify with `registry.is_recorder(<escrow address>) == true`
4. End-to-end check on testnet: register an agent, open + release an
   escrow with that agent as provider, confirm `get_agent` shows
   `task_count = 1`, `reputation = 10`.
5. Record the new contract IDs in `STELLAR_DEPLOYMENTS.md` and update the
   dashboard/SDK address constants.

Until redeploy, the currently deployed testnet contracts
(Registry `CDLTKHIAAQPMJMGRGCUMVDSVWO672OSW3KEHUPEV5GFN3UXVCMIAHTFN`,
Escrow `CBT52U22SJW357F6Y3FGP3GY7IA4BT3XNLUKFSRAPEC732F425ALRL6L`) still run
the old code without the write-back.

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
