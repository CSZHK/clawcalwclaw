# src/tui/CLAUDE.md — TUI Subsystem Rules

Scope: `src/tui/`. Read in conjunction with [`src/CLAUDE.md`](../CLAUDE.md).

---

## TUI Architecture Contract

- **No `TuiBackend` trait** (ADR-1, YAGNI) — ratatui is the only backend; extract a trait only when a second backend emerges (rule-of-three).
- Module map:
  - `app.rs` — TuiApp state machine and async event loop
  - `events.rs` — `TuiEvent` enum + `translate_delta` (sole agent-coupling point)
  - `state.rs` — `TuiState` view-model (zero `agent/` imports)
  - `terminal.rs` — terminal lifecycle: panic hook, signal handlers, raw-mode management
  - `widgets/` — render functions (`chat`, `input`, `tools`, `status`, `sanitize`)
- **Coupling rule**: `state.rs` must never import from `crate::agent`; `events.rs` is the only module that imports agent sentinel constants.

---

## Sentinel Protocol Contract (Critical)

`translate_delta` in `events.rs` maps agent draft-stream payloads to typed `TuiEvent` variants. The match order is **load-bearing**:

```
1. delta == DRAFT_CLEAR_SENTINEL          → TuiEvent::Clear          (exact match)
2. delta.strip_prefix(PROGRESS_BLOCK)     → TuiEvent::ProgressBlock  (prefix match)
3. delta.strip_prefix(PROGRESS)           → TuiEvent::ProgressLine   (prefix match)
4. else                                   → TuiEvent::Delta          (fallthrough)
```

**PROGRESS_BLOCK must be checked before PROGRESS** because `"\x00PROGRESS_BLOCK\x00"` has `"\x00PROGRESS\x00"` as a prefix. Reversing steps 2-3 causes silent data corruption: progress blocks would be misrouted as progress lines, losing tool-output content.

Reference: `src/tui/events.rs:27-41`, sentinel constants in `src/agent/loop_.rs`.

---

## Terminal Lifecycle Contract (Critical)

Under `panic = "abort"` (release profile), `Drop` implementations **never execute** after a panic. Terminal restoration relies exclusively on:

1. **Panic hook** (`terminal.rs:install_panic_hook`) — restores terminal state synchronously before abort.
2. **Signal handlers** (`terminal.rs:install_signal_handlers`) — handles SIGTERM/SIGHUP for Docker/systemd scenarios.

**Initialization order is load-bearing** (`app.rs:run()`):

```
1. install_panic_hook()            ← must be first (covers panics during subsequent steps)
2. install_signal_handlers(token)  ← second (covers signals during raw mode)
3. enable_raw_mode()               ← third
4. EnterAlternateScreen            ← fourth
```

Reference: `src/tui/app.rs:64-77`, `src/tui/terminal.rs`.

**Prohibited pattern**: Do not add `TerminalGuard` with `impl Drop` as a terminal-restore mechanism — it is ineffective under `panic = "abort"`.

---

## Security Constraints

These rules apply to all `src/tui/` code. See also `src/CLAUDE.md` §3.6 (Secure by Default).

- **Render-path sanitization**: All text rendered to the terminal must pass through `widgets/sanitize.rs::sanitize_text()` to strip ANSI/VT escape sequences and control characters.
- **Input size limit**: `InputBox` enforces a 64KB hard limit on user input (`widgets/input.rs`).
- **StatusBar credential ban**: `StatusBar` must never display API keys, tokens, or any credential fields — model name and provider ID only.
- **Logging restrictions**: Logs must not contain user input content or agent response content. Log metadata only (e.g., delta length, event type).

---

## 7.7 Adding a TUI Widget (Change Playbook)

1. Create `src/tui/widgets/<name>.rs` with a `pub fn render(frame, area, state)` signature.
2. Add `pub mod <name>;` to `src/tui/widgets/mod.rs`.
3. In `widgets/mod.rs::render()`, add a `Constraint` entry and corresponding area index for the new widget.
4. All rendered text must pass through `sanitize::sanitize_text()` — no exceptions.
5. If the widget consumes a new `TuiEvent` variant:
   - Add the variant to `events.rs::TuiEvent`.
   - Update `translate_delta` in `events.rs` — maintain sentinel match order (see Sentinel Protocol above).
   - Handle the variant in `app.rs::handle_tui_event()`.
6. Add L1 state tests (`src/tui/state.rs`) for any new state fields.
7. Add L2 render tests (`tests/tui_render_test.rs`) for visual output verification.
8. Run validation:

```bash
cargo test --lib tui
CARGO_BUILD_JOBS=2 cargo test --features tui-ratatui --test tui_render_test --test tui_event_handling_test
```

See `docs/tui-testing.md` for the full test pyramid and CI integration details.

---

## Validation (Quick Reference)

For all TUI changes, run at minimum:

```bash
# L1 unit tests (no feature flag needed)
cargo test --lib tui

# L2 integration tests (requires tui-ratatui feature)
CARGO_BUILD_JOBS=2 cargo test --features tui-ratatui \
    --test tui_render_test --test tui_event_handling_test
```

Full validation matrix: see [`src/CLAUDE.md`](../CLAUDE.md) §8.
Full test strategy: see [`docs/tui-testing.md`](../../docs/tui-testing.md).
