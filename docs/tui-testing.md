# TUI Testing Guide

This document describes how to validate the Terminal User Interface (TUI) module of clawclawclaw.

## Test Architecture

The TUI module uses a three-level testing strategy:

```text
┌─────────────────────────────────────────────────────────────────┐
│                        TUI Testing Pyramid                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│                     ┌──────────────────────┐                    │
│                     │      L3 Capture      │                    │
│                     │ tmux + VHS artifacts │                    │
│                     │ .ascii + .gif + .mp4│                    │
│                     └──────────┬───────────┘                    │
│                                │                                │
│               ┌────────────────┴───────────────┐                │
│               │         Integration (L2)       │                │
│               │ render + event handling tests  │                │
│               │ `--features tui-ratatui`       │                │
│               └────────────────┬───────────────┘                │
│                                │                                │
│      ┌─────────────────────────┴─────────────────────────┐      │
│      │                    Unit Tests (L1)                │      │
│      │           pure state + event translation          │      │
│      └───────────────────────────────────────────────────┘      │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## Test Surfaces

| Level | Surface | Command / Artifact | Description |
| --- | --- | --- | --- |
| L1 | `src/tui/state.rs`, `src/tui/events.rs` | `cargo test --lib tui` | Pure logic and event translation |
| L2 | `tests/tui_render_test.rs`, `tests/tui_event_handling_test.rs` | `cargo test --features tui-ratatui --test tui_render_test --test tui_event_handling_test` | Widget rendering and event/state transitions |
| L3 | `tmux` + `vhs` capture flow | `artifacts/tui/*.ascii`, `artifacts/tui/*.gif`, `artifacts/tui/*.mp4` | Real terminal evidence for smoke scenarios and reviewer-friendly playback |

The repository still contains `tests/tui_e2e_pty.rs` as a manual tmux harness, but the merge-blocking source of truth is the `tui-automation` CI lane plus `scripts/ci/run_tui_tmux_capture.sh`.

## Running Tests

### Quick Tests for Daily Development

```bash
cargo test --lib tui

CARGO_BUILD_JOBS=2 cargo test --features tui-ratatui \
    --test tui_render_test --test tui_event_handling_test
```

### Focused Container Check

`dev/ci.sh` already provides a focused TUI helper for local containerized verification:

```bash
./dev/ci.sh tui
```

That helper currently runs:

```bash
cargo check --locked --features tui-ratatui
cargo test --locked --features tui-ratatui tui:: -- --test-threads=1
```

### Manual Interactive Run

```bash
cargo run --bin clawclawclaw --features tui-ratatui -- tui
```

Useful shortcuts during manual testing:

- `i` enters input mode
- `Esc` exits input mode
- `Enter` sends a message
- `q` exits the TUI
- `Ctrl+C` twice forces termination

### L3 Smoke Capture with tmux + VHS

This is the same flow used by the dedicated CI lane when `tui_changed` is true, and it remains the recommended local command for reviewer-friendly artifact capture.

#### 1. Check capture dependencies

```bash
command -v tmux
command -v vhs
command -v ttyd
command -v ffmpeg
```

#### 2. Build the TUI binary once

```bash
CARGO_BUILD_JOBS=2 cargo build --locked --features tui-ratatui --bin clawclawclaw
```

#### 3. Record the smoke flow and emit all artifacts

```bash
mkdir -p artifacts/tui

cat > /tmp/zeroclaw-tui-smoke.tape <<'__TAPE__'
Output artifacts/tui/tui-smoke.ascii
Output artifacts/tui/tui-smoke.gif
Output artifacts/tui/tui-smoke.mp4
Require tmux
Require ttyd
Require ffmpeg

Set Shell "bash"
Set TypingSpeed 0ms
Set Width 1200
Set Height 800

Type "tmux new-session -d -s zeroclaw-tui 'TERM=xterm-256color target/debug/clawclawclaw tui'"
Enter
Type "tmux attach -t zeroclaw-tui"
Enter
Sleep 3s
Type "q"
Sleep 1s
Type "exit"
Enter
__TAPE__

vhs /tmp/zeroclaw-tui-smoke.tape
```

This produces the canonical artifact bundle:

- `artifacts/tui/tui-smoke.ascii`
- `artifacts/tui/tui-smoke.gif`
- `artifacts/tui/tui-smoke.mp4`

If you need provider-specific coverage, replace `target/debug/clawclawclaw tui` in the tape with the exact invocation you want to capture, for example `target/debug/clawclawclaw tui --provider openai --model gpt-5`.

## CI Expectations

Current repository behavior and expectations are intentionally split:

- `ci-run.yml` remains the merge-blocking baseline and runs generic Rust validation plus `cargo test --locked --verbose`.
- `feature-matrix.yml` currently validates feature compilation lanes, but it does not yet define a dedicated `tui-ratatui` artifact-capture lane.
- `./dev/ci.sh tui` is the current focused pre-PR helper for TUI-specific build/test coverage.
- L3 capture should stay opt-in or manual until a dedicated self-hosted lane exists, because `tmux`, `vhs`, `ttyd`, and `ffmpeg` are not guaranteed on default runners.
- When a TUI capture lane is used locally or in CI, it should upload the full reviewer bundle: `.ascii`, `.gif`, and `.mp4`.

## Adding New TUI Coverage

When adding or changing TUI behavior:

1. Add or update L1 tests for pure state and event logic.
2. Add or update L2 tests for rendering and state transitions.
3. Refresh the tmux + VHS smoke tape or scenario steps when the user-visible flow changes.
4. Keep artifact names stable and easy to diff under `artifacts/tui/`.
5. Prefer small, deterministic capture flows that are easy to rerun on self-hosted or manual CI jobs.

## Troubleshooting

See [troubleshooting.md](troubleshooting.md) for dependency issues, tmux/VHS capture failures, and raw-terminal debugging tips.
