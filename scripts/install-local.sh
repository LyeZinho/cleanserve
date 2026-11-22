#!/bin/sh
# Build CleanServe from source and install it. Use this after changing the code.
#
# Usage:  sh scripts/install-local.sh [--debug]
#   INSTALL_DIR=/usr/local/bin sh scripts/install-local.sh     # custom destination
set -eu

PROFILE="release"
PROFILE_FLAG="--release"
for arg in "$@"; do
  case "$arg" in
    --debug) PROFILE="debug"; PROFILE_FLAG="" ;;
    -h|--help)
      sed -n '2,6p' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *) echo "Unknown option: $arg (try --help)" >&2; exit 1 ;;
  esac
done

if ! command -v cargo >/dev/null 2>&1; then
  echo "error: cargo not found. Install Rust first: https://rustup.rs" >&2
  exit 1
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEST_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

# Some setups have no `x86_64-linux-gnu-gcc` cross-linker name: fall back to plain `gcc`.
if [ -z "${CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER:-}" ] && ! command -v x86_64-linux-gnu-gcc >/dev/null 2>&1; then
  export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=gcc
fi

echo "Building cleanserve ($PROFILE)..."
# shellcheck disable=SC2086
cargo build $PROFILE_FLAG -p cleanserve-cli --manifest-path "$ROOT/Cargo.toml" --target-dir "$ROOT/target"

mkdir -p "$DEST_DIR"
install -m 0755 "$ROOT/target/$PROFILE/cleanserve" "$DEST_DIR/cleanserve"
echo "Installed: $("$DEST_DIR/cleanserve" --version) -> $DEST_DIR/cleanserve"

# A stale copy earlier in PATH would silently keep running the old version.
ACTIVE="$(command -v cleanserve 2>/dev/null || true)"
if [ -z "$ACTIVE" ]; then
  echo "note: $DEST_DIR is not in your PATH. Add it, e.g.: export PATH=\"$DEST_DIR:\$PATH\""
elif [ "$ACTIVE" != "$DEST_DIR/cleanserve" ]; then
  echo "warning: 'cleanserve' in your PATH resolves to $ACTIVE, not the one just installed."
  echo "         Remove it or put $DEST_DIR first in PATH."
fi
