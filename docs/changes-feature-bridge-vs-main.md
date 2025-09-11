# Changes from main to feature/bridge (HEAD)

Comparison generated on 2025-09-11 between:

- Base: `origin/main`
- Target: `HEAD` (currently at `origin/feature/bridge`)

To reproduce locally:

```bash
# from repo root
git fetch --all --prune
git log --no-merges --pretty=format:'%h\t%an\t%ad\t%s' --date=short origin/main..HEAD
git diff --name-status origin/main..HEAD
git diff --stat origin/main..HEAD
```

## Summary

This branch introduces a new bridge-out pallet and relayer, adds an automated payment system integrated into the governance pallet, updates the runtime to include these components, and adds operational scripts and Docker Compose convenience. Tests have been reorganized and expanded significantly around governance proposals and scheduled payments.

High-level highlights:

- Bridge-out functionality scaffolded (`pallets/bridge-out/`) with runtime wiring and a `node` relayer.
- Governance pallet gains scheduled payment functionality with block-based intervals and comprehensive tests.
- Runtime updated to include new pallets and configuration adjustments.
- Operational tooling: `docker-compose.yml`, `scripts/run_bridge_relay.sh`, `scripts/prepare_runtime_upgrade.py`, `scripts/external_libraries.sh`.
- Test suite refactor: governance tests split into dedicated modules with large additions.

## Commit summary (origin/main..HEAD)

Note: author/date/subject as per `git log`.

- b0c8547  bakobiibizo   2025-08-29  fixed build warnings
- 30078ad  Mod-Net CI    2025-08-29  missed pushing bridge code
- b428ff4  J. Zane Cook  2025-06-25  fix: Cargo fmt
- abc2c1b  J. Zane Cook  2025-06-25  fix: Increment runtime spec version
- 4b1a84b  J. Zane Cook  2025-06-22  fix: Move tests to pre-existing folder, following existing structure
- 73f6322  J. Zane Cook  2025-06-22  fix: Removed unused tests
- 1858f6b  J. Zane Cook  2025-06-22  fix: Remove unused integration tests
- 16c0231  J. Zane Cook  2025-06-22  fix: Restore original docs
- dbabecd  bakobiibizo   2025-06-21  missed the refactored payment cycle changes and incremented the version
- f1b273c  bakobiibizo   2025-06-21  removed extra docs the model committed
- b337b41  bakobiibizo   2025-06-21  add documentation to the payments pallet
- 119e29e  bakobiibizo   2025-06-19  simplified the payment schedule to pay on block intervals
- eda347b  bakobiibizo   2025-06-15  added docker compose for standard node
- 8c90f62  bakobiibizo   2025-06-15  completed payment

## Files changed and diff stats

From `git diff --name-status` and `--stat`:

- Added (A):
  - `docker-compose.yml` (+23 lines)
  - `node/src/relayer.rs` (+172)
  - `pallets/bridge-out/Cargo.toml` (+29)
  - `pallets/bridge-out/src/lib.rs` (+78)
  - `pallets/governance/src/payments.rs` (+162)
  - `scripts/external_libraries.sh` (+5)
  - `scripts/prepare_runtime_upgrade.py` (+44)
  - `scripts/run_bridge_relay.sh` (+7)
  - `tests/src/governance/payments.rs` (+350)
  - `tests/src/governance/proposals.rs` (+738)

- Modified (M):
  - `.gitignore` (+1)
  - `Cargo.lock` (mixed, +19 -)
  - `Cargo.toml` (+1)
  - `Dockerfile` (+4 -1)
  - `node/Cargo.toml` (+1)
  - `node/src/cli.rs` (+5)
  - `node/src/command.rs` (+1)
  - `node/src/main.rs` (+1)
  - `node/src/service.rs` (+8 -1)
  - `pallets/governance/Cargo.toml` (+2 -1)
  - `pallets/governance/src/lib.rs` (+151 -?)
  - `pallets/governance/src/weights.rs` (+15)
  - `runtime/Cargo.toml` (+2)
  - `runtime/src/lib.rs` (+14 -?)
  - `tests/src/governance.rs` (-740 net; moved into new modules)

Totals: 25 files changed, ~1816 insertions, ~757 deletions

## Key changes by area

### Bridge-out pallet and relayer

- New pallet: `pallets/bridge-out/` with `Cargo.toml` and `src/lib.rs` scaffolding for outbound bridge functionality.
- Node relayer: `node/src/relayer.rs` introduces a relayer component used by the node to interact with the bridge. Related updates in `node/src/service.rs`, `node/src/cli.rs`, `node/src/command.rs`, and `node/src/main.rs` wire up the relayer.
- Runtime: `runtime/src/lib.rs` and `runtime/Cargo.toml` updated to include the bridge-out pallet in the runtime configuration.

### Governance pallet: scheduled payments

- Added `pallets/governance/src/payments.rs` implementing an automated payment system:
  - Scheduled payments managed within the governance pallet.
  - Block-based disbursement using `next_payment_block` and `payment_interval`.
  - Funds disbursed from Treasury; configurable intervals.
  - Payment windows and completion handling.
- Integration in `pallets/governance/src/lib.rs` and weights in `pallets/governance/src/weights.rs`.
- Tests:
  - `tests/src/governance/payments.rs` covers payment schedule creation, processing, completion, failure handling, and block progression simulation.
  - `tests/src/governance/proposals.rs` contains governance proposal flow and related logic tests.
  - Old monolithic `tests/src/governance.rs` reduced and split across the new modules.

### Tooling and operations

- `docker-compose.yml` for spinning up a standard node more easily.
- `scripts/run_bridge_relay.sh` to run the bridge relayer.
- `scripts/prepare_runtime_upgrade.py` to assist in preparing runtime upgrades.
- `scripts/external_libraries.sh` for external library management.

## Breaking changes and upgrade notes

- Runtime spec version incremented (see commit `abc2c1b`) — a chain upgrade is required. Use `scripts/prepare_runtime_upgrade.py` to stage/validate the upgrade.
- Governance scheduled payments depend on a funded Treasury account in the runtime. Ensure the treasury is funded during testing and on-chain, or payment execution will fail.
- Weights updated for governance; re-run benchmarking if you customize runtime parameters.

## Developer notes

- Bridge relayer: See `node/src/relayer.rs`. Start via `scripts/run_bridge_relay.sh`. Ensure any external endpoints/keys referenced by the relayer are configured via environment or CLI flags.
- Payments: See `pallets/governance/src/payments.rs`. Configuration constants such as `BLOCKS_PER_PAYMENT_CYCLE` and treasury account are defined within the governance pallet/runtime config. Tests in `tests/src/governance/payments.rs` show end-to-end flows.
- Docker Compose: `docker-compose.yml` provides a baseline setup for a standard node; adjust ports/volumes as needed.

## Appendix: full lists

- Full commit list: run the log command in the repro section to see author and subjects for all commits.
- Full file diff: run the diff commands in the repro section for complete changes.
