# TUI Testing Guide

This document describes how to test the Terminal User Interface (TUI) module of clawclawclaw.

## Test Architecture

The TUI module uses a three-level testing strategy:

```
┌─────────────────────────────────────────────────────────────────┐
│                     TUI 测试金字塔                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│                     ┌───────┐                                   │
│                     │  E2E  │ ← portable-pty, 真实终端           │
│                     │  3个  │ ← #[ignore], 需显式运行             │
│                     └───┬───┘                                   │
│                         │                                       │
│               ┌─────────┴─────────┐                              │
│               │   集成测试 (L2)    │                              │
│               │  渲染 5 + 事件 8   │ ← TestBackend (无真实终端)    │
│               │  --features tui    │ ← 中等速度                   │
│               └─────────┴─────────┘                              │
│                         │                                       │
│     ┌───────────────────────┴───────────────────────┐     │
│     │            单元测试 (L1)                │     │
│     │   state 6 + events 5 = 11个测试       │     │
│     │   无feature依赖，毫秒级完成          │     │
│     └───────────────────────────────────────┘     │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## Test Files

| Level | File | Tests | Description |
|-------|------|-------|-------------|
| L1 | `src/tui/state.rs` | 6 | State manipulation (pure logic) |
| L1 | `src/tui/events.rs` | 5 | Event translation (pure logic) |
| L2 | `tests/tui_render_test.rs` | 5 | Widget rendering (TestBackend) |
| L2 | `tests/tui_event_handling_test.rs` | 8 | Event handling + state machine |
| L3 | `tests/tui_e2e_pty.rs` | 3 | Real terminal E2E (portable-pty) |

## Running Tests

### Quick Tests (Recommended for Daily Development)

```bash
# L1 单元测试 - 秒级完成，无额外依赖
cargo test --lib tui

# L2 渲染 + 事件测试 - 需要ratatui feature
CARGO_BUILD_JOBS=2 cargo test --features tui-ratatui \
    --test tui_render_test --test tui_event_handling_test
```

### Full Test Suite

```bash
# 运行所有TUI测试 (L1 + L2)
CARGO_BUILD_JOBS=2 cargo test --features tui-ratatui tui
```

### E2E Tests (Manual/CI Only)

E2E tests are marked with `#[ignore]` because they require compiling the full binary and spawning a real terminal process.

```bash
# 显式运行E2E测试
CARGO_BUILD_JOBS=2 cargo test --features tui-ratatui \
    --test tui_e2e_pty -- --test-threads=1 --ignored
```

### Manual Interactive Testing

```bash
# 直接启动TUI进行手动测试
cargo run --bin clawclawclaw --features tui-ratatui

# 快捷键:
# i     → 进入输入模式
# Esc   → 退出输入模式
# Enter → 发送消息
# q     → 退出TUI
# Ctrl+C ×2 → 强制退出
```

## CPU Usage Control

All test commands include `CARGO_BUILD_JOBS=2` to limit compilation parallelism and avoid saturating shared development hosts.

For test execution, use `--test-threads=1` or `--test-threads=2` to control parallelism.

## CI Integration

TUI tests are integrated into the CI pipeline:

- **L1 tests**: Run automatically in `ci-run.yml` (no feature flag needed)
- **L2 tests**: Run in `feature-matrix.yml` with `tui-ratatui` feature
- **L3 tests**: Manual dispatch only (not in automatic CI)

## Adding New TUI Tests

When adding new TUI functionality:

1. **Unit tests**: Add to `src/tui/state.rs` or `src/tui/events.rs` for pure logic
2. **Render tests**: Add to `tests/tui_render_test.rs` for widget rendering
3. **Event tests**: Add to `tests/tui_event_handling_test.rs` for state transitions
4. **E2E tests**: Add to `tests/tui_e2e_pty.rs` only for critical user flows (mark with `#[ignore]`)

## Troubleshooting

### Test Compilation Errors

If tests fail to compile, ensure:
- `--features tui-ratatui` is specified for L2/L3 tests
- `portable-pty` is in `[dev-dependencies]`

### Test Timeouts

E2E tests have built-in timeouts:
- Startup timeout: 30 seconds
- Interaction timeout: 5 seconds

If tests timeout, check:
- Binary compilation is slow (first run)
- Terminal initialization issues
- Provider configuration problems

### Flaky Tests

If E2E tests are flaky:
1. Increase timeouts in test constants
2. Check for race conditions in async code
3. Ensure proper cleanup in test teardown
