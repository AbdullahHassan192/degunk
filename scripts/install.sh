#!/usr/bin/env sh
set -e

# Degunk installer for macOS and Linux
# Repository: https://github.com/AbdullahHassan192/degunk

OWNER="AbdullahHassan192"
REPO="degunk"

# Determine OS
OS="$(uname -s)"
case "$OS" in
    Darwin)
        OS_TYPE="darwin"
        ;;
    Linux)
        OS_TYPE="linux"
        ;;
    *)
        echo "Error: Unsupported operating system: $OS" >&2
        exit 1
        ;;
esac

# Determine Architecture
ARCH="$(uname -m)"
case "$ARCH" in
    x86_64|amd64)
        ARCH_TYPE="x86_64"
        ;;
    aarch64|arm64)
        ARCH_TYPE="arm64"
        ;;
    *)
        echo "Error: Unsupported architecture: $ARCH" >&2
        exit 1
        ;;
esac

ASSET="degunk-${OS_TYPE}-${ARCH_TYPE}.tar.gz"
DOWNLOAD_URL="https://github.com/${OWNER}/${REPO}/releases/latest/download/${ASSET}"

echo "Installing degunk (${OS_TYPE}/${ARCH_TYPE})..."

# Create temporary directory
TMP_DIR="$(mktemp -d 2>/dev/null || mktemp -d -t 'degunk')"
cleanup() {
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT

ARCHIVE_PATH="${TMP_DIR}/${ASSET}"

# Download release asset
if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$DOWNLOAD_URL" -o "$ARCHIVE_PATH"
elif command -v wget >/dev/null 2>&1; then
    wget -q "$DOWNLOAD_URL" -O "$ARCHIVE_PATH"
else
    echo "Error: Neither curl nor wget was found on your system." >&2
    exit 1
fi

# Extract archive
tar -xzf "$ARCHIVE_PATH" -C "$TMP_DIR"

if [ ! -f "${TMP_DIR}/degunk" ]; then
    echo "Error: Binary not found in release archive." >&2
    exit 1
fi

chmod +x "${TMP_DIR}/degunk"

# Determine install location
if [ -w "/usr/local/bin" ]; then
    INSTALL_DIR="/usr/local/bin"
elif [ "$(id -u)" -eq 0 ]; then
    INSTALL_DIR="/usr/local/bin"
else
    INSTALL_DIR="${HOME}/.local/bin"
    mkdir -p "$INSTALL_DIR"
fi

# Move binary to target directory
mv "${TMP_DIR}/degunk" "${INSTALL_DIR}/degunk"

echo "✓ Successfully installed degunk to ${INSTALL_DIR}/degunk"

# Check if install dir is in PATH
case ":$PATH:" in
    *":${INSTALL_DIR}:"*)
        ;;
    *)
        echo ""
        echo "Notice: ${INSTALL_DIR} is not in your PATH."
        echo "Add it to your shell configuration (e.g. ~/.bashrc or ~/.zshrc):"
        echo "  export PATH=\"\$PATH:${INSTALL_DIR}\""
        ;;
esac

echo ""
echo "Run 'degunk' to scan and clean your workspace."
