# UI Acceptance Runbook

This runbook defines the deterministic UI acceptance contract for the existing `src/tui/` and `web/` surfaces.

## Scope

- PR gate: minimal deterministic P0 critical path
- Nightly/manual lane: dedicated `UI Acceptance` workflow with the same artifact contract today, ready for broader coverage expansion later
- Non-goals: external providers, production tokens, unstable internet dependencies

## TUI P0 Gate

### Local prerequisites

```bash
command -v tmux
command -v script
command -v ffmpeg
command -v vhs   # optional locally; ffmpeg fallback is supported
command -v ttyd  # optional locally; ffmpeg fallback is supported
```

### Local commands

```bash
CARGO_BUILD_JOBS=2 cargo build --locked --features tui-ratatui --bin clawclawclaw
TUI_ARTIFACT_DIR=artifacts/tui/pty \
  cargo test --locked --features tui-ratatui --test tui_e2e_pty -- --test-threads=1
./scripts/ci/run_tui_tmux_capture.sh \
  --binary target/debug/clawclawclaw \
  --artifacts-dir artifacts/tui/capture
```

### Assertions covered

- startup readiness and help overlay
- setup/help path from the initial terminal state
- documented quit path (`q`) and forced quit path (`Ctrl+C` twice)

### Failure evidence

- PTY assertions write per-scenario snapshots into `artifacts/tui/pty/`
- Replay capture writes `artifacts/tui/capture/tui-demo.ascii`
- Replay capture writes `artifacts/tui/capture/tui-demo.gif`
- Replay capture writes `artifacts/tui/capture/tui-demo.mp4`
- The capture helper now fails if any expected replay artifact is missing or empty

## Web P0 Gate

### Local prerequisites

```bash
cd web
npm ci
npx playwright install chromium
```

### Local command

```bash
cd web
PLAYWRIGHT_BROWSERS_PATH="$PWD/.playwright" npm run test:e2e
```

### Assertions covered

- pairing flow unlocks the dashboard
- agent chat renders deterministic websocket tool-call and tool-result frames
- tools page lists deterministic registry data
- config page edits and saves deterministic TOML fixture data

### Failure evidence

Playwright is configured to retain on failure:

- trace
- screenshot
- video
- attached console log
- attached network log
- HTML report

Artifacts are emitted under `web/test-results/` and `web/playwright-report/`.

## CI Layout

### PR / merge-group

- `CI Run` remains the merge-blocking umbrella workflow
- `tui-automation` now includes the PTY assertion suite plus replay capture artifacts
- `web-acceptance` now runs Playwright P0 acceptance with deterministic fixtures
- `CI Required Gate` enforces both jobs when their respective change-scope flags are set

### Nightly / manual

- `.github/workflows/ui-acceptance.yml` runs dedicated TUI and web acceptance jobs for scheduled/manual review
- Current nightly/manual scope mirrors the P0 contract to keep rollout reversible
- Future expansion should layer heavier transport breadth there before widening PR scope

## Extension Strategy

Keep PR scope minimal and deterministic. Add broader coverage in the dedicated UI workflow first, for example:

- TUI: logs, doctor, memory, cron, sub-agent/task-board flows
- Web: SSE/log streaming, doctor, memory, cron, integrations, mobile viewport checks
- Artifact breadth: richer console/network snapshots, HAR export, additional VHS tapes

Promote new scenarios into the PR gate only after they are deterministic and cheap enough to block merges safely.
