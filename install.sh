#!/bin/sh

set -e

BINARY_NAME="cleanserve"
REPO_URL="https://github.com/LyeZinho/cleanserve"
DOWNLOAD_URL_BASE="${REPO_URL}/releases/latest/download"

if [ -t 1 ]; then
  BOLD='\033[1m'
  RED='\033[0;31m'
  GREEN='\033[0;32m'
  BLUE='\033[0;34m'
  NC='\033[0m'
else
  BOLD=''
  RED=''
  GREEN=''
  BLUE=''
  NC=''
fi

info() { printf "${BLUE}info:${NC} %s\n" "$1"; }
warn() { printf "${BOLD}warn:${NC} %s\n" "$1"; }
error() { printf "${RED}error:${NC} %s\n" "$1" >&2; exit 1; }
success() { printf "${GREEN}success:${NC} %s\n" "$1"; }

cleanup() {
  if [ -n "${TMP_DIR}" ] && [ -d "${TMP_DIR}" ]; then
    rm -rf "${TMP_DIR}"
  fi
}

trap cleanup EXIT

printf "\n  ${BOLD}CleanServe Installer${NC}\n\n"

_detect() {
  OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
  ARCH="$(uname -m)"

  case "${OS}" in
    linux)       OS_NAME="linux" ;;
    darwin)      OS_NAME="darwin" ;;
    mingw*|msys*) OS_NAME="windows" ;;
    *)           error "Unsupported OS: ${OS}. For Windows, download the .zip from ${REPO_URL}/releases/latest" ;;
  esac

  case "${ARCH}" in
    x86_64|amd64)  ARCH_NAME="x86_64" ;;
    aarch64|arm64) ARCH_NAME="aarch64" ;;
    *)              error "Unsupported architecture: ${ARCH}" ;;
  esac
}

_detect

case "${OS_NAME}" in
  windows) EXTRACT="unzip -o" ;;
  *)       EXTRACT="tar -xzf" ;;
esac

info "Detected: ${OS_NAME} ${ARCH_NAME}"

if command -v curl >/dev/null 2>&1; then
  DOWNLOAD_CMD="curl -fsSL"
elif command -v wget >/dev/null 2>&1; then
  DOWNLOAD_CMD="wget -qO-"
else
  error "Neither curl nor wget found. Please install one of them."
fi

TMP_DIR="$(mktemp -d)"
if [ "${OS_NAME}" = "windows" ]; then
  EXT="zip"
  FILE_NAME="${BINARY_NAME}-${OS_NAME}-${ARCH_NAME}.${EXT}"
else
  EXT="tar.gz"
  FILE_NAME="${BINARY_NAME}-${OS_NAME}-${ARCH_NAME}.${EXT}"
fi
DOWNLOAD_URL="${DOWNLOAD_URL_BASE}/${FILE_NAME}"

info "Downloading ${BINARY_NAME}..."
if ! ${DOWNLOAD_CMD} "${DOWNLOAD_URL}" > "${TMP_DIR}/${FILE_NAME}"; then
  error "Failed to download ${DOWNLOAD_URL}"
fi

info "Extracting..."
${EXTRACT} -C "${TMP_DIR}" "${TMP_DIR}/${FILE_NAME}"

# Handle .exe on Windows
if [ "${OS_NAME}" = "windows" ]; then
  if [ -f "${TMP_DIR}/${BINARY_NAME}.exe" ]; then
    BIN_PATH="${TMP_DIR}/${BINARY_NAME}.exe"
  else
    error "Binary not found in archive."
  fi
else
  if [ ! -f "${TMP_DIR}/${BINARY_NAME}" ]; then
    error "Binary not found in archive."
  fi
  BIN_PATH="${TMP_DIR}/${BINARY_NAME}"
fi

INSTALL_DIR="/usr/local/bin"
if [ ! -w "${INSTALL_DIR}" ]; then
  INSTALL_DIR="${HOME}/.local/bin"
  mkdir -p "${INSTALL_DIR}"
fi

info "Installing to ${INSTALL_DIR}..."
if [ "${OS_NAME}" = "windows" ]; then
  cp "${BIN_PATH}" "${INSTALL_DIR}/${BINARY_NAME}.exe"
  chmod +x "${INSTALL_DIR}/${BINARY_NAME}.exe"
else
  mv "${BIN_PATH}" "${INSTALL_DIR}/${BINARY_NAME}"
  chmod +x "${INSTALL_DIR}/${BINARY_NAME}"
fi

INSTALLED_NAME="${BINARY_NAME}"
[ "${OS_NAME}" = "windows" ] && INSTALLED_NAME="${BINARY_NAME}.exe"

if "${INSTALL_DIR}/${INSTALLED_NAME}" --version >/dev/null 2>&1; then
  INSTALLED_VERSION=$("${INSTALL_DIR}/${INSTALLED_NAME}" --version | head -n 1)
  success "CleanServe ${INSTALLED_VERSION} installed successfully!"
else
  success "CleanServe installed successfully!"
fi

case ":${PATH}:" in
  *:"${INSTALL_DIR}":*) ;;
  *)
    warn "${INSTALL_DIR} is not in your PATH."

    SHELL_NAME="$(basename "${SHELL}")"
    case "${SHELL_NAME}" in
      zsh)  RC_FILE="~/.zshrc" ;;
      bash) RC_FILE="~/.bashrc" ;;
      fish) RC_FILE="~/.config/fish/config.fish" ;;
      *)    RC_FILE="your shell profile" ;;
    esac

    printf "\n  Add it to your PATH by adding this line to ${RC_FILE}:\n"
    printf "  ${BOLD}export PATH=\"${INSTALL_DIR}:\$PATH\"${NC}\n"
    ;;
esac

printf "\n  Get started:\n"
printf "    cd your-php-project\n"
printf "    ${BINARY_NAME} init\n"
printf "    ${BINARY_NAME} up\n\n"
