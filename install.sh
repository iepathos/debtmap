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
# Default to cargo bin directory if it exists, otherwise use .local/bin
if [ -d "$HOME/.cargo/bin" ]; then
    INSTALL_DIR="${INSTALL_DIR:-$HOME/.cargo/bin}"
else
    INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
fi
GITHUB_API="https://api.github.com/repos/${REPO}"
TEMP_DIR=""
STAGED_BINARY=""

cleanup_temp_dir() {
    if [ -n "$STAGED_BINARY" ] && [ -f "$STAGED_BINARY" ]; then
        rm -f -- "$STAGED_BINARY"
    fi
    STAGED_BINARY=""
    if [ -n "$TEMP_DIR" ] && [ -d "$TEMP_DIR" ]; then
        rm -rf -- "$TEMP_DIR"
    fi
    TEMP_DIR=""
}

trap cleanup_temp_dir EXIT

# Helper functions
error() {
    echo -e "${RED}Error: $1${NC}" >&2
    cleanup_temp_dir
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
            # Prefer musl for better compatibility across different GLIBC versions
            # Users can override with DEBTMAP_USE_GNU=1 if they prefer the gnu build
            if [ "${DEBTMAP_USE_GNU}" = "1" ]; then
                TARGET="x86_64-unknown-linux-gnu"
            else
                TARGET="x86_64-unknown-linux-musl"
            fi
            ;;
        linux-aarch64)
            error "Linux ARM64 release artifacts are not currently published. Use 'cargo install debtmap' or build from source."
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
        if ! RELEASE_INFO=$(curl -fsSL --retry 3 --retry-delay 1 "${GITHUB_API}/releases/latest"); then
            error "Failed to fetch latest release information"
        fi
    elif command -v wget >/dev/null 2>&1; then
        if ! RELEASE_INFO=$(wget -q --tries=3 --timeout=30 -O- "${GITHUB_API}/releases/latest"); then
            error "Failed to fetch latest release information"
        fi
    else
        error "Neither curl nor wget found. Please install one of them."
    fi
    
    LATEST_VERSION=$(echo "$RELEASE_INFO" | grep '"tag_name":' | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/')
    
    if [ -z "$LATEST_VERSION" ]; then
        error "Failed to get latest release version"
    fi
    
    success "Latest version: $LATEST_VERSION"
}

download_file() {
    local url="$1"
    local destination="$2"

    if command -v curl >/dev/null 2>&1; then
        curl -fsSL --retry 3 --retry-delay 1 "$url" -o "$destination"
    elif command -v wget >/dev/null 2>&1; then
        wget -q --tries=3 --timeout=30 "$url" -O "$destination"
    else
        error "Neither curl nor wget found. Please install one of them."
    fi
}

calculate_sha256() {
    local file="$1"

    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$file" | awk '{print $1}'
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$file" | awk '{print $1}'
    else
        error "Neither sha256sum nor shasum found. Cannot verify the release archive."
    fi
}

verify_checksum() {
    local archive_file="$1"
    local checksum_file="$2"
    local expected_checksum
    local actual_checksum

    expected_checksum=$(awk 'NR == 1 {print $1}' "$checksum_file" | tr '[:upper:]' '[:lower:]')
    if ! printf '%s\n' "$expected_checksum" | grep -Eq '^[0-9a-f]{64}$'; then
        error "Release checksum file is invalid"
    fi

    actual_checksum=$(calculate_sha256 "$archive_file")
    if [ "$expected_checksum" != "$actual_checksum" ]; then
        error "Release archive checksum verification failed"
    fi

    success "Release archive checksum verified"
}

# Download and extract binary
download_and_install() {
    local asset_name="debtmap-${TARGET}.${ARCHIVE_EXT}"
    local download_url="https://github.com/${REPO}/releases/download/${LATEST_VERSION}/${asset_name}"
    local checksum_url="${download_url}.sha256"
    local archive_file
    local checksum_file

    TEMP_DIR=$(mktemp -d)
    archive_file="${TEMP_DIR}/${asset_name}"
    checksum_file="${archive_file}.sha256"
    
    info "Downloading debtmap ${LATEST_VERSION} for ${TARGET}..."
    
    download_file "$download_url" "$archive_file" || error "Failed to download release archive"
    download_file "$checksum_url" "$checksum_file" || error "Failed to download release checksum"
    verify_checksum "$archive_file" "$checksum_file"
    
    # Extract
    info "Extracting archive..."
    if [ "$ARCHIVE_EXT" = "tar.gz" ]; then
        tar -xzf "$archive_file" -C "$TEMP_DIR" || error "Failed to extract archive"
    else
        unzip -q "$archive_file" -d "$TEMP_DIR" || error "Failed to extract archive"
    fi
    
    # Create install directory if it doesn't exist
    mkdir -p "$INSTALL_DIR"

    if [ ! -f "${TEMP_DIR}/${BINARY_NAME}" ]; then
        error "Release archive does not contain ${BINARY_NAME}"
    fi
    chmod +x "${TEMP_DIR}/${BINARY_NAME}"
    if ! "${TEMP_DIR}/${BINARY_NAME}" --version >/dev/null 2>&1; then
        error "Downloaded debtmap binary failed validation"
    fi

    STAGED_BINARY=$(mktemp "${INSTALL_DIR}/.${BINARY_NAME}.installing.XXXXXX") || error "Failed to create staging file"
    info "Installing debtmap to ${INSTALL_DIR}..."
    cp "${TEMP_DIR}/${BINARY_NAME}" "$STAGED_BINARY" || error "Failed to stage binary"
    chmod +x "$STAGED_BINARY"
    mv -f "$STAGED_BINARY" "${INSTALL_DIR}/${BINARY_NAME}" || error "Failed to install binary"
    STAGED_BINARY=""
    
    # Cleanup
    cleanup_temp_dir
    
    success "debtmap installed successfully!"
}

# Check if install directory is in PATH and offer to add it
check_path() {
    if ! echo "$PATH" | grep -q "$INSTALL_DIR"; then
        echo ""
        info "Note: ${INSTALL_DIR} is not in your PATH"
        
        # Detect the current shell
        SHELL_NAME=$(basename "$SHELL")
        
        # Determine the appropriate config file
        case "$SHELL_NAME" in
            bash)
                if [ -f "$HOME/.bash_profile" ]; then
                    SHELL_CONFIG="$HOME/.bash_profile"
                elif [ -f "$HOME/.bashrc" ]; then
                    SHELL_CONFIG="$HOME/.bashrc"
                else
                    SHELL_CONFIG="$HOME/.bashrc"
                fi
                ;;
            zsh)
                SHELL_CONFIG="$HOME/.zshrc"
                ;;
            fish)
                SHELL_CONFIG="$HOME/.config/fish/config.fish"
                ;;
            *)
                SHELL_CONFIG=""
                ;;
        esac
        
        # Check if we're running interactively (not piped)
        if [ -t 0 ] && [ -n "$SHELL_CONFIG" ]; then
            echo ""
            echo "Would you like to add ${INSTALL_DIR} to your PATH automatically?"
            echo "This will add the following line to ${SHELL_CONFIG}:"
            echo ""
            if [ "$SHELL_NAME" = "fish" ]; then
                echo "  fish_add_path ${INSTALL_DIR}"
            else
                echo "  export PATH=\"\$PATH:${INSTALL_DIR}\""
            fi
            echo ""
            read -p "Add to PATH? [y/N] " -n 1 -r
            echo ""
            
            if [[ $REPLY =~ ^[Yy]$ ]]; then
                # Check if PATH export already exists in the config file
                if [ "$SHELL_NAME" = "fish" ]; then
                    if grep -q "fish_add_path.*${INSTALL_DIR}" "$SHELL_CONFIG" 2>/dev/null; then
                        info "PATH entry for ${INSTALL_DIR} already exists in ${SHELL_CONFIG}"
                    else
                        {
                            echo ""
                            echo "# Added by debtmap installer"
                            echo "fish_add_path ${INSTALL_DIR}"
                        } >> "$SHELL_CONFIG"
                        success "Added ${INSTALL_DIR} to PATH in ${SHELL_CONFIG}"
                    fi
                else
                    # Check if the PATH export already exists (handle both forms)
                    if grep -q "${INSTALL_DIR}" "$SHELL_CONFIG" 2>/dev/null; then
                        info "PATH entry for ${INSTALL_DIR} already exists in ${SHELL_CONFIG}"
                    else
                        {
                            echo ""
                            echo "# Added by debtmap installer"
                            echo "export PATH=\"\$PATH:${INSTALL_DIR}\""
                        } >> "$SHELL_CONFIG"
                        success "Added ${INSTALL_DIR} to PATH in ${SHELL_CONFIG}"
                    fi
                fi
                echo ""
                info "Please restart your terminal or run:"
                echo "  source ${SHELL_CONFIG}"
            else
                echo ""
                echo "To add it manually, add this line to your shell configuration:"
                if [ "$SHELL_NAME" = "fish" ]; then
                    echo "  fish_add_path ${INSTALL_DIR}"
                    echo ""
                    echo "Add to: ${SHELL_CONFIG}"
                else
                    echo "  export PATH=\"\$PATH:${INSTALL_DIR}\""
                    echo ""
                    echo "Add to: ${SHELL_CONFIG}"
                fi
            fi
        else
            echo ""
            echo "To add it to your PATH, add this line to your shell configuration:"
            echo "  export PATH=\"\$PATH:${INSTALL_DIR}\""
            echo ""
            echo "Common shell config files:"
            echo "  - bash: ~/.bashrc or ~/.bash_profile"
            echo "  - zsh: ~/.zshrc"
            echo "  - fish: ~/.config/fish/config.fish"
        fi
    fi
}

# Verify installation
verify_installation() {
    local version
    local version_output

    if [ ! -x "${INSTALL_DIR}/${BINARY_NAME}" ]; then
        error "Installed binary is missing or not executable: ${INSTALL_DIR}/${BINARY_NAME}"
    fi
    if ! version_output=$("${INSTALL_DIR}/${BINARY_NAME}" --version 2>&1); then
        error "Installed binary failed verification"
    fi
    version=$(printf '%s\n' "$version_output" | head -n1)
    if [ -z "$version" ]; then
        error "Installed binary returned an empty version"
    fi
    success "Installation verified: $version"
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

# The documented `curl ... | bash` path executes from stdin, so tests opt out explicitly.
if [ "${DEBTMAP_INSTALLER_TEST_MODE:-0}" != "1" ]; then
    main "$@"
fi
