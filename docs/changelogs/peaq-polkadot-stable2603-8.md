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

### Added
- Both runtimes expose `XcmPaymentApi`, which returns XCM fee and weight quotes, and `DryRunApi`, which simulates a call or XCM message (`ae7fbacc`). Verified later: AC-12.
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
- **`FixedRateOfForeignAsset` has no unit tests.** `27d34bc5` removed the 399-line trader test file and did not port it. Coverage comes later, from AC-20.
- **Sha3FIPS256 (0x400) uses Frontier's "laptop" weights** (`516,915 + 2,019 × n` ref_time), which were not measured on peaq hardware. AC-11 recorded "cannot be determined", because no benchmark in this runtime measures the precompile. Whether to keep these weights is open item 4 of the upgrade spec.
- **ORML `MultiCurrencyAdapter`** is disabled behind a feature gate that peaq does not enable (AC-79). Its known defects are mitigated by that gate, not fixed. This is an accepted risk: a runtime that enables the feature gets the defects back.
- **Frontier DB version detection** has these gaps, in the fork's `client/db/src/kv/upgrade.rs` and in upstream code:
  - A 5-column database without a version file that already has some number mappings is treated as v3, and the remaining backfill is skipped.
  - The 4-column fallback treats a read error as "no data".
  - The v1→v2 and v2→v3 copies drop iterator read errors but still write version 3.
  - ParityDB has the same "5 columns means v3" gap.
- **ParityDB** in the Frontier DB moved from 0.4.13 to 0.5.6; the main chain database stays on 0.4.13. Whether 0.5 opens a 0.4 Frontier database has not been checked.
- **`--no-dht-bootnode*` flags have no effect.** The node does not start the relay-DHT bootnode tasks (`node/src/parachain/service.rs:345`).
- **Zenlink**: `zenlink-stable-amm`, `zenlink-swap-router`, `zenlink-vault` and `zenlink-gauge` still point at `stable2503-4`. The node does not use them.
- **Dependency forks use mutable branch refs.** Cargo.lock pins the commits, but `cargo update` would follow the branch heads.
