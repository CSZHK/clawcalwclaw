# clawclawclaw 架构设计与工程原则

本文档概述 clawclawclaw 项目的架构设计、关键模块、设计原则和技术约束。

---

## 目录

1. [架构设计总览](#1-架构设计总览)
2. [关键模块详解](#2-关键模块详解)
3. [扩展点汇总](#3-扩展点汇总)
4. [CLI 命令结构](#4-cli-命令结构)
5. [设计灵魂与核心理念](#5-设计灵魂与核心理念)
6. [工程原则](#6-工程原则)
7. [技术约束](#7-技术约束)
8. [反模式](#8-反模式)

---

## 1. 架构设计总览

### 1.1 整体架构模式

clawclawclaw 采用 **Trait + Factory** 架构模式，这是 Rust 生态中实现可扩展性的经典方案：

- **Trait 定义契约** - 每个子系统的接口都是明确的
- **Factory 实现注册** - 运行时根据配置动态选择实现
- **依赖注入** - 组件间松耦合，便于测试和替换

### 1.2 模块关系图

```
                    ┌─────────────────────────────────────────┐
                    │              main.rs                    │
                    │           (CLI 命令入口)                 │
                    └─────────────────────────────────────────┘
                                        │
                                        ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                              lib.rs                                     │
│                      (模块导出 & 命令枚举)                                │
└─────────────────────────────────────────────────────────────────────────┘
                                        │
        ┌───────────────────────────────┼───────────────────────────────┐
        │                               │                               │
        ▼                               ▼                               ▼
┌───────────────┐             ┌───────────────┐               ┌───────────────┐
│    agent/     │◄────────────│   channels/   │──────────────►│   gateway/    │
│  (编排调度)    │             │  (消息通道)    │               │  (Webhook)   │
└───────┬───────┘             └───────┬───────┘               └───────────────┘
        │                             │
        ▼                             ▼
┌───────────────┐             ┌───────────────┐
│  providers/   │             │    memory/    │
│ (LLM 提供者)   │             │  (存储后端)    │
└───────┬───────┘             └───────┬───────┘
        │                             │
        ▼                             ▼
┌───────────────┐             ┌───────────────┐
│    tools/     │◄────────────│   security/   │
│  (工具执行)    │             │  (安全策略)    │
└───────┬───────┘             └───────────────┘
        │
        ▼
┌───────────────┐             ┌───────────────┐
│   runtime/    │             │ peripherals/  │
│  (运行时适配)  │             │  (硬件外设)    │
└───────────────┘             └───────────────┘
```

---

## 2. 关键模块详解

### 2.1 `src/providers/` - LLM 提供者

**职责**: 抽象 LLM API 后端，提供统一接口。

**核心 Trait** (`traits.rs`):

```rust
#[async_trait]
pub trait Provider: Send + Sync {
    // 核心对话
    async fn chat(&self, request: ChatRequest<'_>, model: &str, temperature: f64)
                  -> anyhow::Result<ChatResponse>;

    // 工具调用支持
    async fn chat_with_tools(&self, messages: &[ChatMessage], tools: &[serde_json::Value],
                            model: &str, temperature: f64) -> anyhow::Result<ChatResponse>;

    // 能力声明
    fn supports_native_tools(&self) -> bool;
    fn supports_streaming(&self) -> bool;
    fn supports_vision(&self) -> bool;
}
```

**Factory 注册** (`mod.rs`):

```rust
pub fn create_provider_with_url_and_options(name: &str, api_key: Option<&str>,
                                            api_url: Option<&str>,
                                            options: &ProviderRuntimeOptions)
                                            -> anyhow::Result<Box<dyn Provider>> {
    match name {
        "openrouter" => Ok(Box::new(OpenRouterProvider::new(...))),
        "anthropic" => Ok(Box::new(AnthropicProvider::new(...))),
        "openai" => Ok(Box::new(OpenAiProvider::new(...))),
        "ollama" => Ok(Box::new(OllamaProvider::new(...))),
        "gemini" | "google" => Ok(Box::new(GeminiProvider::new(...))),
        "bedrock" => Ok(Box::new(BedrockProvider::new())),
        // ... 30+ 提供者和别名
    }
}
```

**已实现**: `anthropic`, `openai`, `gemini`, `ollama`, `bedrock`, `openrouter` 及 30+ OpenAI 兼容提供者。

---

### 2.2 `src/channels/` - 消息通道

**职责**: 连接 clawclawclaw 到外部消息平台。

**核心 Trait** (`traits.rs`):

```rust
#[async_trait]
pub trait Channel: Send + Sync {
    fn name(&self) -> &str;

    // 核心消息
    async fn send(&self, message: &SendMessage) -> anyhow::Result<()>;
    async fn listen(&self, tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> anyhow::Result<()>;

    // 可选功能
    async fn health_check(&self) -> bool;
    async fn start_typing(&self, recipient: &str) -> anyhow::Result<()>;
    async fn stop_typing(&self, recipient: &str) -> anyhow::Result<()>;

    // 流式草稿支持
    fn supports_draft_updates(&self) -> bool;
    async fn send_draft(&self, message: &SendMessage) -> anyhow::Result<Option<String>>;
    async fn update_draft(&self, recipient: &str, message_id: &str, text: &str)
                          -> anyhow::Result<Option<String>>;

    // 反应
    async fn add_reaction(&self, channel_id: &str, message_id: &str, emoji: &str)
                          -> anyhow::Result<()>;
}
```

**已实现**: `telegram`, `discord`, `slack`, `whatsapp`, `github`, `matrix`, `imessage`, `email`, `lark`, `signal`, `irc`, `qq`, `nostr`, `mattermost`, `nextcloud_talk`, `bluebubbles`, `wati`, `napcat`, `dingtalk`, `clawdtalk`, `acp`, `linq` (22+ 通道)。

---

### 2.3 `src/tools/` - 工具执行

**职责**: 定义 Agent 的可执行能力。

**核心 Trait** (`traits.rs`):

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> serde_json::Value;

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult>;

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: self.parameters_schema(),
        }
    }
}
```

**工具结果**:

```rust
pub struct ToolResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}
```

**工具类别**:

| 类别 | 工具示例 |
|------|---------|
| **Shell/文件** | `shell`, `file_read`, `file_write`, `file_edit` |
| **网络** | `http_request`, `web_fetch`, `web_search` |
| **浏览器** | `browser`, `browser_open` |
| **记忆** | `memory_store`, `memory_recall`, `memory_list` |
| **调度** | `cron_add`, `cron_list`, `cron_remove` |
| **协作** | `delegate` (子任务委派) |
| **Git** | `git_operations` |
| **搜索** | `glob_search` |
| **文档** | `docx_read` |
| **外部** | `composio` |

---

### 2.4 `src/memory/` - 存储后端

**职责**: Agent 知识的持久化存储和检索。

**核心 Trait** (`traits.rs`):

```rust
#[async_trait]
pub trait Memory: Send + Sync {
    fn name(&self) -> &str;

    async fn store(&self, key: &str, content: &str, category: MemoryCategory,
                   session_id: Option<&str>) -> anyhow::Result<()>;
    async fn recall(&self, query: &str, limit: usize, session_id: Option<&str>)
                    -> anyhow::Result<Vec<MemoryEntry>>;
    async fn get(&self, key: &str) -> anyhow::Result<Option<MemoryEntry>>;
    async fn list(&self, category: Option<&MemoryCategory>, session_id: Option<&str>)
                  -> anyhow::Result<Vec<MemoryEntry>>;
    async fn forget(&self, key: &str) -> anyhow::Result<bool>;
    async fn count(&self) -> anyhow::Result<usize>;
    async fn health_check(&self) -> bool;
    async fn reindex(&self, progress_callback: Option<Box<dyn Fn(usize, usize) + Send + Sync>>)
                     -> anyhow::Result<usize>;
}
```

**记忆分类**:

```rust
pub enum MemoryCategory {
    Core,          // 长期事实、偏好
    Daily,         // 会话日志
    Conversation,  // 对话上下文
    Custom(String),
}
```

**Factory** (`mod.rs`):

```rust
pub fn create_memory(config: &MemoryConfig, workspace_dir: &Path, api_key: Option<&str>)
                     -> anyhow::Result<Box<dyn Memory>> {
    match classify_memory_backend(&backend_name) {
        MemoryBackendKind::Sqlite => Ok(Box::new(sqlite_builder()?)),
        MemoryBackendKind::Lucid => Ok(Box::new(LucidMemory::new(...))),
        MemoryBackendKind::CortexMem => Ok(Box::new(CortexMemMemory::new(...))),
        MemoryBackendKind::Postgres => postgres_builder(),
        MemoryBackendKind::Qdrant => Ok(Box::new(QdrantMemory::new(...))),
        MemoryBackendKind::Markdown => Ok(Box::new(MarkdownMemory::new(...))),
        MemoryBackendKind::None => Ok(Box::new(NoneMemory::new())),
        // ...
    }
}
```

**已实现**: `sqlite`, `markdown`, `qdrant`, `postgres`, `lucid`, `cortex-mem`, `sqlite_qdrant_hybrid`, `none`。

---

### 2.5 `src/security/` - 安全策略

**职责**: 执行自主等级、沙箱隔离和访问控制。

**核心 Trait** (`traits.rs`):

```rust
pub trait Sandbox: Send + Sync {
    fn wrap_command(&self, cmd: &mut Command) -> std::io::Result<()>;
    fn is_available(&self) -> bool;
    fn name(&self) -> &str;
    fn description(&self) -> &str;
}
```

**核心类型**:

| 组件 | 职责 |
|------|------|
| `SecurityPolicy` | 自主等级、工作区边界、访问规则 |
| `AutonomyLevel` | `ReadOnly` / `Supervised` / `Full` |
| `PairingGuard` | 设备配对认证 |
| `SecretStore` | 加密凭证存储 |
| `Sandbox` | OS 级隔离 |

**沙箱实现**: `docker`, `firejail`, `bubblewrap`, `landlock`

**子模块**: `policy`, `pairing`, `secrets`, `audit`, `detect`, `docker`, `firejail`, `bubblewrap`, `landlock`, `otp`, `estop`, `leak_detector`, `prompt_guard`。

---

### 2.6 `src/gateway/` - Webhook 服务器

**职责**: HTTP/WebSocket 服务器，用于 webhook 和外部集成。

**架构**: 基于 Axum 的 HTTP 服务器，包含:
- 请求体限制 (64KB 最大)
- 请求超时 (30s)
- 速率限制
- 幂等键支持

**端点**:
- WhatsApp, GitHub, WATI, Nextcloud Talk, QQ 等 webhook 端点
- OpenAI 兼容 API
- OpenClaw 兼容 API
- WebSocket 支持
- 静态文件服务

---

### 2.7 `src/peripherals/` - 硬件外设

**职责**: 硬件板 (STM32, RPi GPIO) 暴露工具。

**核心 Trait** (`traits.rs`):

```rust
#[async_trait]
pub trait Peripheral: Send + Sync {
    fn name(&self) -> &str;           // 实例名 (如 "nucleo-f401re-0")
    fn board_type(&self) -> &str;     // 板类型 (如 "nucleo-f401re")

    async fn connect(&mut self) -> anyhow::Result<()>;
    async fn disconnect(&mut self) -> anyhow::Result<()>;
    async fn health_check(&self) -> bool;

    fn tools(&self) -> Vec<Box<dyn Tool>>;  // 暴露硬件能力
}
```

**已实现**: `nucleo-f401re`, `rpi-gpio`, `esp32`, `arduino-uno`。

---

### 2.8 `src/runtime/` - 运行时适配器

**职责**: 抽象 Agent 的平台差异。

**核心 Trait** (`traits.rs`):

```rust
pub trait RuntimeAdapter: Send + Sync {
    fn name(&self) -> &str;

    // 能力声明
    fn has_shell_access(&self) -> bool;
    fn has_filesystem_access(&self) -> bool;
    fn supports_long_running(&self) -> bool;
    fn memory_budget(&self) -> u64;

    // 平台特定操作
    fn storage_path(&self) -> PathBuf;
    fn build_shell_command(&self, command: &str, workspace_dir: &Path)
                           -> anyhow::Result<tokio::process::Command>;
}
```

**已实现**: `native`, `docker`, `wasm`。

---

### 2.9 `src/observability/` - 可观测性

**职责**: 遥测和监控。

**核心 Trait** (`traits.rs`):

```rust
pub trait Observer: Send + Sync + 'static {
    fn record_event(&self, event: &ObserverEvent);
    fn record_metric(&self, metric: &ObserverMetric);
    fn flush(&self) {}
    fn name(&self) -> &str;
    fn as_any(&self) -> &dyn Any;
}
```

**事件类型**:

```rust
pub enum ObserverEvent {
    AgentStart { provider: String, model: String },
    LlmRequest { provider: String, model: String, messages_count: usize },
    LlmResponse { provider: String, model: String, duration: Duration, ... },
    ToolCallStart { tool: String },
    ToolCall { tool: String, duration: Duration, success: bool },
    ChannelMessage { channel: String, direction: String },
    HeartbeatTick,
    Error { component: String, message: String },
    // ...
}
```

---

### 2.10 `src/config/` - 配置

**职责**: TOML 配置的 Schema 和加载。

**顶层配置** (`schema.rs`):

```rust
pub struct Config {
    pub workspace_dir: PathBuf,
    pub config_path: PathBuf,
    pub api_key: Option<String>,
    pub api_url: Option<String>,
    pub default_provider: Option<String>,
    pub default_model: Option<String>,
    pub default_temperature: f64,

    // 子系统配置
    pub observability: ObservabilityConfig,
    pub autonomy: AutonomyConfig,
    pub security: SecurityConfig,
    pub runtime: RuntimeConfig,
    pub agent: AgentConfig,
    pub memory: MemoryConfig,
    pub channels_config: ChannelsConfig,
    pub gateway: GatewayConfig,
    pub cron: CronConfig,
    pub heartbeat: HeartbeatConfig,
    pub reliability: ReliabilityConfig,
    // ... 更多
}
```

---

### 2.11 `src/agent/` - 编排调度

**职责**: 核心 Agent 编排循环。

**核心结构** (`agent.rs`):

```rust
pub struct Agent {
    provider: Box<dyn Provider>,       // LLM 后端
    tools: Vec<Box<dyn Tool>>,         // 可用工具
    memory: Arc<dyn Memory>,           // 记忆后端
    observer: Arc<dyn Observer>,       // 可观测性
    prompt_builder: SystemPromptBuilder,
    tool_dispatcher: Box<dyn ToolDispatcher>,  // 工具分发
    memory_loader: Box<dyn MemoryLoader>,
    config: AgentConfig,               // 配置
    model_name: String,
    temperature: f64,
    workspace_dir: PathBuf,
    history: Vec<ConversationMessage>, // 对话历史
}

pub struct AgentBuilder { /* Agent 的 builder 模式 */ }
```

**子模块**:
- `loop_` - 主编排循环、工具调用处理
- `dispatcher` - 工具分发（支持原生 XML 解析和 JSON）
- `session` - 会话管理
- `prompt` - 系统提示构建
- `research` - 主动信息收集
- `classifier` - 查询分类

---

## 3. 扩展点汇总

| 模块 | Trait 文件 | 扩展方式 |
|------|-----------|---------|
| 提供者 | `src/providers/traits.rs` | 实现 `Provider` |
| 通道 | `src/channels/traits.rs` | 实现 `Channel` |
| 工具 | `src/tools/traits.rs` | 实现 `Tool` |
| 记忆 | `src/memory/traits.rs` | 实现 `Memory` |
| 外设 | `src/peripherals/traits.rs` | 实现 `Peripheral` |
| 运行时 | `src/runtime/traits.rs` | 实现 `RuntimeAdapter` |
| 可观测 | `src/observability/traits.rs` | 实现 `Observer` |

**扩展新功能的最佳实践**:
1. 在对应目录实现 Trait
2. 在 `mod.rs` 的 factory 函数中注册
3. 添加配置 schema 支持（如需要）
4. 编写单元测试覆盖边界情况

---

## 4. CLI 命令结构

```
clawclawclaw
├── agent         # 交互式/单次 agent 会话
├── gateway       # HTTP/Webhook 服务器
├── daemon        # 完整运行时 (gateway + channels + heartbeat + scheduler)
├── service       # OS 服务管理 (systemd/launchd)
├── channel       # 通道管理 (list, start, add, remove, bind)
├── memory        # 记忆管理 (list, get, stats, clear, reindex)
├── cron          # 定时任务管理
├── skills        # 技能管理 (list, new, test, install)
├── peripherals   # 硬件外设管理
├── hardware      # 硬件发现和内省
├── doctor        # 诊断
├── status        # 系统状态
├── update        # 自更新
├── estop         # 紧急停止管理
└── migrate       # 数据迁移
```

---

## 5. 设计灵魂与核心理念

### 5.1 六大产品目标

| 目标 | 含义 | 体现 |
|------|------|------|
| **High Performance** | 高性能 | 异步 I/O、连接池、零拷贝 |
| **High Efficiency** | 高效率 | 最小化资源占用，精简依赖 |
| **High Stability** | 高稳定性 | 显式错误处理，无 panic 路径 |
| **High Extensibility** | 高可扩展性 | Trait + Factory 架构 |
| **High Sustainability** | 高可持续性 | 模块化、可维护、文档完备 |
| **High Security** | 高安全性 | 拒绝默认、最小权限、沙箱隔离 |

### 5.2 架构哲学

```
┌─────────────────────────────────────────────────────────────────┐
│                        设计灵魂                                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│   "Trait-driven, modular architecture"                          │
│                                                                 │
│   扩展点明确且可替换 ───► 大多数功能通过实现 Trait + 注册 Factory  │
│                                                                 │
│   安全相关表面是一等公民 ───► gateway/security/tools/runtime     │
│                              高爆炸半径，需要额外审查            │
│                                                                 │
│   性能和二进制大小是产品目标 ───► 不是锦上添花                   │
│                                    Cargo.toml release 优化       │
│                                                                 │
│   配置和运行时契约是用户 API ───► 向后兼容，显式迁移              │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 6. 工程原则

### 6.1 KISS（保持简单愚蠢）

**为什么**: 运行时 + 安全行为必须在压力下可审计。

**要求**:
- 优先直观的控制流，而非巧妙的元编程
- 优先显式 match 分支和类型结构，而非隐藏的动态行为
- 保持错误路径明显且本地化

### 6.2 YAGNI（你不会需要它）

**为什么**: 过早功能增加攻击面和维护负担。

**要求**:
- 没有具体用例，不添加新配置键/Trait方法/功能标志
- 没有当前调用者，不引入"面向未来"的抽象
- 不支持的部分显式报错，而非添加部分假支持

### 6.3 DRY + 三次原则

**为什么**: 天真的 DRY 会创建跨 provider/channel/tool 的脆弱共享抽象。

**要求**:
- 保留清晰时，重复小的本地逻辑
- 只有在重复稳定模式后才提取共享工具（三次原则）
- 提取时保留模块边界，避免隐藏耦合

### 6.4 SRP + ISP（单一职责 + 接口隔离）

**为什么**: Trait 驱动架构已经编码了子系统边界。

**要求**:
- 每个模块专注一个关注点
- 尽可能通过实现现有窄 Trait 来扩展行为
- 避免混合策略+传输+存储的"上帝模块"

### 6.5 Fail Fast + 显式错误

**为什么**: Agent 运行时中的静默回退可能造成不安全或昂贵的行为。

**要求**:
- 对不支持或不安全状态优先使用显式 bail!/error
- 永不静默放宽权限/能力
- 回退行为有意且安全时，文档说明

### 6.6 安全默认 + 最小权限

**为什么**: Gateway/tools/runtime 可以执行有现实副作用的行为。

**要求**:
- 访问和暴露边界默认拒绝
- 永不记录密钥、原始令牌或敏感载荷
- 网络/文件系统/Shell 范围尽可能窄，除非显式合理

### 6.7 确定性 + 可复现性

**为什么**: 可靠的 CI 和低延迟分流依赖确定性行为。

**要求**:
- CI 敏感路径优先可复现命令和锁定依赖行为
- 测试确定性（无常依赖时间/网络的松散测试）
- 本地验证命令映射到 CI 期望

### 6.8 可逆性 + 回滚优先思维

**为什么**: 高 PR 量下快速恢复是强制的。

**要求**:
- 保持变更易于回退（小范围、清晰爆炸半径）
- 风险变更合并前定义回滚路径
- 避免阻止安全回退的混合巨型补丁

---

## 7. 技术约束

### 7.1 架构边界约束

```
┌─────────────────────────────────────────────────────────────────┐
│                      架构边界规则                                │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ✅ 优先通过添加 Trait 实现 + Factory 接线来扩展能力             │
│  ❌ 避免为孤立功能进行跨模块重写                                  │
│                                                                 │
│  ✅ 依赖方向内向契约：具体集成依赖 Trait/Config/Util 层          │
│  ❌ 避免具体集成之间的相互依赖                                    │
│                                                                 │
│  ❌ 不创建跨子系统耦合                                           │
│     (例如: provider 代码导入 channel 内部)                       │
│     (例如: tool 代码直接修改 gateway 策略)                       │
│                                                                 │
│  ✅ 模块职责单一：                                               │
│     ├── agent/   → 编排                                         │
│     ├── channels/ → 传输                                        │
│     ├── providers/ → 模型 I/O                                   │
│     ├── security/ → 策略                                        │
│     └── tools/   → 执行                                         │
│                                                                 │
│  ✅ 共享抽象只在重复使用后引入（三次原则）                        │
│     且至少有一个当前作用域的真实调用者                            │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 7.2 性能与二进制约束

```rust
// Cargo.toml release profile - 优化大小和确定性
[profile.release]
opt-level = "z"      // 优化大小
lto = true           // 链接时优化
codegen-units = 1    // 确定性构建
panic = "abort"      // 减小二进制
strip = true         // 剥离符号
```

**约束**:
- 便利依赖和广泛抽象可能静默回归这些目标
- 添加重依赖需要严格审查

### 7.3 命名契约

| 类别 | 规则 | 示例 |
|------|------|------|
| 模块/文件 | snake_case | `memory_store.rs` |
| 类型/Trait | PascalCase | `SecurityPolicy` |
| 函数/变量 | snake_case | `create_provider` |
| 常量/静态 | SCREAMING_SNAKE_CASE | `MAX_RETRIES` |

**命名原则**:
- 类型命名按领域角色，而非实现细节
  - ✅ `DiscordChannel`, `SecurityPolicy`
  - ❌ `Manager`, `Helper`
- Trait 实现者: `<ProviderName>Provider`, `<ChannelName>Channel`, `<ToolName>Tool`
- Factory 键: 稳定、小写、面向用户 (如 `"openai"`, `"discord"`, `"shell"`)

### 7.4 风险分层

| 风险等级 | 路径 | 审查深度 |
|----------|------|----------|
| **低** | docs/chore/tests-only | 轻量检查 |
| **中** | `src/**` 行为变更无边界/安全影响 | 常规检查 |
| **高** | `src/security/**`, `src/runtime/**`, `src/gateway/**`, `src/tools/**`, `.github/workflows/**` | 全量审查 |

---

## 8. 反模式

以下行为被明确禁止：

| 反模式 | 原因 |
|--------|------|
| ❌ 为小便利添加重依赖 | 影响二进制大小和构建时间 |
| ❌ 静默弱化安全策略或访问约束 | 安全不可妥协 |
| ❌ 添加"以防万一"的配置/功能标志 | YAGNI 原则 |
| ❌ 混合大规模格式化变更和功能变更 | 阻止可逆性 |
| ❌ "顺手"修改不相关模块 | 破坏关注点分离 |
| ❌ 无显式说明绕过失败检查 | 破坏确定性 |
| ❌ 在重构提交中隐藏行为变更副作用 | 破坏可审计性 |
| ❌ 在测试数据/示例/文档/提交中包含个人身份或敏感信息 | 隐私合规 |
| ❌ 未经维护者明确请求尝试仓库品牌重塑/身份替换 | 项目完整性 |
| ❌ 未经维护者明确请求引入新平台表面 | 范围控制 |

---

## 附录：灵魂总结

```
┌─────────────────────────────────────────────────────────────────┐
│                                                                 │
│                    clawclawclaw 的灵魂                               │
│                                                                 │
│        "一个 Rust 优先的自主 Agent 运行时，                       │
│         在性能、安全、可扩展性之间追求极致平衡"                    │
│                                                                 │
│   ┌─────────────┐   ┌─────────────┐   ┌─────────────┐          │
│   │  Trait +    │   │  安全默认   │   │  显式优于   │          │
│   │  Factory    │ + │  拒绝默认   │ + │  隐式       │          │
│   └─────────────┘   └─────────────┘   └─────────────┘          │
│                                                                 │
│   ┌─────────────┐   ┌─────────────┐   ┌─────────────┐          │
│   │  可逆变更   │ + │  三次原则   │ + │  本地验证   │          │
│   │  回滚优先   │   │  避免过度   │   │  映射 CI    │          │
│   └─────────────┘   └─────────────┘   └─────────────┘          │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**为什么这些原则存在**:

- **Agent 运行时执行有现实副作用的操作** → 安全不可妥协
- **Trait 架构是稳定性支柱** → 扩展必须遵循模式
- **高 PR 量是设计约束** → 变更必须可逆、可审计
- **二进制大小是产品目标** → 依赖选择影响用户
