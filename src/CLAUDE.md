# src/CLAUDE.md — Code Conventions & Architecture

Scope: all code under `src/`. Read in conjunction with root [`CLAUDE.md`](../CLAUDE.md).

---

## 2) Deep Architecture Observations (Why This Protocol Exists)

These codebase realities should drive every design decision:

1. **Trait + factory architecture is the stability backbone**
    - Extension points are intentionally explicit and swappable.
    - Most features should be added via trait implementation + factory registration, not cross-cutting rewrites.
2. **Security-critical surfaces are first-class and internet-adjacent**
    - `src/gateway/`, `src/security/`, `src/tools/`, `src/runtime/` carry high blast radius.
    - Defaults already lean secure-by-default (pairing, bind safety, limits, secret handling); keep it that way.
3. **Performance and binary size are product goals, not nice-to-have**
    - `Cargo.toml` release profile and dependency choices optimize for size and determinism.
    - Convenience dependencies and broad abstractions can silently regress these goals.
4. **Config and runtime contracts are user-facing API**
    - `src/config/schema.rs` and CLI commands are effectively public interfaces.
    - Backward compatibility and explicit migration matter.
5. **The project now runs in high-concurrency collaboration mode**
    - CI + docs governance + label routing are part of the product delivery system.
    - PR throughput is a design constraint; not just a maintainer inconvenience.

---

## 3) Engineering Principles (Normative)

These principles are mandatory by default. They are not slogans; they are implementation constraints.

### 3.1 KISS (Keep It Simple, Stupid)

**Why here:** Runtime + security behavior must stay auditable under pressure.

Required:

- Prefer straightforward control flow over clever meta-programming.
- Prefer explicit match branches and typed structs over hidden dynamic behavior.
- Keep error paths obvious and localized.

### 3.2 YAGNI (You Aren't Gonna Need It)

**Why here:** Premature features increase attack surface and maintenance burden.

Required:

- Do not add new config keys, trait methods, feature flags, or workflow branches without a concrete accepted use case.
- Do not introduce speculative "future-proof" abstractions without at least one current caller.
- Keep unsupported paths explicit (error out) rather than adding partial fake support.

### 3.3 DRY + Rule of Three

**Why here:** Naive DRY can create brittle shared abstractions across providers/channels/tools.

Required:

- Duplicate small, local logic when it preserves clarity.
- Extract shared utilities only after repeated, stable patterns (rule-of-three).
- When extracting, preserve module boundaries and avoid hidden coupling.

### 3.4 SRP + ISP (Single Responsibility + Interface Segregation)

**Why here:** Trait-driven architecture already encodes subsystem boundaries.

Required:

- Keep each module focused on one concern.
- Extend behavior by implementing existing narrow traits whenever possible.
- Avoid fat interfaces and "god modules" that mix policy + transport + storage.

### 3.5 Fail Fast + Explicit Errors

**Why here:** Silent fallback in agent runtimes can create unsafe or costly behavior.

Required:

- Prefer explicit `bail!`/errors for unsupported or unsafe states.
- Never silently broaden permissions/capabilities.
- Document fallback behavior when fallback is intentional and safe.

### 3.6 Secure by Default + Least Privilege

**Why here:** Gateway/tools/runtime can execute actions with real-world side effects.

Required:

- Deny-by-default for access and exposure boundaries.
- Never log secrets, raw tokens, or sensitive payloads.
- Keep network/filesystem/shell scope as narrow as possible unless explicitly justified.

### 3.7 Determinism + Reproducibility

**Why here:** Reliable CI and low-latency triage depend on deterministic behavior.

Required:

- Prefer reproducible commands and locked dependency behavior in CI-sensitive paths.
- Keep tests deterministic (no flaky timing/network dependence without guardrails).
- Ensure local validation commands map to CI expectations.

### 3.8 Reversibility + Rollback-First Thinking

**Why here:** Fast recovery is mandatory under high PR volume.

Required:

- Keep changes easy to revert (small scope, clear blast radius).
- For risky changes, define rollback path before merge.
- Avoid mixed mega-patches that block safe rollback.

---

## 4) Repository Map (High-Level)

- `src/main.rs` — CLI entrypoint and command routing
- `src/lib.rs` — module exports and shared command enums
- `src/config/` — schema + config loading/merging
- `src/agent/` — orchestration loop
- `src/gateway/` — webhook/gateway server
- `src/security/` — policy, pairing, secret store
- `src/memory/` — markdown/sqlite memory backends + embeddings/vector merge
- `src/providers/` — model providers and resilient wrapper
- `src/channels/` — Telegram/Discord/Slack/etc channels
- `src/tools/` — tool execution surface (shell, file, memory, browser)
- `src/peripherals/` — hardware peripherals (STM32, RPi GPIO); see `docs/hardware-peripherals-design.md`
- `src/runtime/` — runtime adapters (currently native)

---

## 5) Risk Tiers by Path (Review Depth Contract)

Use these tiers when deciding validation depth and review rigor.

- **Low risk**: docs/chore/tests-only changes
- **Medium risk**: most `src/**` behavior changes without boundary/security impact
- **Medium-High risk**: `src/tui/terminal.rs` (terminal lifecycle, panic hooks, signal handlers), `src/tui/events.rs` (sentinel protocol — silent misroute on error)
- **High risk**: `src/security/**`, `src/runtime/**`, `src/gateway/**`, `src/tools/**`, `.github/workflows/**`, access-control boundaries

When uncertain, classify as higher risk.

---

## 6) Agent Workflow (Required)

1. **Read before write**
    - Inspect existing module, factory wiring, and adjacent tests before editing.
2. **Define scope boundary**
    - One concern per PR; avoid mixed feature+refactor+infra patches.
3. **Implement minimal patch**
    - Apply KISS/YAGNI/DRY rule-of-three explicitly.
4. **Validate by risk tier**
    - Docs-only: lightweight checks.
    - Code/risky changes: full relevant checks and focused scenarios.
5. **Document impact**
    - Update docs/PR notes for behavior, risk, side effects, and rollback.
    - If CLI/config/provider/channel behavior changed, update corresponding runtime-contract references.
    - If docs entry points changed, keep all supported locale README/docs-hub navigation aligned (`en`, `zh-CN`, `ja`, `ru`, `fr`, `vi`, `el`).
    - Run through `docs/i18n-guide.md` and record any explicit i18n deferrals in the PR summary.
6. **Respect queue hygiene**
    - If stacked PR: declare `Depends on #...`.
    - If replacing old PR: declare `Supersedes #...`.

### 6.2 Worktree Workflow (Required for All Task Streams)

Use Git worktrees to isolate every active task stream safely and predictably:

- Use one dedicated worktree per active branch/PR stream; do not implement directly in a shared default workspace.
- Keep each worktree on a single branch and a single concern; do not mix unrelated edits in one worktree.
- Before each commit/push, verify commit hygiene in that worktree (`git status --short` and `git diff --cached`) so only scoped files are included.
- Run validation commands inside the corresponding worktree before commit/PR.
- Name worktrees clearly by scope (for example: `wt/ci-hardening`, `wt/provider-fix`).
- After PR merge/close (or task abandonment), remove stale worktrees/branches and prune refs (`git worktree prune`, `git fetch --prune`).
- Local Codex automation may use one-command cleanup helper: `~/.codex/skills/clawclawclaw-pr-issue-automation/scripts/cleanup_track.sh --repo-dir <repo_dir> --worktree <worktree_path> --branch <branch_name>`.
- PR checkpoint rules from section 6.1 still apply to worktree-based development.

### 6.3 Code Naming Contract (Required)

Apply these naming rules for all code changes unless a subsystem has a stronger existing pattern.

- Use Rust standard casing consistently: modules/files `snake_case`, types/traits/enums `PascalCase`, functions/variables `snake_case`, constants/statics `SCREAMING_SNAKE_CASE`.
- Name types and modules by domain role, not implementation detail (for example `DiscordChannel`, `SecurityPolicy`, `MemoryStore` over vague names like `Manager`/`Helper`).
- Keep trait implementer naming explicit and predictable: `<ProviderName>Provider`, `<ChannelName>Channel`, `<ToolName>Tool`, `<BackendName>Memory`.
- Keep factory registration keys stable, lowercase, and user-facing (for example `"openai"`, `"discord"`, `"shell"`), and avoid alias sprawl without migration need.
- Name tests by behavior/outcome (`<subject>_<expected_behavior>`) and keep fixture identifiers neutral/project-scoped.
- If identity-like naming is required in tests/examples, use clawclawclaw-native labels only (`ZeroClawAgent`, `clawclawclaw_user`, `clawclawclaw_node`).

### 6.4 Architecture Boundary Contract (Required)

Use these rules to keep the trait/factory architecture stable under growth.

- Extend capabilities by adding trait implementations + factory wiring first; avoid cross-module rewrites for isolated features.
- Keep dependency direction inward to contracts: concrete integrations depend on trait/config/util layers, not on other concrete integrations.
- Avoid creating cross-subsystem coupling (for example provider code importing channel internals, tool code mutating gateway policy directly).
- Keep module responsibilities single-purpose: orchestration in `agent/`, transport in `channels/`, model I/O in `providers/`, policy in `security/`, execution in `tools/`.
- Introduce new shared abstractions only after repeated use (rule-of-three), with at least one real caller in current scope.
- For config/schema changes, treat keys as public contract: document defaults, compatibility impact, and migration/rollback path.

---

## 7) Change Playbooks

### 7.1 Adding a Provider

- Implement `Provider` in `src/providers/`.
- Register in `src/providers/mod.rs` factory.
- Add focused tests for factory wiring and error paths.
- Avoid provider-specific behavior leaks into shared orchestration code.

### 7.2 Adding a Channel

- Implement `Channel` in `src/channels/`.
- Keep `send`, `listen`, `health_check`, typing semantics consistent.
- Cover auth/allowlist/health behavior with tests.

### 7.3 Adding a Tool

- Implement `Tool` in `src/tools/` with strict parameter schema.
- Validate and sanitize all inputs.
- Return structured `ToolResult`; avoid panics in runtime path.

### 7.4 Adding a Peripheral

- Implement `Peripheral` in `src/peripherals/`.
- Peripherals expose `tools()` — each tool delegates to the hardware (GPIO, sensors, etc.).
- Register board type in config schema if needed.
- See `docs/hardware-peripherals-design.md` for protocol and firmware notes.

### 7.7 Adding a TUI Widget

See [`src/tui/CLAUDE.md`](tui/CLAUDE.md) section 7.7 for the full playbook.

### 7.5 Security / Runtime / Gateway Changes

- Include threat/risk notes and rollback strategy.
- Add/update tests or validation evidence for failure modes and boundaries.
- Keep observability useful but non-sensitive.
- For `.github/workflows/**` changes, include Actions allowlist impact in PR notes and update `docs/actions-source-policy.md` when sources change.

---

## 8) Validation Matrix

Default local checks for code changes:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Preferred local pre-PR validation path (recommended, not required):

```bash
./dev/ci.sh all
```

Notes:

- Local Docker-based CI is strongly recommended when Docker is available.
- Contributors are not blocked from opening a PR if local Docker CI is unavailable; in that case run the most relevant native checks and document what was run.

Additional expectations by change type:

- **Docs/template-only**:
    - run markdown lint and link-integrity checks
    - if touching README/docs-hub/SUMMARY/collection indexes, verify EN/ZH-CN/JA/RU/FR/VI/EL navigation parity
    - if touching bootstrap docs/scripts, run `bash -n bootstrap.sh scripts/bootstrap.sh scripts/install.sh`
- **Workflow changes**: validate YAML syntax; run workflow lint/sanity checks when available.
- **Security/runtime/gateway/tools**: include at least one boundary/failure-mode validation.
- **TUI changes** (`src/tui/**`):
    - run L1 unit tests: `cargo test --lib tui`
    - run L2 render/event tests: `CARGO_BUILD_JOBS=2 cargo test --features tui-ratatui --test tui_render_test --test tui_event_handling_test`
    - see `docs/tui-testing.md` for full test strategy

If full checks are impractical, run the most relevant subset and document what was skipped and why.
