# Task Plan

## Goal
Establish a merge-blocking UI acceptance gate for the existing TUI (`src/tui/`) and web (`web/`) surfaces, starting with the smallest deterministic P0 slice.

## Phases
- [x] Inspect current UI test surfaces
- [x] Harden TUI P0 acceptance gate
- [x] Bootstrap Web P0 browser gate
- [x] Add CI artifacts and docs
- [x] Run focused validation checks

## Notes
- Active branch/worktree: vk/9c7a-req-20260308-01
- Worktree path: /var/tmp/vibe-kanban/worktrees/9c7a-req-20260308-01/zeroclaw
- Constraints: deterministic fixtures only; no external providers or unstable network in PR gate.
