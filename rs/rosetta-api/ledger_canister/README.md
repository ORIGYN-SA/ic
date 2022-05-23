
# Ledger

This package contains the implementation of the ICP ledger canister.

## Building
To build the ledger, 2 canister has to be build. The archive node canister must be build first because its code is embedded in the ledger canister.

 1. To build the archive node use the following commands:
   ```bash
   cargo build --target wasm32-unknown-unknown --bin ledger-archive-node-canister --release --target-dir build-output
   ic-cdk-optimizer --output ledger-archive-node-canister-new-min.wasm ./build-output/wasm32-unknown-unknown/release/ledger-archive-node-canister.wasm
   ```
 2. export the builded wasm in env var to be added to the ledger canister
   ```bash
   export LEDGER_ARCHIVE_NODE_CANISTER_WASM_PATH=./ledger-archive-node-canister-new-min.wasm
   ```
 3. Build the ledger canister
   ```bash
   build: cargo build --target wasm32-unknown-unknown --package ledger-canister --release --target-dir build-output
   ic-cdk-optimizer --output ledger-canister-new-min.wasm ./build-output/wasm32-unknown-unknown/release/ledger-canister.wasm
   ```

## Deploying locally

Follow the steps below to deploy your copy of the ledger canister to a local replica.

  1. Get a pre-built Ledger canister module and Candid interface files.
     ```sh
     export IC_VERSION=a7058d009494bea7e1d898a3dd7b525922979039
     curl -o ledger.wasm.gz https://download.dfinity.systems/ic/${IC_VERSION}/canisters/ledger-canister_notify-method.wasm.gz
     gunzip ledger.wasm.gz
     curl -o ledger.private.did https://raw.githubusercontent.com/dfinity/ic/${IC_VERSION}/rs/rosetta-api/ledger.did
     curl -o ledger.public.did https://raw.githubusercontent.com/dfinity/ic/${IC_VERSION}/rs/rosetta-api/ledger_canister/ledger.did
     ```
     NOTE: the `IC_VERSION` variable is a commit hash from the http://github.com/dfinity/ic repository.

  1. Make sure you use a recent version of DFX.
     If you don't have DFX installed, follow instructions on https://smartcontracts.org/ to install it.

  1. If you don't have a DFX project yet, follow these instructions to create a new DFX project:
     https://smartcontracts.org/docs/developers-guide/cli-reference/dfx-new.html

  1. Copy the file you obtained at the first step (`ledger.wasm`, `ledger.private.did`, `ledger.public.did`) into the root of your project.

  1. Add the following canister definition to the `dfx.json` file in your project:
     ```json
     {
       "canisters": {
         "ledger": {
           "type": "custom",
           "wasm": "ledger.wasm",
           "candid": "ledger.private.did"
         }
       }
     }
     ```

  1. Start local replica.
     ```sh
     dfx start --background
     ```

  1. Create a new identity that will work as a minting account:
     ```sh
     dfx identity new minter
     dfx identity use minter
     export MINT_ACC=$(dfx ledger account-id)
     ```
     Transfers from the minting account will create `Mint` transactions.
     Transfers to the minting account will create `Burn` transactions.

  1. Switch back to your default identity and record its ledger account identifier.
     ```sh
     dfx identity use default
     export LEDGER_ACC=$(dfx ledger account-id)
     ```

  1. Deploy the ledger canister to your network.
     ```sh
     dfx deploy ledger --argument '(record {minting_account = "'${MINT_ACC}'"; initial_values = vec { record { "'${LEDGER_ACC}'"; record { e8s=100_000_000_000 } }; }; send_whitelist = vec {}})'
     ```

  1. Update the canister definition in the `dfx.json` file to use the public Candid interface:
     ```json
     {
       "canisters": {
         "ledger": {
           "type": "custom",
           "wasm": "ledger.wasm",
           "candid": "ledger.public.did"
         }
       }
     }
     ```

  1. Check that the canister works:
     ```sh
     $ dfx canister call ledger account_balance '(record { account = '$(python3 -c 'print("vec{" + ";".join([str(b) for b in bytes.fromhex("'$LEDGER_ACC'")]) + "}")')' })'
     (record { e8s = 100_000_000_000 : nat64 })
     ```

Your local ICP ledger canister is up and running now.
You can now deploy other canisters that need to communicate with the ledger canister.




```
cargo build --target wasm32-unknown-unknown --package ledger-canister --release --target-dir build-output-3

# optimize
ic-cdk-optimizer --output ledger-canister-optimized.wasm ledger-canister.wasm
```

