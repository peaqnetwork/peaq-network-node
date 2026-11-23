# peaq-polkadot-stable2603-8: node and runtimes on polkadot-sdk stable2603-8

**Branch**: `peaq-polkadot-stable2603-8` (peaq-network-node; continues `peaq-polkadot-stable2603-6` at `6f4772f2`)
**Date**: 2026-11-21
**PR**: none (this upgrade is pushed as branches, no GitHub PR)
**Status**: in-review

> Per-branch changelog fragment. It covers the node and both runtimes on this branch. It also covers the dependency forks the branch builds against: the peaqnetwork/polkadot-sdk fork, which is release `stable2603-8` plus 4 transaction-pool fixes, and 8 other dependency forks. Those forks have no changelog of their own for this upgrade, so their entries are recorded here.
> Items marked "verified later" are checked by tests in the upgrade test plan that run after this branch is pushed. Until they pass, treat those items as expected behaviour, not measured behaviour. The identifiers `AC-n` and `M-Xn` name those tests.

### Changed
- The node and both runtimes now build against polkadot-sdk `stable2603-8`. Before this they built against `stable2503-4`, which the `dev` branch still uses.
  - Dependencies come from `github.com/peaqnetwork` forks on branch `peaq-polkadot-stable2603-8`.
  - Cargo.lock pins each fork to one commit. polkadot-sdk is pinned to `f53ad07a`.
- Building requires Rust 1.95.0. It was 1.82.0. Rust 1.96 breaks the link of the runtime wasm.
- `peaq-node --version` now prints `4.0.1-polkadot-stable2603-8-<commit>`.
- Runtime versions, counted from the port base `cbcb7a7d`:
  - peaq: `spec_version` 113 → 114, `transaction_version` 2 → 3.
  - peaq-dev: `spec_version` 109 → 110, `transaction_version` 2 → 3.
- The EVM now uses upstream `evm` instead of the peaq fork (`267fdea1`).
  - The fork had one peaq commit, `b21947f`: increment the nonce before recording gas.
  - Upstream `9686e6b` (#331) has the same logic, and it is an ancestor of the pinned `a656db9`. EVM behaviour for that path does not change.
- Two new Frontier config values keep the previous behaviour for those two settings. The EVM rule set itself changes, see Breaking. Verified later: AC-38.
  - `AllowUnprotectedTxs = true`: legacy transactions without a chain id are still accepted.
  - `TransactionGasLimit = None`: there is no per-transaction cap below the block gas limit.
- The Ethereum runtime API changes. Tools that call these directly must update their bindings.
  - `call` takes two new arguments, `authorization_list` and `state_override`.
  - `create` takes one new argument, `authorization_list`.
  - The `EVM.call`, `create` and `create2` extrinsics take `authorization_list`.
- EVM error codes change. Clients that decode them by number must update.
  - In Frontier's `TransactionValidationError`, which the transaction pool reports as a custom invalid-transaction code, `EmptyAuthorizationList` (10) and `AuthorizationListTooLarge` (11) are inserted, so `UnknownError` moves from 10 to 12. `TransactionGasLimitExceedsCap` is added as 13, and `GasLimitTooHigh` is renamed `GasLimitExceedsBlockLimit`.
  - In `pallet_evm::Error`, `GasLimitTooHigh` is renamed `GasLimitExceedsBlockLimit`, and `TransactionGasLimitExceedsCap` is added at the end. Existing indices do not change.
- Transaction pool:
  - The default `--pool-type` changes from `single-state` to `fork-aware`. `--pool-type single-state` restores the old pool.
  - The polkadot-sdk fork carries 4 upstream fixes on top of `stable2603-8`:
    - #11733: fixes a wrong in-block event.
    - #12433: maintain the pool on every block.
    - #11731: unban viewless transactions after mempool revalidation.
    - #12362: keep the order of same-priority transactions on re-org. This one is cherry-picked from `c49ececa`.
- Log filters and subscriptions are now backed by Frontier's reorg-aware `LogsJournal`, with its default limits.
  - The journal holds entries; an entry is one new best block, together with the blocks a re-org retracts or enacts at that point. Each entry holds at most 10,000 logs, 4 MiB and 128 blocks, and the whole journal holds at most 512 MiB.
  - With these defaults the journal keeps only the last 128 entries (512 MiB / 4 MiB), which is about the last 128 best blocks.
  - An entry over its limit ends all log subscriptions.
  - A polled filter (`eth_getFilterChanges`) that has not polled within those 128 entries, or that reaches an entry over its limit, is removed, and the call returns an error that says to recreate the filter. A recreated filter does not return the logs it missed; fetch them with `eth_getLogs`.
- `state_traceBlock` now runs through the parachain tracing executor, which the node supplies to the RPC (new in stable2603). The executor registers the proof-size extension the runtime needs, so traces match real execution.
- EVM tracing host functions now take their `Vec<u8>` arguments as a fat pointer (`e35297f6`), as stable2603's `sp-runtime-interface` requires. A tracing node needs a binary from this branch to trace blocks of the new runtimes. Verified later: AC-34, which also traces pre-upgrade blocks.
- XCM fees paid in foreign assets go through the ported `FixedRateOfForeignAsset` and `XcmFungibleFeeHandler` (`27d34bc5`). Verified later: AC-20 and AC-30.
- **Collators author with the slot-based collator instead of lookahead** (`a9a06bc3`). The runtime does not change: velocity stays 1 and `RelayParentOffset` stays 0. Verified later: AC-53 and AC-56 (new binary with the old runtime, and mixed collators).
  - Every node wraps block import in `SlotBasedBlockImport`. A node that does not author drops the wrapper's handle at start-up, so on RPC and archive nodes the wrapper only passes blocks through.
  - Operators: the essential task `aura` is replaced by `slot-based-block-builder` and `slot-based-collation`, and with `RUST_LOG=aura=debug` the authoring line `Slot claimed. Building` becomes `Building block.`. Dashboards and alerts keyed on the old names must be updated.
  - Tested on a local network (AC-15 to AC-17): two slot-based collators, a mix with lookahead in both orders, and a non-collator node. All three passed, with two results that need a reading:
    - AC-15 counts reorgs as the total minus the session boundaries the window crosses: 4 and 0 in two runs (limit 5). Every run loses the last slot of each session, as lookahead did on the same setup on 2026-11-16.
    - AC-16 allows 1 missing block per session. In one run with the lookahead collator taking the first slot, 1 of 35 sessions missed 2: the lookahead collator built on a relay block that a relay fork then dropped. The slot-based collator did not cause it.
- The runtime upgrade migrates `cumulus_pallet_xcmp_queue` storage from version 5 to 7 in one step, with `MigrateXcmpQueueV5ToV7` (`ce93e60b`). Both runtimes have tests in `runtime/*/tests/xcmp_queue_migration.rs` (AC-13). Verified later: AC-14 (live state with channels added) and AC-66 (try-runtime on mainnet state).
  - stable2603 adds `flags` and `queued_bytes` to `OutboundChannelDetails`. Without a migration, a non-empty `OutboundXcmpStatus` fails to decode and the on-chain version stays at 5.
  - Upstream's `MigrateV5ToV6` already writes the v7 layout, so chaining it with `MigrateV6ToV7` either misreads the data or fails with a defensive error when there are two or more channels. peaq does not use that pair.
  - Neither chain had outbound channels when read on 2026-11-13. On both chains the migration therefore only moves the storage version.

### Added
- Both runtimes expose `XcmPaymentApi`, which returns XCM fee and weight quotes, and `DryRunApi`, which simulates a call or XCM message (`ae7fbacc`). Checked by AC-12 on a new local chain at block 0; the parts that need produced blocks are verified later by AC-81.
  - `dry_run_call` returns the XCM a call executed locally in `local_xcm`. This needs the executor to record it, so both runtimes now set `XcmRecorder = PolkadotXcm` (`ea041908`). Recording is on only inside a dry run.
  - `query_delivery_fees` returns `Err(Unimplemented)`, not an empty fee list. peaq charges no delivery fee, and the polkadot-sdk helper answers `Unimplemented` whenever the router's fee is not exactly one asset.
  - Dry-run `emitted_events` leave out the events the XCM executor emits itself (`Sent`, `SendFailed`, `ProcessXcmError`), because `XcmEventEmitter` stays `()`.
  - A dry run against block 0 returns no events, because `frame_system` records no events at genesis.
- Both runtimes implement `KeyToIncludeInRelayProof` (`2a555fee`).
- Type-4 (EIP-7702) transactions are accepted, as part of Osaka (see Breaking). Verified later: AC-31.

### Breaking
- **The EVM rule set moves from Cancun to Osaka.** Neither runtime overrides `pallet_evm::Config::config()`, so both use Frontier's default `Config::osaka()`. EIP-7702 and EIP-7623 follow upstream by the 2026-11-19 decision; CLZ and the modexp changes by the 2026-11-21 decision. What changes for contracts and wallets:
  - EIP-7702: EOAs can delegate to contract code through type-4 transactions.
  - EIP-7623: a transaction with calldata has a minimum gas of 21000 + 10 per zero byte + 40 per non-zero byte of calldata. If the gas limit is below this floor, the transaction pool and block validation reject the transaction; the RPC error is `intrinsic gas too low`. An accepted transaction is charged at least the floor.
  - EIP-7939: a new `CLZ` opcode.
  - The modexp precompile (0x05) costs more under EIP-7883. The minimum goes from 200 to 500 gas, and larger inputs cost several times more than before.
  - Under EIP-7823, a modexp call with a base, exponent or modulus longer than 1024 bytes fails and consumes all remaining gas.
  - The modexp changes are in the precompile code itself, so a runtime `config()` override would not undo them.
- **The Sha3FIPS256 precompile (0x400) is now priced by weight.** It was `60 + 12 per 32-byte word`. It now uses Frontier's default weights, which convert to about 23 gas for 32 bytes and 103 gas for 1 KiB. That is 3 to 5 times cheaper. See Known risks.
- **`eth_getLogs` and filter queries are limited to 1024 blocks** (a request fails when `to - from > 1024`). This is the upstream default and is hard-coded in the node; there is no CLI flag to change it. Indexers that scan from an old block in one query must split the range. Verified later: AC-37.
- **`session.set_keys` now checks the ownership proof.** Collators that call `setKeys(keys, 0x)` get `InvalidProof`. Generate the keys and the proof with `author_rotateKeysWithOwner`, passing the SCALE-encoded owner account. Verified later: AC-40.
- **`transaction_version` 3.** Offline signers and wallets that hard-code the transaction version must update. Otherwise their transactions are rejected after the runtime upgrade.
- **The Frontier database migrates from v2 to v3 on first start** with this binary (`6f4772f2`). Back up `frontier/` before switching binaries.
  - The migration is one-way: an older binary refuses to open the migrated database.
  - It runs at startup and reads whole columns into memory. Its time and memory on a mainnet-sized database have not been measured yet.
- **Do not start a node with `--database auto`.** If the Frontier database needs the upgrade, `auto` may panic. It may also open an empty paritydb, in which case old EVM blocks and transactions return nothing and no error is reported. With the default rocksdb, a failed upgrade stops the node with an error.
- **The relay token reserve is now Asset Hub (para 1000), not the relay chain.** This comes from ORML, which follows the upstream Asset Hub migration.
  - Send DOT to peaq through Asset Hub, not directly from the relay chain.
  - A transfer from peaq to an account on the relay chain lands on the same account on Asset Hub, not on the relay chain.
  - DOT transfers to sibling parachains are routed through Asset Hub.
  - Verified later: AC-18, AC-19.

### Known risks (accepted, not fixed on this branch)
- **XCM fees paid in PEAQ under-count `TotalIssuance`.**
  - This is pre-existing: it has been on mainnet since `aecb104c` (2025-06), and the code is unchanged by this port.
  - `BlockRewardWrapper` (`runtime/*/src/xcm_config.rs`) drops the `Credit` that `UsingComponents` hands it, and dropping it lowers `TotalIssuance`. It then gives the sinks a new imbalance of the same amount.
  - The PEAQ itself is not lost, but `TotalIssuance` ends up lower than the sum of balances by every such fee.
  - A separate work unit fixes this before AC-30 runs.
- **The EIP-3607 check now reads `AccountCodesMetadata`, not `AccountCodes`** (Frontier `92fc798`, `runner/stack.rs:240` and `frame/ethereum/src/lib.rs:610`).
  - A contract stored without metadata is therefore not blocked as a transaction sender. This branch has no migration that backfills the metadata.
  - Exploiting this would require a private key for a contract address, which is not practical.
  - Check M-X5 compares the key counts of the two maps on mainnet. If they differ, a separate work unit adds the backfill migration.
- **`DryRunApi` lets anyone run unmetered work on an RPC node.** A dry run dispatches any call from any origin, including Root, with no weight limit; for example `System::kill_prefix` with a large key count walks a whole storage prefix in one `state_call`. Nothing is written to the chain. Upstream runtimes such as Asset Hub expose the same API the same way. Public RPC nodes must set `--rpc-rate-limit`, `--rpc-max-request-size` and `--rpc-max-response-size`; the upgrade runbook must list them (AC-62).
- **Foreign assets in `XcAssetConfig` are stored under XCM v3 locations on mainnet, but the runtime looks them up under v5.**
  - A read-only query of peaq mainnet on 2026-11-22 found 6 entries, all Astar (para 2004) assets, in each of `AssetLocationUnitsPerSecond`, `AssetLocationToId` and `AssetIdToLocation`, and every location there is `VersionedLocation::V3`.
  - The pallet looks rates and asset ids up with a v5 key since `c7127802` (2025-05, in tag `peaq-v0.0.113`; mainnet ran spec 113 at the 2026-11-19 read-only check), and with a v4 key before that.
  - Inferred from the code, not measured on chain: these assets can neither pay XCM fees nor be matched to a local asset id on the current mainnet runtime, and this stays true after this upgrade unless the migration below ships with it.
  - `query_acceptable_payment_assets` still lists them, because it converts the stored keys.
  - A separate work unit migrates the three maps to v5 before the mainnet upgrade. Verified later: AC-83.
- **A slot-based collator stops its node when one read fails.** In polkadot-sdk `stable2603-8`, the slot-based block builder leaves its loop when it cannot build the inherent data, which reads relay state, or cannot read the validation code hash from the parachain's own state (`block_builder_task.rs:416`, `:424`). It runs as an essential task, so the whole node shuts down. Lookahead gives up only that attempt and tries again at the next relay block. A relay call that comes back with an error is enough to trigger it. Until a separate work unit changes the SDK fork, run collators under a supervisor that restarts them, and alert on `Essential task` in the log.
- **Slot-based collators need monitoring the lookahead collator did not.**
  - The import channel to the collation task is unbounded (its backlog is `substrate_unbounded_channel_size{entity="SlotBasedBlockImportChannel"}`).
  - Block timing follows the local clock, so collators need NTP and a clock-skew alert.
  - If the relay ever assigns peaq more than one core, for example through on-demand orders that anyone can place, the collator tries to build every 6/N seconds for N cores, so each block gets at most the shorter of 2 s and 6/N s, and the last block before the slot passes to another collator must also finish 1 s before the slot ends, or is skipped when too little time is left (`slot_timer.rs`). Velocity stays 1, so the block count does not change. This is inferred from the code, not measured.
- **The xcmp-queue migration assumes the stored channels really are in the v5 layout.** On chain it decodes them the lenient way, the same as upstream, which accepts trailing bytes. If a chain were at storage version 5 but already held the longer v6 or v7 entries, the channels after the first would either be misread and written back without an error, or fail to decode and be left as they are. That can only happen on a chain that once ran a stable2603 runtime without this migration, because the new fields first appear in stable2603. Both deployed runtime tags, `peaq-v0.0.113` and `peaq-dev-v0.0.109`, are built on stable2503-4 (mainnet ran spec 113 at the 2026-11-19 read-only check), and neither chain had outbound channels when read on 2026-11-13. The try-runtime `pre_upgrade` rejects bytes that are not v5. Verified later: AC-50 (agung) and AC-66 (mainnet) run it on live state.
- **`FixedRateOfForeignAsset` has no unit tests.** `27d34bc5` removed the 399-line trader test file and did not port it. Coverage comes later, from AC-20.
- **Sha3FIPS256 (0x400) uses Frontier's "laptop" weights** (`516,915 + 2,019 × n` ref_time), which were not measured on peaq hardware. AC-11 recorded "cannot be determined", because no benchmark in this runtime measures the precompile. These weights are kept for this upgrade by the 2026-11-22 decision, and the price change goes into the upgrade announcement. Gas is derived from these weights, so if collators hash more slowly than the benchmark machine did, a block full of 0x400 calls can take longer than the block's intended EVM time budget.
- **ORML `MultiCurrencyAdapter`** is disabled behind a feature gate that peaq does not enable (AC-79). Its known defects are mitigated by that gate, not fixed. This is an accepted risk: a runtime that enables the feature gets the defects back.
- **Frontier DB version detection** has these gaps, in the fork's `client/db/src/kv/upgrade.rs` and in upstream code:
  - A 5-column database without a version file that already has some number mappings is treated as v3, and the remaining backfill is skipped.
  - The 4-column fallback treats a read error as "no data".
  - The v1→v2 and v2→v3 copies drop iterator read errors but still write version 3.
  - ParityDB has the same "5 columns means v3" gap.
- **ParityDB** in the Frontier DB moved from 0.4.13 to 0.5.6; the main chain database stays on 0.4.13. Whether 0.5 opens a 0.4 Frontier database has not been checked.
- **`--no-dht-bootnode*` flags have no effect.** The node does not start the relay-DHT bootnode tasks (`_paranode_rx` from `build_relay_chain_interface` in `node/src/parachain/service.rs` is never used).
- **Zenlink**: `zenlink-stable-amm`, `zenlink-swap-router`, `zenlink-vault` and `zenlink-gauge` still point at `stable2503-4`. The node does not use them.
- **Dependency forks use mutable branch refs.** Cargo.lock pins the commits, but `cargo update` would follow the branch heads.
