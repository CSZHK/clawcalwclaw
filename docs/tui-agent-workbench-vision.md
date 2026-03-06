# clawclawclaw TUI: Agent-Native Workbench Vision

> 从 chatbot wrapper 到 agent-native workbench 的产品演进方案
> v3 — 补充: Spec 概念、三层 Agent 架构、Use Case 信息流、自适应布局、全局状态机、WaveTerm 竞品参考

---

## 1. 核心论断

**当前 TUI 是一个穿了终端皮肤的 chatbot，不是 agent workbench。**

问题不只是 "agent 执行不可见"——更根本的是 **agent 的工作方式不可管理**。后端已有完整的能力矩阵（skills、SOP workflow、MCP、goals、task plan、sub-agent、git ops、bg jobs），但 TUI 对这些子系统的暴露几乎为零。

```
┌─────────────────────────────────────────────────────────────────────┐
│                    当前 TUI 的本质                                   │
│                                                                     │
│  用户 ──(输入文字)──▶ Agent ──(流式文本)──▶ 用户 ──(输入文字)──▶ …    │
│                                                                     │
│  这是 chatbot 交互模式：一问一答、线性、被动                         │
│  后端的 12 个子系统全部隐藏在文本流背后                               │
└─────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────┐
│                  Agent-Native Workbench 的本质                       │
│                                                                     │
│  用户 ──(定义目标)──▶ Agent ──(分解任务)──▶ 并行执行                 │
│       ◀──(审批请求)──        ──(进度报告)──▶                         │
│       ──(调整方向)──▶        ──(交付产出)──▶                         │
│       ──(管理资源)──▶        ──(编排团队)──▶                         │
│                                                                     │
│  用户是指挥官 + 资源管理者，不是提问者                                │
└─────────────────────────────────────────────────────────────────────┘
```

### 1.1 数据证据

| 维度 | 当前 clawclawclaw TUI | Claude Code CLI | codex TUI |
|------|----------------------|-----------------|-----------|
| AgentEvent 类型数 | 3 | ~15 (推测) | 40+ |
| 状态字段数 | 14 (TuiState) | N/A | 50+ (ChatWidget) |
| Diff 渲染 | 无 | 内联终端 | 独立 diff_render |
| Approval 上下文 | 纯 Y/N | 命令预览+diff | 完整 overlay |
| 并行任务视图 | 无 | sub-agent 列表 | 多线程 ThreadManager |
| Context 预算可见性 | 无 | token 计数 | TokenUsage 详情 |
| Markdown 渲染 | 无 | 原生终端 ANSI | 完整 markdown_render |
| 文件树感知 | 无 | cwd + git status | workspace 集成 |

来源：`docs/tui-architecture-comparison.md`，`src/tui/state.rs:42-57`，竞品调研

### 1.2 后端能力 vs TUI 暴露：冰山图

```
                     ┌───────────────────────┐
                     │  TUI 当前暴露 (~5%)    │
                     │  • 文本聊天            │
                     │  • 简单工具进度        │
                     │  • Y/N 审批            │
  ───────────────────┴───────────────────────┴───────────── 水面线
                     │                       │
                     │  后端已有能力 (95%)     │
                     │                       │
  Skills             │  src/skills/          │  Skill 加载/审计/模板/安装
  SOP Workflow       │  src/sop/             │  SopEngine + 多执行模式 + 审计
  Goals              │  src/goals/           │  GoalEngine + Step 追踪 + 优先级
  Task Plan          │  src/tools/task_plan  │  Session 内任务分解
  Sub-Agents         │  src/tools/subagent_* │  spawn/manage/list/registry
  Delegate           │  src/tools/delegate   │  多 agent 委托 + 负载均衡
  BG Jobs            │  src/tools/bg_run     │  后台异步任务 + 结果注入
  MCP Client         │  src/tools/mcp_*      │  MCP 服务器连接/工具发现
  Git Ops            │  src/tools/git_operations │  结构化 git 操作
  Coordination       │  src/coordination/    │  InMemoryMessageBus + IPC
  Agent IPC          │  src/tools/agents_ipc │  跨 agent 消息 + 状态共享
  Team Orchestration │  src/agent/team_orchestration │  拓扑选择 + 预算估算
                     │                       │
                     └───────────────────────┘
```

[*] 这 12 个子系统是 clawclawclaw 区别于所有竞品的核心能力。TUI workbench 的使命是让用户 **看到、管理、编排** 这些能力，而不只是通过自然语言间接触达它们。

---

## 2. 产品定位重塑

### 2.1 用户画像与核心 Use Case

**目标用户**：终端优先的开发者/运维/安全研究员，习惯 CLI 工作流，拒绝 IDE 锁定。

**核心 Use Case 不是 "和 AI 聊天"，而是：**

| Use Case | 用户期望 | 当前能力 | 差距 |
|----------|---------|---------|------|
| 多文件重构 | 看到每个文件的 diff，逐个审批或批量确认 | 只看到文本流 | 致命 |
| Bug 诊断 | Agent 自主读代码→读日志→定位根因→提出修复 | 能做但过程不可见 | 严重 |
| 项目脚手架 | 定义目标后 agent 自主创建文件结构 | 逐步文本指导 | 严重 |
| 安全审计 | 并行扫描多个模块，汇总 findings | 串行逐个讨论 | 致命 |
| 硬件调试 | 连接外设→读传感器→分析数据→调整参数 | 有 peripheral 工具但 TUI 无专属视图 | 中等 |

### 2.2 竞品矩阵与差异化定位

```
                    高可见性
                       ▲
                       │
              OpenHands │  codex TUI
              (Web GUI) │  (协议驱动)
                       │
                       │  WaveTerm
                       │  (Electron+Agent)
        ───────────────┼───────────────▶ 高自主性
                       │
           aider       │  Claude Code CLI
           (终端原生)   │  (流式终端)
                       │
                       │  clawclawclaw TUI
                       │  (当前位置)
                    低可见性
```

**差异化路径**：不走 OpenHands 的 Web GUI 路线（违背终端优先哲学），也不走 codex 的协议层解耦路线（YAGNI），而是做 **终端原生的高可见性 agent workbench**——在 ratatui 约束下最大化 agent 执行透明度。

### 2.3 核心设计原则

#### Agent 哲学公理（3 条）

统摄 agent 运行时、任务分解、能力编排的底层哲学：

**A1. 强约束用脚本，智能选择交模型**

确定性的东西用代码/脚本硬约束，不确定性的东西交给模型判断。两者的边界必须清晰。

```
确定性（脚本/代码）                 非确定性（模型判断）
──────────────────                ──────────────────
文件权限检查                       选择哪个文件需要修改
安全策略白名单匹配                  判断修改方案的优劣
SOP step 流转控制                  决定是否需要分解任务
审批流程的触发条件                  理解用户意图和上下文
token 预算的硬性截断                选择最优的执行路径
config schema 校验                 生成代码/文档内容
```

[*] 反模式：用 prompt 做本该脚本做的事（如用 prompt 告诉模型"不要修改 .env 文件"——应该用 SecurityPolicy 硬拦截）。同样反模式：用脚本硬编码本该模型判断的事（如用规则引擎决定代码重构方案）。

**A2. Skill 是资源、脚本、提示词、工作流的完美组合单元**

Skill 不是"prompt 模板"——它是 agent 能力的最小可组合单元，将四种要素封装为一个可发现、可审计、可渐进披露的包：

```
┌─ Skill ─────────────────────────────────────────────────────┐
│                                                              │
│  Resources (资源)     脚本/模板/配置/数据文件                  │
│  ──────────────       skill 执行所需的静态资产                │
│                                                              │
│  Scripts (脚本)       确定性的执行逻辑 (A1 左侧)             │
│  ────────────         校验、转换、门控、自动化步骤            │
│                                                              │
│  Prompts (提示词)     非确定性的指导 (A1 右侧)               │
│  ──────────────       塑造模型行为的上下文和约束              │
│                                                              │
│  Workflow (工作流)    step 编排 + 条件分支 + 审批点           │
│  ─────────────        Skill 内的 mini-SOP                    │
│                                                              │
│  渐进披露策略:                                                │
│  触发匹配 → 注入最小上下文 → 按需展开详细指导 → 完成后回收     │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

[*] Skill 在上下文中的渐进式披露是核心：不是一次性把所有 prompt 塞进 context，而是根据任务阶段按需注入。这直接影响 context window 利用率（见 A3）。

**A3. 任务复杂度是三维的，不是一维的**

任务复杂度不能简化为"人工工时"，也不能只看 context window 大小。Agent 面对的复杂度有三个正交维度：

```
┌─────────────────────────────────────────────────────────────────┐
│              Agent 任务复杂度三维模型                              │
│                                                                 │
│  D1. 上下文容量                                                  │
│  ─────────────                                                  │
│  "这个任务需要多少 context window？"                              │
│  度量: 预估 token 消耗 vs 单窗口容量                             │
│  应对: 拆分为多个独立 context window 并行/串行执行                │
│                                                                 │
│  D2. 知识边界                                                    │
│  ─────────                                                      │
│  "模型的训练语料覆盖这个领域吗？"                                 │
│  度量: 模型对目标技术/API/框架的熟悉程度                          │
│  应对: 联网深度研究 → 知识沉淀到 context → 再执行                 │
│                                                                 │
│  D3. 环境不确定性                                                │
│  ───────────                                                    │
│  "方案在真实环境中可行吗？"                                       │
│  度量: 模型无法仿真的外部约束（运行时行为、第三方服务、硬件特性）  │
│  应对: 技术 demo 验证 → 收集真实反馈 → 调整方案                   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

三个维度的相互关系：

```
                    D2 知识边界
                       ▲
                       │
          不熟悉       │       熟悉
      (需要联网研究)    │    (训练语料内)
                       │
    ┌──────────────────┼──────────────────┐
    │  最难             │                 │
    │  需要: 研究       │  纯知识缺口      │
    │  + 验证 + 大窗口  │  需要: 研究      │
    │                  │  + 沉淀          │
    ├──────────────────┼──────────────────┤ ──▶ D3 环境不确定性
    │  纯可行性风险      │                 │
    │  需要: demo 验证  │  标准任务        │
    │  + 探测反馈       │  只看 D1         │
    │                  │                 │
    └──────────────────┴──────────────────┘
          不可控                可控
       (需要 demo)          (可仿真)
```

**D1. 上下文容量**（可量化，context window 为工作单元）

```
传统思维（以人为单元）              Agent 思维（以 context window 为单元）
──────────────────────            ────────────────────────────────────
"这个任务大概需要 3 小时"          "这个任务需要 ~45k tokens 的上下文"
"简单/中等/复杂"                   "1 window / 2 windows / 需要分解"
"2 个人并行做更快"                 "拆成 2 个独立 context 并行执行"
"高级工程师做得更好"               "用更大的 window + 更好的 prompt"

任务分解决策:
┌──────────────────────────────────────────────────────────────┐
│ 预估 context 需求 ≤ 0.75 窗口                                │
│   → Single agent, 无需分解 (with D2/D3 check)               │
│                                                              │
│ 预估 context 需求 0.75–1.5 窗口                              │
│   → LeadSubagent (拆为 2 个 context，降低单窗口压力)         │
│                                                              │
│ 预估 context 需求 > 1.5 窗口                                 │
│   → 必须分解为多个 ≤ 0.75 窗口的子任务                       │
│   → 子任务之间的依赖关系决定拓扑:                             │
│     • 无依赖 → 并行 (StarTeam)                               │
│     • 有序依赖 → 串行 (LeadSubagent pipeline)                │
│     • 双向依赖 → 协作 (MeshTeam)                             │
│                                                              │
│ 0.75 系数 = 预留 25% 给反射/重试/审批交互                    │
└──────────────────────────────────────────────────────────────┘
```

**D2. 知识边界**（需要探测，研究结果沉淀为 context）

模型的训练语料有截止日期，且对小众领域覆盖不均。当任务涉及模型不熟悉的知识时，必须先研究再执行。

```
知识熟悉度评估（由模型自评 + Coordinator 校验）:

┌──────────────────────────────────────────────────────────────┐
│ 高熟悉度 (训练语料充分覆盖)                                    │
│   信号: 模型能直接给出准确代码/方案，无需查阅                    │
│   策略: 直接执行，D1 主导                                      │
│                                                              │
│ 中熟悉度 (语料有覆盖但可能过时)                                 │
│   信号: 模型知道概念但不确定最新 API/版本                        │
│   策略: 轻量研究 → web_fetch 查最新文档 → 沉淀到 context        │
│                                                              │
│ 低熟悉度 (语料稀缺或完全未覆盖)                                 │
│   信号: 新发布的框架/API/协议、小众硬件、私有 SDK                │
│   策略: 深度研究阶段 (独立 context window):                     │
│     1. 联网搜索 → 收集权威文档/示例                             │
│     2. 阅读 + 提取关键知识 → 压缩为结构化笔记                   │
│     3. 知识沉淀到 Skill 资源或 session context                  │
│     4. 基于沉淀知识执行任务                                     │
│                                                              │
│ [*] 深度研究本身消耗 context window，必须计入 D1 预算            │
│     研究产出的知识沉淀可跨 agent 复用 (Skill A2)                │
└──────────────────────────────────────────────────────────────┘
```

**D3. 环境不确定性**（不可推理，必须实验验证）

有些复杂度无法通过更多的 tokens 或更好的知识解决——模型无法仿真真实环境的行为。这类不确定性只能通过实际运行代码来消解。

```
环境不确定性类型:

┌──────────────────────────────────────────────────────────────┐
│ 技术可行性                                                    │
│   "这个 API 真的支持 streaming 吗？文档说支持但没有例子"       │
│   策略: 构建最小 demo → 实际调用 → 根据结果决策                │
│                                                              │
│ 运行时行为                                                    │
│   "并发场景下这个锁机制会不会死锁？"                           │
│   策略: 写压测脚本 → BG Job 执行 → 分析结果                   │
│                                                              │
│ 第三方服务约束                                                │
│   "这个 OAuth provider 的 rate limit 是多少？"                │
│   策略: 探测性请求 → 记录响应头 → 调整设计                    │
│                                                              │
│ 硬件/外设特性                                                 │
│   "这个传感器的实际采样率和标称值一致吗？"                      │
│   策略: Peripheral 工具读取 → 统计实际值 → 校准配置             │
│                                                              │
│ [*] Demo/验证步骤是不可跳过的硬性前置条件                      │
│     结果必须写入 Spec (Agent 可牺牲，产出不可丢)                        │
│     验证失败 → 方案回退点，不是"重试到成功"                    │
└──────────────────────────────────────────────────────────────┘
```

**三维复杂度 → 任务编排策略映射**

```
┌─────────┬──────────┬──────────┬───────────────────────────────┐
│ D1 容量  │ D2 知识  │ D3 环境  │ 编排策略                       │
├─────────┼──────────┼──────────┼───────────────────────────────┤
│ ≤0.75窗 │ 高熟悉   │ 可控     │ Single agent, 直接执行        │
│ ≤0.75窗 │ 低熟悉   │ 可控     │ 先研究(1窗) → 再执行(1窗)     │
│ ≤0.75窗 │ 高熟悉   │ 不可控   │ 先 demo(1窗) → 再执行(1窗)    │
│ ≤0.75窗 │ 低熟悉   │ 不可控   │ 研究(1窗) → demo(1窗) → 执行  │
│ 0.75–   │ 高熟悉   │ 可控     │ LeadSubagent (2 context)      │
│ 1.5窗   │          │          │                               │
│ 0.75–   │ 低/不可控│          │ 研究/demo 前置 → LeadSubagent  │
│ 1.5窗   │          │          │                               │
│ >1.5窗  │ 高熟悉   │ 可控     │ 拆分子任务, 拓扑由依赖决定     │
│ >1.5窗  │ 低熟悉   │ 不可控   │ 研究 + demo 作为前置阶段,      │
│         │          │          │ 知识沉淀后再拆分执行           │
└─────────┴──────────┴──────────┴───────────────────────────────┘

[!] 研究和验证阶段本身也消耗 context window (D1)
    必须在 estimate_budget() 中计入这些前置开销
```

[*] 这改变了拓扑选择的量化标准：不再是"任务看起来复不复杂"，而是三维评估——"context 够不够（D1）、知识够不够（D2）、能不能仿真（D3）"。`team_orchestration.rs` 的 `estimate_budget()` 需要扩展为三维评估入口。

#### Canonical Contract Table

全局唯一真相表——后续所有 section 从此表派生，禁止在其他位置引入不一致的名称或数量。

| 契约维度 | 规范值 | 派生位置 |
|---------|--------|---------|
| **事件总数** | ~20 | 3.4 事件清单, 5.2 决策, 9.2 KPI |
| **全局状态** | ChatOnly / SpecMode / MultiAgent / Daemon | 3.7 状态机 |
| **Agent 层级** | Coordinator / Specialist / Background | 3.2.7 三层架构 |
| **布局模式** | Single (<100) / TwoColumn (100-159) / ThreeColumn (160+) | 3.5.1, 3.7 矩阵 |
| **持久化层级** | L1(磁盘) / L2(可选) / L3(内存) | TUI 原则 6 |
| **复杂度维度** | D1 容量 / D2 知识 / D3 环境 | A3 公理 |
| **编排阈值** | ≤0.75窗(Single) / 0.75-1.5窗(Lead) / >1.5窗(Star/Mesh) | A3, 3.3.1 |
| **Spec 术语** | Spec = view-model, GoalEngine = 权威源, Kanban = Spec 视图模式 | 3.2.5 |

**事件名规范（~20 个）**：

| # | 事件名 | Payload | Phase | TUI 消费者 |
|---|--------|---------|-------|-----------|
| 1 | ToolStart | tool_name, args | 1 | 工具面板 |
| 2 | ToolComplete | tool_name, result, duration | 1 | 工具面板 |
| 3 | Usage | tokens, cost | 1 | Status bar |
| 4 | TaskPlanCreated | steps | 2 | Spec tasks |
| 5 | StepStarted | step_id | 2 | Spec 进度 |
| 6 | StepCompleted | step_id, outcome | 2 | Spec 进度 |
| 7 | FileChanged | path, change_type, agent_id | 1 | 文件变更列表 |
| 8 | DiffReady | path, hunks | 2 | Diff widget |
| 9 | ApprovalNeeded | request | 1 | Approval overlay |
| 10 | ApprovalResolved | id, decision | 1 | Approval 队列 |
| 11 | SubAgentSpawned | id, task, layer | 3 | Agent Tab |
| 12 | SubAgentCompleted | id, result | 3 | Agent Tab |
| 13 | ContextPressure | used, limit | 1 | Context bar |
| 14 | ResearchNeeded | topic, familiarity | 2 | Spec 研究阶段 |
| 15 | DemoVerification | desc, status | 3 | Spec 验证阶段 |
| 16 | SpecCreated | spec_id, title | 2 | 布局切换 |
| 17 | SpecUpdated | spec_id, changes | 2 | Spec 面板 |
| 18 | SpecTaskChanged | spec_id, task_id, status | 2 | Spec tasks |
| 19 | AgentLayerChanged | agent_id, layer | 3 | Agent 侧栏 |
| 20 | BgAgentNotification | agent_id, msg | 3 | 通知/侧栏 |

#### TUI 设计原则（6 条）

从 Agent 哲学派生的 TUI 层设计规则：

1. **Spec 是唯一的进度真相源** — GoalEngine steps + Agent output 汇聚到 Spec，TUI 只读 Spec 渲染，恢复从 Spec 快照重建
2. **事件单向流，状态单向派生** — Agent emit → Channel → TUI State → Widget。禁止反向依赖。唯一反向通道：UserMessage、ApprovalResolved、/ 命令
3. **Agent 可牺牲，产出不可丢** — Agent 失败时保留已产出的 Spec changes/tasks，Coordinator 接管或重新 spawn（派生自 A3：产出是 context window 的结晶，丢失意味着浪费整个窗口）
4. **渐进暴露** — 简单问答=单面板Chat，多步=+Spec，多Agent=+Tabs，Daemon=+SOP Runs。复杂度按事件触发展开（派生自 A2：Skill 的渐进披露 + A3：按窗口需求动态展开）
5. **审批是断点，不是阻塞** — suspend 请求者，不阻塞其他 agent。Daemon headless 时降级为异步通知（派生自 A1：审批触发条件是脚本硬约束，审批决策是人类智能）
6. **持久化三层不混** — L1(磁盘:goals/sop_audit/ipc.db)、L2(可选:spec.json/session.json)、L3(内存:TuiState/BgJobs/SubAgents)

```
持久化层级与恢复策略：

L1 磁盘（必须）        L2 可选持久化          L3 内存（会话级）
──────────────        ────────────          ─────────────────
goals.json            spec.json             TuiState
sop_audit.log         session.json          BgJobs
ipc.db                                      SubAgents
                                            WorkbenchFocus

恢复优先级: L1 → L2 → 从 L1 重建 L3
```

---

## 3. 架构演进方案

### 3.1 从 Chatbot 到 Workbench 的数据结构变革

> "Show me your tables, and I won't usually need your flowcharts." — Fred Brooks

当前数据结构是为 chatbot 设计的：

```rust
// 当前: 线性消息列表 (chatbot 思维)
// src/tui/state.rs:42-57
pub struct TuiState {
    pub messages: Vec<TuiChatMessage>,     // 一维消息流
    pub input_buffer: String,              // 单一输入
    pub progress_block: Option<String>,    // 单一进度
    pub status: TuiStatus,                 // 三态: Idle/Thinking/ToolRunning
    // ... 14 个字段
}
```

Agent workbench 需要的数据结构：

```rust
// 目标: 任务驱动的工作台 (agent 思维)
pub struct WorkbenchState {
    // === 任务层 ===
    pub active_task: Option<TaskContext>,           // 当前目标
    pub active_spec: Option<Spec>,                  // 当前 Spec (view-model)
    pub task_plan: Vec<TaskStep>,                   // 分解后的步骤
    pub step_status: Vec<StepStatus>,               // 每步状态

    // === 执行层 ===
    pub tool_executions: Vec<ToolExecution>,         // 工具调用历史 (结构化)
    pub pending_approvals: VecDeque<ApprovalItem>,   // 审批队列
    pub active_agents: Vec<SubAgentView>,            // 并行 agent 视图

    // === 产出层 ===
    pub file_changes: Vec<FileChange>,               // 文件变更摘要
    pub findings: Vec<Finding>,                       // 分析发现

    // === 上下文层 ===
    pub context_budget: ContextBudget,               // token 预算
    pub workspace_files: Vec<WorkspaceFile>,          // 工作区文件树
    pub session_cost: SessionCost,                    // 成本追踪

    // === 交互层 ===
    pub messages: Vec<TuiChatMessage>,               // 保留聊天 (降级为辅助)
    pub input_buffer: String,
    pub focus: WorkbenchFocus,                       // 焦点面板
}

pub enum WorkbenchFocus {
    Chat,           // 传统聊天
    TaskPlan,       // 任务计划视图
    ToolOutput,     // 工具输出视图
    FileChanges,    // 文件变更视图
    Approvals,      // 审批队列视图
}
```

### 3.2 管理面：TUI 原生资源管理系统

Agent workbench 不只是看 agent 干什么——更重要的是管理 agent **用什么干、怎么干、干到哪**。

#### 3.2.1 七大管理域总览

```
┌─────────────────────────────────────────────────────────────────────┐
│                    TUI 管理面架构                                     │
│                                                                     │
│  ┌─ Skills ──────┐  ┌─ Workflow ─────┐  ┌─ MCP ──────────┐         │
│  │ 列表/启用/禁用 │  │ SOP 列表/执行  │  │ 服务器列表/状态 │         │
│  │ 安装/审计     │  │ 步骤审批/跳过  │  │ 工具发现/调用  │         │
│  │ 触发匹配      │  │ 优先级调整     │  │ 连接/断开      │         │
│  └───────────────┘  └───────────────┘  └───────────────┘         │
│                                                                     │
│  ┌─ Spec/Goals ──┐  ┌─ Git Worktree ─┐  ┌─ Agents ──────┐         │
│  │ Goal 追踪     │  │ worktree 列表  │  │ sub-agent 列表│         │
│  │ Step 进度     │  │ branch 管理    │  │ 委托/召回     │         │
│  │ 看板视图      │  │ diff 预览      │  │ 负载/成本     │         │
│  │ 优先级排序    │  │ commit 管理    │  │ 拓扑选择      │         │
│  └───────────────┘  └───────────────┘  └───────────────┘         │
│                                                                     │
│  ┌─ Commands ────┐                                                  │
│  │ / 命令发现    │  ← 统一的命令入口                                  │
│  │ 快捷键绑定    │                                                    │
│  │ 批量操作      │                                                    │
│  └───────────────┘                                                  │
└─────────────────────────────────────────────────────────────────────┘
```

#### 3.2.2 Skills 管理面

Skill 是 agent 能力的最小可组合单元（公理 A2），将资源、脚本、提示词、工作流封装为一个可发现、可审计、可渐进披露的包。TUI 的 Skills 管理面让用户看到 agent 当前装备了哪些能力，以及这些能力如何被触发和消费。

```
Skill 四要素与 A1 边界的映射：

    Skill
    ├── Resources (静态资产)     ← 脚本侧：确定性
    ├── Scripts (执行逻辑)       ← 脚本侧：校验/转换/门控
    ├── Prompts (上下文指导)     ← 模型侧：塑造行为
    └── Workflow (step 编排)     ← 混合：脚本控流转，模型做决策

    渐进披露生命周期：
    触发匹配 → 注入最小上下文 → 按需展开 → 完成后回收 context
                                              ↑
                                    关键：释放 context window 空间 (A3)
```

后端能力：`src/skills/mod.rs` — Skill 结构体(name/description/version/tags/tools/prompts)，加载/审计/模板/安装。

```
TUI Skills 管理视图：

┌─ Skills ─────────────────────────────────────────────────────┐
│  Name              Version  Tags        Status    Source     │
│  ─────────────────────────────────────────────────────────── │
│  brainstorming      1.0.0   [workflow]   active   global     │
│  code-review        2.1.0   [quality]    active   project    │
│  frontend-design    1.3.0   [ui,react]   active   global     │
│► test-driven-dev    1.0.0   [testing]    disabled project    │
│  webapp-testing     0.9.0   [e2e]        active   open-skill │
│                                                               │
│  [e] enable  [d] disable  [i] install  [a] audit  [/] filter │
└──────────────────────────────────────────────────────────────┘
```

数据流：
```
skills::load_skills_with_config()  ──▶  SkillsRegistry event  ──▶  TuiState.skills
skills::audit_skill_directory()    ──▶  SkillAuditReport      ──▶  TuiState.skill_audits
```

用户操作：
- `/skills` — 打开 skills 管理面板
- `/skill install <name>` — 从 open-skills 仓库安装
- `/skill audit` — 审计当前 skill 目录
- `/skill enable/disable <name>` — 运行时启用/禁用 skill
- 选中 skill 后 Enter — 查看详情 (description, tools, prompts)

#### 3.2.3 Workflow (SOP) 管理面

后端能力：`src/sop/` — SopEngine + SopTrigger(Mqtt/Webhook/Cron/Peripheral) + SopExecutionMode(Auto/Supervised/StepByStep/PriorityBased) + SopAuditLogger。

```
TUI Workflow 管理视图：

┌─ Workflows (SOP) ────────────────────────────────────────────┐
│  Name              Trigger     Mode         Status   Steps   │
│  ─────────────────────────────────────────────────────────── │
│  deploy-prod       cron        supervised   idle     5       │
│  sensor-alert      peripheral  auto         idle     3       │
│► code-review-pr    webhook     step_by_step running  4/7     │
│  nightly-backup    cron        auto         idle     2       │
│                                                               │
│  ──── code-review-pr 执行详情 ────                            │
│  Step 1: Clone repo          [✓] 12s                         │
│  Step 2: Run linter          [✓] 45s                         │
│  Step 3: Security scan       [✓] 120s                        │
│  Step 4: Generate report     [~] running...                  │
│  Step 5: Post comment        [>] pending                     │
│  Step 6: Update status       [>] pending                     │
│  Step 7: Notify team         [>] pending                     │
│                                                               │
│  [r] run  [s] stop  [a] approve-step  [k] skip  [p] priority│
└──────────────────────────────────────────────────────────────┘
```

关键交互：
- SOP `step_by_step` 模式下，每步完成后 TUI 弹出 approval overlay
- SOP `supervised` 模式下，启动前需 approval
- SOP `auto` 模式下，只显示进度（不中断用户）
- 支持运行时切换执行模式（如从 auto 临时切到 step_by_step）

#### 3.2.4 MCP 服务器管理面

后端能力：`src/tools/mcp_client.rs` — McpServer (connect/initialize/list_tools/call_tool)，多传输层 (stdio/HTTP/SSE)。

```
TUI MCP 管理视图：

┌─ MCP Servers ────────────────────────────────────────────────┐
│  Name              Transport  Status      Tools   Latency    │
│  ─────────────────────────────────────────────────────────── │
│  vibe-kanban        stdio      connected   15      12ms      │
│  supabase           http       connected   8       45ms      │
│  playwright         stdio      connected   22      8ms       │
│► claudemem          stdio      error       0       -         │
│  zread              stdio      connected   3       15ms      │
│                                                               │
│  ──── vibe-kanban 工具列表 ────                               │
│  list_workspaces    list_projects    list_issues              │
│  create_issue       update_issue     delete_issue             │
│  list_tags          assign_issue     ...                      │
│                                                               │
│  [c] connect  [d] disconnect  [r] refresh  [t] test-call     │
└──────────────────────────────────────────────────────────────┘
```

关键价值：
- 用户可以直观看到哪些 MCP 服务器可用，哪些挂了
- 点击具体服务器可以看到它提供的工具列表
- 可以手动 reconnect 失败的服务器
- 当 agent 调用 MCP 工具时，工具面板显示来自哪个 MCP 服务器

#### 3.2.5 Spec + GoalEngine 集成

Spec 是 TUI 的中央 view-model——汇聚 GoalEngine steps 和 Agent 产出，是 TUI 渲染进度的唯一真相源。Kanban 降级为 Spec tasks 的一种视图模式。

##### Spec 数据结构

```rust
pub struct Spec {
    pub id: String,
    pub title: String,
    pub sections: Vec<SpecSection>,      // Overview, Architecture, etc.
    pub tasks: Vec<SpecTask>,            // From GoalEngine steps
    pub changes: Vec<AttributedChange>,  // Per-agent file changes
    pub goal_ids: Vec<String>,
}

pub struct AttributedChange {
    pub file_path: String,
    pub change_type: ChangeType,
    pub agent_id: String,
    pub additions: u32,
    pub deletions: u32,
}
```

##### 与后端的关系

```
GoalEngine (L1持久化)          Spec (L2可选持久化)          TUI Widget
goals.json                     spec.json                   SpecPanel
─────────────────             ─────────────────           ─────────────
Goal.steps ──────────────────▶ Spec.tasks ──────────────▶ Kanban 视图
                               Spec.changes ────────────▶ 文件变更列表
                               Spec.sections ───────────▶ 概览面板

TaskPlanTool (L3内存)
session 内任务 ──────────────▶ Spec.tasks (合并)
```

- GoalEngine (L1持久化) 是权威源 → Spec tasks 从中派生
- TaskPlanTool (L3内存) 会话级任务 → Spec tasks
- Spec 是 view-model (L2可选持久化)，不替代 GoalEngine

后端能力：
- `src/goals/engine.rs` — GoalEngine + GoalState + Goal(id/description/status/priority/steps)
- `src/tools/task_plan.rs` — TaskPlanTool + TaskItem(id/title/status:Pending/InProgress/Completed)

```
TUI Spec Kanban 视图：

┌─ Goals ──────────────────────────────────────────────────────┐
│                                                               │
│  Pending          In Progress       Completed                │
│  ───────          ───────────       ─────────                │
│  ┌────────────┐   ┌────────────┐   ┌────────────┐           │
│  │ Refactor   │   │ Fix auth   │   │ Setup CI   │           │
│  │ auth module│   │ timeout    │   │ pipeline   │           │
│  │ ★★☆ medium │   │ ★★★ high   │   │ ★☆☆ low    │           │
│  │ steps: 0/4 │   │ steps: 2/3 │   │ steps: 3/3 │           │
│  └────────────┘   └────────────┘   └────────────┘           │
│  ┌────────────┐                                              │
│  │ Add tests  │                                              │
│  │ for parser │                                              │
│  │ ★★☆ medium │                                              │
│  │ steps: 0/5 │                                              │
│  └────────────┘                                              │
│                                                               │
│  [n] new goal  [Enter] expand steps  [p] priority  [d] done  │
└──────────────────────────────────────────────────────────────┘

展开 Goal 后的 Step 视图：

┌─ Fix auth timeout (★★★ high) ────────────────────────────────┐
│  Step 1: Analyze jwt_handler.py:84    [✓] completed          │
│  Step 2: Implement config-based expiry [~] in_progress       │
│  Step 3: Add tests                     [>] pending           │
│                                                               │
│  Context: Token过期时间硬编码为5分钟导致频繁登出               │
│  Last error: None                                             │
│                                                               │
│  [a] assign to agent  [e] edit step  [s] skip  [b] block    │
└──────────────────────────────────────────────────────────────┘
```

[i] Spec 是 TUI 内原生的进度视图概念，Kanban 是 Spec tasks 按 status 分列的一种视图模式。数据来源是 `GoalEngine`（goals.json）和 `TaskPlanTool`（session 内存），汇聚到 Spec view-model。外部看板工具（如 vibe-kanban MCP）是可选的数据同步目标，不是 Spec 本身。

#### 3.2.6 Git Worktree 管理面

后端能力：`src/tools/git_operations.rs` — GitOperationsTool (status/diff/log/show/branch/commit/add/checkout/stash/reset/revert)

```
TUI Git Worktree 视图：

┌─ Git ────────────────────────────────────────────────────────┐
│  Branch: feat/tui-workbench  ← main                         │
│                                                               │
│  Worktrees:                                                   │
│  ─────────                                                    │
│  main                /home/kang/apps/zeroclaw        active  │
│► feat/tui-workbench  /tmp/wt/tui-workbench           active  │
│  fix/auth-timeout    /tmp/wt/auth-timeout            clean   │
│                                                               │
│  Staged:                                                      │
│  M  src/tui/state.rs                                         │
│  A  src/tui/widgets/goals.rs                                 │
│                                                               │
│  Unstaged:                                                    │
│  M  src/tui/widgets/mod.rs                                   │
│  M  docs/tui-agent-workbench-vision.md                       │
│                                                               │
│  [Enter] view diff  [c] commit  [w] new worktree  [s] stash │
└──────────────────────────────────────────────────────────────┘
```

关键交互：
- 展开文件查看 unified diff（复用 Phase 2 的 diff widget）
- 一键创建 worktree + branch（为新任务隔离环境）
- 与 Goals 联动：为 goal 创建 worktree 时自动关联
- agent 修改文件后自动刷新 git status

#### 3.2.7 Agents 编排管理面

##### 三层 Agent 架构

```
┌─────────────────────────────────────────────────────────────────┐
│  Layer          │  角色              │  TUI 呈现                │
├─────────────────┼────────────────────┼──────────────────────────┤
│  Coordinator    │  main agent loop   │  主对话面板              │
│                 │  DelegateTool lead │                          │
├─────────────────┼────────────────────┼──────────────────────────┤
│  Specialist     │  SubAgentSpawnTool │  独立 Tab                │
│                 │  DelegateTool work │                          │
├─────────────────┼────────────────────┼──────────────────────────┤
│  Background     │  复用 SubAgent     │  侧栏小图标              │
│                 │  + 自动触发        │                          │
└─────────────────┴────────────────────┴──────────────────────────┘
```

```rust
pub enum AgentLayer {
    Coordinator,
    Specialist { task_id: String },
    Background { trigger: BgAgentTrigger },
}

pub enum BgAgentTrigger { FileChange, TaskComplete, Periodic(u64), Manual }

pub struct AgentView {
    pub id: String,
    pub name: String,
    pub layer: AgentLayer,
    pub status: AgentStatus,
    pub task_summary: String,
    pub token_usage: TokenUsage,
    pub cost: f64,
    pub changes: Vec<AttributedChange>,
}
```

##### Background Agent vs BG Job

BG Job = 单次工具调用，无状态。Background Agent = 会话生命周期，命名 agent，跨触发累积上下文。

[!] 实现前提：SubAgentSpawnTool 当前用 NoopObserver → 需改为共享 delta sender 路由输出到 TUI。

后端能力：
- `src/tools/subagent_spawn.rs` — SubAgentSpawnTool
- `src/tools/subagent_manage.rs` — 管理子 agent 生命周期
- `src/tools/delegate.rs` — DelegateTool + AgentSelectionPolicy + 负载均衡
- `src/agent/team_orchestration.rs` — TeamTopology (Single/LeadSubagent/StarTeam/MeshTeam)
- `src/tools/bg_run.rs` — BgJobStore + 后台异步执行
- `src/tools/agents_ipc.rs` — AgentsListTool / AgentsSendTool / AgentsInboxTool / StateGet/SetTool
- `src/coordination/mod.rs` — InMemoryMessageBus + CoordinationEnvelope

```
TUI Agent 编排视图：

┌─ Agent Team ─────────────────────────────────────────────────┐
│  Topology: StarTeam  Budget: $2.50  Elapsed: 4m 32s          │
│                                                               │
│  ┌─ Lead ──────────────────────────────────────────────────┐ │
│  │  Model: openrouter/claude-4  Status: thinking           │ │
│  │  Task: Coordinate auth refactoring                      │ │
│  │  Tokens: 12.4k/100k  Cost: $0.89                       │ │
│  └─────────────────────────────────────────────────────────┘ │
│                                                               │
│  ┌─ Agent-1 ────────────┐  ┌─ Agent-2 ────────────┐         │
│  │  reviewer             │  │  implementer          │         │
│  │  gemini-2.5-flash     │  │  claude-4-sonnet      │         │
│  │  [~] reviewing code   │  │  [~] writing tests    │         │
│  │  tokens: 8.2k         │  │  tokens: 15.1k        │         │
│  │  cost: $0.12          │  │  cost: $0.45           │         │
│  └───────────────────────┘  └───────────────────────┘         │
│                                                               │
│  ┌─ BG Jobs ───────────────────────────────────────────────┐ │
│  │  j-a1b2c3  shell: cargo test    [~] running  45s        │ │
│  │  j-d4e5f6  web_fetch: docs      [✓] complete 12s        │ │
│  └─────────────────────────────────────────────────────────┘ │
│                                                               │
│  [Tab] switch agent  [m] message agent  [k] kill agent       │
│  [t] change topology  [b] adjust budget                      │
└──────────────────────────────────────────────────────────────┘
```

拓扑选择器：
```
┌─ Select Topology ────────────────────────────────────────────┐
│                                                               │
│  ► Single        1 agent, lowest cost                        │
│    LeadSubagent  1 lead + 1 worker, good for code+review     │
│    StarTeam      1 lead + N workers, parallel tasks          │
│    MeshTeam      N peers, collaborative                      │
│                                                               │
│  估算 (基于 team_orchestration.rs)：                          │
│  ─────                                                        │
│  预计 tokens:  45,000 (cache factor: 0.10)                   │
│  预计成本:     $1.80                                         │
│  预计质量:     pass_rate 0.88                                │
│  协调消息:     ~12 条                                        │
│                                                               │
│  [Enter] confirm  [Esc] cancel                               │
└──────────────────────────────────────────────────────────────┘
```

#### 3.2.8 Commands 统一入口

所有管理操作通过 `/` 命令统一入口触达，支持模糊搜索：

```
┌─ Commands ───────────────────────────────────────────────────┐
│  > /sk                                                        │
│                                                               │
│  /skills              打开 Skills 管理面板                    │
│  /skill install       安装 skill                              │
│  /skill audit         审计 skill 安全性                       │
│  /skill enable        启用 skill                              │
│  /skill disable       禁用 skill                              │
│                                                               │
│  ──── 其他命令 ────                                           │
│  /spec                Spec 面板                               │
│  /goal new            创建新目标                              │
│  /workflows           SOP 工作流列表                          │
│  /workflow run         执行工作流                              │
│  /mcp                 MCP 服务器管理                          │
│  /git                 Git/Worktree 管理                       │
│  /agents              Agent 团队编排                          │
│  /agent spawn         创建子 agent                            │
│  /agent delegate      委托任务                                │
│  /bg                  后台任务列表                            │
│  /cost                成本追踪                                │
│  /config              运行时配置                              │
│  /help                快捷键帮助                              │
│                                                               │
│  [Enter] 执行  [Tab] 自动补全  [Esc] 取消                    │
└──────────────────────────────────────────────────────────────┘
```

### 3.3 Agent 编排策略：如何组织 Agent、安排任务

#### 3.3.1 编排模型

```
用户目标到执行的完整链路（公理 A3: context window 为工作单元）：

  用户输入目标
       │
       ▼
  ┌──────────────────┐
  │  Goal 创建       │  goals/engine.rs: GoalEngine
  │  优先级分配       │  GoalPriority: Low/Medium/High/Critical
  └────────┬─────────┘
           │
           ▼
  ┌──────────────────────────────────────────────────────────────┐
  │  三维复杂度评估 (A3)                                         │
  │  estimate_budget() → 三维评估                                │
  │                                                              │
  │  D1 容量: 文件数 × 大小 + prompt + 工具 + Skill (A2)         │
  │  D2 知识: 模型自评熟悉度 → 需要研究? → 研究阶段预算          │
  │  D3 环境: 涉及外部服务/硬件? → 需要 demo? → 验证阶段预算     │
  │                                                              │
  │  总预算 = D1(执行) + D2(研究) + D3(验证) + 25% 预留          │
  └────────┬─────────────────────────────────────────────────────┘
           │
           ▼
  ┌──────────────────┐
  │  Step 分解       │  goals/engine.rs: select_next_actionable()
  │  依赖分析        │  判断哪些 step 可以并行
  └────────┬─────────┘
           │
           ├──── ≤ 0.75 窗口 ──▶ Single topology (直接执行, with D2/D3 check)
           │                     一个 agent, 一个 context window
           │
           ├──── 0.75–1.5 窗口 ─▶ LeadSubagent (拆为 2 个 context，降低单窗口压力)
           │                     拆为 2 个独立 context window
           │
           └──── > 1.5 窗口 ───▶ StarTeam / MeshTeam
                                  拆为 N 个独立 context window
                                  子任务之间的依赖关系决定拓扑:
                                  │ 无依赖 → StarTeam (并行)
                                  │ 有序 → LeadSubagent (pipeline)
                                  │ 双向 → MeshTeam (协作)
                                  │
                                  ▼
                 ┌──────────────────────┐
                 │  Topology 选择       │  team_orchestration.rs
                 │  预算估算            │  token/cost/quality 三维评估
                 │  Agent 分配          │  delegate.rs: AgentSelectionPolicy
                 └──────────┬───────────┘
                            │
                 ┌──────────┴───────────┐
                 │                      │
                 ▼                      ▼
            ┌─────────┐          ┌─────────┐
            │ Agent A  │  ◄──────│ Agent B  │  coordination bus
            │ (review) │  ──────▶│ (impl)   │  消息传递
            └────┬────┘          └────┬────┘
                 │                     │
                 ▼                     ▼
            ┌─────────┐          ┌─────────┐
            │ BG Job   │          │ BG Job   │  bg_run.rs
            │ (test)   │          │ (build)  │  异步执行
            └─────────┘          └─────────┘
```

[*] 0.75 系数预留 25% 给反射/重试/审批交互。这是经验值，可通过 `config.context_reserve_ratio` 调整。

#### 3.3.2 TUI 中的任务分配决策树

用户在 TUI 中定义目标后，系统提供编排建议：

```
┌─ 任务编排建议 ────────────────────────────────────────────────┐
│                                                               │
│  目标: "重构认证模块，添加 JWT 刷新 token 支持"               │
│                                                               │
│  三维复杂度分析 (A3):                                         │
│                                                               │
│  D1 上下文容量:                                                │
│  • 涉及文件: ~8 个 (auth/, tests/, config/)                   │
│  • 文件读取: ~12k tokens                                      │
│  • Skill 注入: ~3k tokens (code-review, test-driven-dev)      │
│  • 工具调用: ~8k tokens (预估 15 次)                          │
│  • Prompt 开销: ~5k tokens                                    │
│  • D1 小计: ~28k tokens                                       │
│                                                               │
│  D2 知识边界:                                  熟悉度: 高      │
│  • JWT refresh token: 训练语料充分覆盖          ✓ 无需研究     │
│                                                               │
│  D3 环境不确定性:                              可控性: 高      │
│  • 纯代码重构，无外部服务依赖                   ✓ 无需 demo    │
│  ─────────────────────────────────────────────                │
│  总预算: ~28k (D1) + 0 (D2) + 0 (D3) + 25% 预留 = ~35k      │
│  窗口容量: 200k │ 有效容量(×0.75): 150k │ 需要: ~35k         │
│  判定: ≤ 0.75 窗口 → 可单 agent 完成                          │
│  可并行度: 高 (review + impl 可拆为独立 context)              │
│                                                               │
│  推荐拓扑: LeadSubagent (拆分可降低单窗口压力+提高质量)       │
│  ─────────                                                    │
│  Lead: 规划 + 协调 + 最终审查 (claude-4, ~15k ctx)            │
│  Worker: 代码实现 (claude-4-sonnet, ~20k ctx, 更快/更便宜)    │
│                                                               │
│  预估:                                                         │
│  • Tokens: ~35,000 (含协调开销)                               │
│  • 成本: ~$1.20                                               │
│  • 质量: pass_rate 0.84                                       │
│                                                               │
│  步骤:                                                         │
│  1. [Lead] 分析现有认证代码结构         (~5k ctx)              │
│  2. [Lead] 设计 JWT 刷新 token 方案     (~8k ctx)              │
│  3. [Worker] 实现 token 刷新逻辑        (~12k ctx)             │
│  4. [Worker] 添加测试用例               (~8k ctx)              │
│  5. [Lead] 代码审查 + 集成测试          (~10k ctx)             │
│                                                               │
│  ► 接受推荐  │  调整拓扑  │  手动编排  │  取消                 │
└──────────────────────────────────────────────────────────────┘
```

#### 3.3.3 运行时 Agent 管理

```
Agent 生命周期管理：

  创建                    执行                     清理
  ─────                   ────                     ────
  /agent spawn            agent 自主执行            自动回收
  └─ 选择模型              ├─ TUI 实时显示输出       └─ 结果汇总到 lead
  └─ 分配任务              ├─ BG jobs 并行            └─ worktree 清理
  └─ 设置预算              ├─ 审批流程中断              └─ 成本统计
  └─ 选择 worktree         └─ 消息总线通信
```

关键管理操作：
- **实时干预**: 向正在执行的 agent 发送消息修改方向
- **暂停/恢复**: 暂停某个 agent 等待人类决策
- **预算控制**: 某个 agent 接近预算时自动暂停 + 通知
- **召回**: 中止 agent 并保留已有产出
- **复制**: 基于某个 agent 的上下文 fork 新 agent

### 3.4 事件类型扩展路径

```
当前 AgentEvent (3)                    目标 AgentEvent (~20)
├── ToolStart                          ├── ToolStart
├── ToolComplete                       ├── ToolComplete
└── Usage                              ├── Usage
                                       │
                                       ├── TaskPlanCreated { steps }
                                       ├── StepStarted { step_id }
                                       ├── StepCompleted { step_id, outcome }
                                       │
                                       ├── FileChanged { path, change_type }
                                       ├── DiffReady { path, hunks }
                                       │
                                       ├── ApprovalNeeded { request }
                                       ├── ApprovalResolved { id, decision }
                                       │
                                       ├── SubAgentSpawned { id, task, layer }
                                       ├── SubAgentCompleted { id, result }
                                       │
                                       ├── ContextPressure { used, limit }
                                       │
                                       ├── ResearchNeeded { topic, familiarity }
                                       ├── DemoVerification { desc, status }
                                       │
                                       ├── SpecCreated { spec_id, title }
                                       ├── SpecUpdated { spec_id, changes }
                                       ├── SpecTaskChanged { spec_id, task_id, status }
                                       ├── AgentLayerChanged { agent_id, layer }
                                       └── BgAgentNotification { agent_id, msg }
```

[!] 设计原则：不一次性加 40+ 事件类型（codex 路线），而是按用户价值递增添加。每个事件类型都必须有对应的 TUI 可视化消费者。

[i] 事件变更说明：
- 事件总数从 ~15 → ~20（+SpecCreated, SpecUpdated, SpecTaskChanged, AgentLayerChanged, BgAgentNotification, ResearchNeeded, DemoVerification）
- `FileChanged` event 增加 `agent_id` 字段（per-agent change attribution）
- `SubAgentSpawned` event 增加 `layer` 字段（标识 Coordinator/Specialist/Background）

### 3.5 布局演进

#### 3.5.1 自适应布局策略

布局不再固定——由终端宽度 × 全局模式自动选择：

```
终端宽度            布局模式           适用场景
─────────          ─────────         ──────────
< 100 cols         Single            Tab 切换，移动终端/tmux 窄窗格
100-159 cols       TwoColumn         Chat 30% + Spec 70%
160+ cols          ThreeColumn       Chat 18% + Spec 50% + Agents 32%
```

#### 3.5.2 Phase 演进 × 布局模式

```
Phase 1: 当前 (chatbot) — 不变
┌────────────────────────────────────────┐
│              Chat Panel                │  ← 占据 90% 空间
├────────────────────────────────────────┤
│           Tool Progress                │  ← 可选，7 行
├────────────────────────────────────────┤
│             Input Box                  │
├────────────────────────────────────────┤
│            Status Bar                  │
└────────────────────────────────────────┘

Phase 2: Spec 中央面板 (Chat 降为左栏)

  TwoColumn (100-159 cols):
  ┌─────────────┬──────────────────────────┐
  │             │                          │
  │  Chat       │      Spec Panel          │
  │  (30%)      │  ┌ Tasks ──────────────┐ │
  │             │  │ Step 1 [✓]          │ │
  │             │  │ Step 2 [~]          │ │
  │             │  │ Step 3 [>]          │ │
  │             │  └─────────────────────┘ │
  │             │  ┌ Changes ────────────┐ │
  │             │  │ M src/auth.rs +12-3 │ │
  │             │  └─────────────────────┘ │
  ├─────────────┴──────────────────────────┤
  │ > input                     ctx: 62%   │
  ├────────────────────────────────────────┤
  │ provider=openrouter | cost=$0.12 | ... │
  └────────────────────────────────────────┘

  Single (< 100 cols): Tab 切换 [Chat] [Spec]

Phase 3: 三栏 + Agent Tabs + Background 区域

  ThreeColumn (160+ cols):
  ┌──────────┬────────────────────────┬─────────────────┐
  │          │                        │  Agents         │
  │  Chat    │      Spec Panel        │  ┌ Lead [~] ──┐ │
  │  (18%)   │      (50%)             │  │ tokens: 12k│ │
  │          │                        │  └────────────┘ │
  │          │  ┌ Tasks ──────────┐   │  ┌ Agent-1 ───┐ │
  │          │  │ Step 1 [✓]     │   │  │ reviewer   │ │
  │          │  │ Step 2 [~]     │   │  │ [~] active │ │
  │          │  └────────────────┘   │  └────────────┘ │
  │          │  ┌ Changes ───────┐   │  ┌ BG ─────────┐│
  │          │  │ M auth.rs      │   │  │ lint   [✓]  ││
  │          │  │ A tests.rs     │   │  │ test   [~]  ││
  │          │  └────────────────┘   │  └─────────────┘│
  ├──────────┴────────────────────────┴─────────────────┤
  │ > message to: [Lead ▼]                              │
  ├─────────────────────────────────────────────────────┤
  │ total cost=$0.45 | agents=3 | ctx=62%               │
  └─────────────────────────────────────────────────────┘

  TwoColumn (100-159 cols): Chat + Spec, Agent Tab 内嵌
  Single (< 100 cols): Tab 切换 [Chat] [Spec] [Agents]

Daemon 模式 (Phase 4):
┌────────────────────────────────────────┐
│ [SOP Runs] [Triggers] [Audit Log]      │  ← Daemon Tab
├──────────────────────┬─────────────────┤
│  SOP Run List        │  Trigger Monitor│
│  ├ deploy-prod [✓]   │  cron: 0 */6   │
│  ├ sensor-alert [~]  │  mqtt: connected│
│  └ backup [>]        │  webhook: idle  │
├──────────────────────┴─────────────────┤
│ cost today: $2.30 / $10.00 limit       │
├────────────────────────────────────────┤
│ daemon uptime: 14h 32m | next: 02:00   │
└────────────────────────────────────────┘
```

### 3.6 Use Case 信息流图

9 个核心 use case 的信息流和状态流转：

#### UC1: 简单问答 (无 Spec)

```
用户输入 ──▶ Coordinator ──▶ Provider ──▶ Delta(文本) ──▶ Chat Panel
```

- 布局不变（Single/Phase 1 模式）
- 不创建 Spec
- 全局模式保持 `ChatOnly`

#### UC2: 单 Agent 任务 (Spec 出现)

```
用户输入 "重构 auth 模块"
    │
    ▼
Coordinator 决定分解
    │
    ├── SpecCreated event ──▶ TUI 布局切换为 Chat + Spec
    │
    ▼
逐步执行:
    TaskPlanCreated { steps: [1, 2, ...] }
    │
    StepStarted { step_id: 1 }
    │ ├── ToolStart { shell: "cargo test" }
    │ ├── ToolComplete { status: success }
    │ ├── FileChanged { path: "src/auth.rs", agent_id: "coordinator" }
    │ ├── DiffReady { path: "src/auth.rs", hunks: [...] }
    │ └── SpecTaskChanged { task_id: 1, status: Completed }
    │
    StepStarted { step_id: 2 }
    │ └── ...
    ▼
StepCompleted (全部) ──▶ Usage { total_tokens, cost } ──▶ 全局模式回退为 ChatOnly
```

#### UC3: 多 Agent 协作 (Spec + Specialist + Background)

```
Coordinator
    │
    ├── SpecCreated ──▶ 选择 Topology ──▶ 派遣 Specialists
    │
    ├── SubAgentSpawned { id: "reviewer", layer: Specialist }
    ├── SubAgentSpawned { id: "implementer", layer: Specialist }
    ├── SubAgentSpawned { id: "lint-bot", layer: Background, trigger: FileChange }
    │
    ▼
并行执行:
    ┌─────────────────────┬─────────────────────┬──────────────────┐
    │ Specialist:reviewer │ Specialist:impl     │ Background:lint  │
    │ 审查现有代码         │ 实现新功能            │ 自动触发 lint    │
    │ → SpecTask 1 更新   │ → SpecTask 2,3 更新  │ → 静默通知       │
    │ → SpecUpdated       │ → AgentLayerChanged │ → AgentLayerChanged │
    └─────────────────────┴─────────────────────┴──────────────────┘
                                │
                                ▼
                    Per-agent change attribution:
                    FileChanged { path, agent_id: "implementer" }
                    SpecUpdated { spec_id, changes: [...] }
                                │
                                ▼
                    SubAgentCompleted { id: "reviewer", result: ok }
                    SubAgentCompleted { id: "implementer", result: ok }
```

#### UC4: SOP Workflow 触发

```
外部事件 (Cron/MQTT/Webhook/Peripheral)
    │
    ▼
SopEngine 匹配 ──▶ 按 execution mode 分流:
    │
    ├── Auto: 后台执行，Spec 静默更新
    │   └── SOP steps 自动推进，TUI 只更新进度条
    │
    ├── Supervised: 启动前审批
    │   └── ApprovalNeeded { context: "SOP: deploy-prod" }
    │       └── 用户 approve ──▶ 全部 steps 自动执行
    │
    └── StepByStep: 每步审批
        └── Step N 完成 ──▶ ApprovalNeeded { context: "Step N+1: ..." }
            └── 支持 [M] 运行时切换为 Auto 模式
```

#### UC5: 分层审批流

```
审批策略分层:

Layer 1: 白名单放行
    SecurityPolicy.whitelist ──▶ 自动执行 + 事后摘要

Layer 2: 常规 overlay (Y/N/E/A)
    文件修改、Shell 命令 ──▶ Approval Overlay
    ┌─────────────────────────────────────┐
    │ shell: rm -rf target/              │
    │                                     │
    │ [Y] approve  [N] deny  [E] edit    │
    │ [A] always-allow this pattern      │
    └─────────────────────────────────────┘

Layer 3: 破坏性二次确认
    DROP TABLE / force push ──▶ Overlay + "Type YES to confirm"

多 Agent 审批:
    审批队列 FIFO，suspend 请求者不阻塞他人:
    ┌─────────────────────────────────────┐
    │ Approval Queue (2 pending)          │
    │ 1. [reviewer] shell: cargo clippy   │
    │ 2. [impl] write: src/auth.rs        │
    │                                     │
    │ [Y] approve  [N] deny  [S] skip    │
    └─────────────────────────────────────┘

    用户决策后 ──▶ ApprovalResolved { id, decision: approve/deny }
SOP 审批:
    审批对象是 step 不是 tool call
```

#### UC6: Context Pressure 流

```
四级阈值:

  0%        50%        75%        90%       100%
  │──Green──│──Yellow──│──Orange──│───Red───│
  │         │          │          │         │
  │ 正常    │ 状态栏   │ 自动压缩 │ 用户决策│
  │         │ 变色提醒 │ compact  │ 暂停+选 │
  │         │          │ history  │ 择策略  │

多 Agent 预算分配:
    Coordinator:     40% of total
    Specialist pool: 50% of total
    Background:      10% of total

单 Agent 耗尽:
    suspend agent ──▶ Coordinator 通知 ──▶ 从其他 pool 借用
```

#### UC7: 错误/恢复流

```
Agent 执行失败:
    重试 (MAX_STEP_ATTEMPTS=3)
        │ 失败
        ▼
    反射 (build_reflection_prompt)
        │ 仍失败
        ▼
    用户干预 (ApprovalNeeded + 错误上下文)

Provider 连接失败:
    重试 (3次 exponential backoff)
        │ 失败
        ▼
    Fallback provider (弹性路由)
        │ 无可用
        ▼
    降级模型 (如 claude-4 → claude-4-sonnet)
        │ 仍失败
        ▼
    用户通知 (ContextPressure event)

MCP 断连:
    自动重连 (3次)
        │ 失败
        ▼
    标记 offline ──▶ agent 跳过该工具 ──▶ TUI 显示 MCP 状态变更
```

#### UC8: 24小时 Daemon 模式

```
外部触发:
    Cron ─────────┐
    MQTT ─────────┤
    Webhook ──────┼──▶ SopEngine 匹配 ──▶ 执行
    Peripheral ───┘

TUI 可选:
    headless:     日志 + 通知 (无 TUI 进程)
                  BgAgentNotification { agent_id, msg } ──▶ 外部通知渠道
    tui --daemon: SOP Runs 列表 + Trigger 监控 + Audit Log

审批超时:
    config.approval_timeout_action:
    ├── auto_approve  (适用于低风险 SOP)
    ├── skip          (跳过当前 step)
    └── cancel        (取消整个 SOP run)

成本控制:
    daily_cost_limit: $10.00
    达到上限 ──▶ 全部 SOP 暂停 ──▶ 通知 + 等待次日重置
```

#### UC9: 会话恢复流

```
场景 A — TUI 重连 daemon (进程仍在):
    TUI 启动 ──▶ 请求状态快照 ──▶ 重建 TuiState ──▶ 订阅事件流
    (无数据丢失，秒级恢复)

场景 B — daemon 完全重启:
    L1(goals.json) + L2(spec.json) ──▶ 重建 WorkbenchState
    ──▶ 提示用户选择: [R] 恢复上次任务 / [N] 新建会话

场景 C — 跨天续做:
    同 B + 额外检查:
    ├── git status 检查冲突 (其他人可能已 push)
    ├── MCP 重连 (服务器可能已重启)
    └── 成本计数器重置 (daily_cost_limit)
```

### 3.7 全局状态机

TUI 全局模式自动流转，驱动布局选择：

```
ChatOnly ──(SpecCreated)──▶ SpecMode ──(SubAgentSpawned)──▶ MultiAgent
    ▲                                                          │
    │                                                          │
    └────────────────(all SpecTasks completed)──────────────────┘

    Any ──(--daemon / SopTrigger)──▶ Daemon ──(daemon stop)──▶ ChatOnly
```

- `all SpecTasks completed` 是派生条件（所有 SpecTaskChanged 状态 = Done），不是独立事件。
- `Daemon` 状态可从任意模式通过 `tui --daemon` 或 SOP trigger 进入（参见 UC8 / Phase 4）。

布局模式自动选择矩阵：

```
                    ChatOnly        SpecMode        MultiAgent      Daemon
                    ────────        ────────        ──────────      ──────
< 100 cols          Single          Single(Tab)     Single(Tab)     Single(Tab)
100-159 cols        Single          TwoColumn       TwoColumn       TwoColumn
160+ cols           Single          TwoColumn       ThreeColumn     ThreeColumn
```

[*] 全局模式由事件驱动，不需要用户手动切换。渐进暴露原则的直接体现——简单问答时用户看到的和 chatbot 一样简洁，复杂任务时面板自动展开。

---

## 4. 功能路线图 (Phase 分期)

### Phase 1: Foundation — 可见性 + 命令入口 (4-6 周)

**目标**：让用户"看到" agent 在做什么，并建立 `/` 命令入口基础设施。

| 功能 | 价值 | 后端对接点 |
|------|------|-----------|
| 结构化工具面板 | 看到工具调用详情 | 扩展 `ToolStart`/`ToolComplete` |
| Context 预算条 | 实时 token 用量 | `Usage` 事件 + `CostTracker` |
| 文件变更摘要 | 文件修改可见 | 新增 `FileChanged` 事件 |
| 增强 Approval overlay | 审批有上下文 | `ApprovalNeeded` + `SecurityPolicy` |
| `/` 命令框架 | 管理入口统一 | 模糊匹配 + 命令注册系统 |
| `/mcp` MCP 状态 | 看到 MCP 服务器连接状况 | `McpServer` 列表 + 状态查询 |
| `/bg` 后台任务 | 看到 BG jobs | `BgJobStore` 查询 |
| Markdown 基础渲染 | 代码块高亮 | 最小 markdown parser |

**架构变更**：
- 事件类型从 3 → ~8
- 新增 `CommandRegistry` + `CommandParser` (TUI 层，不耦合 agent)
- Status bar 从纯文本升级为结构化：provider | model | tokens | cost | status

### Phase 2: Task-Aware — 任务管理 + Skills/Git 集成 (6-8 周)

**目标**：TUI 成为任务驱动的工作台，可管理 goals、skills、git worktree。

| 功能 | 价值 | 后端对接点 |
|------|------|-----------|
| Spec 面板 | 可视化目标、进度和变更 | `GoalEngine` + `TaskPlanTool` + Spec view-model |
| Step 进度追踪 | 步骤级可见性 | `StepStarted`/`StepCompleted` 事件 |
| `/skills` 管理 | 查看/启用/禁用/审计 skills | `skills::load_skills`, `audit_skill_directory` |
| `/git` worktree 管理 | worktree 列表/创建/diff | `GitOperationsTool` |
| Diff 预览 widget | 文件修改对比 | git diff 解析 + 着色渲染 |
| 审批队列 | 多项批量处理 | `VecDeque<ApprovalItem>` |
| 快捷键体系 | Tab/j/k/y/n | 键盘映射系统 |
| 面板切换 | [Chat] [Spec] [Skills] [Git] | `WorkbenchFocus` enum |

**数据结构变更**：
```rust
// 新增到 TuiState
pub goals: Vec<GoalView>,           // 从 GoalEngine 映射
pub task_plan: Vec<TaskStepView>,   // 从 TaskPlanTool 映射
pub skills: Vec<SkillView>,         // 从 skills::load 映射
pub git_status: GitStatusView,      // 从 GitOperationsTool 映射
pub file_changes: Vec<FileChange>,
pub pending_approvals: VecDeque<ApprovalItem>,
pub focus: WorkbenchFocus,
```

### Phase 3: Orchestration — Agent 编排 + Workflow (8-12 周)

**目标**：用户在 TUI 中直接编排 agent 团队、管理 SOP workflow、跨 agent 协作。

| 功能 | 价值 | 后端对接点 |
|------|------|-----------|
| Agent 标签页 | sub-agent 独立视图 | `SubAgentSpawnTool` + events |
| 拓扑选择器 | 可视化选择 team 拓扑 | `team_orchestration.rs` |
| Agent 预算控制 | 每个 agent 成本可见 | `CostTracker` per agent |
| `/workflows` SOP 管理 | SOP 列表/执行/审批 | `SopEngine` + `SopExecutionMode` |
| 跨 agent 消息 | 向特定 agent 发指令 | `InMemoryMessageBus` + `agents_ipc` |
| Agent 负载视图 | 实时负载均衡状态 | `AgentLoadTracker` |
| BG Jobs 关联 | job 归属到 agent | `BgJobStore` + agent_id |
| 三层 Agent 生命周期 | Coordinator/Specialist/Background 层事件可视化 | `AgentLayerChanged` event + `SubAgentSpawned { layer }` |
| 汇总视图 | 多 agent 产出聚合 | coordination context |

**关键前提**（全部已有后端实现）：
- `src/agent/team_orchestration.rs` — TeamTopology + 预算估算
- `src/tools/subagent_spawn.rs` / `delegate.rs` — agent 创建和委托
- `src/sop/` — SopEngine + 多执行模式
- `src/coordination/mod.rs` — InMemoryMessageBus
- `src/tools/agents_ipc.rs` — 跨 agent 消息

### Phase 4: Intelligence — 自适应 + 持久化 (12+ 周)

| 功能 | 价值 | 后端对接点 |
|------|------|-----------|
| 编排建议器 | 根据任务自动推荐拓扑 | `team_orchestration::recommend` |
| Context 自动压缩 | 接近限制时提醒 | `auto_compact_history` |
| 会话持久化/恢复 | 中断后续做 | goals.json + spec.json + session serialize |
| 外部看板同步 (可选) | Goals 同步到外部看板 | MCP 工具调用 (非核心依赖) |
| 自定义面板布局 | 用户配置 | config schema 扩展 |
| SOP 触发监控 | 实时显示 trigger 匹配 | SopTrigger (Mqtt/Webhook/Cron/Peripheral) |
| 性能仪表盘 | 延迟/成本/成功率 | `ObserverMetric` 聚合 |

---

## 5. 关键设计决策

### 5.1 为什么不做协议层分离？

```
┌───────────────────────────────────────────────────────────┐
│ codex 路线: 协议解耦                                       │
│                                                           │
│ Agent Server ──(codex_protocol)──▶ TUI Client             │
│                                  ──▶ Web Client           │
│                                  ──▶ VSCode Extension     │
│                                                           │
│ 优势: 多客户端、协议版本化                                  │
│ 代价: ~10k+ 行额外代码、独立 crate、启动延迟               │
│ 前提: 需要多客户端                                         │
└───────────────────────────────────────────────────────────┘

┌───────────────────────────────────────────────────────────┐
│ clawclawclaw 路线: 事件总线扩展                             │
│                                                           │
│ Agent Loop ──(AgentEvent channel)──▶ TUI                   │
│                                                           │
│ 优势: 零额外延迟、单体可审计、二进制小                       │
│ 代价: 只支持 TUI 一个前端                                   │
│ 前提: 终端优先是产品选择                                     │
└───────────────────────────────────────────────────────────┘
```

[*] 决策：保持单体架构 + 事件总线扩展。当且仅当出现第二个前端需求时（Web UI / VSCode），再考虑协议层提取。这符合 YAGNI 原则。

### 5.2 为什么事件类型控制在 ~20 而不是 40+？

每个事件类型都有维护成本：
1. agent loop 发送逻辑
2. TUI 接收 + 状态更新
3. widget 渲染
4. 测试覆盖

codex 的 40+ 事件类型中，很多是为 Web 客户端设计的（如 `ImageGenerationBegin`、`McpToolCallEnd`）。clawclawclaw 在终端场景下，~20 个高价值事件已覆盖 95% 用户需求。

### 5.3 Approval 设计哲学

```
aider 模式:    全自动执行 → git commit → 用户事后 undo
Claude Code:   关键操作前暂停 → 用户 approve → 执行
codex 模式:    全屏 overlay → 详细上下文 → approve/deny/edit
OpenHands:     沙箱隔离 → 事后审查

clawclawclaw:  分层审批策略
├── 安全策略白名单内   → 自动执行 + 事后摘要
├── 文件修改           → 显示 diff + approve
├── Shell 命令         → 显示命令 + approve
└── 破坏性操作         → 强制 approve + 确认
```

[*] 复用已有 `SecurityPolicy` + `ApprovalManager`。TUI 层只负责渲染上下文和收集决策，不负责策略判断。

---

## 6. 竞品深度对比

### 6.1 交互范式比较

```
┌─────────────┬───────────────────────┬──────────────────────────────┐
│   产品       │      交互范式         │      用户角色                │
├─────────────┼───────────────────────┼──────────────────────────────┤
│ ChatGPT     │ 对话式 (turn-based)   │ 提问者                      │
│ aider       │ 对话+自动执行         │ pair programmer              │
│ Claude Code │ 指令式+审批           │ 指挥官 (轻量)                │
│ codex       │ 任务式+多线程         │ 项目经理                     │
│ OpenHands   │ 任务式+沙箱           │ 外部审查者                   │
│ Devin       │ 全自治+进度报告       │ 项目 owner                   │
│ WaveTerm    │ 终端+agent编排        │ 运维工程师                   │
├─────────────┼───────────────────────┼──────────────────────────────┤
│ claw (目标)  │ 目标式+分层审批+团队  │ 指挥官 (终端原生)            │
└─────────────┴───────────────────────┴──────────────────────────────┘
```

### 6.2 功能矩阵（执行面 + 管理面）

**执行面**（agent 做事时用户看到什么）：

| 功能 | aider | Claude Code | codex | OpenHands | WaveTerm | claw 当前 | claw 目标 |
|------|-------|-------------|-------|-----------|----------|-----------|-----------|
| Diff 预览 | git diff | 终端内联 | 专用渲染 | Web diff | 终端内联 | 无 | Phase 2 |
| 任务分解 | 无 | 隐式 | 显式 | plan agent | 隐式 | 无 | Phase 2 |
| 并行执行 | 无 | sub-agent | ThreadMgr | 多 agent | agent tab | 后端有 | Phase 3 |
| 审批上下文 | 自动提交 | 命令预览 | 全屏 overlay | 事后审查 | 命令预览 | 纯 Y/N | Phase 1 |
| Context 管理 | 手动 /tokens | 自动 | 自动+展示 | 自动 | 自动 | 无可见 | Phase 1 |
| 终端原生 | 是 | 是 | 是 | 否 (Web) | 是 (Electron) | 是 | 是 |

**管理面**（用户如何组织和编排 agent 工作）：

| 功能 | aider | Claude Code | codex | OpenHands | WaveTerm | claw 当前 | claw 目标 |
|------|-------|-------------|-------|-----------|----------|-----------|-----------|
| Skills 管理 | 无 | 无原生 UI | 无 | 无 | 无 | 后端有 | Phase 2 |
| Workflow/SOP | 无 | 无 | 无 | workflow yaml | 无 | 后端有 | Phase 3 |
| MCP 管理 | 无 | 配置文件 | 无 | 无 | 无 | 后端有 | Phase 1 |
| Spec/Goal | 无 | TodoWrite | 无 | task manager | agent spec | 后端有 | Phase 2 |
| Git Worktree | git 集成 | git 集成 | sandbox | Docker | git 集成 | 后端有 | Phase 2 |
| Agent 编排 | 无 | sub-agent | 多线程 | multi-agent | agent tab | 后端有 | Phase 3 |
| 拓扑选择 | 无 | 无 | 无 | 无 | 无 | 后端有 | Phase 3 |
| BG Jobs | 无 | 后台 agent | 无 | 异步任务 | 后台任务 | 后端有 | Phase 1 |
| 成本控制 | 无 | 内置 | 无 | 无 | 无 | 后端有 | Phase 1 |
| 硬件外设 | 否 | 否 | 否 | 否 | 否 | 后端有 | Phase 4 |
| 会话持久化 | 有 | 有 | 有 | 有 | 有 | 无 | Phase 4 |
| 外部看板同步 | 无 | 无 | 无 | 无 | 无 | MCP 可达 | Phase 4 (可选) |

[*] 管理面是 clawclawclaw 最大的差异化机会。**没有任何竞品在终端内提供 skills/workflow/agent-topology/SOP 的原生管理 UI。** 这些能力在后端都已实现，TUI 只需把它们暴露出来。

### 6.3 独特优势 (护城河)

clawclawclaw 有五个竞品不具备的优势：

1. **硬件外设集成** — `src/peripherals/` 支持 STM32/RPi GPIO，TUI 可以成为嵌入式开发的 agent 终端
2. **多 Provider 路由** — 内置弹性路由 + 故障转移，不绑定单一 AI 提供商
3. **安全策略引擎** — `SecurityPolicy` + canary tokens + 细粒度权限，适合企业/安全场景
4. **SOP 工作流引擎** — 完整的 trigger/execution-mode/audit 系统，竞品没有等价物
5. **Agent 拓扑编排** — 基于预算的团队拓扑推荐 (Single/LeadSubagent/Star/Mesh)，不是简单的 "spawn sub-agent"

TUI workbench 的使命是把这 5 个后端护城河变成用户可操作的前端体验。

---

## 7. 实现优先级与依赖关系

```
Phase 1: Foundation              Phase 2: Task+Manage           Phase 3: Orchestration
(可见性 + 命令入口)               (任务 + 资源管理)              (Agent 编排 + Workflow)

┌─────────────┐                 ┌─────────────┐                ┌─────────────┐
│ / 命令框架   │────────────────▶│ /skills 管理 │                │ /workflows  │
│ CommandReg   │                │ /git 管理    │                │  SOP 管理   │
└──────┬──────┘                 └──────┬──────┘                └──────┬──────┘
       │                               │                              │
       ▼                               ▼                              ▼
┌─────────────┐                 ┌─────────────┐                ┌─────────────┐
│ 结构化工具   │────────────────▶│ Spec         │────────────────▶│ Agent Tab   │
│ 面板         │                │ 看板面板     │                │ 团队视图    │
└──────┬──────┘                 └──────┬──────┘                └──────┬──────┘
       │                               │                              │
       ▼                               ▼                              ▼
┌─────────────┐                 ┌─────────────┐                ┌─────────────┐
│ /mcp 状态    │                │ Diff 预览    │                │ 拓扑选择器  │
│ /bg 任务     │                │ widget       │                │ 负载均衡    │
└──────┬──────┘                 └──────┬──────┘                └──────┬──────┘
       │                               │                              │
       ▼                               ▼                              ▼
┌─────────────┐                 ┌─────────────┐                ┌─────────────┐
│ Context 预算 │                │ 审批队列     │                │ 跨 agent    │
│ 增强 Approval│────────────────▶│ 面板切换     │────────────────▶│ 消息/IPC    │
└─────────────┘                 └─────────────┘                └─────────────┘

关键依赖链：
1. Phase 1 的 / 命令框架 → Phase 2-3 所有管理命令的注册基础
2. Phase 1 的结构化工具面板 → Phase 2 Goals 面板的渲染基础
3. Phase 1 的增强 Approval → Phase 2 审批队列 → Phase 3 SOP step-by-step 审批
4. Phase 2 的面板切换 (WorkbenchFocus) → Phase 3 Agent Tab 切换
5. Phase 2 的 /git worktree → Phase 3 agent 隔离执行环境
```

---

## 8. 技术约束与风险

### 8.1 必须遵守的约束

| 约束 | 来源 | 影响 |
|------|------|------|
| `state.rs` 零 agent 耦合 | `src/tui/CLAUDE.md` | 所有 agent 数据通过事件通道传递 |
| `panic = "abort"` | Cargo.toml release | 不能依赖 Drop 做清理 |
| 二进制大小敏感 | 产品目标 | 不引入重量级渲染依赖 |
| 文本消毒 | 安全约束 | 所有新 widget 都过 `sanitize_text()` |
| bounded channel (256) | 背压控制 | 高频事件需考虑丢弃策略 |

### 8.2 风险评估

| 风险 | 概率 | 影响 | 缓解 |
|------|------|------|------|
| 事件通道背压导致 agent 阻塞 | 中 | 高 | 保持 try_send，高频事件做 coalesce |
| TUI 复杂度膨胀失控 | 高 | 中 | 每 Phase 完成后 code review，控制总行数 |
| 多面板布局在小终端下不可用 | 中 | 中 | 自适应布局：<100 列时退化为单面板 |
| 渲染性能：大量工具调用导致卡顿 | 低 | 中 | 虚拟滚动 + 历史事件压缩 |

---

## 9. 成功指标

### 9.1 产品指标

| 指标 | 当前基线 | Phase 1 目标 | Phase 3 目标 |
|------|---------|-------------|-------------|
| 用户每次 session 的 agent 动作可见率 | ~10% (只看到文本流) | 80% (工具+文件+cost) | 95% |
| 审批决策平均耗时 | 高 (缺乏上下文) | 降低 50% | 降低 70% |
| 多文件任务完成效率 | 1x (逐步指导) | 1.5x (任务计划可见) | 3x (并行 agent) |

### 9.2 工程指标

| 指标 | 约束 |
|------|------|
| TUI 核心代码行数 | Phase 1: ≤1500行, Phase 2: ≤3000行, Phase 3: ≤5000行 |
| 新增 crate 依赖 | Phase 1-2: 0, Phase 3: ≤1 |
| AgentEvent 类型数 | Phase 1: ≤8, Phase 2: ≤12, Phase 3: ≤20 |
| 测试覆盖率 | 新增代码 ≥80% |

---

## 10. 总结：从 Chatbot 到 Commander Console

```
clawclawclaw TUI 的演进路径：

  Chatbot Wrapper ──▶ Visible Agent ──▶ Resource Manager ──▶ Commander Console
    (当前)              (Phase 1)         (Phase 2)          (Phase 3-4)

  两条并行演进轴：

  执行面 (看到 agent 做什么)：
  ─────────────────────────────────────────────────────────────────
  文本流 → 结构化工具面板 → 任务步骤追踪 → 多 agent 并行视图

  管理面 (组织 agent 用什么/怎么做)：
  ─────────────────────────────────────────────────────────────────
  无管理 → / 命令 + MCP/BG → Skills/Git/Goals → Workflow/Topology/IPC

  用户角色演进：
  ─────────────────────────────────────────────────────────────────
  提问者 → 审查者 → 资源管理者 → 指挥官

  核心能力暴露：
  ─────────────────────────────────────────────────────────────────
  3/12 子系统 → 8/12 → 11/12 → 12/12 (全部后端能力在 TUI 中可达)
```

**核心设计哲学**：

1. **后端能力全暴露** — TUI 不是另起炉灶，而是把已有的 12 个后端子系统变成可操作的前端体验
2. **终端原生效率** — 不追求 Web GUI 的视觉丰富度，追求键盘操作效率（/ 命令 + 快捷键 + Tab 切换）
3. **管理面 = 差异化** — 执行面（看 agent 干活）竞品都有；管理面（组织 skills/workflow/topology）是独特优势
4. **渐进暴露** — 简单任务时用户看到的和 chatbot 一样简洁；复杂任务时自动展开管理面板

---

## 参考来源

- 内部: `docs/tui-architecture-comparison.md`, `src/tui/` 完整代码
- 内部: `src/agent/team_orchestration.rs`, `src/tools/subagent_*.rs`
- [Claude Code Agent Teams](https://code.claude.com/docs/en/agent-teams)
- [Agent Design Patterns (Lance Martin)](https://rlancemartin.github.io/2026/01/09/agent_design/)
- [Terminal-Based Agent Engineering (SitePoint)](https://www.sitepoint.com/terminal-based-agent-engineering-the--claude-code--workflow/)
- [Top 5 CLI Coding Agents 2026 (DEV)](https://dev.to/lightningdev123/top-5-cli-coding-agents-in-2026-3pia)
- [Aider Edit Formats](https://aider.chat/docs/more/edit-formats.html)
- [Code Surgery: How AI Assistants Edit Files](https://fabianhertwig.com/blog/coding-assistants-file-edits/)
- [OpenHands Platform](https://openhands.dev/)
- [OpenHands Agent SDK (arxiv)](https://arxiv.org/html/2511.03690v1)
- [Building a TUI is Easy Now (Hatchet)](https://hatchet.run/blog/tuis-are-easy-now)
- [LIT-TUI: Terminal Research Platform](https://lit.ai/blog/2025/06/27/lit-tui-a-terminal-research-platform-for-ai-development/)
- [Coding for the Agentic World (Addy Osmani)](https://addyo.substack.com/p/coding-for-the-future-agentic-world)
- [AG-UI Protocol (DataCamp)](https://www.datacamp.com/tutorial/ag-ui)
- [OpenCode TUI Docs](https://opencode.ai/docs/tui/)
- [OpenAgentsControl](https://github.com/darrenhinde/OpenAgentsControl)
- [Terminal-Bench](https://www.tbench.ai/)
