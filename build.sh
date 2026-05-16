#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

if ! command -v cargo &>/dev/null; then
  if [ -x "$HOME/.cargo/bin/cargo" ]; then
    export PATH="$HOME/.cargo/bin:$PATH"
  else
    echo "Error: cargo not found. Install Rust from https://rustup.rs/" >&2
    exit 1
  fi
fi

echo "Building cep (release)..."
cargo build --release

echo "Build complete: $SCRIPT_DIR/target/release/cep"
