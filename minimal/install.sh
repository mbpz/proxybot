#!/usr/bin/env sh
# One-command installer for the minimal ProxyBot CLI.
#
#   curl -fsSL https://raw.githubusercontent.com/mbpz/proxybot/main/minimal/install.sh | sh
#
# Overrides:
#   PROXYBOT_VERSION=v0.1.0   install a specific release (tag minimal-v<version>)
#   PROXYBOT_PREFIX=/usr/local/bin
#   PROXYBOT_REPO=owner/repo
set -eu

REPO="${PROXYBOT_REPO:-mbpz/proxybot}"
VERSION="${PROXYBOT_VERSION:-latest}"
PREFIX="${PROXYBOT_PREFIX:-$HOME/.local/bin}"
ARCH="$(uname -m)"

case "$ARCH" in
  arm64) TARGET="aarch64-apple-darwin" ;;
  x86_64) TARGET="x86_64-apple-darwin" ;;
  *)
    echo "unsupported architecture: $ARCH (ProxyBot ships macOS arm64/x64 builds)" >&2
    exit 1
    ;;
esac

if [ "$(uname -s)" != "Darwin" ]; then
  echo "unsupported platform: $(uname -s) (ProxyBot currently targets macOS)" >&2
  exit 1
fi

if [ -n "${PROXYBOT_DOWNLOAD_BASE:-}" ]; then
  # Mirror / offline base URL that serves the same proxybot-min-*.tar.gz assets.
  BASE="${PROXYBOT_DOWNLOAD_BASE}"
elif [ "$VERSION" = "latest" ]; then
  BASE="https://github.com/${REPO}/releases/latest/download"
else
  BASE="https://github.com/${REPO}/releases/download/minimal-v${VERSION}"
fi
ASSET="proxybot-min-${TARGET}.tar.gz"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT INT TERM

# Offline/development path: install an already built binary directly.
if [ -n "${PROXYBOT_LOCAL_BINARY:-}" ]; then
  mkdir -p "$PREFIX"
  install -m 0755 "$PROXYBOT_LOCAL_BINARY" "${PREFIX}/proxybot"
  echo "Installed $( "${PREFIX}/proxybot" --version ) to ${PREFIX}/proxybot"
  exit 0
fi

echo "Downloading ${ASSET} from ${BASE} ..."
curl --http1.1 --fail --location --silent --show-error --retry 5 --retry-all-errors \
  "${BASE}/${ASSET}" -o "${TMP}/${ASSET}"
curl --http1.1 --fail --location --silent --show-error --retry 5 --retry-all-errors \
  "${BASE}/${ASSET}.sha256" -o "${TMP}/${ASSET}.sha256"

EXPECTED="$(cut -d ' ' -f1 "${TMP}/${ASSET}.sha256")"
ACTUAL="$(shasum -a 256 "${TMP}/${ASSET}" | cut -d ' ' -f1)"
if [ "$EXPECTED" != "$ACTUAL" ]; then
  echo "checksum mismatch for ${ASSET}" >&2
  echo "expected: $EXPECTED" >&2
  echo "actual:   $ACTUAL" >&2
  exit 1
fi
echo "Checksum verified."

mkdir -p "$PREFIX"
tar -xzf "${TMP}/${ASSET}" -C "$TMP"
install -m 0755 "${TMP}/proxybot" "${PREFIX}/proxybot"

echo
echo "Installed $( "${PREFIX}/proxybot" --version ) to ${PREFIX}/proxybot"
case ":$PATH:" in
  *":${PREFIX}:"*) ;;
  *)
    echo
    echo "Add ${PREFIX} to PATH, for example:"
    echo "  echo 'export PATH=\"${PREFIX}:\$PATH\"' >> ~/.zshrc && source ~/.zshrc"
    ;;
esac
echo
echo "Next:"
echo "  proxybot start            # explicit proxy + web UI"
echo "  proxybot start --gateway  # transparent Wi-Fi gateway mode (asks for admin)"
