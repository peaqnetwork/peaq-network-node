# EVM tracing node: WASM runtime overrides

Running a peaq node with EVM tracing enabled (`debug` / `trace` RPC methods) requires runtime overrides: historical runtimes recompiled with the `evm-tracing` feature. Without them, tracing calls fail on any block produced by a runtime version that predates tracing support.

All overrides for peaq mainnet are published as assets on a rolling GitHub release:

**https://github.com/peaqnetwork/peaq-network-node/releases/tag/evm-tracing-wasm-overrides**

The release also includes `peaq-raw.json`, the peaq mainnet raw chain spec.

## Download

With the GitHub CLI:

```bash
mkdir -p wasm-overrides
gh release download evm-tracing-wasm-overrides -R peaqnetwork/peaq-network-node -p '*.wasm' -D wasm-overrides
```

Or with curl, fetch individual files:

```bash
curl -LO https://github.com/peaqnetwork/peaq-network-node/releases/download/evm-tracing-wasm-overrides/peaq_runtime.compact.compressed.wasm.peaq.v0.0.112-evm.wasm
```

## Run

Build or use a node binary with the `evm-tracing` feature, then start it with:

```bash
peaq-node \
  --chain peaq-raw.json \
  --ethapi=debug,trace,txpool \
  --wasm-runtime-overrides=wasm-overrides \
  ... # your usual flags
```

The node selects the correct override per block by reading the `spec_version` embedded in each wasm file, so the whole folder can be passed as-is. Filenames are informational only.

## Updating

When a new runtime version ships, the matching tracing override is added to the same release, so the link above always has the full set.
