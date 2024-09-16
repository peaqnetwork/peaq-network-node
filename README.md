# Peaq-network-node

## Getting Started

### Rust Setup

First, complete the [basic Rust setup instructions](./docs/rust-setup.md).

### Run

Currently, because we are moving to the parachain, we need to use parachain-launch to start the
parachain. Please refer to this project, [parachain-launch](https://github.com/peaqnetwork/parachain-launch)
, to more information.

### Build

The `cargo run` command will perform an initial build. Use the following command to build the node
without launching it:

```sh
cargo build --release
```

### Embedded Docs

Once the project has been built, the following command can be used to explore all parameters and
subcommands:

```sh
./target/release/peaq-node -h
```

## Run

The provided `cargo run` command will launch a temporary node and its state will be discarded after
you terminate the process. After the project has been built, there are other ways to launch the
node.

### Single-Node Development Chain

Because we are the parachain now, we don't support the Single-Node Development Chain. However, you can start the parachain
by parachain-launch.

### Connect with Polkadot-JS Apps Front-end

Once the node template is running locally, you can connect it with **Polkadot-JS Apps** front-end
to interact with your chain. [Click
here](https://polkadot.js.org/apps/#/explorer?rpc=ws://localhost:9944) connecting the Apps to your
local node template.

### Multi-Node Local Testnet

The same as the Single-Node Development Chain.

### Node

A blockchain node is an application that allows users to participate in a blockchain network.
Substrate-based blockchain nodes expose a number of capabilities:

- Networking: Substrate nodes use the [`libp2p`](https://libp2p.io/) networking stack to allow the
  nodes in the network to communicate with one another.
- Consensus: Blockchains must have a way to come to
  [consensus](https://docs.substrate.io/v3/advanced/consensus) on the state of the
  network. Substrate makes it possible to supply custom consensus engines and also ships with
  several consensus mechanisms that have been built on top of
  [Web3 Foundation research](https://research.web3.foundation/en/latest/polkadot/NPoS/index.html).
- RPC Server: A remote procedure call (RPC) server is used to interact with Substrate nodes.

There are several files in the `node` directory - take special note of the following:

- [`chain_spec.rs`](./node/src/chain_spec.rs): A
  [chain specification](https://docs.substrate.io/v3/runtime/chain-specs) is a
  source code file that defines a Substrate chain's initial (genesis) state. Chain specifications
  are useful for development and testing, and critical when architecting the launch of a
  production chain. Take note of the `development_config` and `testnet_genesis` functions, which
  are used to define the genesis state for the local development chain configuration. These
  functions identify some
  [well-known accounts](https://docs.substrate.io/v3/tools/subkey#well-known-keys)
  and use them to configure the blockchain's initial state.
- [`service.rs`](./node/src/service.rs): This file defines the node implementation. Take note of
  the libraries that this file imports and the names of the functions it invokes. In particular,
  there are references to consensus-related topics, such as the
  [longest chain rule](https://docs.substrate.io/v3/advanced/consensus#longest-chain-rule),
  the [Aura](https://docs.substrate.io/v3/advanced/consensus#aura) block authoring
  mechanism and the
  [GRANDPA](https://docs.substrate.io/v3/advanced/consensus#grandpa) finality
  gadget.

After the node has been [built](#build), refer to the embedded documentation to learn more about the
capabilities and configuration parameters that it exposes:

```shell
./target/release/peaq-node --help
```

### Runtime

In Substrate, the terms
"[runtime](https://docs.substrate.io/v3/getting-started/glossary#runtime)" and
"[state transition function](https://docs.substrate.io/v3/getting-started/glossary#state-transition-function-stf)"
are analogous - they refer to the core logic of the blockchain that is responsible for validating
blocks and executing the state changes they define. The Substrate project in this repository uses
the [FRAME](https://docs.substrate.io/v3/runtime/frame) framework to construct a
blockchain runtime. FRAME allows runtime developers to declare domain-specific logic in modules
called "pallets". At the heart of FRAME is a helpful
[macro language](https://docs.substrate.io/v3/runtime/macros) that makes it easy to
create pallets and flexibly compose them to create blockchains that can address
[a variety of needs](https://www.substrate.io/substrate-users/).

Review the [FRAME runtime implementation](./runtime/src/lib.rs) included in this template and note
the following:

- This file configures several pallets to include in the runtime. Each pallet configuration is
  defined by a code block that begins with `impl $PALLET_NAME::Config for Runtime`.
- The pallets are composed into a single runtime by way of the
  [`construct_runtime!`](https://crates.parity.io/frame_support/macro.construct_runtime.html)
  macro, which is part of the core
  [FRAME Support](https://docs.substrate.io/v3/runtime/frame#support-crate)
  library.

### Pallets

The runtime in this project is constructed using many FRAME pallets that ship with the
[core Substrate repository](https://github.com/paritytech/substrate/tree/master/frame) and a
template pallet that is [defined in the `pallets`](./pallets/template/src/lib.rs) directory.

A FRAME pallet is compromised of a number of blockchain primitives:

- Storage: FRAME defines a rich set of powerful
  [storage abstractions](https://docs.substrate.io/v3/runtime/storage) that makes
  it easy to use Substrate's efficient key-value database to manage the evolving state of a
  blockchain.
- Dispatchables: FRAME pallets define special types of functions that can be invoked (dispatched)
  from outside of the runtime in order to update its state.
- Events: Substrate uses [events and errors](https://docs.substrate.io/v3/runtime/events-and-errors)
  to notify users of important changes in the runtime.
- Errors: When a dispatchable fails, it returns an error.
- Config: The `Config` configuration interface is used to define the types and parameters upon
  which a FRAME pallet depends.

### Run in Docker

First, install [Docker](https://docs.docker.com/get-docker/) and
[Docker Compose](https://docs.docker.com/compose/install/).

Please use following command to run peaq-network-node parachian in the docker container connected with polkadot relaychain running in the PEAQ development environment.

#### PEAQ-Dev env

 ```bash
docker run -v peaq-dev-storage/chain-data -p 9944:9944 peaq/parachain:peaq-dev-v0.0.101 \
--parachain-id 2000 \
--chain ./node/src/chain-specs/peaq-dev-raw.json \
--base-path chain-data \
--port 30333 \
--rpc-port 9944 \
--rpc-external --rpc-cors=all \
--execution wasm \
-- \
--execution wasm \
--chain ./node/src/chain-specs/rococo-local-raw.json \
--port 30343 \
--rpc-port 9977
 ```

#### Krest env

 ```bash
docker run -v krest-storage:/chain-data -p 9944:9944 -p 9933:9933 peaq/parachain:krest-v0.0.7 \
--parachain-id 2241 \
--chain ./node/src/chain-specs/krest-raw.json \
--base-path chain-data \
--port 30333 \
--rpc-port 9944 \
--rpc-cors=all \
--execution wasm \
-- \
--execution wasm \
--chain ./node/src/chain-specs/kusama.json \
--port 30343 \
--sync warp \
--rpc-port 9977
 ```

#### Peaq env

 ```bash
docker run -v peaq-storage:/chain-data -p 9944:9944 peaq/parachain:peaq-v0.0.101 \
--parachain-id 3338 \
--chain ./node/src/chain-specs/peaq-raw.json \
--base-path chain-data \
--port 30333 \
--rpc-port 9944 \
--rpc-cors=all \
--execution wasm \
-- \
--execution wasm \
--port 30343 \
--sync warp \
--rpc-port 9977
 ```

Once you run this command, wait for a few second. Now the peaq parachian should be running in the docker container that is connected to relaychain running in PEAQ dev environament.

### Run on your local machine with Docker commands

Please follow the steps given below to run peaq-network-node parachian on your local machine connected with polkadot relaychain running in the PEAQ development environment. It is assumed that you have already downloaded the source code
for peaq-network-node from the git repository

1. Download the source code from the git repository:

#### PEAQ-Dev env
```bash
git clone --branch peaq-dev-v0.0.101 https://github.com/peaqnetwork/peaq-network-node.git
```

#### Krest env
```bash
git clone --branch krest-v0.0.7 https://github.com/peaqnetwork/peaq-network-node.git
```

#### Peaq env
```bash
git clone --branch peaq-v0.0.101 https://github.com/peaqnetwork/peaq-network-node.git
```

2. CD into the peaq-network-node directory:
```bash
cd peaq-network-node
```

3. Create the following folder:
```bash
mkdir ./.local
```

The folder .local is needed because that is where data such as session keys are stored for validators. Also we bind mount from the container folder /root/.local to the host machine project root folder ./.local.

4. Compile the source code:
```bash
./scripts/docker_run.sh cargo build --release
```

5. Now run the following script to start a peaq-network-node parachain that will connect to the polkadot relay chain running in peaq development environment:

```bash
# PEAQ-Dev env
./scripts/docker_run.sh \
./target/release/peaq-node \
--parachain-id 2000 \
--chain ./node/src/chain-specs/peaq-dev-raw.json \
--base-path chain-data \
--port 30333 \
--rpc-port 9944 \
--execution wasm \
-- \
--execution wasm \
--chain ./node/src/chain-specs/rococo-local-raw.json \
--port 30343 \
--rpc-port 9977
```

```bash
# Krest env
./scripts/docker_run.sh \
./target/release/peaq-node \
--parachain-id 2241 \
--chain ./node/src/chain-specs/krest-raw.json \
--base-path chain-data \
--port 30333 \
--rpc-port 9944 \
--execution wasm \
-- \
--execution wasm \
--chain ./node/src/chain-specs/kusama.json \
--port 30343 \
--pruning=16 \
--sync warp \
--rpc-port 9977
```

```bash
# Peaq env
./scripts/docker_run.sh \
./target/release/peaq-node \
--parachain-id 3338 \
--chain ./node/src/chain-specs/peaq-raw.json \
--base-path chain-data \
--port 30333 \
--rpc-port 9944 \
--execution wasm \
-- \
--execution wasm \
--port 30343 \
--pruning=16 \
--sync warp \
--rpc-port 9977

This command will first compile your code (if it is not already compiled), and then start a peaq-network-node parachain. The node running on your local machine will take sometime to sync up. Make sure that the parachain blocks are generated.

You can also replace the default command by appending your own. A few useful ones are as follows:

```bash
# Check whether the code is compilable
./scripts/docker_run.sh cargo check
```

### Parachain Launch

1. Please use the [peaq-node-builder](https://github.com/peaqnetwork/peaq-node-builder) to build the project

2. Create the docker images
```sh
docker build -f scripts/Dockerfile.parachain-launch -t peaqtest .
```

3. Please use the [parachain-launch](https://github.com/peaqnetwork/parachain-launch) to run the local parachain
