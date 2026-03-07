#!/usr/bin/env bash
set -euo pipefail

# Install a pinned ttyd binary into a writable bin directory.
# Usage: ./scripts/ci/install_ttyd.sh <bin_dir> [version]
# For non-default versions, set TTYD_SHA256 to the expected binary digest.

BIN_DIR="${1:-${RUNNER_TEMP:-/tmp}/bin}"
VERSION="${2:-${TTYD_VERSION:-1.7.7}}"
TTYD_SHA256_OVERRIDE="${TTYD_SHA256:-}"

download_file() {
  local url="$1"
  local output="$2"
  if command -v curl >/dev/null 2>&1; then
    curl -sSfL "${url}" -o "${output}"
    return
  fi
  if command -v wget >/dev/null 2>&1; then
    wget -qO "${output}" "${url}"
    return
  fi
  echo "Missing downloader: install curl or wget" >&2
  exit 127
}

verify_sha256() {
  local checksum_file="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum -c "${checksum_file}"
    return
  fi
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 -c "${checksum_file}"
    return
  fi
  echo "Neither sha256sum nor shasum is available for checksum verification." >&2
  exit 127
}

os_name="$(uname -s | tr '[:upper:]' '[:lower:]')"
if [ "${os_name}" != "linux" ]; then
  echo "Unsupported OS for ttyd installer: ${os_name}" >&2
  exit 2
fi

arch_name="$(uname -m)"
case "${arch_name}" in
  x86_64|amd64)
    asset_name="ttyd.x86_64"
    pinned_sha256="8a217c968aba172e0dbf3f34447218dc015bc4d5e59bf51db2f2cd12b7be4f55"
    ;;
  aarch64|arm64)
    asset_name="ttyd.aarch64"
    pinned_sha256="b38acadd89d1d396a0f5649aa52c539edbad07f4bc7348b27b4f4b7219dd4165"
    ;;
  *)
    echo "Unsupported architecture for ttyd installer: ${arch_name}" >&2
    exit 2
    ;;
esac

binary_sha256="${pinned_sha256}"
if [ "${VERSION}" != "1.7.7" ]; then
  if [ -z "${TTYD_SHA256_OVERRIDE}" ]; then
    echo "Non-default ttyd version ${VERSION} requires TTYD_SHA256 to keep installs deterministic." >&2
    exit 2
  fi
  binary_sha256="${TTYD_SHA256_OVERRIDE}"
fi

base_url="https://github.com/tsl0922/ttyd/releases/download/${VERSION}"

mkdir -p "${BIN_DIR}"
tmp_dir="$(mktemp -d)"
trap 'rm -rf "${tmp_dir}"' EXIT

download_file "${base_url}/${asset_name}" "${tmp_dir}/${asset_name}"
printf '%s  %s\n' "${binary_sha256}" "${asset_name}" > "${tmp_dir}/ttyd.sha256"
(
  cd "${tmp_dir}"
  verify_sha256 ttyd.sha256
)

install -m 0755 "${tmp_dir}/${asset_name}" "${BIN_DIR}/ttyd"
echo "Installed ttyd ${VERSION} to ${BIN_DIR}/ttyd"
