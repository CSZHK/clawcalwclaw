#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Run the prebuilt TUI binary inside tmux, smoke-check startup/quit, and render VHS artifacts.

Usage:
  ./scripts/ci/run_tui_tmux_capture.sh --binary <path> [--artifacts-dir <dir>] [--startup-timeout-seconds <n>]
EOF
}

require_cmd() {
  local cmd="$1"
  if ! command -v "${cmd}" >/dev/null 2>&1; then
    echo "Missing required command: ${cmd}" >&2
    exit 1
  fi
}

have_cmd() {
  command -v "$1" >/dev/null 2>&1
}

bootstrap_tui_config() {
  local binary="$1"
  local home_dir="$2"
  local config_dir="$3"
  local workspace_dir="$4"

  env     HOME="${home_dir}"     CLAWCLAWCLAW_CONFIG_DIR="${config_dir}"     CLAWCLAWCLAW_WORKSPACE="${workspace_dir}"     TERM=xterm-256color     "${binary}" onboard --force --no-totp --provider openrouter >/dev/null

  env     HOME="${home_dir}"     CLAWCLAWCLAW_CONFIG_DIR="${config_dir}"     CLAWCLAWCLAW_WORKSPACE="${workspace_dir}"     TERM=xterm-256color     "${binary}" config set security.otp.enabled false >/dev/null
}

wait_for_session_output() {
  local session_name="$1"
  local output_file="$2"
  local timeout_secs="$3"
  local deadline=$((SECONDS + timeout_secs))

  while [ "${SECONDS}" -lt "${deadline}" ]; do
    if ! tmux has-session -t "${session_name}" 2>/dev/null; then
      echo "tmux session ${session_name} exited before the TUI became ready" >&2
      return 1
    fi

    tmux capture-pane -p -e -t "${session_name}:0.0" -S -200 > "${output_file}" || true
    local printable_count
    printable_count="$(tr -cd '[:alnum:]' < "${output_file}" | wc -c | tr -d '[:space:]')"
    if [ "${printable_count:-0}" -ge 12 ]; then
      return 0
    fi

    sleep 1
  done

  echo "Timed out waiting for readable TUI output in ${session_name}" >&2
  return 1
}

wait_for_session_exit() {
  local session_name="$1"
  local timeout_secs="$2"
  local deadline=$((SECONDS + timeout_secs))

  while [ "${SECONDS}" -lt "${deadline}" ]; do
    if ! tmux has-session -t "${session_name}" 2>/dev/null; then
      return 0
    fi
    sleep 1
  done

  echo "Timed out waiting for tmux session ${session_name} to exit" >&2
  return 1
}

start_tui_session() {
  local session_name="$1"
  local command_string
  printf -v command_string 'env TERM=xterm-256color RUST_LOG=error HOME=%q CLAWCLAWCLAW_CONFIG_DIR=%q CLAWCLAWCLAW_WORKSPACE=%q %q %q %q %q'     "${tmp_home_dir}"     "${tmp_config_dir}"     "${tmp_workspace_dir}"     "${binary_path}"     "tui"     "--provider"     "openrouter"
  tmux new-session -d -s "${session_name}" "${command_string}"
}

render_ascii_with_ffmpeg() {
  local ascii_file="$1"
  local gif_path="$2"
  local mp4_path="$3"
  local font_file=""

  if command -v fc-match >/dev/null 2>&1; then
    font_file="$(fc-match monospace -f '%{file}\n' | head -n 1)"
  fi
  if [ -z "${font_file}" ] || [ ! -f "${font_file}" ]; then
    font_file="/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf"
  fi
  if [ ! -f "${font_file}" ]; then
    echo "Unable to locate a monospace font for ffmpeg fallback rendering." >&2
    exit 1
  fi

  ffmpeg -y \
    -f lavfi -i color=c=0x111111:s=1400x900:d=3 \
    -vf "drawtext=fontfile='${font_file}':textfile='${ascii_file}':fontcolor=white:fontsize=20:line_spacing=6:x=40:y=40" \
    -pix_fmt yuv420p \
    "${mp4_path}" >/dev/null 2>&1

  ffmpeg -y -i "${mp4_path}" "${gif_path}" >/dev/null 2>&1
}

binary_path=""
artifacts_dir="artifacts/tui"
startup_timeout_secs="${TUI_STARTUP_TIMEOUT_SECS:-30}"
smoke_session=""
capture_session=""
tmp_root=""
tmp_home_dir=""
tmp_config_dir=""
tmp_workspace_dir=""

while [ "$#" -gt 0 ]; do
  case "$1" in
    --binary)
      binary_path="$2"
      shift 2
      ;;
    --artifacts-dir)
      artifacts_dir="$2"
      shift 2
      ;;
    --startup-timeout-seconds)
      startup_timeout_secs="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [ -z "${binary_path}" ]; then
  echo "--binary is required" >&2
  usage >&2
  exit 2
fi

if [ ! -x "${binary_path}" ]; then
  echo "TUI binary is not executable: ${binary_path}" >&2
  exit 2
fi

require_cmd tmux
require_cmd ffmpeg

mkdir -p "${artifacts_dir}"
artifacts_dir="$(cd "${artifacts_dir}" && pwd)"

tmp_root="$(mktemp -d)"
tmp_home_dir="${tmp_root}/home"
tmp_config_dir="${tmp_root}/config"
tmp_workspace_dir="${tmp_root}/workspace"
mkdir -p "${tmp_home_dir}" "${tmp_config_dir}" "${tmp_workspace_dir}"

bootstrap_tui_config "${binary_path}" "${tmp_home_dir}" "${tmp_config_dir}" "${tmp_workspace_dir}"

cleanup() {
  if [ -n "${smoke_session}" ] && tmux has-session -t "${smoke_session}" 2>/dev/null; then
    tmux kill-session -t "${smoke_session}" || true
  fi
  if [ -n "${capture_session}" ] && tmux has-session -t "${capture_session}" 2>/dev/null; then
    tmux kill-session -t "${capture_session}" || true
  fi
  if [ -n "${tmp_root}" ] && [ -d "${tmp_root}" ]; then
    rm -rf "${tmp_root}"
  fi
}
trap cleanup EXIT

session_suffix="${GITHUB_RUN_ID:-local}-${GITHUB_RUN_ATTEMPT:-0}-$$"
smoke_session="zc-tui-smoke-${session_suffix}"
capture_session="zc-tui-capture-${session_suffix}"

smoke_capture_path="${artifacts_dir}/tui-smoke-pane.txt"
capture_probe_path="${artifacts_dir}/tui-capture-pane.txt"
tape_path="${artifacts_dir}/tui-demo.tape"

rm -f \
  "${smoke_capture_path}" \
  "${capture_probe_path}" \
  "${tape_path}" \
  "${artifacts_dir}/tui-demo.ascii" \
  "${artifacts_dir}/tui-demo.gif" \
  "${artifacts_dir}/tui-demo.mp4"

start_tui_session "${smoke_session}"
wait_for_session_output "${smoke_session}" "${smoke_capture_path}" "${startup_timeout_secs}"
tmux send-keys -t "${smoke_session}:0.0" C-d
wait_for_session_exit "${smoke_session}" 15

start_tui_session "${capture_session}"
wait_for_session_output "${capture_session}" "${capture_probe_path}" "${startup_timeout_secs}"

cat > "${tape_path}" <<EOF
Output ${artifacts_dir}/tui-demo.ascii
Output ${artifacts_dir}/tui-demo.gif
Output ${artifacts_dir}/tui-demo.mp4
Require tmux
Require ttyd
Require ffmpeg
Set Shell "bash"
Set FontSize 18
Set Width 1400
Set Height 900
Set TypingSpeed 0ms

Hide
Type "tmux attach-session -t ${capture_session}"
Enter
Show
Sleep 2s
EOF

if have_cmd vhs && have_cmd ttyd; then
  vhs "${tape_path}"
else
  echo "vhs/ttyd unavailable; using ffmpeg fallback artifact renderer." >&2
  cp "${capture_probe_path}" "${artifacts_dir}/tui-demo.ascii"
  render_ascii_with_ffmpeg \
    "${artifacts_dir}/tui-demo.ascii" \
    "${artifacts_dir}/tui-demo.gif" \
    "${artifacts_dir}/tui-demo.mp4"
fi
if ! wait_for_session_exit "${capture_session}" 5; then
  echo "Capture session did not exit on its own; cleaning up tmux session after artifact generation." >&2
  tmux kill-session -t "${capture_session}" >/dev/null 2>&1 || true
fi

printf 'Generated TUI artifacts in %s\n' "${artifacts_dir}"
find "${artifacts_dir}" -maxdepth 1 -type f | sort
