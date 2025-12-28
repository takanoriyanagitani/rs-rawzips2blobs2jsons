#!/bin/bash
set -euo pipefail

echo "Building all WASI binaries..."

# Build all binaries for the wasm32-wasip1 target using the release-wasi profile.
# The --bins flag ensures all binaries defined in Cargo.toml are built.
# The resulting .wasm files will be located in target/wasm32-wasip1/release-wasi/
cargo build --target wasm32-wasip1 --profile release-wasi --bins

echo "All WASI binaries have been built to target/wasm32-wasip1/release-wasi/"
echo "You can find them at: $(pwd)/target/wasm32-wasip1/release-wasi/"