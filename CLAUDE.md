# CLAUDE.md — clawclawclaw Agent Engineering Protocol

This file defines the default working protocol for Claude agents in this repository.
Scope: entire repository.

## Quick Navigation

- [Code Conventions & Architecture](src/CLAUDE.md) — engineering principles, naming, boundary contracts, change playbooks, validation
- [Documentation System](docs/CLAUDE.md) — docs IA, i18n contract, runtime-contract references
- [PR / CI / Collaboration](.github/CLAUDE.md) — branch flow, PR discipline, attribution templates

## 1) Project Snapshot (Read First)

clawclawclaw is a Rust-first autonomous agent runtime optimized for:

- high performance
- high efficiency
- high stability
- high extensibility
- high sustainability
- high security

Core architecture is trait-driven and modular. Most extension work should be done by implementing traits and registering in factory modules.

Key extension points:

- `src/providers/traits.rs` (`Provider`)
- `src/channels/traits.rs` (`Channel`)
- `src/tools/traits.rs` (`Tool`)
- `src/memory/traits.rs` (`Memory`)
- `src/observability/traits.rs` (`Observer`)
- `src/runtime/traits.rs` (`RuntimeAdapter`)
- `src/peripherals/traits.rs` (`Peripheral`) — hardware boards (STM32, RPi GPIO)

## Tech Stack

- **Language**: Rust (edition 2021)
- **Async runtime**: Tokio
- **CLI**: clap
- **Config**: TOML via custom schema (`src/config/schema.rs`)
- **Storage**: SQLite (memory), Markdown files
- **Optional**: `tui-ratatui` feature (ratatui 0.30 + crossterm 0.28)
- **CI**: GitHub Actions (see `docs/ci-map.md`)
- **Release profile**: `panic = "abort"`, size-optimized

## Common Commands

```bash
# Format check
cargo fmt --all -- --check

# Lint (warnings as errors)
cargo clippy --all-targets -- -D warnings

# Tests
cargo test

# Full pre-PR validation (recommended)
./dev/ci.sh all

# Build with TUI feature
cargo build --features tui-ratatui
```

## 10) Anti-Patterns (Do Not)

- Do not add heavy dependencies for minor convenience.
- Do not silently weaken security policy or access constraints.
- Do not add speculative config/feature flags "just in case".
- Do not mix massive formatting-only changes with functional changes.
- Do not modify unrelated modules "while here".
- Do not bypass failing checks without explicit explanation.
- Do not hide behavior-changing side effects in refactor commits.
- Do not include personal identity or sensitive information in test data, examples, docs, or commits.
- Do not attempt repository rebranding/identity replacement unless maintainers explicitly requested it in the current scope.
- Do not introduce new platform surfaces (for example `web` apps, dashboards, frontend stacks, or UI portals) unless maintainers explicitly requested them in the current scope.

## 11) Handoff Template (Agent -> Agent / Maintainer)

When handing off work, include:

1. What changed
2. What did not change
3. Validation run and results
4. Remaining risks / unknowns
5. Next recommended action

## 12) Vibe Coding Guardrails

When working in fast iterative mode:

- Keep each iteration reversible (small commits, clear rollback).
- Validate assumptions with code search before implementing.
- Prefer deterministic behavior over clever shortcuts.
- Do not "ship and hope" on security-sensitive paths.
- If uncertain, leave a concrete TODO with verification context, not a hidden guess.
