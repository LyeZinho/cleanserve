#!/bin/sh

set -e

BINARY_NAME="cleanserve"
VERSION="0.1.0"
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

OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"

case "${OS}" in
  linux) OS_NAME="linux" ;;
  darwin) OS_NAME="darwin" ;;
  msys*|cygwin*|mingw*|nt)
    error "Windows detected. Please use the Windows installer or download the binary directly from ${REPO_URL}/releases."
    ;;
  *) error "Unsupported OS: ${OS}" ;;
esac

case "${ARCH}" in
  x86_64|amd64) ARCH_NAME="x86_64" ;;
  aarch64|arm64) ARCH_NAME="aarch64" ;;
  *) error "Unsupported architecture: ${ARCH}" ;;
esac

info "Detected: ${OS_NAME} ${ARCH_NAME}"

if command -v curl >/dev/null 2>&1; then
  DOWNLOAD_CMD="curl -fsSL"
elif command -v wget >/dev/null 2>&1; then
  DOWNLOAD_CMD="wget -qO-"
else
  error "Neither curl nor wget found. Please install one of them."
fi

if ! command -v tar >/dev/null 2>&1; then
  error "tar is required but not found."
fi

TMP_DIR="$(mktemp -d)"
FILE_NAME="${BINARY_NAME}-${OS_NAME}-${ARCH_NAME}.tar.gz"
DOWNLOAD_URL="${DOWNLOAD_URL_BASE}/${FILE_NAME}"

info "Downloading ${BINARY_NAME} v${VERSION}..."
if ! ${DOWNLOAD_CMD} "${DOWNLOAD_URL}" > "${TMP_DIR}/${FILE_NAME}"; then
  error "Failed to download ${DOWNLOAD_URL}"
fi

info "Extracting..."
tar -xzf "${TMP_DIR}/${FILE_NAME}" -C "${TMP_DIR}"
if [ ! -f "${TMP_DIR}/${BINARY_NAME}" ]; then
  error "Binary not found in archive."
fi

INSTALL_DIR="/usr/local/bin"
if [ ! -w "${INSTALL_DIR}" ]; then
  INSTALL_DIR="${HOME}/.local/bin"
  mkdir -p "${INSTALL_DIR}"
fi

info "Installing to ${INSTALL_DIR}/${BINARY_NAME}..."
mv "${TMP_DIR}/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"
chmod +x "${INSTALL_DIR}/${BINARY_NAME}"

if ! command -v "${BINARY_NAME}" >/dev/null 2>&1 && [ ! -x "${INSTALL_DIR}/${BINARY_NAME}" ]; then
  error "Installation failed: ${BINARY_NAME} not found."
fi

if "${INSTALL_DIR}/${BINARY_NAME}" --version >/dev/null 2>&1; then
  INSTALLED_VERSION=$("${INSTALL_DIR}/${BINARY_NAME}" --version | head -n 1)
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
