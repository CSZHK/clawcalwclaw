# Agent 提示词架构

> 文档日期：2026-03-03
> 分析基准：`src/agent/prompt.rs`、`src/skills/mod.rs`、`src/agent/agent.rs`

---

## 概述

ZeroClaw agent 的系统提示词通过 **Section 组合流水线** 构建：`SystemPromptBuilder` 持有一组有序的 `PromptSection` 实现，`build()` 时依次调用各 section，将非空输出 `trim_end + "\n\n"` 拼接为最终系统提示词。

---

## 核心结构

### SystemPromptBuilder

```
src/agent/prompt.rs:47-85
```

```
┌─────────────────────────────────────────────────────────────────┐
│              SystemPromptBuilder                                 │
│              sections: Vec<Box<dyn PromptSection>>              │
└──────────────────────┬──────────────────────────────────────────┘
                       │ build(&ctx) → 遍历各 section，trim_end+\n\n 拼接
                       ▼
┌──────────────────────────────────────────────────────────────────┐
│                      PromptContext (L32)                         │
├──────────────────────────────────────────────────────────────────┤
│  workspace_dir            &Path                                  │
│  model_name               &str                                   │
│  tools                    &[Box<dyn Tool>]                       │
│  skills                   &[Skill]                               │
│  skills_prompt_mode       SkillsPromptInjectionMode (Full|Compact)│
│  identity_config          Option<&IdentityConfig>                │
│  dispatcher_instructions  &str  ← tool_dispatcher.prompt_instructions│
└──────────────────────────────────────────────────────────────────┘
```

### PromptSection Trait

```rust
// src/agent/prompt.rs:42-45
pub trait PromptSection: Send + Sync {
    fn name(&self) -> &str;
    fn build(&self, ctx: &PromptContext<'_>) -> Result<String>;
}
```

空输出（`trim().is_empty()`）的 section 自动跳过，不产生任何输出。

---

## 8 个默认 Section（with_defaults()，顺序固定）

```
src/agent/prompt.rs:53-65
```

| 顺序 | Section | 内容 |
|------|---------|------|
| 1 | `IdentitySection` | 身份/人格（workspace 文件 + 可选 aieos JSON） |
| 2 | `ToolsSection` | 工具列表（name, description, parameters_schema） |
| 3 | `SafetySection` | 固化安全规则（硬编码，无上下文依赖） |
| 4 | `SkillsSection` | 技能 XML（Full/Compact 双模式） |
| 5 | `WorkspaceSection` | 当前工作目录绝对路径 |
| 6 | `DateTimeSection` | 当前时间戳（支持热更新） |
| 7 | `RuntimeSection` | host / OS / model_name |
| 8 | `ChannelMediaSection` | 媒体标记格式说明（Voice / IMAGE / Document） |

---

## Section 详解

### 1. IdentitySection — 双轨身份系统

```
src/agent/prompt.rs:96-148
```

```
IdentitySection.build()
       │
       ├─ is_aieos_configured(config)?
       │       ├─ Yes → load_aieos_identity() → aieos_to_system_prompt()
       │       │         结构化 JSON 格式（aieos 字段）→ 渲染为文本
       │       │         has_aieos = true
       │       │
       │       └─ No / 加载失败 → 降级到 openclaw 模式
       │
       ├─ 无论 aieos 是否成功，始终追加 workspace 文件（按序）：
       │    AGENTS.md, SOUL.md, TOOLS.md, IDENTITY.md,
       │    USER.md, HEARTBEAT.md, BOOTSTRAP.md
       │    [MEMORY.md 仅在文件存在时注入]
       │
       └─ identity_config.extra_files → normalize_openclaw_identity_extra_file()
               路径遍历防护：阻止 "../" 和绝对路径
               最终追加自定义 markdown 文件
```

**文件注入规则**（`inject_workspace_file()`，L258）：
- 每个文件截断上限：`BOOTSTRAP_MAX_CHARS = 20_000` 字符（约 20KB/文件）
- 超出时追加 `` [... truncated at 20000 chars — use `read` for full file] `` 提示
- 文件不存在时注入 `[File not found: <name>]` 占位符（不静默跳过）

### 2. ToolsSection

```
src/agent/prompt.rs:151-173
```

为每个工具输出：

```
- **{name}**: {description}
  Parameters: `{parameters_schema}`
```

追加 `dispatcher_instructions`（来自 `tool_dispatcher.prompt_instructions(&tools)`），用于说明工具调用的格式约定（如 XML 标签格式、JSON 格式等，取决于 dispatcher 实现）。

### 3. SafetySection

```
src/agent/prompt.rs:175-183
```

硬编码安全规则，不依赖任何上下文：
- 不外泄私有数据
- 破坏性操作先询问
- 不绕过审批机制
- 优先使用 `trash` 而非 `rm`
- 不确定时先询问

### 4. SkillsSection — XML 双模式

```
src/agent/prompt.rs:185-197
src/skills/mod.rs:785-852
```

**Full 模式**（默认）：

```xml
## Available Skills

Skill instructions and tool metadata are preloaded below.
Follow these instructions directly; do not read skill files at runtime unless the user asks.

<available_skills>
  <skill>
    <name>skill-name</name>
    <description>...</description>
    <location>/absolute/path/to/skill</location>
    <instructions>
      <instruction>...</instruction>
    </instructions>
    <tools>
      <tool>
        <name>tool-name</name>
        <description>...</description>
        <kind>shell|http|script</kind>
      </tool>
    </tools>
  </skill>
</available_skills>
```

**Compact 模式**：
- 仅注入 `name + description + location`
- 例外：`Skill.always = true` 的技能强制注入完整 `instructions`
- 按需读取技能文件，减少 token 消耗

### 5–8. 其余 Section

| Section | 输出格式 |
|---------|---------|
| `WorkspaceSection` | `## Workspace\n\nWorking directory: \`{path}\`` |
| `DateTimeSection` | `## Current Date & Time\n\n{YYYY-MM-DD HH:MM:SS} ({TZ})` |
| `RuntimeSection` | `## Runtime\n\nHost: {hostname} \| OS: {os} \| Model: {model}` |
| `ChannelMediaSection` | `[Voice]`、`[IMAGE:<path>]`、`[Document: <name>]` 的格式说明 |

---

## 长会话热更新：refresh_prompt_datetime()

```
src/agent/prompt.rs:13-30
```

```rust
pub fn refresh_prompt_datetime(prompt: &mut String) {
    // 1. 定位 "## Current Date & Time\n\n" 标头
    // 2. 精准替换该行内容（字节级 replace_range）
    // 3. 不重建整个 prompt，O(n) 单次 scan
}
```

用于长会话中保持时间戳准确，无需全量重建 prompt。

---

## 完整调用链

```
src/agent/agent.rs:394  build_system_prompt()
         │
         ├─ tool_dispatcher.prompt_instructions(&self.tools)
         │       → dispatcher_instructions（工具调用格式说明）
         │
         └─ SystemPromptBuilder.build(&ctx)
                  │
                  └─ for section in &sections { section.build(&ctx) }
                            │
                            ├─ 空 section 自动跳过
                            └─ 非空 section → trim_end + "\n\n" → 拼接
```

---

## 可扩展性

`add_section(Box<dyn PromptSection>)` 允许在默认 8 个 section 之外追加自定义 section，无需修改核心逻辑（开闭原则）。

**注意**：`src/channels/mod.rs:4534` 中 channel 侧直接调用 `skills_to_prompt_with_mode()`，绕过 builder 独立构建 channel 专属提示词，与 agent 提示词构建路径并行。

---

## 相关文件

| 文件 | 职责 |
|------|------|
| `src/agent/prompt.rs` | Section 定义、PromptContext、SystemPromptBuilder |
| `src/skills/mod.rs` | Skill 结构、skills_to_prompt_with_mode() |
| `src/agent/agent.rs:394` | build_system_prompt() 调用点 |
| `src/config/schema.rs:1207` | SkillsPromptInjectionMode enum |
| `src/identity.rs` | aieos 身份加载（load_aieos_identity、aieos_to_system_prompt） |
