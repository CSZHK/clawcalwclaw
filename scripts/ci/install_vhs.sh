#!/usr/bin/env bash
set -euo pipefail

# Install a pinned VHS binary into a writable bin directory.
# Usage: ./scripts/ci/install_vhs.sh <bin_dir> [version]
# For non-default versions, set VHS_SHA256 to the expected archive digest.

BIN_DIR="${1:-${RUNNER_TEMP:-/tmp}/bin}"
VERSION="${2:-${VHS_VERSION:-v0.10.0}}"
VHS_SHA256_OVERRIDE="${VHS_SHA256:-}"

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
  echo "Unsupported OS for VHS installer: ${os_name}" >&2
  exit 2
fi

arch_name="$(uname -m)"
case "${arch_name}" in
  x86_64|amd64)
    archive_suffix="Linux_x86_64"
    pinned_sha256="b552c3870aca101dcafe533cfef32dceb7b783400ad32642e728775c9f125407"
    ;;
  aarch64|arm64)
    archive_suffix="Linux_arm64"
    pinned_sha256="6d7300028d4641b9dc004a05cf411d40f1e12e560cc6fca985dd90504b6652a7"
    ;;
  *)
    echo "Unsupported architecture for VHS installer: ${arch_name}" >&2
    exit 2
    ;;
esac

archive_name="vhs_${VERSION#v}_${archive_suffix}.tar.gz"
archive_sha256="${pinned_sha256}"
if [ "${VERSION}" != "v0.10.0" ]; then
  if [ -z "${VHS_SHA256_OVERRIDE}" ]; then
    echo "Non-default VHS version ${VERSION} requires VHS_SHA256 to keep installs deterministic." >&2
    exit 2
  fi
  archive_sha256="${VHS_SHA256_OVERRIDE}"
fi

base_url="https://github.com/charmbracelet/vhs/releases/download/${VERSION}"

mkdir -p "${BIN_DIR}"
tmp_dir="$(mktemp -d)"
trap 'rm -rf "${tmp_dir}"' EXIT

download_file "${base_url}/${archive_name}" "${tmp_dir}/${archive_name}"
printf '%s  %s\n' "${archive_sha256}" "${archive_name}" > "${tmp_dir}/vhs.sha256"
(
  cd "${tmp_dir}"
  verify_sha256 vhs.sha256
)

tar -xzf "${tmp_dir}/${archive_name}" -C "${tmp_dir}"
binary_path="$(find "${tmp_dir}" -type f -name vhs -print -quit)"
if [ -z "${binary_path}" ]; then
  echo "Failed to locate vhs binary in ${archive_name}" >&2
  exit 1
fi

install -m 0755 "${binary_path}" "${BIN_DIR}/vhs"
echo "Installed VHS ${VERSION} to ${BIN_DIR}/vhs"
