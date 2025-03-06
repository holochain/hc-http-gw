#!/usr/bin/env bash

set -euo pipefail

cargo build --release --target wasm32-unknown-unknown -p integrity
cargo build --release --target wasm32-unknown-unknown -p coordinator1
cargo build --release --target wasm32-unknown-unknown -p coordinator2

pushd package/dna1 || exit 1
hc dna pack .
popd || exit 1

pushd package/dna2 || exit 1
hc dna pack .
popd || exit 1

pushd package/happ1 || exit 1
hc app pack .
popd || exit 1

pushd package/happ2 || exit 1
hc app pack .
popd || exit 1
