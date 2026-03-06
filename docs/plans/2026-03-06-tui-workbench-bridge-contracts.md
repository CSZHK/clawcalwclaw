# TUI Workbench Bridge Contracts (2026-03-06)

Status: implementation-spec draft  
Type: narrow follow-up design note  
Refs: `docs/plans/2026-03-06-tui-agent-workbench-proposal.md`, `docs/plans/2026-03-06-tui-workbench-implementation-checklist.md`, `docs/plan/rust-tui-plan-high-level.md`

---

## 1. 目的

本文档是 `docs/plans/2026-03-06-tui-agent-workbench-proposal.md` 的配套细化规格，目标是冻结 3 个足够窄、可直接编码的桥接契约：

1. `Approval Bridge`
2. `Sub-agent Observability Bridge`
3. `SpecView / TaskBoardView Projection`

本文档只解决“怎么把现有 runtime 能力安全、清晰地接到 TUI”这个问题，不扩展为新的平台架构。

---

## 2. 设计约束

所有契约必须满足以下仓库约束：

- 保持终端原生单体架构，不新增协议 crate。
- 不把 TUI 视图层变成新的权威状态源。
- 不阻塞 agent/sub-agent 热路径。
- 不在 Phase 1/2 默认开放新的高风险写操作。
- 不泄露 prompts、secrets、原始敏感载荷。
- 小步可回滚：每个 bridge 都应能独立引入、独立回退。

当前代码锚点：

- `src/tui/app.rs`
- `src/tui/events.rs`
- `src/agent/loop_.rs`
- `src/approval/mod.rs`
- `src/tools/subagent_spawn.rs`
- `src/tools/subagent_registry.rs`
- `src/goals/engine.rs`
- `src/tools/task_plan.rs`

---

## 3. Contract A — Approval Bridge

### 3.1 目标

让 TUI 成为 `NonCliApprovalContext` 的一个正式消费者，使 supervised tool execution 的非 CLI 审批流程可以在 TUI 内完整闭环。

当前差距：`src/tui/app.rs` 调用 `run_tool_call_loop_with_non_cli_approval_context(..., None, ...)`，说明 TUI 还没有把 prompt/confirm/resolve 接起来。

### 3.2 现有 authority

现有 authority 已经存在于 runtime：

- pending request authority: `ApprovalManager`
- prompt payload: `NonCliApprovalPrompt`
- resolution authority: `ApprovalManager::record_non_cli_pending_resolution`

TUI 只负责：

- 接收 prompt
- 渲染 queue / overlay
- 发起 approve / deny 动作
- 本地维护 view state

### 3.3 签名

建议新增一个 TUI 内部桥接模块，例如 `src/tui/approval_bridge.rs`，只提供 helpers，不新增 trait。

```rust
pub struct ApprovalQueueItem {
    pub request_id: String,
    pub tool_name: String,
    pub arguments_preview: String,
    pub requested_at: std::time::Instant,
    pub status: ApprovalQueueStatus,
}

pub enum ApprovalQueueStatus {
    Pending,
    Approved,
    Denied,
    Expired,
    Failed,
}

pub enum ApprovalBridgeEvent {
    PromptReceived(ApprovalQueueItem),
    PromptResolved {
        request_id: String,
        decision: crate::approval::ApprovalResponse,
    },
    PromptFailed {
        request_id: String,
        reason: String,
    },
}

pub fn build_tui_non_cli_approval_context(
    sender: String,
    reply_target: String,
    prompt_tx: tokio::sync::mpsc::UnboundedSender<crate::agent::loop_::NonCliApprovalPrompt>,
) -> crate::agent::loop_::NonCliApprovalContext;

pub fn spawn_approval_prompt_bridge(
    prompt_rx: tokio::sync::mpsc::UnboundedReceiver<crate::agent::loop_::NonCliApprovalPrompt>,
    ui_tx: tokio::sync::mpsc::UnboundedSender<ApprovalBridgeEvent>,
) -> tokio::task::JoinHandle<()>;

pub fn resolve_approval_request(
    approval_manager: &crate::approval::ApprovalManager,
    request_id: &str,
    sender: &str,
    channel_name: &str,
    reply_target: &str,
    decision: crate::approval::ApprovalResponse,
) -> Result<(), ApprovalBridgeError>;
```

### 3.4 数据模型

#### Runtime input

现有 runtime prompt 结构：

```rust
pub(crate) struct NonCliApprovalPrompt {
    pub request_id: String,
    pub tool_name: String,
    pub arguments: serde_json::Value,
}
```

#### TUI projection

TUI 不应长期持有或展示未经处理的完整参数大对象。渲染面默认只保留：

- `request_id`
- `tool_name`
- `arguments_preview`
- `requested_at`
- `status`

`arguments_preview` 建议策略：

- 限制长度
- 截断深层 JSON
- 过滤 secrets / tokens / key-like 字段
- 保留“足够让人做审批判断”的摘要

### 3.5 正常流程

```text
TUI run loop
  ↓
create prompt channel
  ↓
build NonCliApprovalContext
  ↓
pass Some(context) into run_tool_call_loop_with_non_cli_approval_context(...)
  ↓
runtime creates pending request in ApprovalManager
  ↓
runtime sends NonCliApprovalPrompt over prompt_tx
  ↓
TUI prompt bridge receives prompt
  ↓
PromptReceived -> queue item / overlay
  ↓
user chooses approve or deny
  ↓
resolve_approval_request(...)
  ├─ confirm_non_cli_pending_request / reject_non_cli_pending_request
  └─ record_non_cli_pending_resolution(Yes/No)
  ↓
agent loop consumes resolution and continues or fails closed
```

### 3.6 异常与失败策略

| 场景 | 来源 | TUI 行为 | Runtime 行为 |
|---|---|---|---|
| request 不存在 | `ApprovalManager` | 标记 `Failed` | fail closed |
| request 已过期 | `ApprovalManager` | 标记 `Expired` | fail closed |
| requester mismatch | `ApprovalManager` | 标记 `Failed`，提示上下文不匹配 | fail closed |
| TUI 关闭/取消 | `CancellationToken` | 清理本地 queue | runtime 视为 `No` |
| prompt bridge 崩溃 | TUI task | 记录错误，停止审批 UI | runtime 最终超时为 `No` |

**关键规则**：

- 任何 TUI bridge 故障都不能导致 tool 默认放行。
- 任何未显式 `Yes` 的结论都按 deny 处理。
- TUI 不直接修改 allowlist，只调用现有 `ApprovalManager` 能力。

### 3.7 配置

Phase 1 不新增 config key，直接复用 runtime 现有时间窗口：

- `NON_CLI_APPROVAL_WAIT_TIMEOUT_SECS`
- `NON_CLI_APPROVAL_POLL_INTERVAL_MS`

TUI 侧只允许局部 UI 常量，例如：

- overlay 刷新节流
- 参数预览长度上限

这些常量应先保留为模块内常量，不升级为公开配置。

### 3.8 测试场景

| 场景 | 输入 | 预期 |
|---|---|---|
| prompt 到达 | `NonCliApprovalPrompt` | queue 中新增一条 `Pending` |
| approve 正常闭环 | valid request + matching sender/channel/reply target | runtime 收到 `Yes`，queue 变 `Approved` |
| deny 正常闭环 | valid request | runtime 收到 `No`，queue 变 `Denied` |
| requester mismatch | 错误 sender/channel/reply target | 不放行，queue 变 `Failed` |
| timeout | 无用户动作 | runtime fail closed，queue 变 `Expired` 或 `Denied` |

---

## 4. Contract B — Sub-agent Observability Bridge

### 4.1 目标

让 sub-agent 在不改变 `SubAgentRegistry` authority 的前提下，向 TUI 提供可归属、低开销、可降级的实时执行可见性。

当前差距：`src/tools/subagent_spawn.rs` 在 agentic sub-agent 路径中使用 `NoopObserver`，导致 TUI 最多只能看到 registry 终态，无法看到实时工具/LLM 进度。

### 4.2 现有 authority

现有 authority 应继续保持：

- session lifecycle authority: `SubAgentRegistry`
- sub-agent execution authority: `subagent_spawn` task body
- observability vocabulary: `ObserverEvent` / `ObserverMetric`

TUI 只负责消费一个“安全子集”的 telemetry。

### 4.3 签名

建议不新增新的全局 observability trait，而是新增一个 TUI forwarding observer，实现现有 `Observer`。

```rust
pub struct SubagentTelemetryEvent {
    pub session_id: String,
    pub agent_name: String,
    pub event: crate::observability::ObserverEvent,
    pub at: std::time::Instant,
}

pub struct TuiForwardingObserver {
    session_id: String,
    agent_name: String,
    ui_tx: tokio::sync::mpsc::UnboundedSender<SubagentTelemetryEvent>,
}

impl crate::observability::Observer for TuiForwardingObserver {
    fn record_event(&self, event: &crate::observability::ObserverEvent);
    fn record_metric(&self, metric: &crate::observability::ObserverMetric);
    fn name(&self) -> &str;
}

pub fn build_subagent_observer(
    session_id: String,
    agent_name: String,
    ui_tx: tokio::sync::mpsc::UnboundedSender<SubagentTelemetryEvent>,
) -> std::sync::Arc<dyn crate::observability::Observer>;
```

若需要避免 `UnboundedSender`，也可以使用 bounded channel + try_send；但核心契约不变：**observer 不阻塞 agent 热路径**。

### 4.4 数据模型

TUI 不直接把 `ObserverEvent` 原样塞进 UI state，而是投影到 agent pane：

```rust
pub struct SubAgentProjectionItem {
    pub session_id: String,
    pub agent_name: String,
    pub status: SubAgentViewStatus,
    pub task_summary: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub last_event_summary: Option<String>,
    pub last_tool_name: Option<String>,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub error_summary: Option<String>,
}

pub enum SubAgentViewStatus {
    Running,
    Completed,
    Failed,
    Killed,
}
```

投影来源：

- lifecycle/status: `SubAgentRegistry`
- live telemetry: forwarding observer
- final result summary: registry result

### 4.5 正常流程

```text
subagent_spawn
  ↓
registry.try_insert(session)
  ↓
create TuiForwardingObserver(session_id, agent_name, ui_tx)
  ↓
pass observer into agentic background run
  ↓
observer emits safe telemetry events
  ↓
TUI updates SubAgentProjectionItem
  ↓
registry.complete/fail/kill remains final authority
  ↓
TUI merges terminal status from registry with last live telemetry
```

### 4.6 允许的事件范围

Phase 3 之前建议只消费现有安全事件，不新增 observability schema：

- `AgentStart`
- `LlmRequest`
- `LlmResponse`
- `ToolCallStart`
- `ToolCall`
- `AgentEnd`
- `Error`

**不允许**：

- 原始 prompt 内容
- 原始模型输出全文
- 工具敏感参数全文
- 任何 secrets/token-like payload

### 4.7 异常与失败策略

| 场景 | 处理 |
|---|---|
| TUI receiver 不存在 | observer 静默降级，不影响 sub-agent 执行 |
| TUI channel 背压/关闭 | 丢弃或 coalesce 事件，不阻塞执行 |
| telemetry 丢失 | registry 终态仍然正确 |
| observer 构造失败 | 回退到 `NoopObserver`，但要显式打 debug/warn |

**关键规则**：

- `SubAgentRegistry` 继续是 lifecycle authority。
- live telemetry 只是增强可见性，不可驱动真实执行状态转换。
- TUI 不可基于单条 telemetry 推断“completed”；必须等 registry 终态。

### 4.8 配置

Phase 1-3 不新增公共配置。先允许局部常量：

- 每个 sub-agent 的最近事件保留数
- 同类事件 coalesce 窗口
- agent pane 最大缓存行数

### 4.9 测试场景

| 场景 | 输入 | 预期 |
|---|---|---|
| tool start event | `ObserverEvent::ToolCallStart` | 对应 agent pane 显示工具运行中 |
| llm response event | `ObserverEvent::LlmResponse` | token / latency 摘要更新 |
| ui detached | receiver dropped | sub-agent 仍完成，registry 终态正确 |
| final complete | registry.complete | pane 进入 `Completed` |
| event flood | 多条快速 telemetry | UI 不阻塞、不崩溃、允许 coalesce |

---

## 5. Contract C — SpecView / TaskBoardView Projection

### 5.1 目标

定义一个严格只读的任务投影视图，把 durable goals 与 session-scoped task plan 同屏展示，但绝不把它变成新的 authority。

### 5.2 现有 authority

已有 authority：

- durable goals: `GoalEngine`
- session plan: `TaskPlanTool`

当前关键事实：

- `GoalEngine` 已有 typed state。
- `TaskPlanTool` 当前只有内部 `TaskItem`，缺少 typed snapshot API。

因此，这个 contract 的一个前置条件是：**为 `TaskPlanTool` 增加只读 snapshot 能力，或者提取等价的只读 session-plan adapter。**

### 5.3 签名

建议先定义投影层使用的数据结构与读取接口，而不是先改渲染。

```rust
pub enum TaskAuthorityKey {
    GoalStep {
        goal_id: String,
        step_id: String,
    },
    SessionTask {
        task_id: usize,
    },
}

pub enum TaskBoardStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Blocked,
}

pub struct TaskBoardItem {
    pub authority: TaskAuthorityKey,
    pub title: String,
    pub status: TaskBoardStatus,
    pub priority_label: Option<String>,
    pub group_label: String,
    pub detail_summary: Option<String>,
}

pub struct TaskBoardView {
    pub durable_items: Vec<TaskBoardItem>,
    pub session_items: Vec<TaskBoardItem>,
    pub merged_items: Vec<TaskBoardItem>,
    pub refreshed_at: String,
}

pub struct TaskPlanSnapshotItem {
    pub id: usize,
    pub title: String,
    pub status: TaskBoardStatus,
}

impl TaskPlanTool {
    pub fn snapshot(&self) -> Vec<TaskPlanSnapshotItem>;
}

pub async fn build_task_board_view(
    goal_engine: &crate::goals::engine::GoalEngine,
    task_plan: Option<&crate::tools::task_plan::TaskPlanTool>,
) -> anyhow::Result<TaskBoardView>;
```

### 5.4 数据映射

#### Goals → TaskBoardItem

| GoalEngine source | TaskBoard field |
|---|---|
| `Goal.id` + `Step.id` | `authority = GoalStep` |
| `Step.description` | `title` |
| `Step.status` | `status` |
| `Goal.priority` | `priority_label` |
| `Goal.description` | `group_label` |
| `Step.result` / `Goal.last_error` | `detail_summary` |

#### Session plan → TaskBoardItem

| TaskPlan source | TaskBoard field |
|---|---|
| `TaskItem.id` | `authority = SessionTask` |
| `TaskItem.title` | `title` |
| `TaskItem.status` | `status` |
| N/A | `priority_label = None` |
| fixed label | `group_label = "Session Plan"` |
| N/A | `detail_summary = None` |

### 5.5 正常流程

```text
TUI task pane refresh
  ↓
load GoalState from GoalEngine
  ↓
read TaskPlanTool snapshot (if available)
  ↓
map both sources into TaskBoardItem
  ↓
merge into TaskBoardView
  ↓
render grouped columns / grouped list
```

### 5.6 合并规则

- 不按标题去重。
- 不跨 authority 自动同步状态。
- `GoalStep` 与 `SessionTask` 即使标题相同也必须保留为不同项。
- `merged_items` 只是排序/渲染便利视图，不代表写回目标。

### 5.7 异常与失败策略

| 场景 | 行为 |
|---|---|
| goals.json 不存在 | durable 区为空，不视为错误 |
| GoalEngine 读取失败 | task pane 降级显示错误摘要 |
| TaskPlan snapshot 不可用 | 只显示 durable goals |
| 单项状态未知 | 显式映射为 fallback 状态，不静默伪装为 `Pending` |

**关键规则**：

- Task board 不得隐式写回 `GoalEngine`。
- Task board 不得通过 UI 排序/拖拽改变 authority 数据。
- 若未来支持编辑，必须走显式命令和显式 authority adapter。

### 5.8 配置

Phase 2 前不新增公开配置。局部常量允许：

- refresh interval
- 每组最多显示条目数
- detail summary 截断长度

### 5.9 测试场景

| 场景 | 输入 | 预期 |
|---|---|---|
| only goals | GoalState present, no session plan | 正常生成 durable items |
| only session plan | no goals, TaskPlan snapshot present | 正常生成 session items |
| same title, different source | goal step 与 session task 标题相同 | merged view 保留两条 |
| blocked/failed mapping | GoalEngine failed/blocked | 状态正确映射 |
| snapshot read-only | build view | 不修改 authority state |

---

## 6. 集成顺序

推荐集成顺序如下：

1. `Approval Bridge`
   - 因为它直接影响 supervised tool safety。
2. `TaskBoardView Projection`
   - 因为它是 read-only，可先交付 operator value。
3. `Sub-agent Observability Bridge`
   - 因为它需要对 sub-agent 执行链做更深的可观测接线。

这个顺序有两个好处：

- 先补安全闭环，再补任务可见性，最后补多 agent 深度可见性。
- 每一步都能单独验证，不需要大爆炸式改造。

---

## 7. 验收门槛

任一 bridge 开始编码前，必须满足以下门槛：

- 有明确 authority owner。
- 有明确只读/可变边界。
- 有明确的失败关闭策略。
- 有至少 3 个 focused tests。
- 不新增未经验证的公开 config surface。

---

## 8. 非目标

本文档不定义以下内容：

- `/git`、`/mcp`、`/workflow` 的 mutating commands
- 多窗格布局细节
- markdown/diff 渲染实现
- daemon lifecycle UI
- 远程客户端协议

这些都应在本文件之外单独设计。
