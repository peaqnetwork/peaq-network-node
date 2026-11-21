# peaq-polkadot-stable2603-8: node and runtimes on polkadot-sdk stable2603-8

**Branch**: `peaq-polkadot-stable2603-8` (peaq-network-node; continues `peaq-polkadot-stable2603-6` at `6f4772f2`)
**Date**: 2026-11-21
**PR**: none (this upgrade is pushed as branches, no GitHub PR)
**Status**: in-review

> Per-branch changelog fragment. It covers the node and both runtimes on this branch, plus the dependency forks it builds against: the peaqnetwork/polkadot-sdk fork (4 transaction-pool fixes) and 8 dependency forks. Those forks have no changelog of their own for this upgrade, so their entries are recorded here.
> Items marked "verified later" are checked by acceptance tests that run after this branch is pushed. Until they pass, treat those items as expected behaviour, not measured behaviour.

### Changed
- The node and both runtimes now build against polkadot-sdk `stable2603-8`. Before this they built against `stable2503-4`, the version the `dev` branch still uses. Dependencies come from `github.com/peaqnetwork` forks on branch `peaq-polkadot-stable2603-8`. Cargo.lock pins each fork to one commit. polkadot-sdk is pinned to `f53ad07a`.
- The node version string is now `4.0.1-polkadot-stable2603-8-<commit>`.
- Runtime versions:
  - peaq: `spec_version` 113 → 114, `transaction_version` 2 → 3.
  - peaq-dev: `spec_version` 109 → 110, `transaction_version` 2 → 3.
  - Both are counted from the port base `cbcb7a7d`.
- The EVM now uses upstream `evm` instead of the peaq fork (`267fdea1`). The fork's one peaq commit, `b21947f` (increment the nonce before recording gas), has the same logic as upstream `9686e6b` (#331). That upstream commit is an ancestor of the pinned `a656db9`. So EVM behaviour for that path does not change.
- Two new Frontier config values keep the previous behaviour for those two settings (the EVM rule set itself changes, see Breaking):
  - `AllowUnprotectedTxs = true`: legacy transactions without a chain id are still accepted.
  - `TransactionGasLimit = None`: there is no per-transaction cap below the block gas limit.
  - Verified later: AC-38.
- Transaction pool: the node uses the fork-aware pool. The polkadot-sdk fork carries 4 upstream fixes on top of `stable2603-8`:
  - #11733: fixes a wrong in-block event.
  - #12433: maintain the pool on every block.
  - #11731: unban viewless transactions after mempool revalidation.
  - #12362: keep the order of same-priority transactions on re-org.
- EVM tracing host functions now take their `Vec<u8>` arguments as a fat pointer (`e35297f6`), as stable2603's `sp-runtime-interface` requires. A tracing node needs a binary from this branch to trace blocks of the new runtimes. Verified later: AC-34, which also traces pre-upgrade blocks.
- XCM fees paid in foreign assets go through the ported `FixedRateOfForeignAsset` and `XcmFungibleFeeHandler` (`27d34bc5`). Verified later: AC-20 and AC-30, which check that total issuance does not change.

### Added
- Both runtimes implement `KeyToIncludeInRelayProof` (`2a555fee`).
- Type-4 (EIP-7702) transactions are accepted, as part of Osaka (see Breaking). Verified later: AC-31.

### Breaking
- **The EVM rule set moves from Cancun to Osaka.** Neither runtime overrides `pallet_evm::Config::config()`, so both use Frontier's default `Config::osaka()`. This follows upstream, as decided on 2026-11-19. What changes for contracts and wallets:
  - EIP-7702: EOAs can delegate to contract code through type-4 transactions.
  - EIP-7623: transactions with a lot of calldata pay a minimum gas floor: 10 gas per zero byte and 40 gas per non-zero byte of calldata, on top of 21000. A transaction whose gas limit fit the old cost can now run out of gas.
  - EIP-7939: a new `CLZ` opcode.
  - The modexp precompile (0x05) costs more under EIP-7883: the minimum goes from 200 to 500 gas, and the complexity term is doubled.
  - Under EIP-7823, modexp calls with a base, exponent or modulus longer than 1024 bytes now fail.
  - The modexp changes are in the precompile code itself, so a runtime `config()` override would not undo them.
- **`eth_getLogs` and filter queries are limited to a range of 1024 blocks**, the upstream default. Indexers that scan from an old block in one query must split the range. Verified later: AC-37.
- **`session.set_keys` now checks the ownership proof.** Collators that call `setKeys(keys, 0x)` get `InvalidProof`. Generate keys and the proof with `author_rotateKeysWithOwner`. Verified later: AC-40.
- **`transaction_version` 3.** Offline signers and wallets that hard-code the transaction version must update. Otherwise their transactions are rejected after the runtime upgrade.
- **The Frontier database migrates from v2 to v3 on first start** with this binary (`6f4772f2`). The migration is one-way: an older binary refuses to open the migrated database. Back up `frontier/` before switching binaries.
- **Do not start a node with `--database auto`.** If the Frontier database needs the upgrade, `auto` may panic, or it may open an empty paritydb. In the paritydb case, old EVM blocks and transactions return nothing and no error is reported. With the default rocksdb, a failed upgrade stops the node with an error.
- **The relay token reserve is now Asset Hub (para 1000), not the relay chain.** This comes from ORML (upstream Asset Hub migration). Send DOT to peaq through Asset Hub, not directly from the relay chain.

### Known risks (accepted, not fixed on this branch)
- **XCM fees paid in PEAQ under-count `TotalIssuance`.** This is pre-existing: it has been on mainnet since `aecb104c` (2025-06), and the code is unchanged by this port. `BlockRewardWrapper` (`runtime/*/src/xcm_config.rs`) drops the `Credit` that `UsingComponents` hands it, and dropping it lowers `TotalIssuance`. It then gives the sinks a new imbalance of the same amount. The PEAQ itself is not lost, but `TotalIssuance` ends up lower than the sum of balances by every such fee. A separate work unit fixes this before the acceptance test AC-30 runs.
- **The EIP-3607 check now reads `AccountCodesMetadata`, not `AccountCodes`.** Frontier `92fc798` makes this change at `runner/stack.rs:240` and `frame/ethereum/src/lib.rs:610`. A contract stored without metadata is therefore not blocked as a transaction sender, and this branch has no migration that backfills the metadata. Exploiting this would require a private key for a contract address, which is not practical. The upgrade checks on mainnet that the key counts of the two maps are equal (check M-X5). If they differ, a separate work unit adds the backfill migration.
- **`FixedRateOfForeignAsset` has no unit tests.** `27d34bc5` removed the 399-line trader test file and did not port it. Coverage comes later, from AC-20.
- **Sha3FIPS256 precompile (0x400)** is priced with Frontier's default "laptop" weights (`516,915 + 2,019 × n` ref_time). The acceptance check AC-11 recorded "cannot be determined", because no benchmark in this runtime measures the precompile. The decision is open item 4 of the upgrade spec.
- **ORML `MultiCurrencyAdapter`** is disabled behind a feature gate that peaq does not enable (AC-79). Its known defects are mitigated by that gate, not fixed. This is an accepted risk: a runtime that enables the feature gets them back.
- **Frontier DB version detection**, in the fork's `client/db/src/kv/upgrade.rs` and upstream code:
  - A 5-column database without a version file that already has some number mappings is treated as v3, and the remaining backfill is skipped.
  - The 4-column fallback treats a read error as "no data".
  - The v1→v2 and v2→v3 copies drop iterator read errors but still write version 3.
  - ParityDB has the same "5 columns means v3" gap.
- **ParityDB** in the Frontier DB moved from 0.4 to 0.5.4. Whether 0.5 opens a 0.4 database has not been checked.
- **Zenlink**: `zenlink-stable-amm`, `zenlink-swap-router`, `zenlink-vault` and `zenlink-gauge` still point at `stable2503-4`. The node does not use them.
- **Dependency forks use mutable branch refs.** Cargo.lock pins the commits, but `cargo update` would follow the branch heads.
