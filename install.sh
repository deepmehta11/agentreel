#!/bin/bash
# AgentReel installer — downloads the latest binary for your platform
# Usage: curl -fsSL https://raw.githubusercontent.com/deepmehta11/agentreel/main/install.sh | bash

set -euo pipefail

REPO="deepmehta11/agentreel"
INSTALL_DIR="${AGENTREEL_INSTALL_DIR:-$HOME/.local/bin}"

# Detect platform
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux)  PLATFORM="linux" ;;
  Darwin) PLATFORM="macos" ;;
  *) echo "Unsupported OS: $OS"; exit 1 ;;
esac

case "$ARCH" in
  x86_64|amd64) ARCH_NAME="x86_64" ;;
  aarch64|arm64) ARCH_NAME="aarch64" ;;
  *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

ARCHIVE="agentreel-${PLATFORM}-${ARCH_NAME}.tar.gz"

echo "AgentReel Installer"
echo "  Platform: ${PLATFORM}/${ARCH_NAME}"
echo "  Install to: ${INSTALL_DIR}"
echo ""

# Get latest release URL
LATEST_URL=$(curl -s "https://api.github.com/repos/${REPO}/releases/latest" | grep "browser_download_url.*${ARCHIVE}" | cut -d '"' -f 4)

if [ -z "$LATEST_URL" ]; then
  echo "No release found for ${ARCHIVE}."
  echo ""
  echo "Install from source instead:"
  echo "  git clone https://github.com/${REPO}.git"
  echo "  cd agentreel"
  echo "  cargo install --path crates/agentreel-cli"
  exit 1
fi

echo "Downloading ${ARCHIVE}..."
TMP_DIR=$(mktemp -d)
curl -fsSL "$LATEST_URL" -o "${TMP_DIR}/${ARCHIVE}"

echo "Extracting..."
tar xzf "${TMP_DIR}/${ARCHIVE}" -C "${TMP_DIR}"

echo "Installing to ${INSTALL_DIR}..."
mkdir -p "${INSTALL_DIR}"
mv "${TMP_DIR}/agentreel" "${INSTALL_DIR}/agentreel"
chmod +x "${INSTALL_DIR}/agentreel"

rm -rf "${TMP_DIR}"

# Check if install dir is in PATH
if ! echo "$PATH" | grep -q "$INSTALL_DIR"; then
  echo ""
  echo "Add to your PATH:"
  echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
  echo ""
  echo "Or add to your shell config:"
  echo "  echo 'export PATH=\"${INSTALL_DIR}:\$PATH\"' >> ~/.bashrc"
fi

echo ""
echo "Done! Run 'agentreel --help' to get started."
echo ""
echo "Quick start:"
echo "  agentreel record -- python my_agent.py"
echo "  agentreel view trajectory.json --full"
