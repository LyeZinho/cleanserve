#!/usr/bin/env sh
set -e
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export CARGO_TARGET_DIR="$ROOT/target"
cargo build --release -p cleanserve-cli --manifest-path "$ROOT/Cargo.toml"
install -m 0755 "$ROOT/target/release/cleanserve" "${INSTALL_DIR:-$HOME/.local/bin}/cleanserve"
echo "Installed cleanserve to ${INSTALL_DIR:-$HOME/.local/bin}/cleanserve"
