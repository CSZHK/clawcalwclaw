# TUI Agent Workbench Proposal (2026-03-06)

Status: Proposal draft  
Type: time-bound implementation proposal  
Scope: terminal-native TUI workbench evolution  

---

## 1. Purpose

This document is a new implementation-oriented proposal for the TUI workbench track.

It is intentionally **parallel to**, not a replacement for, the broader TUI workbench vision draft. The goal here is to turn the high-level direction into a smaller set of repository-aligned decisions, explicit constraints, phased delivery boundaries, and validation gates.

This proposal is also intended to complement the existing TUI implementation plan in `docs/plan/rust-tui-plan-high-level.md` and the runtime lifecycle/state-machine RFIs in `docs/project/f1-3-agent-lifecycle-state-machine-rfi-2026-03-01.md` and `docs/project/q0-3-stop-reason-state-machine-rfi-2026-03-01.md`.

---

## 2. Goal

Evolve the current TUI from a chat-first terminal UI into a **terminal-native workbench** that improves visibility, task coordination, approval clarity, and operator control **without** introducing a protocol split, web surface, or speculative platform abstraction.

### Success definition

- Preserve the current terminal-native architecture and lightweight dependency posture.
- Add visibility before adding control.
- Keep source-of-truth boundaries explicit.
- Keep each phase reversible and independently reviewable.
- Avoid introducing new public runtime/config contracts in Phase 1 unless implementation proves they are necessary.

---

## 3. Non-goals

The following are explicitly out of scope for this proposal:

1. A Web UI, dashboard, or browser-based operator surface.
2. A new protocol crate or client/server split for TUI communication.
3. Replacing `GoalEngine` or `TaskPlanTool` with a new persistence authority.
4. Reworking daemon lifecycle semantics inside the TUI proposal itself.
5. Bulk navigation/i18n changes to docs entry points in the same change.

---

## 4. Current Repository Facts

The proposal is anchored to current code, not to an assumed future architecture.

### 4.1 Existing TUI event path

The current TUI consumes a bounded `mpsc` delta stream and translates raw delta strings into `TuiEvent` variants.

Relevant implementation:

- `src/tui/app.rs`
- `src/tui/events.rs`
- `src/tui/state.rs`

Current facts:

- The live event input is `String` delta payloads, not a separate protocol layer.
- `TuiEvent` is a UI-facing event enum.
- `TuiState` is a small UI view-model and intentionally avoids importing agent internals.

### 4.2 Existing TUI state boundary

`TuiState` is currently a compact chat-focused state object with:

- messages
- input buffer/history
- progress line/block
- provider/model labels
- streaming cursor state

This is a good baseline because it keeps UI state small, explicit, and auditable.

### 4.3 Existing approval capability

The agent loop already has non-CLI approval machinery, including `NonCliApprovalContext` and `ApprovalManager` integration.

However, the current TUI path still invokes `run_tool_call_loop_with_non_cli_approval_context(..., None, ...)`, which means the TUI does not yet expose the richer approval flow as a first-class workbench surface.

### 4.4 Existing sub-agent capability

The repository already contains:

- `src/tools/subagent_spawn.rs`
- `src/tools/subagent_manage.rs`
- `src/tools/subagent_list.rs`
- `src/tools/delegate.rs`
- `src/agent/team_orchestration.rs`
- `src/coordination/mod.rs`
- `src/tools/agents_ipc.rs`

But the current spawned sub-agent path still uses a `NoopObserver`, so the TUI cannot yet treat sub-agent execution as a rich live feed. Today it is closer to a background-result surface than to a full process-observability surface.

### 4.5 Existing authoritative task sources

The repository already has two distinct task/goal sources with different persistence models:

- `src/goals/engine.rs`
  - persistent
  - workspace state under `state/goals.json`
- `src/tools/task_plan.rs`
  - session-scoped
  - intentionally in-memory only

This distinction is important and should not be blurred by the workbench UI.

### 4.6 Existing management surfaces

The repository already has usable backend capability for:

- MCP connectivity: `src/tools/mcp_client.rs`
- background jobs: `src/tools/bg_run.rs`
- git operations: `src/tools/git_operations.rs`
- load-aware multi-agent evaluation: `src/agent/team_orchestration.rs`

This makes a **read-only first** workbench phase realistic.

---

## 5. Core Decisions

This section records the minimal decision set needed to make the workbench track implementable.

### Decision 1 — Keep a terminal-native monolith

**Decision:** continue with the in-process terminal-native architecture rather than introducing a protocol/client split.

**Options considered:**

| Option | Summary | Result |
|---|---|---|
| A | Separate protocol crate + TUI client/server split | Rejected |
| B | Terminal-native monolith with event-bridge extension | Chosen |

**Why B:**

- Matches current `src/tui/` structure and YAGNI posture.
- Preserves startup simplicity and binary-size discipline.
- Keeps the blast radius smaller for early phases.

**Why not A now:**

- No current multi-client requirement.
- Would introduce substantial structural overhead before operator value is proven.
- Would change the failure/debug surface for a still-young TUI subsystem.

### Decision 2 — Introduce a projection layer, not a god-state

**Decision:** do not evolve `TuiState` directly into one massive `WorkbenchState`; instead, introduce a derived workbench projection composed of smaller domain slices.

**Options considered:**

| Option | Summary | Result |
|---|---|---|
| A | One large `WorkbenchState` storing every concern | Rejected |
| B | Domain projections (`tasks`, `approvals`, `agents`, `workspace`, `metrics`) | Chosen |

**Why B:**

- Better matches SRP and current TUI boundaries.
- Makes per-phase rollout easier.
- Keeps rendering and state mutation localized.

**Proposed shape:**

```rust
pub struct WorkbenchProjection {
    pub chat: ChatPaneState,
    pub tasks: TaskPaneState,
    pub approvals: ApprovalPaneState,
    pub agents: AgentPaneState,
    pub workspace: WorkspacePaneState,
    pub metrics: MetricsPaneState,
    pub focus: WorkbenchFocus,
}
```

### Decision 3 — Keep authority and projection separate

**Decision:** introduce a `SpecView`/`TaskBoardView` concept as a **derived TUI projection**, not as a new authority layer.

**Options considered:**

| Option | Summary | Result |
|---|---|---|
| A | Make `Spec` the single source of truth for progress | Rejected |
| B | Keep `GoalEngine` and `TaskPlanTool` authoritative, derive `SpecView` in TUI | Chosen |

**Why B:**

- It matches current repository semantics.
- It avoids inventing a third authority with unclear persistence rules.
- It keeps rollback simple: revert the projection, not task persistence.

**Authority contract:**

| Concern | Authority | Persistence |
|---|---|---|
| durable goals | `GoalEngine` | workspace disk |
| session plan | `TaskPlanTool` | in-memory |
| workbench task board | `SpecView` / `TaskBoardView` | derived projection |

### Decision 4 — Separate view mode from runtime mode

**Decision:** do not use one state machine for both layout/view mode and runtime lifecycle.

**Options considered:**

| Option | Summary | Result |
|---|---|---|
| A | One global state machine for Chat/Spec/MultiAgent/Daemon | Rejected |
| B | `ViewMode` separate from runtime lifecycle | Chosen |

**Why B:**

- `Chat`, `Task`, and `Agents` are UI/view concerns.
- daemon lifecycle belongs to runtime/health supervision and already has a separate RFI track.
- Mixing them would create confusing transitions and ambiguous recovery behavior.

**Proposed split:**

```rust
pub enum ViewMode {
    Chat,
    Task,
    Agents,
    DaemonDashboard,
}
```

Runtime lifecycle remains governed by the separate lifecycle/state-machine work.

### Decision 5 — Read-only first, control surfaces later

**Decision:** Phase 1 and the first part of Phase 2 prioritize visibility and read-only inspection before mutating controls.

**Why:**

- Safer for security-sensitive surfaces.
- Easier to validate.
- Delivers operator value quickly.
- Avoids prematurely binding UI interaction to security/approval policy.

---

## 6. Proposed Workbench Model

### 6.1 Three-layer model

```text
Runtime facts
  ├─ delta stream
  ├─ tool progress
  ├─ approval prompts
  ├─ goals state
  ├─ session task plan
  ├─ bg job store
  ├─ mcp registry
  ├─ git status/diff
  └─ sub-agent registry

        ↓ bridge / projection

Workbench projection
  ├─ chat
  ├─ tasks
  ├─ approvals
  ├─ agents
  ├─ workspace
  └─ metrics

        ↓ render

Widgets / overlays / command palette
```

### 6.2 Event-bridge contract

Not every workbench update should become a new runtime event variant. The TUI should bridge multiple kinds of inputs into a stable projection.

Recommended event classes:

| Class | Source | Typical consumer |
|---|---|---|
| `StreamDelta` | agent delta channel | chat/progress panes |
| `ToolTelemetry` | tool observer/progress bridge | tool timeline |
| `ApprovalPrompt` | approval manager bridge | approval overlay/queue |
| `TaskProjectionRefresh` | goals/task-plan refresh | task board |
| `WorkspaceProjectionRefresh` | git/mcp/bg polling or event bridge | workspace panes |
| `SubagentProjectionRefresh` | registry + observer bridge | agent panes |
| `ViewIntent` | keyboard/command palette | focus/mode changes |

This avoids overloading one enum name with multiple layers of meaning.

### 6.3 View surfaces

The workbench should grow through view surfaces, not through one giant all-visible dashboard.

Initial surfaces:

- Chat pane
- Tool timeline pane
- Approval overlay / queue
- Task board view
- Workspace status view (`git`, `mcp`, `bg`)
- Agent roster view

---

## 7. Phase Plan

### Phase 1 — Visible Agent

**Goal:** make the current TUI clearly show what is already happening.

**Scope:**

- richer tool progress presentation
- token/cost/context presentation
- markdown baseline rendering
- approval overlay with better context
- read-only `/mcp` panel
- read-only `/bg` panel
- read-only `/git` summary

**Not in scope:**

- new worktree creation from TUI
- commit/stash/reset actions
- agent-team topology control
- workflow mutation controls

**Required preconditions:**

- approval prompt bridge from runtime to TUI
- projection-safe coalescing strategy for high-frequency updates

### Phase 2 — Task-aware Workbench

**Goal:** introduce a derived task board that merges durable goals and session plans without creating a new authority.

**Scope:**

- `SpecView` / `TaskBoardView`
- file-change summary
- diff preview widget
- read-only agent roster summary
- command palette organization by view vs control

**Required preconditions:**

- explicit source-of-truth contract in docs and code
- stable task projection refresh model

### Phase 3 — Controlled Orchestration

**Goal:** add selective control surfaces once observability bridges exist.

**Scope:**

- approval queue actions
- agent tabs/roster with live progress
- topology recommendation view
- selected workflow controls
- explicit mutating commands gated by approval/security policy

**Required preconditions:**

- sub-agent observer bridge
- approval queue behavior under concurrency
- per-surface permission model review

### Phase 4 — Optional runtime intelligence

This proposal does not require a Phase 4 commitment. Any future recommendation/automation layer must be justified by usage evidence after the first three phases are proven.

---

## 8. Preconditions and Blockers

The workbench track should not present itself as “just UI work.” Several bridges must exist first.

### Hard preconditions

1. **Approval bridge**
   - TUI must consume non-CLI approval prompts as a first-class interaction.
2. **Sub-agent observability bridge**
   - sub-agent execution must emit usable live signals instead of only final results.
3. **Projection refresh policy**
   - high-frequency updates must be coalesced or sampled to respect the bounded channel.
4. **View/runtime split**
   - view mode must not be used as a substitute for runtime lifecycle state.

### Soft preconditions

1. command palette taxonomy (`view` vs `mutate`)
2. per-pane focus/navigation model
3. small-terminal fallback layout rules

---

## 9. Validation Matrix

Any implementation derived from this proposal should validate at least the following scenarios.

| Area | Validation target |
|---|---|
| layout fallback | narrow terminal (`<100 cols`) remains usable |
| bounded channel safety | high-frequency updates do not freeze or flood the TUI |
| sentinel correctness | progress block/line routing remains correct |
| panic/signal recovery | terminal restores cleanly under panic, SIGTERM, SIGHUP |
| approval concurrency | multiple pending approvals stay attributable and resolvable |
| projection correctness | goal/task board does not mutate authoritative state implicitly |
| sub-agent visibility | live progress appears before final result handoff |
| read-only controls | view commands cannot mutate state by accident |

### Expected local validation shape

- targeted TUI unit tests
- targeted TUI integration/render tests
- focused approval-flow scenario tests
- focused sub-agent observability scenario tests once that bridge exists

---

## 10. Risks and Mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| projection becomes a god-state | medium/high | split by pane/domain, review field growth every phase |
| event naming drifts from implementation reality | medium | document bridge classes separately from runtime enums |
| approval UI outruns approval policy | high | make approval bridge a precondition, not a follow-up |
| sub-agent UI promises exceed backend observability | high | gate Phase 3 on observer bridge completion |
| workbench becomes mutation-heavy too early | high | read-only first, explicit mutating commands later |
| docs drift between plan/vision/proposal | medium | keep this file explicitly labeled as implementation proposal |

---

## 11. Rollback Strategy

This proposal is designed to keep rollback simple:

- Phase 1 should avoid schema/config changes where possible.
- Phase 2 should introduce derived projections, not new durable authorities.
- Phase 3 controls should be isolated behind explicit command/approval boundaries.

If a phase under-delivers or proves unstable, the rollback path is to revert that phase’s view/control layer without invalidating the existing TUI chat baseline.

---

## 12. Recommended Next Step

Before implementation starts, produce one small follow-up design note that freezes three concrete interfaces:

1. approval bridge contract
2. sub-agent observability bridge contract
3. `SpecView` / `TaskBoardView` projection contract

That note should be implementation-facing and narrow enough to map directly to code changes in `src/tui/`, `src/agent/`, and `src/tools/`.

---

## References

- `docs/plan/rust-tui-plan-high-level.md`
- `docs/project/f1-3-agent-lifecycle-state-machine-rfi-2026-03-01.md`
- `docs/project/q0-3-stop-reason-state-machine-rfi-2026-03-01.md`
- `src/tui/app.rs`
- `src/tui/events.rs`
- `src/tui/state.rs`
- `src/tools/subagent_spawn.rs`
- `src/tools/task_plan.rs`
- `src/goals/engine.rs`
- `src/tools/mcp_client.rs`
- `src/tools/bg_run.rs`
- `src/agent/team_orchestration.rs`
