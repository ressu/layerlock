#!/usr/bin/env sh
# install.sh — Download, verify, and install layerlock
# Usage: sh install.sh
#   VERSION        env var, defaults to latest GitHub release
#   INSTALL_DIR    env var, defaults to /usr/local/bin
#   BINARY_SHA256  env var, optional SHA-256 to verify the binary (skips downloading checksums file)
#                  accepts "sha256:<hex>" (GitHub UI format) or bare "<hex>"
set -eu

REPO="ressu/layerlock"
VERSION="${VERSION:-}"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
PRERELEASE="${PRERELEASE:-0}"
BINARY_SHA256="${BINARY_SHA256:-}"
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

# Download binary
curl -fsSL "${BASE_URL}/${BINARY_NAME}" -o "${TMP_DIR}/${BINARY_NAME}"

# Verify checksum
cd "$TMP_DIR"
if [ -n "$BINARY_SHA256" ]; then
  # Strip optional "sha256:" prefix (as shown in GitHub UI)
  EXPECTED_HASH="${BINARY_SHA256#sha256:}"
  COMPUTED_HASH="$(sha256sum "${BINARY_NAME}" | awk '{print $1}')"
  if [ "$COMPUTED_HASH" != "$EXPECTED_HASH" ]; then
    echo "Checksum mismatch for ${BINARY_NAME}" >&2
    echo "  expected: ${EXPECTED_HASH}" >&2
    echo "  computed: ${COMPUTED_HASH}" >&2
    exit 1
  fi
else
  curl -fsSL "${BASE_URL}/${CHECKSUM_FILE}" -o "${TMP_DIR}/${CHECKSUM_FILE}"
  grep "${BINARY_NAME}" "${CHECKSUM_FILE}" | sha256sum -c -
fi

if [ -w "${INSTALL_DIR}" ]; then
  install -m 755 "${BINARY_NAME}" "${INSTALL_DIR}/layerlock"
else
  sudo install -m 755 "${BINARY_NAME}" "${INSTALL_DIR}/layerlock"
fi

echo "Installed to ${INSTALL_DIR}/layerlock"
"${INSTALL_DIR}/layerlock" --version 2>/dev/null || true
