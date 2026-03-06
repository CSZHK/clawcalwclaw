# TUI 框架与通信渲染机制对比

本文档对比 clawclawclaw 与 OpenAI codex 的 TUI 架构设计。

## 架构总览

```
┌─────────────────────────────────────────────────────────────────────────────────────┐
│                           架构对比总览                                               │
├───────────────────────────────┬─────────────────────────────────────────────────────┤
│         clawclawclaw          │                    codex                            │
├───────────────────────────────┼─────────────────────────────────────────────────────┤
│  单体应用，TUI 内嵌 agent loop  │  客户端-服务器架构，协议层解耦                        │
│  task_local! 隐式上下文传递    │  显式 Event/Op 协议，JSON-RPC 风格                   │
│  ~500 行 TUI 核心代码          │  ~10k+ 行 TUI 代码，独立 crate                       │
│  ratatui 直接渲染              │  自定义 Terminal + FrameRequester 抽象              │
└───────────────────────────────┴─────────────────────────────────────────────────────┘
```

## 1. 通信架构

```
clawclawclaw                              codex
┌─────────────────┐                      ┌─────────────────┐
│    Agent Loop   │                      │   App Server    │
│   (loop_.rs)    │                      │ (app-server/)   │
└────────┬────────┘                      └────────┬────────┘
         │                                        │
         │ task_local!                            │ Event { id, msg }
         │ scope_agent_events()                   │ EventMsg enum
         │                                        │
         ▼                                        ▼
┌─────────────────┐    mpsc channel       ┌─────────────────┐
│  TUI Event Loop │◄────────────────────  │  ThreadEvent    │
│   (app.rs)      │   AgentEvent enum     │  Channel        │
└────────┬────────┘   (bounded: 256)       └─────────────────┘
         │                                        │
         │ TuiState mutation                     │ capacity: 32768
         │                                       │
         ▼                                        ▼
┌─────────────────┐                      ┌─────────────────┐
│   ratatui       │                      │   ChatWidget    │
│   widgets       │                      │   (chatwidget/) │
└─────────────────┘                      └─────────────────┘
```

### 关键差异

| 维度 | clawclawclaw | codex |
|------|--------------|-------|
| **协议定义** | `AgentEvent` (3 variants) | `EventMsg` (40+ variants) |
| **通道类型** | `mpsc::channel` (有界) | `mpsc::channel` (有界) |
| **通道容量** | 256 (try_send 非阻塞) | 32768 |
| **上下文传递** | `task_local!` + `scope()` | 显式参数 + `AppEventSender` |
| **协议层** | 无独立协议 crate | `codex_protocol` crate |

## 2. 事件类型对比

```
clawclawclaw::AgentEvent              codex::EventMsg
├── ToolStart { name, hint }          ├── TurnStarted
├── ToolComplete { name, success,     ├── TurnComplete
│   duration_secs }                   ├── TokenCount
└── Usage { input_tokens,             ├── AgentMessage
    output_tokens, cost_usd }         ├── AgentMessageDelta
                                      ├── AgentReasoning
                                      ├── ExecCommandBegin
                                      ├── ExecCommandEnd
                                      ├── McpToolCallBegin
                                      ├── McpToolCallEnd
                                      ├── WebSearchBegin
                                      ├── WebSearchEnd
                                      ├── ImageGenerationBegin
                                      ├── ... (40+ more)
```

### clawclawclaw AgentEvent 设计理由

clawclawclaw 故意保持事件类型极简：

1. **YAGNI 原则** — 只添加生产环境真实需要的事件类型
2. **零样板传递** — `task_local!` + `scope()` 无需修改函数签名
3. **静默降级** — `try_with` 在无 sender 时静默跳过，不阻塞 agent loop

## 3. 渲染架构

```
clawclawclaw                              codex
┌────────────────────────────────┐    ┌────────────────────────────────┐
│         widgets/mod.rs         │    │       chatwidget.rs            │
│  ┌──────────────────────────┐  │    │  ┌──────────────────────────┐  │
│  │ Layout::default()        │  │    │  │ HistoryCell trait        │  │
│  │ .constraints([           │  │    │  │ - render() -> Vec<Line>  │  │
│  │   Length(chat_height),   │  │    │  │ - active_cell            │  │
│  │   Length(tools_height),  │  │    │  │ - transcript overlay     │  │
│  │   Length(input_height),  │  │    │  └──────────────────────────┘  │
│  │   Length(1), // status   │  │    │                                │
│  │ ])                       │  │    │  ┌──────────────────────────┐  │
│  └──────────────────────────┘  │    │  │ BottomPane               │  │
│                                │    │  │ - chat_composer          │  │
│  直接调用 widget::render()     │    │  │ - approval_overlay       │  │
│                                │    │  │ - file_search_popup      │  │
└────────────────────────────────┘    │  │ - skill_popup            │  │
                                      │  └──────────────────────────┘  │
                                      └────────────────────────────────┘
```

### 关键差异

| 维度 | clawclawclaw | codex |
|------|--------------|-------|
| **渲染模式** | 直接 ratatui widget | `HistoryCell` trait + 动态 cell |
| **Overlay 支持** | 简单 `Clear` + `centered_rect` | 完整 overlay 系统 (Ctrl+T transcript) |
| **底部面板** | 固定 input box | 可切换的多视图 bottom pane |
| **Markdown** | 无原生支持 | 完整 markdown 渲染 (`markdown_render.rs`) |
| **Diff 渲染** | 无 | `diff_render.rs` 专门处理 |

## 4. 终端生命周期

```
clawclawclaw                              codex
┌────────────────────────────────┐    ┌────────────────────────────────┐
│ panic = "abort"                │    │ 标准 panic hook                │
│                                │    │                                │
│ 1. install_panic_hook()        │    │ 1. set_modes()                 │
│ 2. install_signal_handlers()   │    │ 2. enable_raw_mode()           │
│ 3. enable_raw_mode()           │    │ 3. EnterAlternateScreen        │
│ 4. EnterAlternateScreen        │    │ 4. PushKeyboardEnhancementFlags│
│                                │    │ 5. EnableBracketedPaste        │
│ ⚠️ Drop 无效，依赖 hook        │    │ 6. EnableFocusChange           │
└────────────────────────────────┘    └────────────────────────────────┘
```

### 关键差异

| 维度 | clawclawclaw | codex |
|------|--------------|-------|
| **Panic 策略** | `abort` | 标准 unwinding |
| **信号处理** | SIGTERM/SIGHUP only | 同 + Ctrl-Z suspend/resume |
| **Bracketed Paste** | 无 | 启用 |
| **Keyboard Enhancement** | 无 | 启用 (modifier disambiguation) |
| **Focus Tracking** | 无 | 启用 (desktop notifications) |

### clawclawclaw panic=abort 设计理由

```
Release profile 中的 panic = "abort" 决定了终端恢复策略：

❌ 禁止模式：
   impl Drop for TerminalGuard {
       fn drop(&mut self) { restore_terminal(); }  // panic 时永不执行
   }

✅ 正确模式：
   panic::set_hook(Box::new(|_| {
       let _ = restore();  // 在 abort 前同步执行
   }));
```

参考：`src/tui/terminal.rs`，`src/tui/app.rs:64-77`

## 5. 状态管理

```
clawclawclaw::TuiState              codex::ChatWidget
├── messages: Vec<TuiChatMessage>   ├── history_cells: Vec<Box<dyn HistoryCell>>
├── input_buffer: String            ├── active_cell: Option<Box<dyn HistoryCell>>
├── mode: InputMode                 ├── thread_input_state: ThreadInputState
├── tool_calls: Vec<ToolCallEntry>  ├── agent_turn_running: bool
├── session_input_tokens: u64       ├── token_usage: TokenUsage
├── session_output_tokens: u64      ├── pending_approval: Option<ApprovalRequest>
├── session_cost_usd: f64           ├── mcp_startup_status: Option<McpStartupStatus>
├── show_help: bool                 ├── show_transcript_overlay: bool
└── pending_approval: Option<PendingApproval>  └── ... (50+ fields)
```

### clawclawclaw 状态设计原则

1. **零 agent 耦合** — `state.rs` 禁止导入 `crate::agent`
2. **单一职责** — 状态只存储，不处理业务逻辑
3. **最小字段集** — 只添加生产必需的状态字段

## 6. Approval 流程

```
clawclawclaw                              codex
┌────────────────────────────────┐    ┌────────────────────────────────┐
│ NonCliApprovalContext          │    │ ExecApprovalRequestEvent       │
│ ├── approval_prompt_rx         │    │ ├── request_id                 │
│ └── resolve via ApprovalManager│    │ ├── command                    │
│                                │    │ ├── cwd                        │
│ TUI 监听 approval_prompt_rx     │    │ └── reason                     │
│ Y/N → record_non_cli_pending   │    │                                │
│       _resolution()            │    │ AppEvent::FullScreenApproval   │
└────────────────────────────────┘    │ ├── approval_overlay view      │
                                      │ └── multiple approval types    │
                                      └────────────────────────────────┘
```

### clawclawclaw Approval 设计

复用现有 `NonCliApprovalContext` (Telegram/Discord channels 使用相同机制)：

```rust
// loop_.rs
pub(crate) struct NonCliApprovalContext {
    approval_prompt_rx: mpsc::Receiver<NonCliApprovalPrompt>,
}

// app.rs
fn resolve_approval(approval_manager: &ApprovalManager, request_id: &str, approved: bool) {
    let decision = if approved { ApprovalResponse::Yes } else { ApprovalResponse::No };
    approval_manager.record_non_cli_pending_resolution(request_id, decision);
}
```

## 7. 总结对比

| 维度 | clawclawclaw | codex |
|------|--------------|-------|
| **复杂度** | 极简 (~500 行核心) | 企业级 (~10k+ 行) |
| **协议层** | 无独立协议 | 独立 `codex_protocol` crate |
| **事件总线** | 3 种事件类型 | 40+ 种事件类型 |
| **多线程** | 无 (单 agent) | 多线程 ThreadManager |
| **可扩展性** | 通过 trait 扩展 | 通过协议扩展 + plugin 系统 |
| **二进制大小** | 优化目标 | 较大 |
| **启动速度** | 快 | 较慢 |
| **适用场景** | 单机 CLI agent | 多客户端协作平台 |

## 8. 设计决策记录

### 为什么 clawclawclaw 不采用 codex 模式？

1. **YAGNI** — 无多客户端需求，协议层是过度设计
2. **性能目标** — 二进制大小和启动速度是产品目标
3. **复杂度控制** — 单体应用更易审计和维护
4. **安全边界** — 减少攻击面，agent loop 与 TUI 同进程

### 何时考虑迁移到 codex 模式？

- 需要远程 agent 执行
- 需要多客户端 (Web UI, VSCode extension)
- 需要协议版本化和向后兼容保证
- 需要第三方客户端接入

## 9. TUI 功能完备性矩阵

| 功能 | 状态 | 实现位置 | 备注 |
|------|------|----------|------|
| **核心架构** ||||
| AgentEvent typed channel | ✅ | `loop_.rs`, `app.rs` | bounded: 256 |
| Terminal resize handling | ✅ | `app.rs` | `terminal.autoresize()` |
| Panic hook recovery | ✅ | `terminal.rs` | panic=abort 安全 |
| Signal handlers | ✅ | `terminal.rs` | SIGTERM/SIGHUP |
| **渲染组件** ||||
| Chat panel | ✅ | `widgets/chat.rs` | text stream |
| Tool panel | ✅ | `widgets/tools.rs` | structured table |
| Input box | ✅ | `widgets/input.rs` | 64KB limit |
| Status bar | ✅ | `widgets/status.rs` | tokens + cost |
| Help overlay | ✅ | `widgets/help.rs` | `?` toggle |
| Approval modal | ✅ | `widgets/approval.rs` | Y/N confirmation |
| **事件处理** ||||
| Sentinel protocol | ✅ | `events.rs` | DRAFT_CLEAR, PROGRESS, PROGRESS_BLOCK |
| AgentEvent routing | ✅ | `app.rs` | tokio::select! |
| Keyboard input | ✅ | `app.rs` | crossterm Event::Key |
| **安全约束** ||||
| Text sanitization | ✅ | `sanitize.rs` | ANSI/control strip |
| Credential ban in status | ✅ | `status.rs` | model name only |
| No user content logging | ✅ | 全局 | metadata only |

### 架构补齐记录

| 补齐项 | 原状态 | 修复 | Commit |
|--------|--------|------|--------|
| Terminal resize | ❌ 无处理 | `terminal.autoresize()` on Event::Resize | feature branch |
| AgentEvent channel | ❌ unbounded | bounded(256) + `try_send()` | feature branch |

## 10. 扩展指南：添加新 TUI 功能

### 10.1 添加新的 AgentEvent 类型

```
1. 定义事件 (src/agent/loop_.rs)
   ├─ pub(crate) enum AgentEvent { ... }
   └─ 添加新 variant

2. 发送事件 (src/agent/loop_.rs)
   ├─ 在相关业务逻辑处调用 emit_agent_event()
   └─ 例: LLM 响应后发送 Usage

3. TUI 接收 (src/tui/app.rs)
   ├─ tokio::select! 添加 event_rx 分支
   └─ 匹配新 variant，更新 TuiState

4. 状态更新 (src/tui/state.rs)
   ├─ 添加对应状态字段
   └─ 实现更新方法

5. 渲染 (src/tui/widgets/*.rs)
   └─ 读取状态字段渲染
```

### 10.2 添加新 Widget

```
1. 创建 widget 文件
   src/tui/widgets/<name>.rs
   pub fn render(frame: &mut Frame, area: Rect, state: &TuiState)

2. 注册模块
   src/tui/widgets/mod.rs
   pub mod <name>;

3. 添加 layout 约束
   widgets/mod.rs::render()
   Constraint::Length(N)

4. 调用渲染
   widgets/mod.rs::render()
   <name>::render(frame, areas[N], state)

5. 测试
   tests/tui_render_test.rs
   添加渲染验证测试
```

### 10.3 添加 Overlay/Modal

```
1. 创建 overlay widget
   src/tui/widgets/<name>.rs
   使用 Clear + centered_rect 模式

2. 状态控制
   src/tui/state.rs
   show_<name>: bool

3. 键盘绑定
   src/tui/app.rs
   在 handle_tui_event() 添加切换逻辑

4. 渲染顺序
   src/tui/widgets/mod.rs
   在所有 widget 之后渲染 overlay
```

### 10.4 关键约束

| 约束 | 说明 | 文件 |
|------|------|------|
| 状态零耦合 | `state.rs` 禁止导入 `crate::agent` | `src/tui/CLAUDE.md` |
| 文本消毒 | 所有渲染文本必须过 `sanitize_text()` | `widgets/sanitize.rs` |
| 非阻塞发送 | 使用 `try_send()` 避免 agent 阻塞 | `loop_.rs` |
| Sentinel 顺序 | PROGRESS_BLOCK 必须在 PROGRESS 之前检查 | `events.rs` |
| Hook 初始化顺序 | panic_hook → signal_handlers → raw_mode | `app.rs:64-77` |

## 参考

- clawclawclaw TUI: `src/tui/`
- clawclawclaw Agent Loop: `src/agent/loop_.rs`
- clawclawclaw 状态耦合规则: `src/tui/CLAUDE.md`
- codex TUI: `codex-rs/tui/`
- codex Protocol: `codex-rs/protocol/`
- codex App Server: `codex-rs/app-server/`
