# Substrate Node Template

A new FRAME-based Substrate node, ready for hacking.

## Build

Install Rust:

```bash
curl https://sh.rustup.rs -sSf | sh
```

Initialize your Wasm Build environment:

```bash
./scripts/init.sh
```

Build Wasm and native code:

```bash
cargo build --release
```

## Run

### Single Node Development Chain

Purge any existing developer chain state:

```bash
./target/release/dock-testnet purge-chain --dev
```

Start a development chain with:

```bash
./target/release/dock-testnet --dev
```

Detailed logs may be shown by running the node with the following environment variables set: `RUST_LOG=debug RUST_BACKTRACE=1 cargo run -- --dev`.

### Multi-Node Local Testnet

If you want to see the multi-node consensus algorithm in action locally, then you can create a local testnet with two validator nodes for Alice and Bob, who are the initial authorities of the genesis chain that have been endowed with testnet units.

Optionally, give each node a name and expose them so they are listed on the Polkadot [telemetry site](https://telemetry.polkadot.io/#/Local%20Testnet).

You'll need two terminal windows open.

We'll start Alice's substrate node first on default TCP port 30333 with her chain database stored locally at `/tmp/alice`. The bootnode ID of her node is `QmRpheLN4JWdAnY7HGJfWFNbfkQCb6tFf4vvA6hgjMZKrR`, which is generated from the `--node-key` value that we specify below:

```bash
cargo run -- \
  --base-path /tmp/alice \
  --chain=local \
  --alice \
  --node-key 0000000000000000000000000000000000000000000000000000000000000001 \
  --telemetry-url ws://telemetry.polkadot.io:1024 \
  --validator
```

In the second terminal, we'll start Bob's substrate node on a different TCP port of 30334, and with his chain database stored locally at `/tmp/bob`. We'll specify a value for the `--bootnodes` option that will connect his node to Alice's bootnode ID on TCP port 30333:

```bash
cargo run -- \
  --base-path /tmp/bob \
  --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/QmRpheLN4JWdAnY7HGJfWFNbfkQCb6tFf4vvA6hgjMZKrR \
  --chain=local \
  --bob \
  --port 30334 \
  --telemetry-url ws://telemetry.polkadot.io:1024 \
  --validator
```

Additional CLI usage options are available and may be shown by running `cargo run -- --help`.

## Polkadot-js UI
To use this chain from [polkadot-js UI](https://polkadot.js.org/apps), some structures need to be created in the `Settings > Developer` section. 
The structures can be found in [developer.json file](./developer.json).

## Docker image for push to release node.
The docker image runs the chain with id `remdev`. The chain runs in development mode and has only one authority. 
Contact the dev team to get the secret phrase for authority and root key. Following is the RPC call for adding keys
```
curl http://localhost:9933 -H "Content-Type:application/json;charset=utf-8" -d \
  '{
    "jsonrpc":"2.0",
    "id":1,
    "method":"author_insertKey",
    "params": [
      "<aura/gran>",
      "<mnemonic phrase>",
      "<public key>"
    ]
  }'
```
The keys are currently added manually on the machine.


To clear the chain state, run
```
./target/release/dock-testnet purge-chain --dev --chain remdev -y
```

To run the `remdev` chain, run
```
./target/release/dock-testnet --dev --chain remdev
```

TODO: The clear state and key storing should be done (conditional) on the run time arguments. The secret phrase should be a runtime arg as well.

Storage directory for node is
```
/root/.local/share/node-template/chains/remdev
``` 

## Dev tips
1. For faster builds during testing use flag `SKIP_WASM_BUILD=1`. This will not generate WASM but only native code. 
1. To use `println!` like Rust in `decl_module`'s functions, run the test or binary with flag `SKIP_WASM_BUILD=1` 

## Advanced: Generate Your Own Substrate Node Template

A substrate node template is always based on a certain version of Substrate. You can inspect it by
opening [Cargo.toml](Cargo.toml) and see the template referred to a specific Substrate commit(
`rev` field), branch, or version.

You can generate your own Substrate node-template based on a particular Substrate
version/commit by running following commands:

```bash
# git clone from the main Substrate repo
git clone https://github.com/paritytech/substrate.git
cd substrate

# Switch to a particular branch or commit of the Substrate repo your node-template based on
git checkout <branch/tag/sha1>

# Run the helper script to generate a node template.
# This script compiles Substrate and takes a while to complete. It takes a relative file path
#   from the current dir. to output the compressed node template.
.maintain/node-template-release.sh ../node-template.tar.gz
```

Noted though you will likely get faster and more thorough support if you stick with the releases
provided in this repository.

