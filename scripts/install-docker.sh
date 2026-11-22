#!/bin/sh
# Install Docker Engine on common Linux distributions (best-effort).
#
# Docker is OPTIONAL for CleanServe: it is only needed for `cleanserve stack` (MySQL, Redis, ...)
# and as a fallback to run Composer when no PHP with the phar extension / system composer exists.
#
# Usage:  sh scripts/install-docker.sh        (or: cleanserve setup docker)
set -eu

if command -v docker >/dev/null 2>&1 && docker info >/dev/null 2>&1; then
  echo "Docker is already installed and running."
  exit 0
fi

if [ "$(uname -s)" != "Linux" ]; then
  echo "This script supports Linux only. Install Docker Desktop: https://docs.docker.com/get-docker/"
  exit 1
fi

# Run privileged commands with sudo unless we already are root.
if [ "$(id -u)" -eq 0 ]; then
  SUDO=""
elif command -v sudo >/dev/null 2>&1; then
  SUDO="sudo"
else
  echo "Need root privileges: run as root or install sudo." >&2
  exit 1
fi

ID=""
if [ -f /etc/os-release ]; then
  # shellcheck disable=SC1091
  . /etc/os-release
fi

enable_service() {
  if command -v systemctl >/dev/null 2>&1; then
    $SUDO systemctl enable --now docker
  elif command -v rc-update >/dev/null 2>&1; then
    $SUDO rc-update add docker default || true
    $SUDO rc-service docker start || true
  else
    $SUDO service docker start || true
  fi
}

install_get_docker() {
  echo "Using Docker's official convenience script (get.docker.com)..."
  if ! command -v curl >/dev/null 2>&1; then
    echo "curl is required." >&2
    exit 1
  fi
  curl -fsSL https://get.docker.com | $SUDO sh
}

case "${ID}" in
  arch|cachyos|manjaro|endeavouros|garuda)
    echo "Installing via pacman..."
    $SUDO pacman -Sy --needed --noconfirm docker docker-compose
    enable_service
    ;;
  ubuntu|debian|linuxmint|pop)
    echo "Installing via apt..."
    $SUDO apt-get update
    $SUDO apt-get install -y docker.io docker-compose-v2 \
      || $SUDO apt-get install -y docker.io docker-compose-plugin \
      || $SUDO apt-get install -y docker.io docker-compose
    enable_service
    ;;
  fedora|rhel|centos|rocky|almalinux|nobara)
    echo "Installing via dnf/yum..."
    if command -v dnf >/dev/null 2>&1; then
      $SUDO dnf install -y moby-engine docker-compose || $SUDO dnf install -y docker docker-compose
    else
      $SUDO yum install -y docker docker-compose
    fi
    enable_service
    ;;
  opensuse*|sles)
    echo "Installing via zypper..."
    $SUDO zypper -n install docker docker-compose || $SUDO zypper -n install docker
    enable_service
    ;;
  alpine)
    echo "Installing via apk..."
    $SUDO apk add docker docker-cli-compose
    enable_service
    ;;
  gentoo)
    echo "Gentoo builds from source (this can take a while)..."
    $SUDO emerge --ask=n app-containers/docker app-containers/docker-compose
    enable_service
    ;;
  nixos)
    echo "NixOS: set 'virtualisation.docker.enable = true;' in configuration.nix and rebuild."
    exit 1
    ;;
  *)
    echo "Unknown distro '${ID}'."
    install_get_docker
    enable_service
    ;;
esac

# Let the current user talk to the daemon without sudo.
USER_NAME="${USER:-$(id -un)}"
if [ "$(id -u)" -ne 0 ] && ! id -nG "${USER_NAME}" 2>/dev/null | grep -qw docker; then
  echo "Adding ${USER_NAME} to the docker group (log out and back in, or run: newgrp docker)..."
  $SUDO usermod -aG docker "${USER_NAME}" || true
fi

if docker info >/dev/null 2>&1; then
  echo "Docker is ready."
else
  echo "Docker is installed but not reachable yet. Log out/in (docker group) or start the service,"
  echo "then run: cleanserve doctor"
fi
