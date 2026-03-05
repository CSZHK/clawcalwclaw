#!/usr/bin/env bash
set -euo pipefail

if [ -f "dev/docker-compose.ci.yml" ]; then
  COMPOSE_FILE="dev/docker-compose.ci.yml"
elif [ -f "docker-compose.ci.yml" ] && [ "$(basename "$(pwd)")" = "dev" ]; then
  COMPOSE_FILE="docker-compose.ci.yml"
else
  echo "❌ Run this script from repo root or dev/ directory."
  exit 1
fi

compose_cmd=(docker compose -f "$COMPOSE_FILE")
SMOKE_CACHE_DIR="${SMOKE_CACHE_DIR:-.cache/buildx-smoke}"
CI_MIRROR_PROFILE="${CLAWCLAWCLAW_CI_MIRROR:-${CLAWCLAWCLAW_BOOTSTRAP_MIRROR:-rsproxy}}"

configure_ci_mirror_env() {
  local normalized_profile
  normalized_profile="$(printf '%s' "$CI_MIRROR_PROFILE" | tr '[:upper:]' '[:lower:]')"

  case "$normalized_profile" in
    ""|none)
      return 0
      ;;
    rsproxy)
      export CARGO_REGISTRIES_CRATES_IO_PROTOCOL="${CARGO_REGISTRIES_CRATES_IO_PROTOCOL:-sparse}"
      export CARGO_REGISTRIES_CRATES_IO_INDEX="${CARGO_REGISTRIES_CRATES_IO_INDEX:-sparse+https://rsproxy.cn/index/}"
      export RUSTUP_DIST_SERVER="${RUSTUP_DIST_SERVER:-https://rsproxy.cn}"
      export RUSTUP_UPDATE_ROOT="${RUSTUP_UPDATE_ROOT:-https://rsproxy.cn/rustup}"
      echo "Using CI Rust mirror profile: rsproxy"
      ;;
    *)
      echo "❌ Unsupported CLAWCLAWCLAW_CI_MIRROR='$CI_MIRROR_PROFILE' (supported: rsproxy, none)."
      exit 1
      ;;
  esac
}

run_in_ci() {
  local cmd="$1"
  "${compose_cmd[@]}" run --rm local-ci bash -c "$cmd"
}

build_smoke_image() {
  if docker buildx version >/dev/null 2>&1; then
    mkdir -p "$SMOKE_CACHE_DIR"
    local build_args=(
      --load
      --target dev
      --cache-to "type=local,dest=$SMOKE_CACHE_DIR,mode=max"
      -t clawclawclaw-local-smoke:latest
      .
    )
    if [ -f "$SMOKE_CACHE_DIR/index.json" ]; then
      build_args=(--cache-from "type=local,src=$SMOKE_CACHE_DIR" "${build_args[@]}")
    fi
    docker buildx build "${build_args[@]}"
  else
    DOCKER_BUILDKIT=1 docker build --target dev -t clawclawclaw-local-smoke:latest .
  fi
}

print_help() {
  cat <<'EOF'
clawclawclaw Local CI in Docker

Usage: ./dev/ci.sh <command>

Commands:
  build-image   Build/update the local CI image
  shell         Open an interactive shell inside the CI container
  lint          Run rustfmt + clippy correctness gate (container only)
  lint-strict   Run rustfmt + full clippy warnings gate (container only)
  lint-delta    Run strict lint delta gate on changed Rust lines (container only)
  tui           Run TUI-focused checks (feature build + TUI tests)
  test          Run cargo test (container only)
  build         Run release build smoke check (container only)
  audit         Run cargo audit (container only)
  deny          Run cargo deny check (container only)
  security      Run cargo audit + cargo deny (container only)
  docker-smoke  Build and verify runtime image (host docker daemon)
  all           Run lint, test, build, security, docker-smoke
  clean         Remove local CI containers and volumes

Environment:
  CLAWCLAWCLAW_CI_MIRROR=rsproxy|none
                Rust/Cargo mirror profile for local CI image and container runs (default: rsproxy).
EOF
}

if [ $# -lt 1 ]; then
  print_help
  exit 1
fi

configure_ci_mirror_env

case "$1" in
  build-image)
    "${compose_cmd[@]}" build local-ci
    ;;

  shell)
    "${compose_cmd[@]}" run --rm local-ci bash
    ;;

  lint)
    run_in_ci "./scripts/ci/rust_quality_gate.sh"
    ;;

  lint-strict)
    run_in_ci "./scripts/ci/rust_quality_gate.sh --strict"
    ;;

  lint-delta)
    run_in_ci "./scripts/ci/rust_strict_delta_gate.sh"
    ;;

  tui)
    run_in_ci "cargo check --locked --features tui-ratatui"
    run_in_ci "cargo test --locked --features tui-ratatui tui:: -- --test-threads=1"
    ;;

  test)
    run_in_ci "cargo test --locked --verbose"
    ;;

  build)
    run_in_ci "cargo build --release --locked --verbose"
    ;;

  audit)
    run_in_ci "cargo audit"
    ;;

  deny)
    run_in_ci "cargo deny check licenses sources"
    ;;

  security)
    run_in_ci "cargo deny check licenses sources"
    run_in_ci "cargo audit"
    ;;

  docker-smoke)
    build_smoke_image
    docker run --rm clawclawclaw-local-smoke:latest --version
    ;;

  all)
    run_in_ci "./scripts/ci/rust_quality_gate.sh"
    run_in_ci "cargo test --locked --verbose"
    run_in_ci "cargo build --release --locked --verbose"
    run_in_ci "cargo deny check licenses sources"
    run_in_ci "cargo audit"
    build_smoke_image
    docker run --rm clawclawclaw-local-smoke:latest --version
    ;;

  clean)
    "${compose_cmd[@]}" down -v --remove-orphans
    ;;

  *)
    print_help
    exit 1
    ;;
esac
