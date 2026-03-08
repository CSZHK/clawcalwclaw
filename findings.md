# Findings

- Session started; findings will be appended as codebase evidence is collected.
- Verified clean dedicated worktree at `/var/tmp/vibe-kanban/worktrees/9c7a-req-20260308-01/zeroclaw` on branch `vk/9c7a-req-20260308-01`.
- Root repository contains only the top-level `AGENTS.md`; no deeper AGENTS overrides were found.
- `docs/plans/2026-03-08-ui-acceptance-test-plan.md` is not present in this worktree; treat the user-provided path as an out-of-tree/local draft reference unless later found elsewhere.
- `docs/tui-testing.md` already documents L1/L2/L3 TUI testing and points to `tui-automation` plus `run_tui_tmux_capture.sh` as the current artifact-producing lane.
- `scripts/ci/run_tui_tmux_capture.sh` already bootstraps a local config/workspace, starts tmux-backed TUI sessions, captures pane output, and emits `.ascii`/`.gif`/`.mp4` artifacts with a `vhs` or ffmpeg fallback.
- `web/package.json` currently has only `dev`, `build`, and `preview`; there is no browser E2E runner or test script yet.
- The web shell exposes a pairing gate before authenticated routes; likely P0 browser coverage can center on pairing, then `/agent`, `/tools`, and `/config` flows behind stubbed API/auth responses.
- Existing PTY TUI scenarios already cover startup/help/setup/quit, double-ctrl-c quit, and small-terminal layout; the main hardening gap is that they are ignored and not wired into CI as assertions.
- Current CI TUI smoke capture is artifact-friendly but not assertion-rich; it validates readiness heuristically and records playback artifacts without checking explicit UI text transitions.
- TUI P0 gate now promotes the existing PTY harness into merge-blocking assertions while preserving replay capture as a separate evidence layer.
- Web acceptance bootstrap uses Playwright with deterministic HTTP/WebSocket fixtures, retained trace/screenshot/video artifacts, and attached console/network logs on failure.
- PR gating is enforced in `ci-run.yml`; a separate `ui-acceptance.yml` scheduled/manual lane is added as the expansion path for broader UI coverage.
- Focused validation passed for `scripts/ci/detect_change_scope.sh`, workflow YAML parsing, `web` build, Playwright P0 acceptance, and the TUI PTY acceptance suite under Rust 1.92.0.
- Global `cargo fmt --all -- --check` still reports pre-existing unrelated formatting drift in other repository files; the task-specific Rust file was formatted directly instead of broadening scope.
