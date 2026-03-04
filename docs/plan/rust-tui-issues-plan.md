# clawclawclaw TUI 模块 — Issue 拆分计划

## Context

clawclawclaw 需要原生 Rust TUI 模块（`clawclawclaw tui` 命令），提供分屏聊天、流式响应、工具执行进度等富终端交互。当前 `src/tui/` 不存在，feature flag 未定义，完全从零实现。

源计划：`docs/plan/rust-tui-plan.md`（v2，已通过多 Agent 审查）

> **审查记录**: Codex CLI (gpt-5.3-codex) 审查 → FAIL，修复 3 ❌ + 7 ⚠️ 后通过。

---

## 依赖关系图

```
Issue #1 (Cargo.toml + deps)
    │
    ├──▶ Issue #2 (terminal.rs)  ──────────────────────┐
    │                                                    │
    ├──▶ Issue #3 (mod.rs + state.rs) ◀─── Widget 起点  │
    │        │                          │                │
    │        ▼                          │                │
    ├──▶ Issue #4 (events.rs)          │                │
    │        │                          │                │
    │        ▼                          ▼                ▼
    └──▶ Issue #5 (app.rs 骨架) ◀─── 依赖 #2 #3 #4
              │                    ▲
              │                    │
              │    Issue #6-#9 (Widgets) ── 阻塞 #3，可四路并行
              │         │
              │         ▼
              │    Issue #10 (Layout 组合) ◀── 依赖 #5 + #6-#9
              │         │
              ▼         ▼
         Issue #11 (Agent 集成 + 完整状态机) ◀── 依赖 #10
              │
              ├──▶ Issue #12 (键盘 + Ctrl+C 双击)
              │
              ├──▶ Issue #13 (CLI 命令注册) ◀── 阻塞 #5（非 #11）
              │
              └──▶ Issue #14 (测试 + 安全验证)
                        │
                        ▼
                   Issue #15 (模块文档)
```

---

## Issue 清单

### Issue #1: Cargo.toml feature flag 与依赖声明

**指派**: Codex CLI agent
**预估**: 0.5h
**阻塞**: 无（首发任务）
**被阻塞**: #2, #3, #4, #5

**任务**:
1. `Cargo.toml` `[features]` 段新增:
   ```toml
   tui-ratatui = ["dep:ratatui", "dep:crossterm", "dep:strip-ansi-escapes"]
   ```
2. `Cargo.toml` `[dependencies]` 段新增:
   ```toml
   ratatui = { version = "0.30", optional = true, default-features = false, features = ["crossterm"] }
   crossterm = { version = "0.28", optional = true, default-features = false, features = ["event-stream"] }
   strip-ansi-escapes = { version = "0.2", optional = true }
   ```
3. `src/lib.rs` 新增条件编译:
   ```rust
   #[cfg(feature = "tui-ratatui")]
   pub mod tui;
   ```
4. 验证 `futures-util` 现有 feature 包含 `StreamExt`（`stream` feature），若不包含需在 `tui-ratatui` feature 中补充

**验收**:
- `cargo check --features tui-ratatui` 编译通过（tui 模块可为空 mod）
- `cargo check` （无 feature）不引入 ratatui/crossterm/strip-ansi-escapes
- `cargo tree -i crossterm --features tui-ratatui` 仅一个版本
- `default = []` 未变
- 二进制大小 gate: `cargo build --release --features tui-ratatui` 后 `ls -la target/release/clawclawclaw` 记录基线值（CI 硬限制 22MB）

**关键文件**:
- `Cargo.toml`: features 段 ~L217, dependencies 段
- `src/lib.rs`: 模块导出 ~L40-80

---

### Issue #2: 终端恢复机制 — terminal.rs

**指派**: Claude Code agent
**预估**: 3h
**阻塞**: #1
**被阻塞**: #5

**任务**:
实现 `src/tui/terminal.rs`，这是 Phase 1 硬性要求（ADR-2）：

1. `install_panic_hook()` — 在 panic=abort 触发前恢复终端状态
   - 通过 `ratatui::crossterm` re-export 访问（不直接 `use crossterm::`）
   - `disable_raw_mode()` + `LeaveAlternateScreen` + `Show cursor`
   - 链式调用 `std::panic::take_hook()` 保留原有 hook

2. `install_signal_handlers(cancel: CancellationToken)` — Unix SIGTERM/SIGHUP
   - `#[cfg(unix)]` 条件编译
   - 恢复终端后 `cancel.cancel()`

3. 初始化顺序文档注释：panic hook → signal handlers → raw mode → alternate screen

**验收**:
- 在 PTY 中触发 panic → 终端 raw mode 已恢复
- 发送 SIGTERM → 终端恢复后进程退出
- `cargo clippy --features tui-ratatui -- -D warnings` 无警告
- 代码中无 `use crossterm::` 直接导入

**关键文件**:
- 新建: `src/tui/terminal.rs`
- 参考: `Cargo.toml:257` (`panic = "abort"`)

---

### Issue #3: 模块骨架 + 状态类型 — mod.rs + state.rs

**指派**: Codex CLI agent
**预估**: 2.5h
**阻塞**: #1
**被阻塞**: #4, #5, #6, #7, #8, #9（Widget 起点）

**任务**:
1. `src/tui/mod.rs`:
   - 声明子模块: `app`, `events`, `state`, `terminal`, `widgets`
   - 导出 `pub async fn run(config: &Config) -> Result<()>` 入口
   - 注意: 顶层配置类型为 `Config`（非 `AppConfig`），见 `src/config/schema.rs`

2. `src/tui/state.rs`:
   - `TuiState` 结构体（自含 view-model，零 `agent/` 导入）
   - 字段: `messages: Vec<TuiChatMessage>`, `input_buffer: String`, `input_history: Vec<String>`, `scroll_offset: usize`, `should_quit: bool`, `mode: InputMode`, `progress_block: Option<String>`
   - `TuiChatMessage` 为 TUI 内部类型（role + content），不复用 agent 的 ChatMessage
   - `InputMode` enum: `Normal`, `Editing`

3. `src/tui/widgets/mod.rs`: 空骨架，声明 `chat`, `input`, `tools`, `status` 子模块

**验收**:
- `cargo check --features tui-ratatui` 编译通过
- `TuiState` 无 `use crate::agent::` 导入
- state.rs 单元测试: 基本状态转换（push message, toggle mode, scroll bounds）

**关键文件**:
- 新建: `src/tui/mod.rs`, `src/tui/state.rs`, `src/tui/widgets/mod.rs`
- 参考: `src/config/schema.rs:190` (Config 类型)

---

### Issue #4: 事件类型 + Sentinel 解析 — events.rs

**指派**: Codex CLI agent
**预估**: 2h
**阻塞**: #1, #3
**被阻塞**: #5

**任务**:
1. `src/tui/events.rs`:
   - `TuiEvent` enum 定义（Delta, Clear, ProgressLine, ProgressBlock, UserMessage, Cancel, Quit, Key, Resize）
   - `translate_delta(delta: String) -> TuiEvent` 函数
   - 使用 `strip_prefix` 解析三个 sentinel（不用 regex）
   - 导入路径: `crate::agent::loop_::DRAFT_*_SENTINEL`

2. Sentinel 匹配说明:
   - 三个 sentinel 的 NUL byte 位置使它们互不为前缀关系：
     - `CLEAR` = `"\x00CLEAR\x00"`
     - `PROGRESS` = `"\x00PROGRESS\x00"`
     - `PROGRESS_BLOCK` = `"\x00PROGRESS_BLOCK\x00"`
   - `"\x00PROGRESS\x00"` 不是 `"\x00PROGRESS_BLOCK\x00"` 的前缀（第 10 字节分别是 `\x00` vs `_`）
   - 因此 `strip_prefix` 天然正确区分，匹配顺序无关紧要
   - 推荐顺序: `==` CLEAR → `strip_prefix` PROGRESS_BLOCK → `strip_prefix` PROGRESS → else Delta

**验收**:
- 单元测试覆盖:
  - CLEAR sentinel → `TuiEvent::Clear`
  - PROGRESS + payload → `ProgressLine { text: payload }`
  - PROGRESS_BLOCK + payload → `ProgressBlock { content: payload }`
  - 普通文本 → `Delta { text }`
  - 空 PROGRESS payload → `ProgressLine { text: "" }`（良性空行）
  - null byte 注入测试（`"\x00FAKE\x00"` → 不匹配任何 sentinel → Delta）
- `cargo test --features tui-ratatui` 通过

**关键文件**:
- 新建: `src/tui/events.rs`
- 参考: `src/agent/loop_.rs:265-277` (sentinel 常量)
- 参考: `src/channels/mod.rs:723-728` (现有解析逻辑)

---

### Issue #5: TuiApp 骨架 + 事件循环 — app.rs

**指派**: Claude Code agent
**预估**: 2h
**阻塞**: #2, #3, #4
**被阻塞**: #10, #13

**任务**:
1. `src/tui/app.rs`:
   - `pub async fn run(config: &Config) -> Result<()>` 完整框架
   - 初始化顺序: `install_panic_hook()` → `install_signal_handlers()` → `enable_raw_mode()` → `EnterAlternateScreen`
   - 事件循环: `tokio::select!` + `EventStream` + `delta_rx`（ADR-3）
   - 退出清理: `LeaveAlternateScreen` + `disable_raw_mode()`
   - 占位 `render()` 函数（画一个空框架即可）
   - `delta_tx`/`delta_rx` 通道创建（buffer=256）

2. 所有 crossterm 访问通过 `ratatui::crossterm` re-export
3. `EventStream` 需要 `futures_util::StreamExt`——验证 crate feature 已启用

**验收**:
- `cargo build --features tui-ratatui` 编译通过
- `cargo run --features tui-ratatui -- tui` 能启动空 TUI 框架，按 q 正常退出
- 终端状态在正常退出和 Ctrl+C 后恢复正常
- 事件循环使用 `EventStream`（非 `poll()`）

**关键文件**:
- 新建: `src/tui/app.rs`
- 依赖: `src/tui/terminal.rs` (#2), `src/tui/state.rs` (#3), `src/tui/events.rs` (#4)

---

### Issue #6: ChatPanel Widget — chat.rs

**指派**: Codex CLI agent
**预估**: 3h
**阻塞**: #3（需要 TuiState + TuiChatMessage 类型）
**可并行**: #7, #8, #9

**任务**:
1. `src/tui/widgets/chat.rs`:
   - 实现 `ratatui::Widget` trait（或 StatefulWidget）
   - 消息历史渲染（role 标签 + 内容）
   - Page Up/Down 滚动支持（通过 `TuiState.scroll_offset`）
   - ANSI 转义序列过滤（渲染前 sanitize，使用 `strip-ansi-escapes` crate）
   - 自动滚动到底部（新消息到达时）

2. `sanitize_display(text: &str) -> String`:
   - 使用 `strip_ansi_escapes::strip_str()` 移除 ANSI/VT 转义序列
   - 过滤 C0 控制字符（保留 \n \t）
   - 过滤 C1 控制字符（\x80-\x9f）

**验收**:
- 消息正确渲染（user/assistant 区分显示）
- 含 `\x1b[2J` 的文本被过滤，不执行清屏
- 含 `\x1b]0;title\x07` 的文本被过滤，不注入标题
- 滚动边界正确（不越界）

**关键文件**:
- 新建: `src/tui/widgets/chat.rs`
- 依赖: `strip-ansi-escapes` (Issue #1 中声明)

---

### Issue #7: InputBox Widget — input.rs

**指派**: Codex CLI agent
**预估**: 3h
**阻塞**: #3
**可并行**: #6, #8, #9

**任务**:
1. `src/tui/widgets/input.rs`:
   - 多行输入框渲染
   - 输入历史导航（上/下键）
   - 64KB 输入硬限制 (`MAX_INPUT_BYTES = 64 * 1024`)
   - `sanitize_input(raw: &str) -> String` — 使用 `strip-ansi-escapes` + C0/C1 手动过滤

2. 光标位置跟踪和渲染

**验收**:
- 输入超过 64KB 时截断（不 panic）
- 粘贴含 ANSI 转义的内容被过滤
- 上/下键正确导航历史
- Enter 提交消息，Shift+Enter 换行（或其他多行方案）

**关键文件**:
- 新建: `src/tui/widgets/input.rs`
- 参考: 源计划安全约束段

---

### Issue #8: ToolOutput Widget — tools.rs

**指派**: Codex CLI agent
**预估**: 2h
**阻塞**: #3
**可并行**: #6, #7, #9

**任务**:
1. `src/tui/widgets/tools.rs`:
   - 由 `TuiEvent::ProgressBlock` 驱动
   - 渲染 ProgressTracker 输出（⏳/✅/❌ 前缀行）
   - 就地更新（替换上一个 block，不追加）

**验收**:
- ProgressBlock 事件正确替换前一个进度块
- 空 ProgressBlock 不导致 UI 故障
- emoji 前缀正确显示（⏳ running, ✅ success, ❌ failed）

**关键文件**:
- 新建: `src/tui/widgets/tools.rs`
- 参考: `src/agent/loop_.rs:490-533` (ProgressTracker)

---

### Issue #9: StatusBar Widget — status.rs

**指派**: Codex CLI agent
**预估**: 1h
**阻塞**: #3
**可并行**: #6, #7, #8

**任务**:
1. `src/tui/widgets/status.rs`:
   - 显示: 模型名称、提供商 ID、当前状态（idle/thinking/tool_running）
   - **不得**显示 API key、token 或任何凭证字段

**验收**:
- 状态栏正确显示模型/提供商信息
- grep 代码确认无 key/token/secret 字段引用
- 不同状态正确切换显示

**关键文件**:
- 新建: `src/tui/widgets/status.rs`

---

### Issue #10: Layout 组合 + Resize 处理

**指派**: Claude Code agent
**预估**: 2h
**阻塞**: #5, #6, #7, #8, #9
**被阻塞**: #11

**任务**:
1. `src/tui/widgets/mod.rs` 完善:
   - 组合四个 widget 到统一 layout
   - 使用 `ratatui::layout::Layout` 分割区域
   - 上部: ChatPanel（占主体）
   - 中部: ToolOutput（可折叠，无进度时隐藏）
   - 下部: InputBox
   - 底部: StatusBar（单行）

2. `render(frame: &mut Frame, state: &TuiState)` 函数

3. Resize 处理: 终端大小变化时重新计算 layout（源计划 Phase 3 "Resize 处理" 任务归属于此）

**验收**:
- 四个 widget 正确布局
- Resize 事件后布局自适应
- ToolOutput 无内容时自动折叠

**关键文件**:
- 修改: `src/tui/widgets/mod.rs`
- 修改: `src/tui/app.rs` (替换占位 render)

---

### Issue #11: Agent 集成 + 完整状态机

**指派**: Claude Code agent
**预估**: 4h
**阻塞**: #10
**被阻塞**: #12, #14

**任务**:
1. `src/tui/app.rs` 完善:
   - 接入 `run_tool_call_loop_with_non_cli_approval_context()`
   - `delta_tx` 传递给 agent loop 的 `on_delta` 参数
   - 用户消息 → 构建 ChatMessage → 调用 agent loop
   - Agent 响应通过 `delta_rx` 流式接收
   - per-message `CancellationToken` 创建和管理

2. 会话历史管理（复用 agent loop 的 history Vec）
3. 系统提示加载（复用现有 `build_system_prompt` 逻辑）

**验收**:
- 发送消息 → 收到流式响应 → ChatPanel 实时更新
- 工具执行 → ToolOutput 面板显示进度
- 对话上下文在多轮中保持

**关键文件**:
- 修改: `src/tui/app.rs`
- 参考: `src/agent/loop_.rs:972-992` (tool loop 签名)
- 参考: `src/channels/mod.rs:3638-3716` (delta channel 模式)

---

### Issue #12: 键盘处理 + Ctrl+C 双击保护

**指派**: Claude Code agent
**预估**: 3h
**阻塞**: #11
**可并行**: #13

**任务**:
1. 完整键盘映射:
   - `Enter`: 提交消息（Editing 模式）
   - `Esc`: Normal → 退出编辑
   - `q` (Normal 模式): 退出 TUI
   - `Ctrl+D`: 退出 TUI
   - `Ctrl+C`: 取消当前 in-flight 请求（TUI 继续运行）
   - 双击 `Ctrl+C`（300ms 内）: 强制退出 + 恢复终端
   - `Page Up/Down`: 滚动聊天历史
   - `Up/Down` (Editing 模式): 输入历史导航
   - `i` (Normal 模式): 进入编辑

2. CancellationToken 集成:
   - `Cancel` → `token.cancel()`（取消当前请求）
   - `Quit` → `token.cancel()` + `should_quit = true`

**验收**:
- 测试场景 #4: Ctrl+C 取消请求，TUI 继续运行
- 测试场景 #14: 300ms 内双击 Ctrl+C → 强制退出，终端状态正常
- 测试场景 #5: q / Ctrl+D 正常退出

**关键文件**:
- 修改: `src/tui/app.rs`

---

### Issue #13: CLI 命令注册

**指派**: Codex CLI agent
**预估**: 2h
**阻塞**: #5（需要 `tui::run()` 存在即可，不需要等 Agent 集成）
**可并行**: #6-#12

**任务**:
1. `src/main.rs`:
   - `Commands` enum 新增 `Tui` variant（`#[cfg(feature = "tui-ratatui")]`）
   - 可选参数: `--provider`, `--model`（复用 Agent 命令的参数）
   - match 分支调用 `tui::run(config)`

2. 无 feature 时的 UX 设计:
   - 方案 A: 始终保留 `Tui` variant 但 match 内 `compile_error!` 或返回友好错误
   - 方案 B: cfg-gate 整个 variant，接受 clap "unknown subcommand" 行为
   - 推荐方案 A——用户体验更好

3. 确保 `cargo check` 和 `cargo check --features tui-ratatui` 均编译通过（cfg-gate 不能破坏无 feature 构建）

**验收**:
- `cargo run --features tui-ratatui -- tui` 启动 TUI
- `cargo run -- tui` (无 feature) 给出 "TUI feature not enabled, rebuild with --features tui-ratatui" 或类似提示
- `cargo run --features tui-ratatui -- --help` 显示 tui 子命令
- `cargo check` (无 feature) 编译通过

**关键文件**:
- 修改: `src/main.rs:147-570` (Commands enum)
- 修改: `src/main.rs` (match handler)

---

### Issue #14: 测试 + 安全验证

**指派**: Claude Code agent
**预估**: 5h
**阻塞**: #12, #13

**任务**:

#### 单元测试
1. `src/tui/state.rs` — TuiState 状态转换测试
2. `src/tui/events.rs` — sentinel 解析全覆盖（含 null byte 注入）

#### 安全测试
3. ANSI 注入（输入侧）: `\x1b[2J` 被 `sanitize_input` 过滤
4. ANSI 注入（渲染侧）: `\x1b]0;malicious\x07` 被 `sanitize_display` 过滤
5. Sentinel 边界: `"\x00PROGRESS\x00"` 无载荷 → `ProgressLine { text: "" }`
6. 64KB 输入限制测试

#### 集成验证
7. 完整 CI 通过:
   ```bash
   cargo fmt --all -- --check
   cargo clippy --all-targets --features tui-ratatui -- -D warnings
   cargo test --features tui-ratatui
   cargo build --features tui-ratatui
   cargo tree -i crossterm --features tui-ratatui  # 单版本
   ```

**验收**:
- 所有测试通过
- `./dev/ci.sh all` 不回归
- 安全测试用例 #9-#14 全部覆盖

---

### Issue #15: 模块文档

**指派**: Codex CLI agent
**预估**: 1h
**阻塞**: #14
**被阻塞**: 无（最终任务）

**任务**:
1. `src/tui/` 各文件顶部模块级文档注释（`//!`）：
   - `mod.rs`: 模块概述、feature flag 要求、启动方式
   - `app.rs`: 事件循环架构、初始化顺序
   - `terminal.rs`: panic hook 设计决策（ADR-2）
   - `events.rs`: sentinel 协议说明
   - `state.rs`: view-model 隔离原则
   - `widgets/*.rs`: 各 widget 用途

2. 源计划 Phase 4 中 "模块文档 1h" 对应此 issue

**验收**:
- `cargo doc --features tui-ratatui --no-deps` 无警告
- 每个 `src/tui/*.rs` 文件都有 `//!` 模块文档

**关键文件**:
- 修改: `src/tui/` 下所有 .rs 文件

---

## 执行时间线

```
Week 1:
  Day 1    ─── Issue #1 (Cargo.toml + deps)     ← Codex CLI
  Day 1-2  ─── Issue #2 (terminal.rs)            ← Claude Code   ┐
  Day 1-2  ─── Issue #3 (mod.rs + state.rs)      ← Codex CLI     ├ 并行
  Day 2    ─── Issue #4 (events.rs)              ← Codex CLI     ┘
  Day 2-3  ─── Issue #13 (CLI 命令注册)           ← Codex CLI   ← 提前启动
  Day 3    ─── Issue #5 (app.rs 骨架)            ← Claude Code

Week 2:
  Day 3-4  ─── Issue #6 (ChatPanel)              ← Codex CLI     ┐
  Day 3-4  ─── Issue #7 (InputBox)               ← Codex CLI     ├ 四路并行
  Day 3    ─── Issue #8 (ToolOutput)             ← Codex CLI     │
  Day 3    ─── Issue #9 (StatusBar)              ← Codex CLI     ┘
  Day 4-5  ─── Issue #10 (Layout + Resize)       ← Claude Code

Week 3:
  Day 5-7  ─── Issue #11 (Agent 集成)            ← Claude Code
  Day 7-8  ─── Issue #12 (键盘 + 双击保护)       ← Claude Code
  Day 8-9  ─── Issue #14 (测试 + 安全)           ← Claude Code
  Day 9    ─── Issue #15 (模块文档)              ← Codex CLI
```

**总计: ~9 工作日**

## Agent 分工原则

| Agent 类型 | 适合任务 | 分配的 Issue |
|-----------|---------|-------------|
| **Claude Code** | 需要交互式探索、复杂集成、安全关键路径 | #2, #5, #10, #11, #12, #14 |
| **Codex CLI** | 定义明确、自包含、可并行的实现任务 | #1, #3, #4, #6, #7, #8, #9, #13, #15 |

## Codex 审查修复记录

| 原始问题 | 严重度 | 修复内容 |
|---------|--------|---------|
| Phase 4 `模块文档` 缺失 | ❌ | 新增 Issue #15 |
| `strip-ansi-escapes` 依赖未声明 | ❌ | Issue #1 补充该 crate |
| Sentinel 匹配顺序描述错误 | ❌ | Issue #4 修正——NUL byte 使三者互不为前缀 |
| #13 过度阻塞于 #11 | ⚠️ | 降低到阻塞 #5 |
| #6-#9 过度阻塞于 #5 | ⚠️ | 降低到阻塞 #3（Widget 只需类型定义） |
| `AppConfig` 类型名有误 | ⚠️ | 全局替换为 `Config` |
| 无 feature 时缺乏友好提示 | ⚠️ | Issue #13 增加 UX 设计方案 |
| `futures_util::StreamExt` feature 未验证 | ⚠️ | Issue #1 验收中增加检查 |
| #3/#4 可改为 Codex CLI | ⚠️ | 已调整指派 |
| 二进制大小缺乏具体 gate | ⚠️ | Issue #1 验收增加 size 记录 |

## 风险项

1. **二进制大小**: ratatui + crossterm + strip-ansi-escapes 增量需在 Issue #1 后实测，CI 有 22MB 硬限制
2. **crossterm 版本分裂**: Issue #1 验收必须确认 `cargo tree -i crossterm` 单版本
3. **Agent 集成复杂度**: Issue #11 是最大风险点，`run_tool_call_loop` 参数多（20 个），需仔细对接
4. **Windows 支持降级**: signal handlers 仅 Unix，Windows 仅 Ctrl+C — 需在 Issue #2 中处理
5. **futures-util feature**: 现有声明可能仅启用 `sink`，需验证 `stream` feature 是否已包含
