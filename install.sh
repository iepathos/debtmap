#!/bin/bash

# Debtmap installer script
# This script automatically detects your OS and architecture, downloads the appropriate
# debtmap binary from the latest GitHub release, and installs it to your system.

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
REPO="iepathos/debtmap"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
GITHUB_API="https://api.github.com/repos/${REPO}"

# Helper functions
error() {
    echo -e "${RED}Error: $1${NC}" >&2
    exit 1
}

success() {
    echo -e "${GREEN}✓ $1${NC}"
}

info() {
    echo -e "${YELLOW}→ $1${NC}"
}

# Detect OS
detect_os() {
    case "$(uname -s)" in
        Linux*)     OS="linux";;
        Darwin*)    OS="darwin";;
        CYGWIN*|MINGW*|MSYS*) OS="windows";;
        *)          error "Unsupported operating system: $(uname -s)";;
    esac
}

# Detect architecture
detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64)   ARCH="x86_64";;
        aarch64|arm64)  ARCH="aarch64";;
        *)              error "Unsupported architecture: $(uname -m)";;
    esac
}

# Determine target triple
get_target() {
    detect_os
    detect_arch
    
    case "${OS}-${ARCH}" in
        linux-x86_64)
            # Check if musl or gnu
            if ldd /bin/ls 2>&1 | grep -q musl; then
                TARGET="x86_64-unknown-linux-musl"
            else
                TARGET="x86_64-unknown-linux-gnu"
            fi
            ;;
        linux-aarch64)
            TARGET="aarch64-unknown-linux-gnu"
            ;;
        darwin-x86_64)
            TARGET="x86_64-apple-darwin"
            ;;
        darwin-aarch64)
            TARGET="aarch64-apple-darwin"
            ;;
        windows-x86_64)
            TARGET="x86_64-pc-windows-msvc"
            BINARY_NAME="debtmap.exe"
            ARCHIVE_EXT="zip"
            ;;
        *)
            error "Unsupported platform: ${OS}-${ARCH}"
            ;;
    esac
    
    # Set defaults if not Windows
    BINARY_NAME="${BINARY_NAME:-debtmap}"
    ARCHIVE_EXT="${ARCHIVE_EXT:-tar.gz}"
}

# Get latest release tag from GitHub
get_latest_release() {
    info "Fetching latest release information..."
    
    if command -v curl >/dev/null 2>&1; then
        RELEASE_INFO=$(curl -s "${GITHUB_API}/releases/latest")
    elif command -v wget >/dev/null 2>&1; then
        RELEASE_INFO=$(wget -qO- "${GITHUB_API}/releases/latest")
    else
        error "Neither curl nor wget found. Please install one of them."
    fi
    
    LATEST_VERSION=$(echo "$RELEASE_INFO" | grep '"tag_name":' | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/')
    
    if [ -z "$LATEST_VERSION" ]; then
        error "Failed to get latest release version"
    fi
    
    success "Latest version: $LATEST_VERSION"
}

# Download and extract binary
download_and_install() {
    local download_url="https://github.com/${REPO}/releases/download/${LATEST_VERSION}/debtmap-${TARGET}.${ARCHIVE_EXT}"
    local temp_dir=$(mktemp -d)
    local archive_file="${temp_dir}/debtmap.${ARCHIVE_EXT}"
    
    info "Downloading debtmap ${LATEST_VERSION} for ${TARGET}..."
    
    # Download
    if command -v curl >/dev/null 2>&1; then
        curl -sL "$download_url" -o "$archive_file" || error "Failed to download release"
    else
        wget -q "$download_url" -O "$archive_file" || error "Failed to download release"
    fi
    
    # Extract
    info "Extracting archive..."
    cd "$temp_dir"
    if [ "$ARCHIVE_EXT" = "tar.gz" ]; then
        tar -xzf "$archive_file" || error "Failed to extract archive"
    else
        unzip -q "$archive_file" || error "Failed to extract archive"
    fi
    
    # Create install directory if it doesn't exist
    mkdir -p "$INSTALL_DIR"
    
    # Install binary
    info "Installing debtmap to ${INSTALL_DIR}..."
    mv "$BINARY_NAME" "$INSTALL_DIR/" || error "Failed to install binary"
    chmod +x "${INSTALL_DIR}/${BINARY_NAME}"
    
    # Cleanup
    rm -rf "$temp_dir"
    
    success "debtmap installed successfully!"
}

# Check if install directory is in PATH
check_path() {
    if ! echo "$PATH" | grep -q "$INSTALL_DIR"; then
        echo ""
        info "Note: ${INSTALL_DIR} is not in your PATH"
        echo "Add it to your shell configuration file:"
        echo ""
        echo "  export PATH=\"\$PATH:${INSTALL_DIR}\""
        echo ""
        echo "For bash, add to ~/.bashrc or ~/.bash_profile"
        echo "For zsh, add to ~/.zshrc"
        echo "For fish, run: set -U fish_user_paths ${INSTALL_DIR} \$fish_user_paths"
    fi
}

# Verify installation
verify_installation() {
    if command -v debtmap >/dev/null 2>&1; then
        local version=$(debtmap --version 2>&1 | head -n1)
        success "Installation verified: $version"
    else
        info "Run 'debtmap --version' to verify installation after updating your PATH"
    fi
}

# Main installation flow
main() {
    echo "==================================="
    echo "     Debtmap Installer"
    echo "==================================="
    echo ""
    
    # Detect platform
    get_target
    info "Detected platform: ${TARGET}"
    
    # Get latest release
    get_latest_release
    
    # Download and install
    download_and_install
    
    # Check PATH
    check_path
    
    # Verify
    verify_installation
    
    echo ""
    echo "==================================="
    echo "     Installation Complete!"
    echo "==================================="
    echo ""
    echo "Get started with:"
    echo "  debtmap analyze ."
    echo ""
    echo "For more information:"
    echo "  debtmap --help"
    echo ""
}

# Run main function
main "$@"