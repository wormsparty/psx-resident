#!/usr/bin/env bash
# Compile le jeu en WebAssembly et génère le glue JS dans web/pkg.
# Prérequis : rustup target add wasm32-unknown-unknown
#             cargo install wasm-bindgen-cli --version 0.2.129
set -euo pipefail
cd "$(dirname "$0")"
cargo build --release --lib --target wasm32-unknown-unknown
wasm-bindgen --target web --no-typescript --out-dir web/pkg \
  target/wasm32-unknown-unknown/release/psx_resident.wasm
echo "OK. Lancez un serveur : python3 -m http.server -d web 8080"
echo "puis ouvrez http://localhost:8080"
