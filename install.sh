#!/usr/bin/env sh
# install.sh: downloads layerlock, checks its SHA-256 checksum, and installs it.
# Usage: sh install.sh
# Environment variables:
#   VERSION        version to install, for example v1.0.0 (default: the latest release)
#   PRERELEASE     set to 1 to install the latest pre-release (default: 0)
#   INSTALL_DIR    folder to install into (default: /usr/local/bin)
#   BINARY_SHA256  checksum the binary must match, as "sha256:<hex>" or "<hex>". With this set,
#                  the script doesn't download the checksums file.
set -eu

REPO="ressu/layerlock"
VERSION="${VERSION:-}"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
PRERELEASE="${PRERELEASE:-0}"
BINARY_SHA256="${BINARY_SHA256:-}"
TMP_DIR="$(mktemp -d)"

cleanup() { rm -rf "$TMP_DIR"; }
trap cleanup EXIT

# Detect architecture. LEGACY_ARCH is the asset name used by releases up to v1.0.0 (built with Go),
# which had no armv6 build.
case "$(uname -m)" in
  x86_64)             ARCH="x86_64";  LEGACY_ARCH="amd64" ;;
  aarch64|arm64)      ARCH="aarch64"; LEGACY_ARCH="arm64" ;;
  armv7*|armv8l)      ARCH="armv7";   LEGACY_ARCH="armv7" ;;
  armv6*)             ARCH="armv6";   LEGACY_ARCH="" ;;
  *) echo "layerlock has no build for $(uname -m) processors." >&2; exit 1 ;;
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
  echo "Can't find the latest layerlock release on GitHub." >&2
  echo "To install a version you choose, run (with the version, for example v1.0.0):" >&2
  echo "  curl -fsSL https://raw.githubusercontent.com/${REPO}/main/install.sh | VERSION=<version> sh" >&2
  exit 1
fi

echo "Installing layerlock ${VERSION} (${ARCH})..."

BASE_URL="https://github.com/${REPO}/releases/download/${VERSION}"
BINARY_NAME="layerlock-linux-${ARCH}"
CHECKSUM_FILE="layerlock-checksums.txt"

# Download binary, falling back to the asset names of older releases
if ! curl -fsSL "${BASE_URL}/${BINARY_NAME}" -o "${TMP_DIR}/${BINARY_NAME}" 2>/dev/null; then
  if [ -z "$LEGACY_ARCH" ]; then
    echo "layerlock ${VERSION} has no build for $(uname -m) processors. Install a newer version." >&2
    exit 1
  fi
  BINARY_NAME="layerlock_linux_${LEGACY_ARCH}"
  CHECKSUM_FILE="layerlock_checksums.txt"
  curl -fsSL "${BASE_URL}/${BINARY_NAME}" -o "${TMP_DIR}/${BINARY_NAME}"
fi

# Verify checksum
cd "$TMP_DIR"
if [ -n "$BINARY_SHA256" ]; then
  # Strip optional "sha256:" prefix (as shown in GitHub UI)
  EXPECTED_HASH="${BINARY_SHA256#sha256:}"
  COMPUTED_HASH="$(sha256sum "${BINARY_NAME}" | awk '{print $1}')"
  if [ "$COMPUTED_HASH" != "$EXPECTED_HASH" ]; then
    echo "${BINARY_NAME} doesn't match BINARY_SHA256, so it wasn't installed." >&2
    echo "  BINARY_SHA256: ${EXPECTED_HASH}" >&2
    echo "  download:      ${COMPUTED_HASH}" >&2
    echo "Check that BINARY_SHA256 is the checksum of ${BINARY_NAME} in layerlock ${VERSION}." >&2
    exit 1
  fi
else
  curl -fsSL "${BASE_URL}/${CHECKSUM_FILE}" -o "${TMP_DIR}/${CHECKSUM_FILE}"
  grep " ${BINARY_NAME}\$" "${CHECKSUM_FILE}" | sha256sum -c -
fi

if [ -w "${INSTALL_DIR}" ]; then
  install -m 755 "${BINARY_NAME}" "${INSTALL_DIR}/layerlock"
else
  sudo install -m 755 "${BINARY_NAME}" "${INSTALL_DIR}/layerlock"
fi

echo "Installed layerlock in ${INSTALL_DIR}."
"${INSTALL_DIR}/layerlock" --version 2>/dev/null || true
