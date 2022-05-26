#!/bin/bash
set -euo pipefail

cargo build --target wasm32-unknown-unknown --release --bin ledger-canister/archive_node -p ledger-canister
# This is the worlds most primitive way of doing tree shaking, but it trims 18MB of the size of the canister
wasm2wat ../target/wasm32-unknown-unknown/release/ledger-archive-node-canister.wasm -o ../target/wasm32-unknown-unknown/release/ledger-archive-node-canister.wasm20220525fix.wat
wat2wasm ../target/wasm32-unknown-unknown/release/ledger-archive-node-canister.wasm20220525fix.wat -o ../target/wasm32-unknown-unknown/release/ledger-archive-node-canister.wasmr-min20220525fix.wasm
