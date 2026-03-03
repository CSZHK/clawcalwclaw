# ZeroClaw TUI 模块实现计划（修订版 v2）

> 修订说明：本版本基于多智能体团队审查（arch-reviewer / security-reviewer / impl-reviewer）
> 修复了 4 个 Critical、5 个 High、4 个 Medium 问题。

---

## 上下文

**目标**: 为 ZeroClaw 添加原生 Rust TUI（终端用户界面）模块，提供富交互式界面。

**为什么需要这个**:
- 现有 CLI 模式（rustyline REPL）功能有限，无法显示流式响应和工具执行进度
- 用户需要一个更直观的可视化界面来与 Agent 交互
- 终端界面保持轻量级，无需额外依赖（如 Electron）

**预期结果**: `zeroclaw tui` 命令启动富终端界面，支持：
- 分屏聊天历史（可滚动）
- 多行输入（带历史导航）
- 实时工具执行进度
- 状态栏显示模型/提供商信息

---

## 架构决策记录

### ADR-1: 不引入 TuiBackend Trait（YAGNI）

**决策**: 直接实现 `src/tui/` 具体模块，不定义 `TuiBackend` trait。

**理由**:
- CLAUDE.md §3.2（YAGNI）：无具体调用者不引入抽象
- CLAUDE.md §3.3（Rule of Three）：现有 trait 均有 7+ 真实实现
- 当前仅 ratatui 一个后端，trait 为投机性抽象
- 若未来出现第二个后端，届时按 rule-of-three 提取

**影响**: 移除 `traits.rs` 和 `backend.rs`，合并为 `src/tui/app.rs` 中的具体实现。

### ADR-2: panic=abort 下终端恢复必须使用 panic hook

**决策**: 依赖 `std::panic::set_hook()` 而非 RAII Drop。

**理由**:
- `Cargo.toml:257` release profile 使用 `panic = "abort"`
- abort 模式下 `Drop` 实现**永远不会执行**
- `TerminalGuard { impl Drop }` 在 release 构建中完全失效
- `std::panic::set_hook` 在 abort 发生前同步执行，是唯一有效机制

### ADR-3: 事件循环使用 EventStream（非阻塞）

**决策**: 使用 `ratatui::crossterm::event::EventStream` 异步流，而非 `poll()` 阻塞调用。

**理由**:
- `crossterm::event::poll()` 是同步阻塞，在 Tokio 上下文中阻塞 runtime 线程
- `EventStream` 提供 `Stream<Item = Result<Event>>` 与 `tokio::select!` 原生集成
- 通过 `ratatui::crossterm` re-export 访问，与其余代码路径保持一致

---

## 模块结构

> 相较原始计划：移除 `traits.rs` 和 `backend.rs`（ADR-1）；新增 `terminal.rs` 重要性提升为 Phase 1 硬性要求（ADR-2）。

```
src/tui/
├── mod.rs              # 模块导出，run() 入口函数
├── app.rs              # TuiApp 状态机和事件循环（含 ratatui 渲染）
├── widgets/
│   ├── mod.rs          # Widget 导出
│   ├── chat.rs         # ChatPanel - 消息历史 + 滚动
│   ├── input.rs        # InputBox - 多行输入 + 历史（含大小限制/ANSI 过滤）
│   ├── tools.rs        # ToolOutput - 工具执行进度（PROGRESS_BLOCK 驱动）
│   └── status.rs       # StatusBar - 模型/状态信息（不含敏感数据）
├── events.rs           # TuiEvent enum + 事件分发（含 sentinel 解析）
├── state.rs            # TuiState（自含 view-model，零 agent/ 导入）
└── terminal.rs         # *** Phase 1 硬性要求 *** 终端初始化/panic hook/信号处理
```

---

## 核心设计

### Sentinel 协议（修订）

`on_delta` channel 携带混合格式字符串，必须按以下规则分发：

```
实际发送格式（src/agent/loop_.rs:267-277）：

┌─────────────────────────────────────────────────────────────────────┐
│ 常量名                      │ 值                  │ 载荷     │ 匹配  │
├─────────────────────────────┼─────────────────────┼──────────┼───────┤
│ DRAFT_CLEAR_SENTINEL        │ "\x00CLEAR\x00"      │ 无       │ ==    │
│ DRAFT_PROGRESS_SENTINEL     │ "\x00PROGRESS\x00"   │ 有前缀   │ prefix│
│ DRAFT_PROGRESS_BLOCK_SENTINEL│"\x00PROGRESS_BLOCK\x00"│有前缀 │ prefix│
└─────────────────────────────┴─────────────────────┴──────────┴───────┘

实际发送示例：
  tx.send(DRAFT_CLEAR_SENTINEL.to_string()).await             // 单独发送，无载荷
  tx.send(format!("{DRAFT_PROGRESS_SENTINEL}🤔 Thinking...\n"))  // 前缀 + 消息
  tx.send(progress_tracker.render_delta()).await              // 前缀 + 完整进度块
```

**正确的 `translate_delta` 实现**（参考 `src/channels/mod.rs:723-728`）：

```rust
use crate::agent::loop_::{
    DRAFT_CLEAR_SENTINEL,
    DRAFT_PROGRESS_SENTINEL,
    DRAFT_PROGRESS_BLOCK_SENTINEL,
};

fn translate_delta(delta: String) -> TuiEvent {
    if delta == DRAFT_CLEAR_SENTINEL {
        TuiEvent::Clear
    } else if let Some(msg) = delta.strip_prefix(DRAFT_PROGRESS_SENTINEL) {
        TuiEvent::ProgressLine { text: msg.to_string() }
    } else if let Some(block) = delta.strip_prefix(DRAFT_PROGRESS_BLOCK_SENTINEL) {
        TuiEvent::ProgressBlock { content: block.to_string() }
    } else {
        TuiEvent::Delta { text: delta }
    }
}
```

> 注：sentinels 为 `pub(crate)`，`src/tui/` 通过
> `crate::agent::loop_::DRAFT_*_SENTINEL` 直接访问，无需修改可见性。

### TuiEvent（替代 TuiOutputEvent）

```rust
// src/tui/events.rs
pub enum TuiEvent {
    // 来自 agent on_delta channel
    Delta { text: String },
    Clear,
    ProgressLine { text: String },    // DRAFT_PROGRESS_SENTINEL 载荷
    ProgressBlock { content: String }, // DRAFT_PROGRESS_BLOCK_SENTINEL 载荷（驱动 ToolOutput widget）

    // 来自用户输入
    UserMessage { content: String },
    Cancel,   // Ctrl+C：取消当前 in-flight 请求
    Quit,     // q / Ctrl+D：终止 TUI

    // 来自终端（使用 ratatui::crossterm re-export，避免版本分裂）
    Key(ratatui::crossterm::event::KeyEvent),
    Resize(u16, u16),
}
```

### Agent 集成方式

```
┌─────────────────┐  on_delta: mpsc::Sender<String>  ┌──────────────────┐
│   Agent loop    │ ──────────────────────────────▶  │  translate_delta │
│  (src/agent/)   │                                   │  (src/tui/events)│
└────────┬────────┘                                   └────────┬─────────┘
         │                                                     │ TuiEvent enum
         │ per-message CancellationToken                       ▼
         ◀─────────────────────────────────────── TuiApp.handle_event()
```

**CancellationToken 生命周期**（修订，per-message 模式）：

```rust
// 每条用户消息创建新 token（匹配现有 channels 模式）
let token = CancellationToken::new();
let child_token = token.child_token();  // 传给 agent loop

// 事件分发：
TuiEvent::Cancel → token.cancel()          // 取消当前请求，TUI 继续运行
TuiEvent::Quit   → token.cancel(); exit()  // 取消请求 + 退出 TUI

// Ctrl+C 双击保护：第二次 Ctrl+C（300ms 内）强制退出并恢复终端
```

### 终端初始化/恢复（Phase 1 硬性要求）

```rust
// src/tui/terminal.rs — 必须在进入 raw mode 之前调用

pub fn install_panic_hook() {
    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // panic = "abort" 下 Drop 不执行，此处是唯一有效恢复点
        // 通过 ratatui::crossterm re-export 访问（与 app.rs 保持一致）
        let _ = ratatui::crossterm::terminal::disable_raw_mode();
        let _ = ratatui::crossterm::execute!(
            std::io::stdout(),
            ratatui::crossterm::terminal::LeaveAlternateScreen,
            ratatui::crossterm::cursor::Show,
        );
        prev_hook(info);
    }));
}

pub async fn install_signal_handlers(cancel: CancellationToken) {
    // Unix: SIGTERM + SIGHUP（Docker/systemd 场景）
    #[cfg(unix)]
    {
        let mut sigterm = tokio::signal::unix::signal(SignalKind::terminate()).unwrap();
        let mut sighup  = tokio::signal::unix::signal(SignalKind::hangup()).unwrap();
        tokio::spawn(async move {
            tokio::select! {
                _ = sigterm.recv() => {},
                _ = sighup.recv()  => {},
            }
            let _ = ratatui::crossterm::terminal::disable_raw_mode();
            let _ = ratatui::crossterm::execute!(
                std::io::stdout(),
                ratatui::crossterm::terminal::LeaveAlternateScreen,
            );
            cancel.cancel();
        });
    }
}

// 初始化顺序（src/tui/app.rs::run()）：
// 1. install_panic_hook()            ← 必须第一步
// 2. install_signal_handlers(token)  ← 第二步
// 3. enable_raw_mode()               ← 第三步
// 4. EnterAlternateScreen            ← 第四步
// 5. 事件循环 ...
// 退出时：LeaveAlternateScreen + disable_raw_mode（正常路径）
```

### 事件循环（异步，非阻塞）

```rust
// src/tui/app.rs — 核心循环结构
// 全部通过 ratatui::crossterm re-export 访问，避免单独声明 crossterm 依赖
use ratatui::crossterm::event::EventStream;
use futures_util::StreamExt;

pub async fn run(config: &AppConfig) -> Result<()> {
    install_panic_hook();

    // 终端初始化
    ratatui::crossterm::terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    ratatui::crossterm::execute!(stdout, ratatui::crossterm::terminal::EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    let (delta_tx, mut delta_rx) = tokio::sync::mpsc::channel::<String>(256);
    let mut event_stream = EventStream::new();
    let session_cancel = CancellationToken::new();

    install_signal_handlers(session_cancel.child_token()).await;

    // 主事件循环
    loop {
        terminal.draw(|f| render(f, &state))?;

        tokio::select! {
            // 键盘/resize 事件（异步，不阻塞 runtime）
            Some(Ok(event)) = event_stream.next() => {
                handle_crossterm_event(event, &mut state)?;
            }
            // Agent delta channel
            Some(delta) = delta_rx.recv() => {
                let tui_event = translate_delta(delta);
                handle_tui_event(tui_event, &mut state);
            }
            // 会话级取消（SIGTERM/SIGHUP）
            _ = session_cancel.cancelled() => break,
        }

        if state.should_quit { break; }
    }

    // 正常退出清理（同样通过 ratatui::crossterm re-export）
    ratatui::crossterm::terminal::disable_raw_mode()?;
    ratatui::crossterm::execute!(terminal.backend_mut(), ratatui::crossterm::terminal::LeaveAlternateScreen)?;
    Ok(())
}
```

---

## 关键文件修改

| 文件 | 修改内容 |
|------|----------|
| `Cargo.toml` | feature flag: `tui-ratatui = ["dep:ratatui", "dep:crossterm"]`；ratatui 0.30 + crossterm(event-stream) |
| `src/lib.rs` | `#[cfg(feature = "tui-ratatui")] pub mod tui;` |
| `src/main.rs` | 添加 `Tui` 命令到 `Commands` enum |

---

## 安全约束（新增）

### InputBox 输入安全

```rust
// src/tui/widgets/input.rs
const MAX_INPUT_BYTES: usize = 64 * 1024; // 64KB 硬限制

fn sanitize_input(raw: &str) -> String {
    // 两步过滤：
    // 1. strip_ansi_escapes 移除 ANSI/VT 转义序列（\x1b[...m, \x1b]...BEL 等）
    // 2. 手动过滤剩余 C0 控制字符（\x00-\x1f），仅保留 allowlist：\n(\x0a) \t(\x09)
    //    同样过滤 C1 控制字符（\x80-\x9f，UTF-8 两字节形式：\xc2\x80 - \xc2\x9f）
    let stripped = strip_ansi_escapes::strip_str(raw);
    stripped.chars().filter(|&c| {
        c == '\n' || c == '\t' || (c as u32 > 0x1f && !(0x80..=0x9f).contains(&(c as u32)))
    }).collect()
}
```

### ChatPanel 渲染安全

Agent 响应（尤其是 shell 工具输出）可能含 ANSI 转义序列：
- 渲染前对 Delta 内容调用同一 `sanitize_input`
- 防止 `\x1b[2J`（清屏）、`\x1b]0;title\x07`（标题注入）等

### 日志策略（禁止记录敏感数据）

```rust
// 允许：
tracing::debug!("TUI received delta, len={}", delta.len());
tracing::info!("TUI session started");

// 禁止：
tracing::debug!("TUI received: {}", delta);    // ← 可能含敏感内容
tracing::debug!("User input: {}", input);      // ← 禁止
```

### StatusBar 展示约束

状态栏显示模型名称和提供商 ID，**不得**显示 API key、token 或任何凭证字段。

---

## 依赖（修订）

```toml
[features]
# 遵循 <subsystem>-<backend> 命名惯例（如 channel-matrix、browser-native）
tui-ratatui = ["dep:ratatui", "dep:crossterm"]  # crossterm 仅为启用 event-stream feature

[dependencies]
# ratatui 0.30 通过 crossterm feature 提供 re-export（ratatui::crossterm）
# 代码全部通过 ratatui::crossterm 访问，不直接 use crossterm::
# 单独声明 crossterm 仅为启用 event-stream feature（EventStream 异步事件流）
# 使用 ratatui::crossterm 访问 crossterm API，避免版本分裂
ratatui = { version = "0.30", optional = true, default-features = false, features = ["crossterm"] }
# EventStream（异步键盘事件）需要 crossterm 的 event-stream feature。
# ratatui 0.30 的 crossterm feature 默认不启用 event-stream，需额外添加：
crossterm = { version = "0.28", optional = true, default-features = false, features = ["event-stream"] }
# tui-ratatui feature 同时启用两者：
# tui-ratatui = ["dep:ratatui", "dep:crossterm"]
#
# 代码中统一通过 ratatui::crossterm 访问 crossterm API（防止版本分裂）；
# 单独声明 crossterm 仅为启用 event-stream feature，不直接 use crossterm::。
# Phase 1 验证：cargo tree -i crossterm → 应只有一个版本。
```

> 验证步骤：`cargo tree -i crossterm` 确认单版本；`cargo audit` 无已知漏洞。

---

## 实现阶段

### Phase 1: 核心基础设施（最高优先级）

| 任务 | 时间 | 产出 | 说明 |
|------|------|------|------|
| Cargo.toml feature flag | 0.5h | `tui-ratatui = [...]` | 遵循 \<subsystem\>-\<backend\> 命名 |
| **终端恢复机制** | **3h** | `src/tui/terminal.rs` | **panic hook + SIGTERM handler（Phase 1 硬性要求，ADR-2）** |
| 模块骨架 | 0.5h | `src/tui/mod.rs` | 无 traits.rs/backend.rs（ADR-1）|
| 状态类型 | 2h | `src/tui/state.rs` | 自含 view-model，零 agent/ 导入 |
| 事件类型和 sentinel 解析 | 2h | `src/tui/events.rs` | 三 sentinel 正确分发（strip_prefix）|
| TuiApp 骨架 | 2h | `src/tui/app.rs` | EventStream + tokio::select! 事件循环（ADR-3）|

### Phase 2: Widget 系统

| 任务 | 时间 | 产出 | 说明 |
|------|------|------|------|
| ChatPanel + 滚动 | 3h | `src/tui/widgets/chat.rs` | 含 ANSI 过滤 |
| InputBox + 历史 | 3h | `src/tui/widgets/input.rs` | 64KB 限制 + ANSI sanitize |
| ToolOutput 面板 | 2h | `src/tui/widgets/tools.rs` | 由 ProgressBlock 事件驱动 |
| StatusBar | 1h | `src/tui/widgets/status.rs` | 不含敏感字段 |
| Layout 组合 | 2h | `src/tui/widgets/mod.rs` | |

### Phase 3: Agent 集成

| 任务 | 时间 | 产出 | 说明 |
|------|------|------|------|
| 完整 TuiApp 状态机 | 4h | `src/tui/app.rs` | |
| 键盘处理 | 2h | | vim 风格 + 标准；Ctrl+C 双击保护 |
| Resize 处理 | 1h | | |
| CancellationToken 集成 | 2h | | per-message token；Cancel vs Quit 区分 |
| CLI 命令集成 | 2h | `src/main.rs` | |

### Phase 4: 测试和文档

| 任务 | 时间 | 产出 | 说明 |
|------|------|------|------|
| 单元测试 | 2h | `src/tui/state.rs` | TuiState 状态转换 |
| sentinel 解析测试 | 2h | `src/tui/events.rs` | 三 sentinel 全覆盖，含 null byte 注入 |
| **安全测试**（见下） | **3h** | | **新增** |
| 模块文档 | 1h | | |

**总计: 约 8-10 天**（原 10-12 天，因移除 trait 层简化了结构）

---

## 验证计划

### 本地验证

```bash
# 1. 编译检查（注意 feature flag 已改名）
cargo build --features tui-ratatui

# 2. 运行 TUI
cargo run --features tui-ratatui -- tui

# 3. 依赖审查
cargo tree -i crossterm      # 验证单版本，无分裂
cargo audit                   # 无已知漏洞

# 4. 功能测试场景
# - 发送消息，验证流式响应显示
# - 执行工具，验证 ToolOutput 面板进度（PROGRESS_BLOCK 驱动）
# - Ctrl+C 取消当前请求，TUI 继续运行（非退出）
# - 按 q 退出，验证终端正常恢复
# - 调整窗口大小，验证布局适应
# - 上下键导航输入历史
# - Page Up/Down 滚动聊天历史
```

### 功能测试用例

1. **基本交互**: 用户消息 → Agent 响应流式显示
2. **流式输出**: Delta 正确累积到 ChatPanel
3. **工具执行**: PROGRESS_BLOCK sentinel 解析并显示在 ToolOutput 面板
4. **取消（Cancel）**: Ctrl+C 取消 in-flight 请求，TUI 继续运行
5. **退出（Quit）**: q / Ctrl+D 正常退出，终端恢复
6. **Resize**: 布局适应终端大小变化
7. **输入历史**: 上下键导航
8. **滚动**: Page Up/Down 滚动聊天历史

### 安全测试用例（新增）

9. **Panic 恢复**: 在 PTY 中触发 panic，验证终端 raw mode 已恢复（不依赖 Drop）
10. **SIGTERM 处理**: 发送 SIGTERM，验证终端恢复后进程退出
11. **ANSI 注入（输入侧）**: 粘贴含 `\x1b[2J` 的内容到 InputBox，验证被过滤
12. **ANSI 注入（渲染侧）**: Agent 返回含 `\x1b]0;malicious\x07` 的 delta，验证不渲染为转义
13. **Sentinel 边界**: Agent 输出为 `"\x00PROGRESS\x00"` 字面字符串（无附加载荷）时，`translate_delta` 将其解析为 `ProgressLine { text: "" }`（空进度行），渲染为空进度条目而非 Chat delta；这是安全的良性结果（空进度行不可见）。验证：空 ProgressLine 不会导致 ChatPanel 内容被清除，也不会触发任何 UI 故障。
14. **双击 Ctrl+C**: 300ms 内连续两次 Ctrl+C，验证强制退出且终端状态正常

---

## 约束和考虑

1. **二进制大小**: Feature-gated（`tui-ratatui`），仅 `--features tui-ratatui` 时编译，预计增加 200-300KB（Phase 1 后实测验证）
2. **跨平台**: Linux/macOS 完全支持，Windows 10+ 需要 Windows Terminal（SIGTERM 处理降级为仅 Ctrl+C）
3. **架构一致性**: 无投机性 trait，直接具体模块实现（ADR-1）
4. **向后兼容**: 不影响现有 CLI 模式；feature flag 不进入 `default = []`
5. **panic = "abort"**: 终端恢复依赖 panic hook，不依赖 Drop（ADR-2）
6. **数据安全**: TUI 渲染路径不记录用户输入内容和 Agent 响应内容
