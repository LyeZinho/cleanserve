#!/bin/sh
# Install Docker Engine on common Linux distributions (best-effort).
# CleanServe uses Docker for Composer and optional stack services (MySQL, Redis, …).
set -e

if command -v docker >/dev/null 2>&1 && docker info >/dev/null 2>&1; then
  echo "Docker is already installed and running."
  exit 0
fi

if [ "$(uname -s)" != "Linux" ]; then
  echo "This script supports Linux only. Install Docker Desktop from https://docs.docker.com/get-docker/"
  exit 1
fi

if [ -f /etc/os-release ]; then
  # shellcheck disable=SC1091
  . /etc/os-release
else
  ID=""
fi

install_get_docker() {
  echo "Using Docker official convenience script (get.docker.com)..."
  curl -fsSL https://get.docker.com | sh
}

case "${ID}" in
  arch|cachyos|manjaro|endeavouros|garuda)
    echo "Installing via pacman (docker)..."
    sudo pacman -Sy --needed docker docker-compose
    sudo systemctl enable --now docker
    ;;
  ubuntu|debian|linuxmint|pop)
    echo "Installing via apt (docker.io)..."
    sudo apt-get update
    sudo apt-get install -y docker.io docker-compose-plugin || sudo apt-get install -y docker.io docker-compose
    sudo systemctl enable --now docker
    ;;
  fedora|rhel|centos|rocky|almalinux|nobara)
    echo "Installing via dnf (moby-engine / docker)..."
    if command -v dnf >/dev/null 2>&1; then
      sudo dnf install -y moby-engine docker-compose-plugin || sudo dnf install -y docker docker-compose
    else
      sudo yum install -y docker docker-compose
    fi
    sudo systemctl enable --now docker
    ;;
  opensuse*|sles)
    echo "Installing via zypper..."
    sudo zypper -n install docker docker-compose || sudo zypper -n install docker
    sudo systemctl enable --now docker
    ;;
  gentoo)
    echo "Installing via emerge (app-containers/docker)..."
    echo "Note: Gentoo builds from source; this may take a while."
    sudo emerge -av app-containers/docker app-containers/docker-compose
    sudo rc-update add docker default
    sudo rc-service docker start
    ;;
  alpine)
    echo "Installing via apk..."
    sudo apk add docker docker-cli-compose
    sudo rc-update add docker boot
    sudo service docker start
    ;;
  nixos)
    echo "NixOS: enable docker in configuration.nix (virtualisation.docker.enable = true;)"
    exit 1
    ;;
  *)
    echo "Unknown distro '${ID}'. Falling back to get.docker.com..."
    install_get_docker
    ;;
esac

if id -nG "${USER}" 2>/dev/null | grep -qw docker; then
  :
else
  echo "Adding ${USER} to group docker (log out/in or run: newgrp docker)..."
  sudo usermod -aG docker "${USER}" 2>/dev/null || true
fi

if docker info >/dev/null 2>&1; then
  echo "Docker is ready."
else
  echo "Docker installed. Start the daemon or re-login, then run: cleanserve doctor"
fi
