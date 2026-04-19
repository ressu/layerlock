#!/usr/bin/env sh
# install.sh — Download, verify, and install layerlock
# Usage: sh install.sh [VERSION] [INSTALL_DIR]
#   VERSION     defaults to latest GitHub release
#   INSTALL_DIR defaults to /usr/local/bin
set -eu

REPO="ressu/layerlock"
VERSION="${VERSION:-}"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
PRERELEASE="${PRERELEASE:-0}"
TMP_DIR="$(mktemp -d)"

cleanup() { rm -rf "$TMP_DIR"; }
trap cleanup EXIT

# Detect architecture
case "$(uname -m)" in
  x86_64)             ARCH="amd64"  ;;
  aarch64|arm64)      ARCH="arm64"  ;;
  armv7*|armv6*)      ARCH="armv7"  ;;
  *) echo "Unsupported architecture: $(uname -m)" >&2; exit 1 ;;
esac

# Resolve latest version if not specified
if [ -z "$VERSION" ]; then
  if [ "$PRERELEASE" = "1" ]; then
    VERSION="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases" \
      | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": *"\(.*\)".*/\1/')"
  else
    VERSION="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
      | grep '"tag_name"' | sed 's/.*"tag_name": *"\(.*\)".*/\1/')"
  fi
fi

if [ -z "$VERSION" ]; then
  echo "Could not determine latest version. Use VERSION=vX.Y.Z to specify explicitly." >&2
  exit 1
fi

echo "Installing layerlock ${VERSION} (${ARCH})..."

BASE_URL="https://github.com/${REPO}/releases/download/${VERSION}"
BINARY_NAME="layerlock_linux_${ARCH}"
CHECKSUM_FILE="layerlock_checksums.txt"

# Download binary and checksum file
curl -fsSL "${BASE_URL}/${BINARY_NAME}" -o "${TMP_DIR}/layerlock"
curl -fsSL "${BASE_URL}/${CHECKSUM_FILE}" -o "${TMP_DIR}/${CHECKSUM_FILE}"

# Verify checksum
cd "$TMP_DIR"
grep "${BINARY_NAME}" "${CHECKSUM_FILE}" | sha256sum -c -

chmod +x layerlock
mv layerlock "${INSTALL_DIR}/layerlock"

echo "Installed to ${INSTALL_DIR}/layerlock"
"${INSTALL_DIR}/layerlock" --version 2>/dev/null || true
