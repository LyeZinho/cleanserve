#!/bin/sh
# Build cleanserve from this repo and install to ~/.local/bin (overrides GitHub release).
set -e
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export CARGO_TARGET_DIR="${ROOT}/target"

if [ -z "${CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER:-}" ] && ! command -v x86_64-linux-gnu-gcc >/dev/null 2>&1; then
  export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=gcc
fi

echo "Building cleanserve-cli (release)..."
cargo build -p cleanserve-cli --release

BIN="${ROOT}/target/release/cleanserve"
if [ ! -f "$BIN" ]; then
  echo "error: binary not found at $BIN" >&2
  exit 1
fi

DEST="${HOME}/.local/bin/cleanserve"
mkdir -p "$(dirname "$DEST")"
install -m 755 "$BIN" "$DEST"

echo "Installed: $DEST"
"$DEST" --version
