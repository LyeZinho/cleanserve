#!/bin/sh
# CleanServe installer.
#
#   curl -fsSL https://raw.githubusercontent.com/LyeZinho/cleanserve/main/install.sh | sh
#   curl -fsSL .../install.sh | sh -s -- --version v0.4.0
#
# What it does
#   1. Downloads the release for your platform and verifies its SHA-256.
#   2. Checks the new binary runs on this machine.
#   3. Finds CleanServe copies already installed. Any copy older than MIN_COMPATIBLE is
#      deleted (no migration, no patching of old versions), then the new one is installed.
#
# Options (or the matching environment variable):
#   -v, --version <tag>   Install a specific release, e.g. v0.4.0   (CLEANSERVE_VERSION)
#   -d, --dir <path>      Install directory                          (CLEANSERVE_INSTALL_DIR)
#       --docker          Also install Docker (optional)             (CLEANSERVE_INSTALL_DOCKER=1)
#       --purge           Also delete ~/.cleanserve (PHP runtimes, caches, certificates)
#       --no-verify       Skip the SHA-256 check (not recommended)
#       --uninstall       Remove CleanServe (add --purge to remove its data too)
#   -h, --help            Show this help
#
# Default directory: /usr/local/bin if writable, otherwise ~/.local/bin.
# CLEANSERVE_BASE_URL overrides the download location (mirrors, offline testing).
set -eu

# Oldest version that is still compatible with a fresh install. Anything older is removed.
# Raise this when a release stops working with copies installed before it.
MIN_COMPATIBLE="0.4.0"

BINARY_NAME="cleanserve"
REPO_URL="https://github.com/LyeZinho/cleanserve"
DATA_DIR="${HOME}/.cleanserve"

VERSION="${CLEANSERVE_VERSION:-latest}"
INSTALL_DIR="${CLEANSERVE_INSTALL_DIR:-}"
WANT_DOCKER="${CLEANSERVE_INSTALL_DOCKER:-}"
VERIFY=1
UNINSTALL=0
PURGE=0

if [ -t 1 ]; then
  BOLD='\033[1m'; RED='\033[0;31m'; GREEN='\033[0;32m'; BLUE='\033[0;34m'; YELLOW='\033[0;33m'; NC='\033[0m'
else
  BOLD=''; RED=''; GREEN=''; BLUE=''; YELLOW=''; NC=''
fi

info()    { printf "${BLUE}info:${NC} %s\n" "$1"; }
warn()    { printf "${YELLOW}warn:${NC} %s\n" "$1"; }
error()   { printf "${RED}error:${NC} %s\n" "$1" >&2; exit 1; }
success() { printf "${GREEN}success:${NC} %s\n" "$1"; }

usage() {
  cat <<'EOF'
CleanServe installer

Usage: install.sh [options]

  -v, --version <tag>   Install a specific release, e.g. v0.4.0
  -d, --dir <path>      Install directory (default: /usr/local/bin, else ~/.local/bin)
      --docker          Also install Docker (optional)
      --purge           Also delete ~/.cleanserve (PHP runtimes, caches, certificates)
      --no-verify       Skip the SHA-256 check (not recommended)
      --uninstall       Remove CleanServe (add --purge to remove its data too)
  -h, --help            Show this help

Copies older than the minimum compatible version are deleted and replaced.
EOF
  exit 0
}

while [ $# -gt 0 ]; do
  case "$1" in
    -v|--version) [ $# -ge 2 ] || error "$1 needs a value"; VERSION="$2"; shift 2 ;;
    -d|--dir)     [ $# -ge 2 ] || error "$1 needs a value"; INSTALL_DIR="$2"; shift 2 ;;
    --docker)     WANT_DOCKER=1; shift ;;
    --purge)      PURGE=1; shift ;;
    --no-verify)  VERIFY=0; shift ;;
    --uninstall)  UNINSTALL=1; shift ;;
    -h|--help)    usage ;;
    *)            error "Unknown option: $1 (try --help)" ;;
  esac
done

TMP_DIR=""
cleanup() { if [ -n "${TMP_DIR}" ] && [ -d "${TMP_DIR}" ]; then rm -rf "${TMP_DIR}"; fi; return 0; }
trap cleanup EXIT INT TERM

# ---- platform ---------------------------------------------------------------------------------

OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"

case "${OS}" in
  linux)        OS_NAME="linux" ;;
  darwin)       OS_NAME="darwin" ;;
  mingw*|msys*) OS_NAME="windows" ;;
  *)            error "Unsupported OS: ${OS}. For Windows download the .zip from ${REPO_URL}/releases/latest" ;;
esac
case "${ARCH}" in
  x86_64|amd64)  ARCH_NAME="x86_64" ;;
  aarch64|arm64) ARCH_NAME="aarch64" ;;
  *)             error "Unsupported architecture: ${ARCH}" ;;
esac

if [ "${OS_NAME}" = "windows" ]; then
  EXT="zip"; BIN_FILE="${BINARY_NAME}.exe"
else
  EXT="tar.gz"; BIN_FILE="${BINARY_NAME}"
fi

if [ -z "${INSTALL_DIR}" ]; then
  if [ -w /usr/local/bin ]; then INSTALL_DIR="/usr/local/bin"; else INSTALL_DIR="${HOME}/.local/bin"; fi
fi
TARGET="${INSTALL_DIR}/${BIN_FILE}"

# ---- helpers ----------------------------------------------------------------------------------

# Succeeds when version $1 >= $2 (both X.Y.Z, any suffix is ignored).
version_ge() {
  awk -v a="$1" -v b="$2" 'BEGIN {
    split(a, x, /[.+-]/); split(b, y, /[.+-]/)
    for (i = 1; i <= 3; i++) {
      if (x[i] + 0 > y[i] + 0) exit 0
      if (x[i] + 0 < y[i] + 0) exit 1
    }
    exit 0
  }'
}

# Print the X.Y.Z that binary $1 reports ("cleanserve X.Y.Z"), or nothing if it is not
# CleanServe / does not run.
reported_version() {
  _out=""
  if command -v timeout >/dev/null 2>&1; then
    _out="$(timeout 5 "$1" --version 2>/dev/null </dev/null | head -n 1)" || _out=""
  else
    _out="$("$1" --version 2>/dev/null </dev/null | head -n 1)" || _out=""
  fi
  printf '%s\n' "${_out}" | sed -n 's/^cleanserve \([0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*\).*/\1/p' | head -n 1
}

# Newline-separated directories where a CleanServe binary might live (deduplicated).
candidate_dirs() {
  _seen="
"
  _list="${INSTALL_DIR}:${PATH:-}:${HOME}/.local/bin:/usr/local/bin:${HOME}/.cargo/bin"
  _old_ifs="${IFS}"; IFS=:
  for _d in ${_list}; do
    [ -n "${_d}" ] || continue
    _d="${_d%/}"
    case "${_seen}" in *"
${_d}
"*) continue ;; esac
    _seen="${_seen}${_d}
"
    printf '%s\n' "${_d}"
  done
  IFS="${_old_ifs}"
}

remove_data() {
  if [ -d "${DATA_DIR}" ]; then
    rm -rf "${DATA_DIR}" && success "Removed ${DATA_DIR}"
  else
    info "No data directory at ${DATA_DIR}"
  fi
}

# ---- uninstall --------------------------------------------------------------------------------

printf "\n  ${BOLD}CleanServe installer${NC}\n\n"

if [ "${UNINSTALL}" = "1" ]; then
  FOUND=0
  candidate_dirs > "${TMPDIR:-/tmp}/.cleanserve-dirs.$$"
  while IFS= read -r d; do
    f="${d}/${BIN_FILE}"
    [ -f "${f}" ] || continue
    [ -n "$(reported_version "${f}")" ] || continue
    FOUND=1
    if rm -f "${f}" 2>/dev/null; then success "Removed ${f}"; else warn "Could not remove ${f} (permissions): sudo rm '${f}'"; fi
  done < "${TMPDIR:-/tmp}/.cleanserve-dirs.$$"
  rm -f "${TMPDIR:-/tmp}/.cleanserve-dirs.$$"
  [ "${FOUND}" = "1" ] || warn "No CleanServe binary found."
  if [ "${PURGE}" = "1" ]; then
    remove_data
  else
    printf "  PHP runtimes and caches stay in %s (add --purge to delete them).\n" "${DATA_DIR}"
  fi
  printf "\n"
  exit 0
fi

# ---- download ---------------------------------------------------------------------------------

if command -v curl >/dev/null 2>&1; then
  fetch() { curl -fsSL "$1" -o "$2"; }
elif command -v wget >/dev/null 2>&1; then
  fetch() { wget -qO "$2" "$1"; }
else
  error "Neither curl nor wget found. Please install one of them."
fi

sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | cut -d' ' -f1
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | cut -d' ' -f1
  else
    return 1
  fi
}

if [ -n "${CLEANSERVE_BASE_URL:-}" ]; then
  BASE_URL="${CLEANSERVE_BASE_URL%/}"
elif [ "${VERSION}" = "latest" ]; then
  BASE_URL="${REPO_URL}/releases/latest/download"
else
  BASE_URL="${REPO_URL}/releases/download/${VERSION}"
fi

FILE_NAME="${BINARY_NAME}-${OS_NAME}-${ARCH_NAME}.${EXT}"
TMP_DIR="$(mktemp -d)"

info "Detected ${OS_NAME}/${ARCH_NAME}; installing ${VERSION} to ${INSTALL_DIR}"
info "Downloading ${FILE_NAME}..."
if ! fetch "${BASE_URL}/${FILE_NAME}" "${TMP_DIR}/${FILE_NAME}"; then
  error "Download failed: ${BASE_URL}/${FILE_NAME}
       Check the version, your connection, or whether this platform has a prebuilt release.
       You can always build from source: git clone ${REPO_URL} && sh cleanserve/scripts/install-local.sh"
fi

if [ "${VERIFY}" = "1" ]; then
  if fetch "${BASE_URL}/SHA256SUMS" "${TMP_DIR}/SHA256SUMS" 2>/dev/null; then
    EXPECTED="$(awk -v f="${FILE_NAME}" '{n=$2; sub(/^\*/, "", n); if (n == f) print $1}' "${TMP_DIR}/SHA256SUMS" | head -n 1)"
    if [ -z "${EXPECTED}" ]; then
      warn "${FILE_NAME} is not listed in SHA256SUMS; skipping verification."
    elif ACTUAL="$(sha256_of "${TMP_DIR}/${FILE_NAME}")"; then
      [ "${ACTUAL}" = "${EXPECTED}" ] || error "Checksum mismatch for ${FILE_NAME} (expected ${EXPECTED}, got ${ACTUAL}). Aborting."
      info "Checksum verified."
    else
      warn "No sha256sum/shasum available; skipping verification."
    fi
  else
    warn "This release has no SHA256SUMS; skipping verification."
  fi
fi

info "Extracting..."
mkdir -p "${TMP_DIR}/x"
if [ "${EXT}" = "zip" ]; then
  command -v unzip >/dev/null 2>&1 || error "unzip is required on Windows."
  unzip -qo "${TMP_DIR}/${FILE_NAME}" -d "${TMP_DIR}/x" || error "Extraction failed (corrupt download or no disk space in ${TMPDIR:-/tmp})."
else
  tar -xzf "${TMP_DIR}/${FILE_NAME}" -C "${TMP_DIR}/x" || error "Extraction failed (corrupt download or no disk space in ${TMPDIR:-/tmp})."
fi
NEW_BIN="${TMP_DIR}/x/${BIN_FILE}"
[ -f "${NEW_BIN}" ] || error "Binary not found in archive."
chmod +x "${NEW_BIN}"

# Prove the new binary works here BEFORE touching anything that is already installed.
NEW_VERSION="$(reported_version "${NEW_BIN}")"
[ -n "${NEW_VERSION}" ] || error "The downloaded binary does not run on this system; nothing was changed."
STALE_RELEASE=0
if ! version_ge "${NEW_VERSION}" "${MIN_COMPATIBLE}"; then
  # Deleting an old copy to install another one that is just as old would only be noise.
  STALE_RELEASE=1
  warn "The ${VERSION} release only contains CleanServe ${NEW_VERSION}; ${MIN_COMPATIBLE} or newer has not been published yet."
  warn "It will be installed as-is. For the current code: git clone ${REPO_URL} && sh cleanserve/scripts/install-local.sh"
fi

# ---- remove incompatible installs -------------------------------------------------------------

mkdir -p "${INSTALL_DIR}" 2>/dev/null || error "Cannot create ${INSTALL_DIR}. Try --dir <writable path> or run with sudo."
[ -w "${INSTALL_DIR}" ] || error "${INSTALL_DIR} is not writable. Try --dir ~/.local/bin or run with sudo."

REPLACED_FROM=""
DIRS_FILE="${TMP_DIR}/dirs"
candidate_dirs > "${DIRS_FILE}"
while IFS= read -r d; do
  f="${d}/${BIN_FILE}"
  [ -f "${f}" ] || continue
  old="$(reported_version "${f}")"

  if [ "${f}" = "${TARGET}" ]; then
    # Replaced below either way; just say what happens to it.
    if [ -z "${old}" ]; then
      warn "${f} does not look like a working CleanServe; it will be replaced."
    elif [ "${STALE_RELEASE}" = "1" ] || version_ge "${old}" "${MIN_COMPATIBLE}"; then
      REPLACED_FROM="${old}"
    else
      info "Found CleanServe ${old} at ${f}: older than ${MIN_COMPATIBLE}, removing and reinstalling."
      REPLACED_FROM="${old}"
      rm -f "${f}"
    fi
    continue
  fi

  # Other copies: never touch something that is not CleanServe.
  if [ -z "${old}" ]; then
    warn "${f} exists but is not a recognisable CleanServe; leaving it alone."
  elif [ "${STALE_RELEASE}" = "1" ] || version_ge "${old}" "${MIN_COMPATIBLE}"; then
    :  # compatible copy (or nothing better to offer), keep it; the PATH check at the end warns about shadowing
  elif rm -f "${f}" 2>/dev/null; then
    info "Removed outdated CleanServe ${old} at ${f}."
  else
    warn "Outdated CleanServe ${old} at ${f} could not be removed (permissions): sudo rm '${f}'"
  fi
done < "${DIRS_FILE}"

# ---- install ----------------------------------------------------------------------------------

# Copy next to the target and rename: atomic, and safe even if an old binary is still running.
cp "${NEW_BIN}" "${INSTALL_DIR}/.${BIN_FILE}.new"
chmod +x "${INSTALL_DIR}/.${BIN_FILE}.new"
mv -f "${INSTALL_DIR}/.${BIN_FILE}.new" "${TARGET}"
case "${BIN_FILE}" in *.exe) ;; *) ln -sf "${BIN_FILE}" "${INSTALL_DIR}/cs" 2>/dev/null || true ;; esac  # short alias

INSTALLED="$(reported_version "${TARGET}")"
[ -n "${INSTALLED}" ] || error "The installed binary does not run (${TARGET})."
if [ -n "${REPLACED_FROM}" ] && [ "${REPLACED_FROM}" != "${INSTALLED}" ]; then
  success "CleanServe ${REPLACED_FROM} -> ${INSTALLED} (${TARGET})"
else
  success "CleanServe ${INSTALLED} installed to ${TARGET}"
fi

if [ "${PURGE}" = "1" ]; then
  remove_data
fi

# A server started by an old copy keeps running with the old code until it is stopped.
if command -v pgrep >/dev/null 2>&1 && pgrep -x "${BINARY_NAME}" >/dev/null 2>&1; then
  warn "A CleanServe server is still running with the previous version. Stop it (Ctrl+C in its terminal, or: cleanserve down) and start it again."
fi

# ---- optional Docker --------------------------------------------------------------------------

case "${WANT_DOCKER}" in
  1|yes|true)
    info "Installing Docker (optional)..."
    "${TARGET}" setup docker || warn "Docker install failed — retry later with: cleanserve setup docker"
    ;;
esac

# ---- PATH -------------------------------------------------------------------------------------

case ":${PATH}:" in
  *:"${INSTALL_DIR}":*) ;;
  *)
    warn "${INSTALL_DIR} is not in your PATH."
    case "$(basename "${SHELL:-sh}")" in
      fish) printf "\n  Add it with:\n  ${BOLD}fish_add_path %s${NC}\n" "${INSTALL_DIR}" ;;
      zsh)  printf "\n  Add this line to ~/.zshrc:\n  ${BOLD}export PATH=\"%s:\$PATH\"${NC}\n" "${INSTALL_DIR}" ;;
      bash) printf "\n  Add this line to ~/.bashrc:\n  ${BOLD}export PATH=\"%s:\$PATH\"${NC}\n" "${INSTALL_DIR}" ;;
      *)    printf "\n  Add this to your shell profile:\n  ${BOLD}export PATH=\"%s:\$PATH\"${NC}\n" "${INSTALL_DIR}" ;;
    esac
    ;;
esac

ACTIVE="$(command -v "${BINARY_NAME}" 2>/dev/null || true)"
if [ -n "${ACTIVE}" ] && [ "${ACTIVE}" != "${TARGET}" ]; then
  warn "'${BINARY_NAME}' in your PATH resolves to ${ACTIVE}, not ${TARGET}. Put ${INSTALL_DIR} first in PATH or remove that copy."
fi

printf "\n  Check your setup:  ${BOLD}cleanserve doctor${NC}\n"
printf "  Get started:\n"
printf "    cd your-php-project\n"
printf "    cleanserve init\n"
printf "    cleanserve up\n\n"
