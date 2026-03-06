# TUI Workbench Implementation Checklist (2026-03-06)

Status: implementation checklist draft  
Type: code-oriented delivery checklist  
Scope: `src/tui/`, `src/agent/`, `src/tools/`

---

## 1. Purpose

This document turns the 2026-03-06 TUI workbench proposal set into a file-scoped implementation checklist.

It is not a new vision document. Its job is to answer one narrower question:

> What exact repository files need to change, in what order, with what boundaries, and with what minimum validation, before coding starts?

This checklist depends on the following companion documents:

- `docs/plans/2026-03-06-tui-agent-workbench-proposal.md`
- `docs/plans/2026-03-06-tui-workbench-bridge-contracts.md`
- `docs/tui-agent-workbench-vision.md`

---

## 2. Repository Baseline (As Of `9c62d744`)

Before coding, anchor to the current repository facts instead of to an imagined future state.

### 2.1 Already present

- TUI already creates and passes `NonCliApprovalContext` into
  `run_tool_call_loop_with_non_cli_approval_context(...)` from
  `src/tui/app.rs`.
- TUI already renders a basic approval modal via
  `src/tui/widgets/approval.rs`.
- `GoalEngine` already exposes a read path via `GoalEngine::load_state()` in
  `src/goals/engine.rs`.
- `TaskPlanTool` already keeps session-scoped tasks in memory in
  `src/tools/task_plan.rs`.
- Sub-agent lifecycle authority already exists in `SubAgentRegistry` and
  related tools.

### 2.2 Gaps that still block workbench implementation

- Approval UI is currently **single-slot**, not queue-based:
  `src/tui/state.rs` stores `pending_approval: Option<PendingApproval>`.
- TUI approval resolution currently records only final resolution in
  `src/tui/app.rs`; it does **not** confirm or reject through the existing
  `ApprovalManager` authority API first.
- Approval argument preview is currently simple string truncation, not a
  contract-aware redacted preview.
- `TaskPlanTool` has no public read-only snapshot API for projection building.
- TUI currently cannot access same-session strong typed handles for
  `TaskPlanTool` or `SubAgentRegistry`; `TuiRuntimeContext` only keeps a tool
  registry and the `Tool` trait has no downcast path.
- Agentic sub-agent execution still uses a local `NoopObserver` path in
  `src/tools/subagent_spawn.rs`.
- No dedicated TUI projection module exists yet for task board or sub-agent
  live pane state.

### 2.3 Coding rule for this track

Do not treat the checklist as permission for a big-bang refactor. Each file
change should remain small, reviewable, and reversible.

---

## 3. Delivery Order

Implement in the following order and do not overlap tracks unless the previous
track's exit gate in this section is already met.

1. **Approval Bridge hardening**
2. **Task board read-only projection**
3. **Sub-agent observability forwarding**

Rationale:

- Approval closes a supervised safety loop and should land first.
- Task board is read-only and adds operator value with low authority risk.
- Sub-agent telemetry is the deepest runtime touch and should land last.

### 3.1 Track exit gates

#### Approval Bridge exit gate

This track is complete only when all of the following are true:

- TUI approval intake is queue-based rather than single-slot.
- Approval resolution first calls runtime authority methods
  (`confirm_non_cli_pending_request(...)` or
  `reject_non_cli_pending_request(...)`) and then records the matching
  `ApprovalResponse`.
- Any bridge failure resolves fail-closed.
- Focused approval tests in §9.1 pass.

#### Task board exit gate

This track is complete only when all of the following are true:

- `bootstrap_runtime(...)` returns or stores explicit workbench read handles for
  the same-session authorities needed by TUI.
- `TaskPlanTool::snapshot()` or an equivalent read-only adapter exists.
- Task board projection building lives outside `src/tui/state.rs` and outside
  widget modules.
- Focused projection tests in §9.2 pass.

#### Sub-agent observability exit gate

This track is complete only when all of the following are true:

- Telemetry wiring order is explicit and reproducible at bootstrap time.
- `src/tools/**` remains TUI-agnostic and does not import `src/tui/**`.
- `SubAgentRegistry` remains the only lifecycle authority.
- Focused sub-agent observability tests in §9.3 pass.

---

## 4. `src/tui/` Checklist

This section defines the TUI-facing implementation surface. TUI remains a
projection/rendering layer, not an authority owner.

### 4.1 `src/tui/state.rs`

- [ ] **Goal**: keep render-state lean and explicit.
- [ ] **Change**: replace the single `pending_approval` slot with queue-first
  approval render state, but do **not** move projection-building logic or
  authority reads into this file.
- [ ] **Input/output contract**: TUI state stores only render-safe fields such
  as IDs, redacted summaries, statuses, timestamps, and lightweight labels; it
  must not retain raw secret-bearing payloads as authority data.
- [ ] **Failure / rollback behavior**: unresolved approval items stay explicitly
  pending/failed/expired; no state transition may imply approval unless a
  positive runtime resolution has already succeeded.
- [ ] **Minimum validation**: add L1 tests for enqueue/dequeue/next-item
  approval flow, queue overwrite prevention, and render-state update behavior.

Suggested sub-steps:

- [ ] Introduce an approval queue representation such as
  `VecDeque<ApprovalQueueItem>` or an equivalent explicit ordered container.
- [ ] Add terminal statuses for approval render items: `Pending`, `Approved`,
  `Denied`, `Failed`, `Expired`.
- [ ] Keep task board and sub-agent pane data here as already-built projection
  snapshots only; do not place projection mapping logic here.

### 4.2 `src/tui/projections.rs` (new)

- [ ] **Goal**: host TUI projection builders and adapters in one explicit module
  so `src/tui/state.rs` does not drift into a god-state.
- [ ] **Change**: add read-only builders for `TaskBoardView`, sub-agent pane
  projection, status mapping, and any coalescing helpers needed by the
  workbench panes.
- [ ] **Input/output contract**: this module consumes typed read handles,
  registry reads, snapshot items, and safe telemetry summaries; it does not
  write to `GoalEngine`, `TaskPlanTool`, or `SubAgentRegistry`.
- [ ] **Failure / rollback behavior**: projection build failures degrade to
  explicit empty/error views instead of mutating authority state or inventing
  defaults.
- [ ] **Minimum validation**: add focused unit tests here for goal/task mapping,
  same-title multi-authority preservation, unknown-status fallback, and
  telemetry merge/coalesce behavior.

Implementation note:

- The task board projection builder mentioned in §9.2 should live here unless a
  narrower dedicated workbench module is introduced.

### 4.3 `src/tui/app.rs`

- [ ] **Goal**: keep the TUI run loop as the single bridge integrator for
  approval prompts, projection refreshes, runtime handles, and operator key
  handling.
- [ ] **Change**: extend `TuiRuntimeContext` to hold a small typed workbench
  handles bundle and any optional projection receivers required by the TUI.
- [ ] **Input/output contract**: prompt intake accepts `NonCliApprovalPrompt`;
  workbench refresh uses strong typed handles rather than reverse-looking up
  state from `Box<dyn Tool>`.
- [ ] **Failure / rollback behavior**: any approval bridge error must resolve as
  deny/fail-closed; missing workbench handles degrade to chat-first rendering,
  not to unsafe inferred state.
- [ ] **Minimum validation**: add focused unit/integration coverage for prompt
  intake, queue progression, explicit approve/deny, requester mismatch,
  timeout/expired handling, modal key swallowing, and workbench-handle absence.

Suggested sub-steps:

- [ ] Centralize TUI non-CLI approval identity constants so `sender`,
  `channel_name`, and `reply_target` cannot drift.
- [ ] Extend `TuiRuntimeContext` with a small typed bundle such as
  `WorkbenchReadHandles`, for example:
  `GoalEngine`, `Arc<TaskPlanTool>`, `Arc<SubAgentRegistry>`, and optional
  workbench telemetry plumbing.
- [ ] Update `handle_approval_prompt(...)` to enqueue rather than overwrite.
- [ ] Replace the current `resolve_approval(...)` helper with a
  runtime-authoritative flow that first calls
  `confirm_non_cli_pending_request(...)` or
  `reject_non_cli_pending_request(...)`, then records the matching
  `ApprovalResponse`.
- [ ] Surface bridge failures as explicit TUI status or system messages instead
  of silently dropping the request.
- [ ] Clear only the active approval item on task completion; do not erase
  unrelated queued approvals.
- [ ] Refresh task board and sub-agent pane projections from the typed handles
  bundle rather than from widget-local state.

Preferred bootstrap wiring order:

- [ ] Either create optional sub-agent telemetry plumbing before calling
  `bootstrap_runtime(...)` and pass it in, or have `bootstrap_runtime(...)`
  create and return the full plumbing as part of `TuiRuntimeContext`.
- [ ] Do not leave this ordering implicit; the checklist implementation must
  choose one path and document it in code comments close to bootstrap wiring.

### 4.4 `src/tui/events.rs`

- [ ] **Goal**: keep event vocabulary narrow and typed.
- [ ] **Change**: extend `TuiEvent` only if implementation proves a new typed
  event is needed for projection refresh, pane focus, or queue navigation.
- [ ] **Input/output contract**: sentinel translation order must remain
  unchanged for draft stream events.
- [ ] **Failure / rollback behavior**: unknown or malformed payloads continue to
  degrade to safe no-op or plain delta behavior rather than misrouting
  sentinels.
- [ ] **Minimum validation**: add translation tests only for newly introduced
  variants; do not regress existing sentinel coverage.

Implementation note:

- Prefer keeping approval queue navigation in local key handling first.
- Introduce new `TuiEvent` variants only when the TUI loop becomes harder to
  reason about without them.

### 4.5 `src/tui/widgets/approval.rs`

- [ ] **Goal**: render the approval queue as a safe operator decision surface,
  not as a raw payload dump.
- [ ] **Change**: evolve the modal from single-request rendering to active-item
  rendering with queue position, explicit fail-closed copy, and sanitized
  argument preview.
- [ ] **Input/output contract**: widget consumes only render-safe approval item
  fields and sanitizes every rendered string.
- [ ] **Failure / rollback behavior**: if preview generation fails upstream,
  render a bounded fallback summary rather than panic or expose raw JSON.
- [ ] **Minimum validation**: add render tests for pending item, multi-item
  queue indicator, long/truncated preview, and failed/expired item copy.

### 4.6 `src/tui/widgets/mod.rs`

- [ ] **Goal**: keep layout composition explicit and auditable.
- [ ] **Change**: add new workbench widgets in small steps, preserving approval
  modal precedence and existing chat usability.
- [ ] **Input/output contract**: approval overlay remains top priority;
  read-only workbench panes render only when their projections are present.
- [ ] **Failure / rollback behavior**: if a new pane projection is absent or
  invalid, the layout falls back to existing chat-first rendering rather than
  crashing.
- [ ] **Minimum validation**: update render tests to cover layout with and
  without task board / sub-agent panes.

### 4.7 `src/tui/widgets/task_board.rs` (new)

- [ ] **Goal**: render a read-only grouped task view for durable goals and
  session tasks.
- [ ] **Change**: add a dedicated widget rather than embedding task board
  rendering into `chat.rs` or `status.rs`.
- [ ] **Input/output contract**: widget accepts `TaskBoardView` only; it must
  not call `GoalEngine` or `TaskPlanTool` directly.
- [ ] **Failure / rollback behavior**: empty goals/session data render as
  empty-state copy, not as fake pending tasks.
- [ ] **Minimum validation**: add render tests for goals-only, session-only,
  mixed same-title items, and degraded error-banner states.

### 4.8 `src/tui/widgets/subagents.rs` (new)

- [ ] **Goal**: render a low-noise sub-agent execution pane from registry plus
  telemetry projection.
- [ ] **Change**: add a dedicated widget with explicit status rendering and
  bounded recent-event summaries.
- [ ] **Input/output contract**: widget consumes `SubAgentProjectionItem` only;
  it must not infer lifecycle completion from a single telemetry event.
- [ ] **Failure / rollback behavior**: telemetry gaps degrade to stale-but-safe
  UI; registry terminal state remains authoritative.
- [ ] **Minimum validation**: add render tests for running, completed, failed,
  and telemetry-detached states.

---

## 5. `src/agent/` Checklist

This section defines the runtime touch points that TUI depends on. The goal is
to reuse existing authority APIs before adding new ones.

### 5.1 `src/agent/loop_.rs`

- [ ] **Goal**: keep the tool loop approval contract stable while making TUI
  integration explicit and testable.
- [ ] **Change**: keep `NonCliApprovalPrompt` minimal unless implementation
  proves another field is strictly required for projection safety.
- [ ] **Input/output contract**: runtime continues to own pending request
  creation, matching, timeout, and final allow/deny semantics.
- [ ] **Failure / rollback behavior**: any missing or mismatched confirmation
  path still ends in deny/fail-closed behavior.
- [ ] **Minimum validation**: extend focused tests around
  `run_tool_call_loop_with_non_cli_approval_context(...)` to cover TUI-like
  approve, deny, mismatch, and timeout flows.

Suggested sub-steps:

- [ ] Confirm the current `NonCliApprovalPrompt` shape is sufficient for queue
  projection.
- [ ] If a timestamp or preview hint is needed, keep it internal to the
  runtime/TUI bridge and avoid creating a new public config surface.
- [ ] Preserve the current sentinel/event behavior unrelated to approvals.

Implementation restraint:

- Do not split out a new TUI protocol crate.
- Do not move approval authority into `src/tui/`.

---

## 6. `src/tools/` Checklist

This section covers session task projection and sub-agent observability.

### 6.1 `src/tools/mod.rs`

- [ ] **Goal**: keep tool wiring explicit, small, and TUI-agnostic.
- [ ] **Change**: extend tool bootstrap to return not only tool objects but also
  a small typed handles bundle for same-session read paths needed by TUI.
- [ ] **Input/output contract**: any new bundle should expose only the typed
  handles needed for TUI read/projection flows, for example `Arc<TaskPlanTool>`
  and `Arc<SubAgentRegistry>`; `src/tools/**` must not import `src/tui/**`.
- [ ] **Failure / rollback behavior**: default tool setup must still work when
  workbench hooks are absent; TUI should degrade to chat-only or reduced
  visibility rather than force a tool bootstrap failure.
- [ ] **Minimum validation**: update constructor/factory tests only where
  signatures actually change.

Suggested sub-steps:

- [ ] Add a typed bundle returned alongside the tool registry from
  `tools::all_tools_with_runtime(...)` or an equivalent helper.
- [ ] Thread that bundle through `bootstrap_runtime(...)` into
  `TuiRuntimeContext`.
- [ ] Keep any sub-agent telemetry hook expressed in observability terms,
  such as an observer factory, rather than a TUI sender type.

### 6.2 `src/tools/task_plan.rs`

- [ ] **Goal**: expose a read-only session task snapshot without weakening the
  existing session-scoped authority model.
- [ ] **Change**: add a small snapshot API and projection-friendly public
  snapshot item type.
- [ ] **Input/output contract**: snapshot returns immutable, read-only task
  IDs/titles/statuses and does not expose interior mutability handles.
- [ ] **Failure / rollback behavior**: unavailable snapshot support degrades to
  “session plan unavailable”; it must not mutate task state or fabricate
  defaults.
- [ ] **Minimum validation**: add unit tests for empty snapshot, ordered
  snapshot, and snapshot stability under create/add/update/delete operations.

Suggested sub-steps:

- [ ] Introduce a projection-safe `TaskPlanSnapshotItem`.
- [ ] Add `TaskPlanTool::snapshot(&self) -> Vec<TaskPlanSnapshotItem>` or an
  equivalent read-only adapter.
- [ ] Map internal statuses into the task board vocabulary explicitly rather
  than by stringly typed reuse.

### 6.3 `src/tools/subagent_spawn.rs`

- [ ] **Goal**: replace the local noop observer path in the agentic branch with
  a forwarding-observer path that remains non-blocking and optional.
- [ ] **Change**: wire a safe observability hook into agentic sub-agent
  execution while keeping `SubAgentRegistry` as lifecycle authority.
- [ ] **Input/output contract**: forwarded telemetry must exclude raw
  prompt/model output and sensitive tool payloads; event delivery must be
  bounded or lossy rather than blocking.
- [ ] **Failure / rollback behavior**: any observer/channel failure degrades to
  shared `crate::observability::NoopObserver`-equivalent visibility loss, not
  to execution failure or false completion.
- [ ] **Minimum validation**: add unit/integration coverage for forwarded tool
  start/response events, dropped receiver behavior, and registry terminal-state
  precedence.

Suggested sub-steps:

- [ ] Reuse shared `crate::observability::NoopObserver` as the fallback path;
  do not expand the current local noop duplication.
- [ ] Accept an optional observer factory or equivalent observability hook from
  bootstrap/tool wiring rather than a raw TUI sender type.
- [ ] Ensure the observer implementation never blocks the sub-agent hot path.

### 6.4 `src/tools/subagent_registry.rs`

- [ ] **Goal**: remain the single lifecycle authority for sub-agent sessions.
- [ ] **Change**: add projection-friendly read helpers only if current query
  methods are insufficient.
- [ ] **Input/output contract**: registry remains authoritative for
  running/completed/failed/killed state and final result summary.
- [ ] **Failure / rollback behavior**: missing telemetry must not change
  registry state semantics.
- [ ] **Minimum validation**: add tests only if new read helpers are
  introduced.

---

## 7. Supporting Files Outside The Primary Roots

These are not the main coding target, but they may need minimal, tightly scoped
changes.

### 7.1 `src/goals/engine.rs`

- [ ] Prefer consuming the existing `GoalEngine::load_state()` read API as-is.
- [ ] Only add helpers here if task board projection becomes materially clearer
  and the helper remains read-only.

### 7.2 `src/approval/mod.rs`

- [ ] Reuse existing `confirm_non_cli_pending_request(...)`,
  `reject_non_cli_pending_request(...)`, and
  `record_non_cli_pending_resolution(...)` before considering new approval APIs.
- [ ] Add new APIs here only if current authority methods cannot express the
  TUI queue contract cleanly.

### 7.3 `src/observability/traits.rs`

- [ ] Reuse the existing `Observer` trait.
- [ ] Do not introduce a parallel TUI-specific observability trait unless a
  concrete limitation appears during implementation.

### 7.4 `src/observability/noop.rs`

- [ ] Reuse the shared noop observer as the default fallback for degraded
  visibility paths.
- [ ] Do not keep adding local noop implementations in tool modules.

### 7.5 `src/tools/traits.rs`

- [ ] Do not extend the `Tool` trait with a downcast-only hook just to support
  TUI workbench reads.
- [ ] Prefer explicit typed-handle threading over reverse lookup from
  `Box<dyn Tool>`.

---

## 8. Authority Map

The checklist is only valid if authority ownership stays explicit:

- **Approval authority**: `ApprovalManager`
- **Durable task authority**: `GoalEngine`
- **Session task authority**: `TaskPlanTool`
- **Sub-agent lifecycle authority**: `SubAgentRegistry`
- **TUI render authority**: `TuiState` and widgets, but only for render-state
  and projection snapshots
- **Projection builder host**: `src/tui/projections.rs` or an equivalent
  dedicated workbench projection module

TUI may read from these authorities and render projections, but it must not
become a write authority for them.

---

## 9. Focused Validation Matrix

Run the smallest relevant validation first, then the existing TUI suite.

### 9.1 Approval Bridge

- [ ] `cargo test --lib tui`
- [ ] `CARGO_BUILD_JOBS=2 cargo test --features tui-ratatui --test tui_event_handling_test --test tui_render_test`
- [ ] Focused coverage for TUI approve / deny / mismatch / timeout behavior

### 9.2 Task Board Projection

- [ ] Focused unit tests in `src/tools/task_plan.rs` for snapshot semantics
- [ ] Focused unit tests in `src/tui/projections.rs` for task board builder
  semantics
- [ ] `CARGO_BUILD_JOBS=2 cargo test --features tui-ratatui --test tui_render_test`

### 9.3 Sub-agent Observability

- [ ] Focused tests in `src/tools/subagent_spawn.rs` and/or related integration
  coverage
- [ ] Focused unit tests in `src/tui/projections.rs` for telemetry merge/coalesce
- [ ] `cargo test --lib tui`
- [ ] `CARGO_BUILD_JOBS=2 cargo test --features tui-ratatui --test tui_event_handling_test --test tui_render_test`

### 9.4 Docs / hygiene

Preferred repository scripts:

- [ ] `DOCS_FILES=docs/plans/2026-03-06-tui-workbench-implementation-checklist.md bash scripts/ci/docs_quality_gate.sh`
- [ ] `BASE_SHA=<valid-base-commit> DOCS_FILES=docs/plans/2026-03-06-tui-workbench-implementation-checklist.md bash scripts/ci/docs_links_gate.sh`

Allowed local fallback when `lychee` is unavailable:

- [ ] `git diff --check`
- [ ] `markdownlint-cli2 docs/plans/2026-03-06-tui-workbench-implementation-checklist.md`
- [ ] Relative-link existence check for newly added local links referenced by
  the changed document

---

## 10. Definition Of Done For Coding Start

This checklist is “ready for implementation” only when all of the following are
true:

- Each new pane or bridge has a named authority owner from §8.
- Every file-level change has an explicit read-only vs mutating boundary.
- Approval flow is fail-closed by construction.
- Task board stays projection-only.
- Same-session typed handles are available to TUI without `Tool` downcasting.
- Sub-agent telemetry is explicitly non-authoritative and TUI-agnostic at the
  tool layer.
- Validation is specified before the first code patch lands.

---

## 11. Rollback Rules

If a track becomes noisy or risky, roll back in this order:

1. Remove the new widget/pane rendering.
2. Remove the projection layer changes.
3. Remove runtime/tool wiring changes.

Do **not** roll back by weakening approval safety or by merging authority into
TUI state.

---

## 12. Non-goals For The First Coding Pass

The first coding pass should still avoid the following:

- Web UI or remote client protocol work
- New public config keys for TUI workbench behavior
- Drag-and-drop or mutating task board UX
- Rich diff/spec rendering beyond bounded text summaries
- Reworking daemon lifecycle or background job architecture
- Large layout overhauls before the three bridge tracks are proven
