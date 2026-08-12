#!/usr/bin/env bash

set -euo pipefail

PROJECT_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
TEST_ROOT=$(mktemp -d)

# shellcheck source=../install.sh
DEBTMAP_INSTALLER_TEST_MODE=1
source "${PROJECT_ROOT}/install.sh"
trap 'cleanup_temp_dir; rm -rf -- "$TEST_ROOT"' EXIT

fail() {
    printf 'FAIL: %s\n' "$1" >&2
    exit 1
}

assert_equal() {
    local expected="$1"
    local actual="$2"
    local label="$3"

    [ "$expected" = "$actual" ] || fail "${label}: expected '${expected}', got '${actual}'"
}

target_for() (
    local system_name="$1"
    local machine_name="$2"
    local use_gnu="${3:-0}"

    uname() {
        if [ "$1" = "-s" ]; then
            printf '%s\n' "$system_name"
        else
            printf '%s\n' "$machine_name"
        fi
    }

    DEBTMAP_USE_GNU="$use_gnu"
    get_target
    printf '%s|%s|%s\n' "$TARGET" "$BINARY_NAME" "$ARCHIVE_EXT"
)

test_supported_target_matrix() {
    assert_equal "x86_64-unknown-linux-musl|debtmap|tar.gz" "$(target_for Linux x86_64)" "Linux musl target"
    assert_equal "x86_64-unknown-linux-gnu|debtmap|tar.gz" "$(target_for Linux x86_64 1)" "Linux GNU target"
    assert_equal "x86_64-apple-darwin|debtmap|tar.gz" "$(target_for Darwin x86_64)" "Intel macOS target"
    assert_equal "aarch64-apple-darwin|debtmap|tar.gz" "$(target_for Darwin arm64)" "Apple Silicon target"
    assert_equal "x86_64-pc-windows-msvc|debtmap.exe|zip" "$(target_for MINGW64_NT-10.0 x86_64)" "Windows target"
}

test_unpublished_target_is_rejected() {
    local output

    if output=$(target_for Linux aarch64 2>&1); then
        fail "Linux ARM64 unexpectedly selected an unpublished artifact"
    fi
    printf '%s\n' "$output" | grep -q "not currently published" || fail "Linux ARM64 diagnostic is unclear"
}

test_targets_match_release_matrix() {
    local release_workflow="${PROJECT_ROOT}/.github/workflows/release.yml"
    local target

    for target in \
        x86_64-unknown-linux-musl \
        x86_64-unknown-linux-gnu \
        x86_64-apple-darwin \
        aarch64-apple-darwin \
        x86_64-pc-windows-msvc
    do
        grep -q "target: ${target}" "$release_workflow" || fail "installer target ${target} is absent from release matrix"
    done
}

write_test_binary() {
    local path="$1"

    printf '#!/usr/bin/env bash\nprintf "debtmap test-version\\n"\n' > "$path"
    chmod +x "$path"
}

prepare_archive() {
    local assets_dir="$1"
    local asset_name="$2"
    local payload_dir="${TEST_ROOT}/payload"

    mkdir -p "$assets_dir" "$payload_dir"
    write_test_binary "${payload_dir}/debtmap"
    tar -czf "${assets_dir}/${asset_name}" -C "$payload_dir" debtmap
    calculate_sha256 "${assets_dir}/${asset_name}" > "${assets_dir}/${asset_name}.sha256"
}

install_from_assets() (
    local assets_dir="$1"
    local install_dir="$2"
    local target="${3:-x86_64-unknown-linux-musl}"
    local archive_ext="${4:-tar.gz}"
    local binary_name="${5:-debtmap}"

    download_file() {
        local url="$1"
        local destination="$2"
        cp "${assets_dir}/${url##*/}" "$destination"
    }

    LATEST_VERSION="v-test"
    TARGET="$target"
    ARCHIVE_EXT="$archive_ext"
    BINARY_NAME="$binary_name"
    INSTALL_DIR="$install_dir"
    download_and_install
)

test_verified_archive_installs() {
    local assets_dir="${TEST_ROOT}/valid-assets"
    local install_dir="${TEST_ROOT}/valid-install"
    local asset_name="debtmap-x86_64-unknown-linux-musl.tar.gz"

    prepare_archive "$assets_dir" "$asset_name"
    install_from_assets "$assets_dir" "$install_dir"
    [ -x "${install_dir}/debtmap" ] || fail "verified binary was not installed"
    assert_equal "debtmap test-version" "$("${install_dir}/debtmap" --version)" "installed binary"
    [ -z "$(find "$install_dir" -name '.debtmap.installing.*' -print -quit)" ] || fail "installer staging file leaked"
}

test_verified_zip_installs() {
    local assets_dir="${TEST_ROOT}/zip-assets"
    local install_dir="${TEST_ROOT}/zip-install"
    local payload_dir="${TEST_ROOT}/zip-payload"
    local asset_name="debtmap-x86_64-pc-windows-msvc.zip"

    mkdir -p "$assets_dir" "$payload_dir"
    write_test_binary "${payload_dir}/debtmap.exe"
    (cd "$payload_dir" && zip -q "${assets_dir}/${asset_name}" debtmap.exe)
    calculate_sha256 "${assets_dir}/${asset_name}" > "${assets_dir}/${asset_name}.sha256"
    install_from_assets "$assets_dir" "$install_dir" "x86_64-pc-windows-msvc" "zip" "debtmap.exe"
    [ -x "${install_dir}/debtmap.exe" ] || fail "verified Windows binary was not installed"
}

test_corrupt_archive_is_rejected() {
    local assets_dir="${TEST_ROOT}/corrupt-assets"
    local install_dir="${TEST_ROOT}/corrupt-install"
    local asset_name="debtmap-x86_64-unknown-linux-musl.tar.gz"
    local temp_parent="${TEST_ROOT}/corrupt-temp"
    local output

    prepare_archive "$assets_dir" "$asset_name"
    printf 'corruption' >> "${assets_dir}/${asset_name}"
    mkdir -p "$install_dir" "$temp_parent"
    printf 'existing binary\n' > "${install_dir}/debtmap"

    if output=$(TMPDIR="$temp_parent" install_from_assets "$assets_dir" "$install_dir" 2>&1); then
        fail "corrupt archive unexpectedly installed"
    fi
    printf '%s\n' "$output" | grep -q "checksum verification failed" || fail "checksum failure diagnostic is unclear"
    assert_equal "existing binary" "$(sed -n '1p' "${install_dir}/debtmap")" "existing binary after checksum failure"
    [ -z "$(find "$temp_parent" -mindepth 1 -print -quit)" ] || fail "checksum failure leaked installer temporary files"
}

test_invalid_binary_preserves_existing_install() {
    local assets_dir="${TEST_ROOT}/invalid-assets"
    local install_dir="${TEST_ROOT}/invalid-install"
    local payload_dir="${TEST_ROOT}/invalid-payload"
    local asset_name="debtmap-x86_64-unknown-linux-musl.tar.gz"
    local output

    mkdir -p "$assets_dir" "$install_dir" "$payload_dir"
    printf '#!/usr/bin/env bash\nexit 1\n' > "${payload_dir}/debtmap"
    tar -czf "${assets_dir}/${asset_name}" -C "$payload_dir" debtmap
    calculate_sha256 "${assets_dir}/${asset_name}" > "${assets_dir}/${asset_name}.sha256"
    printf 'existing binary\n' > "${install_dir}/debtmap"

    if output=$(install_from_assets "$assets_dir" "$install_dir" 2>&1); then
        fail "invalid binary unexpectedly installed"
    fi
    printf '%s\n' "$output" | grep -q "failed validation" || fail "binary validation diagnostic is unclear"
    assert_equal "existing binary" "$(sed -n '1p' "${install_dir}/debtmap")" "existing binary after validation failure"
}

test_curl_download_fails_on_http_errors() {
    local fake_bin="${TEST_ROOT}/fake-bin"
    local args_file="${TEST_ROOT}/curl-args"
    local output_file="${TEST_ROOT}/curl-output"

    mkdir -p "$fake_bin"
    # The generated fake expands these variables at runtime.
    # shellcheck disable=SC2016
    printf '#!/usr/bin/env bash\nprintf "%%s\\n" "$@" > "$CURL_ARGS_FILE"\n: > "${@: -1}"\n' > "${fake_bin}/curl"
    chmod +x "${fake_bin}/curl"

    CURL_ARGS_FILE="$args_file" PATH="${fake_bin}:${PATH}" download_file "https://example.invalid/archive" "$output_file"
    grep -q -- '-fsSL' "$args_file" || fail "curl download does not fail on HTTP errors"
    grep -q -- '--retry' "$args_file" || fail "curl download does not retry transient errors"
}

test_wget_download_contract() (
    local args_file="${TEST_ROOT}/wget-args"
    local output_file="${TEST_ROOT}/wget-output"

    # download_file invokes this shell builtin override indirectly.
    # shellcheck disable=SC2329
    command() {
        if [ "$1" = "-v" ] && [ "$2" = "curl" ]; then
            return 1
        fi
        if [ "$1" = "-v" ] && [ "$2" = "wget" ]; then
            return 0
        fi
        builtin command "$@"
    }
    wget() {
        printf '%s\n' "$@" > "$args_file"
        : > "${*: -1}"
    }

    download_file "https://example.invalid/archive" "$output_file"
    grep -q -- '--tries=3' "$args_file" || fail "wget download does not retry transient errors"
    grep -q -- '-O' "$args_file" || fail "wget download does not use the requested destination"
)

test_release_fetch_failure_is_descriptive() (
    local output

    curl() {
        return 22
    }

    if output=$(get_latest_release 2>&1); then
        fail "failed release request unexpectedly succeeded"
    fi
    printf '%s\n' "$output" | grep -q "Failed to fetch latest release information" || fail "release fetch failure lacks context"
)

test_checksum_format_is_validated() {
    local archive="${TEST_ROOT}/checksum-archive"
    local checksum="${TEST_ROOT}/checksum-archive.sha256"
    local output

    printf 'archive\n' > "$archive"
    printf 'not-a-checksum\n' > "$checksum"
    if output=$(verify_checksum "$archive" "$checksum" 2>&1); then
        fail "malformed checksum unexpectedly passed"
    fi
    printf '%s\n' "$output" | grep -q "checksum file is invalid" || fail "malformed checksum diagnostic is unclear"
}

test_cleanup_removes_only_installer_temp_dir() {
    TEMP_DIR="${TEST_ROOT}/installer-temp"
    mkdir -p "$TEMP_DIR"
    printf 'temporary\n' > "${TEMP_DIR}/artifact"

    cleanup_temp_dir
    [ ! -e "${TEST_ROOT}/installer-temp" ] || fail "installer temporary directory was not cleaned"
    [ -d "$TEST_ROOT" ] || fail "cleanup removed the test root"
}

test_verification_uses_installed_binary_status() {
    local output

    INSTALL_DIR="/usr/bin"
    BINARY_NAME="false"
    if output=$(verify_installation 2>&1); then
        fail "verification ignored the installed binary exit status"
    fi
    printf '%s\n' "$output" | grep -q "failed verification" || fail "verification failure diagnostic is unclear"
}

test_release_publishes_after_assets() {
    local release_workflow="${PROJECT_ROOT}/.github/workflows/release.yml"

    grep -q -- '--draft=true' "$release_workflow" || fail "release is visible before assets upload"
    grep -q '^  publish-release:' "$release_workflow" || fail "release workflow has no publication job"
    grep -q 'needs: \[create-release, build-release\]' "$release_workflow" || fail "publication does not wait for every build"
    grep -q 'gh release edit .*--draft=false' "$release_workflow" || fail "completed draft is never published"
}

test_supported_target_matrix
test_unpublished_target_is_rejected
test_targets_match_release_matrix
test_verified_archive_installs
test_verified_zip_installs
test_corrupt_archive_is_rejected
test_invalid_binary_preserves_existing_install
test_curl_download_fails_on_http_errors
test_wget_download_contract
test_release_fetch_failure_is_descriptive
test_checksum_format_is_validated
test_cleanup_removes_only_installer_temp_dir
test_verification_uses_installed_binary_status
test_release_publishes_after_assets

printf 'Installer contract tests passed\n'
