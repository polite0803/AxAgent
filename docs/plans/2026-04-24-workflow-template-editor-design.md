# 工作流模版编辑器 - 设计方案与实施计划

> 文档版本: v1.0
> 创建日期: 2026-04-24
> 项目: AxAgent

## 1. 概述

### 1.1 背景与目标

AxAgent 现有的工作流模板以硬编码形式存在于 `WorkflowTemplateSelector.tsx` 中，限制了用户自定义和扩展能力。本项目旨在实现：

- 将预设模板从硬编码迁移至数据库存储
- 提供可视化 DAG 编辑器支持拖拽创建自定义模板
- 集成 AI 辅助设计能力

### 1.2 功能范围

| 类别 | 功能 |
|------|------|
| **模板管理** | 创建、编辑、删除、导入导出模板 |
| **可视化编辑** | 拖拽式 DAG 编辑器，支持节点连线 |
| **节点类型** | 触发器、Agent、LLM、条件分支、并行分支、循环等 15+ 节点 |
| **流程控制** | 条件路由、并行执行、循环迭代、错误处理 |
| **AI 辅助** | 自然语言生成工作流、Prompt 优化、节点推荐 |
| **预设模板** | 代码审查、Bug 修复、文档生成、测试生成等 |

---

## 2. 系统架构

### 2.1 整体架构图

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           Frontend (React)                              │
├─────────────────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────┐    │
│  │  WorkflowEditor │  │  NodePalette     │  │  PropertiesPanel    │    │
│  │  (主编辑器)     │  │  (节点调色板)     │  │  (属性配置面板)      │    │
│  └────────┬────────┘  └────────┬────────┘  └──────────┬──────────┘    │
│           │                    │                      │               │
│           └────────────────────┼──────────────────────┘               │
│                                ▼                                        │
│                    ┌─────────────────────┐                              │
│                    │  useWorkflowEditor  │                              │
│                    │  (Zustand Store)    │                              │
│                    └──────────┬──────────┘                              │
│                               │                                          │
├───────────────────────────────┼────────────────────────────────────────┤
│                    Tauri Commands (IPC)                                  │
├───────────────────────────────┼────────────────────────────────────────┤
│                           Backend (Rust)                                │
│  ┌────────────────────────────▼────────────────────────────────────┐    │
│  │                    Command Layer                                 │    │
│  │  workflow_template.rs  │  workflow_execute.rs  │  ai_assist.rs │    │
│  └────────────────────────────┬────────────────────────────────────┘    │
│                               │                                          │
│  ┌────────────────────────────▼────────────────────────────────────┐    │
│  │                    Service Layer                                 │    │
│  │  WorkflowTemplateService  │  WorkflowEngine  │  ValidationSvc  │    │
│  └────────────────────────────┬────────────────────────────────────┘    │
│                               │                                          │
│  ┌────────────────────────────▼────────────────────────────────────┐    │
│  │                    Repository Layer                             │    │
│  │  workflow_template.rs  │  scheduled_task.rs  │  settings.rs    │    │
│  └────────────────────────────┬────────────────────────────────────┘    │
│                               │                                          │
│  ┌────────────────────────────▼────────────────────────────────────┐    │
│  │                    Database (SQLite + SeaORM)                    │    │
│  │  workflow_templates  │  scheduled_tasks  │  conversations      │    │
│  └──────────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────────┘
```

### 2.2 技术栈

| 层级 | 技术选型 |
|------|----------|
| **前端框架** | React 19 + TypeScript |
| **状态管理** | Zustand 5 |
| **DAG 引擎** | @xyflow/react (React Flow 12) |
| **拖拽** | @dnd-kit/core |
| **UI 组件** | Ant Design 6 + antd-style |
| **代码编辑** | Monaco Editor |
| **后端框架** | Tauri 2 + Rust |
| **数据库** | SQLite + SeaORM |
| **工作流引擎** | axagent-runtime |

---

## 3. 数据模型设计

### 3.1 数据库 Schema

#### workflow_templates 表

```sql
CREATE TABLE workflow_templates (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    icon TEXT DEFAULT 'Bot',
    tags TEXT,                          -- JSON array
    version INTEGER DEFAULT 1,
    is_preset BOOLEAN DEFAULT FALSE,
    is_editable BOOLEAN DEFAULT TRUE,
    is_public BOOLEAN DEFAULT FALSE,

    -- 核心定义
    trigger_config TEXT,               -- JSON: TriggerConfig
    nodes TEXT NOT NULL,               -- JSON array: Vec<WorkflowNode>
    edges TEXT NOT NULL,               -- JSON array: Vec<WorkflowEdge>

    -- Schema 定义
    input_schema TEXT,
    output_schema TEXT,

    -- 变量
    variables TEXT,                     -- JSON array: Vec<Variable>

    -- 错误处理
    error_config TEXT,

    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,

    UNIQUE(name, version)
);

CREATE INDEX idx_workflow_templates_preset ON workflow_templates(is_preset);
CREATE INDEX idx_workflow_templates_tags ON workflow_templates(tags);
CREATE INDEX idx_workflow_templates_updated ON workflow_templates(updated_at);
```

### 3.2 Rust 数据结构

#### 模板实体

```rust
// src-tauri/crates/core/src/entity/workflow_template.rs

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "workflow_templates")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    #[sea_orm(column_type = "Text")]
    pub tags: String,                  // JSON
    pub version: i32,
    pub is_preset: bool,
    pub is_editable: bool,
    pub is_public: bool,
    #[sea_orm(column_type = "Text")]
    pub trigger_config: String,         // JSON
    #[sea_orm(column_type = "Text")]
    pub nodes: String,                  // JSON
    #[sea_orm(column_type = "Text")]
    pub edges: String,                  // JSON
    pub input_schema: Option<String>,
    pub output_schema: Option<String>,
    #[sea_orm(column_type = "Text")]
    pub variables: String,              // JSON
    pub error_config: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
```

#### 节点类型枚举

```rust
// src-tauri/crates/core/src/entity/workflow_node.rs

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkflowNode {
    Trigger(TriggerNode),
    Agent(AgentNode),
    Llm(LlmNode),
    Condition(ConditionNode),
    Parallel(ParallelNode),
    Loop(LoopNode),
    Merge(MergeNode),
    Delay(DelayNode),
    Tool(ToolNode),
    Code(CodeNode),
    SubWorkflow(SubWorkflowNode),
    DocumentParser(DocumentParserNode),
    VectorRetrieve(VectorRetrieveNode),
    End(EndNode),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkflowNodeBase {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub position: Position,
    pub retry: RetryConfig,
    pub timeout: Option<u64>,
    pub enabled: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RetryConfig {
    pub enabled: bool,
    pub max_retries: u32,
    pub backoff_type: BackoffType,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BackoffType {
    Linear,
    Exponential,
    Fixed,
}
```

#### Agent 节点

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: AgentNodeConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentNodeConfig {
    pub role: AgentRole,
    pub system_prompt: String,
    pub context_sources: Vec<String>,
    pub output_var: String,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub tools: Vec<String>,
    pub output_mode: OutputMode,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AgentRole {
    Researcher,
    Planner,
    Developer,
    Reviewer,
    Synthesizer,
    Executor,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum OutputMode {
    Json,
    Text,
    Artifact,
}
```

#### 条件分支节点

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConditionNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: ConditionNodeConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConditionNodeConfig {
    pub conditions: Vec<Condition>,
    pub logical_op: LogicalOperator,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Condition {
    pub var_path: String,
    pub operator: CompareOperator,
    pub value: serde_json::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CompareOperator {
    Eq, Ne, Gt, Lt, Gte, Lte,
    Contains, NotContains,
    StartsWith, EndsWith,
    RegexMatch,
    IsEmpty, IsNotEmpty,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LogicalOperator {
    And,
    Or,
}
```

#### 并行分支节点

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParallelNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: ParallelNodeConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParallelNodeConfig {
    pub branches: Vec<Branch>,
    pub wait_for_all: bool,
    pub timeout: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Branch {
    pub id: String,
    pub title: String,
    pub steps: Vec<String>,  // 节点 ID 列表
}
```

#### 循环节点

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoopNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: LoopNodeConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoopNodeConfig {
    pub loop_type: LoopType,
    pub items_var: Option<String>,
    pub iteratee_var: Option<String>,
    pub max_iterations: Option<u32>,
    pub continue_condition: Option<String>,
    pub continue_on_error: bool,
    pub body_steps: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LoopType {
    ForEach,
    While,
    DoWhile,
    Until,
}
```

### 3.3 边结构

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkflowEdge {
    pub id: String,
    pub source: String,
    pub source_handle: Option<String>,
    pub target: String,
    pub target_handle: Option<String>,
    pub edge_type: EdgeType,
    pub label: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum EdgeType {
    Direct,
    ConditionTrue,
    ConditionFalse,
    LoopBack,
    ParallelBranch,
    Merge,
    Error,
}
```

---

## 4. 后端 API 设计

### 4.1 Tauri Commands

```rust
// src-tauri/src/commands/workflow_template.rs

#[tauri::command]
pub async fn list_workflow_templates(
    app_state: State<'_, AppState>,
    filter: Option<TemplateFilter>,
) -> Result<Vec<WorkflowTemplateResponse>, String>;

#[tauri::command]
pub async fn get_workflow_template(
    app_state: State<'_, AppState>,
    id: String,
) -> Result<WorkflowTemplateResponse, String>;

#[tauri::command]
pub async fn create_workflow_template(
    app_state: State<'_, AppState>,
    input: WorkflowTemplateInput,
) -> Result<WorkflowTemplateResponse, String>;

#[tauri::command]
pub async fn update_workflow_template(
    app_state: State<'_, AppState>,
    id: String,
    input: WorkflowTemplateInput,
) -> Result<WorkflowTemplateResponse, String>;

#[tauri::command]
pub async fn delete_workflow_template(
    app_state: State<'_, AppState>,
    id: String,
) -> Result<(), String>;

#[tauri::command]
pub async fn duplicate_workflow_template(
    app_state: State<'_, AppState>,
    id: String,
    new_name: String,
) -> Result<WorkflowTemplateResponse, String>;

#[tauri::command]
pub async fn import_workflow_template(
    app_state: State<'_, AppState>,
    json_content: String,
) -> Result<WorkflowTemplateResponse, String>;

#[tauri::command]
pub async fn export_workflow_template(
    app_state: State<'_, AppState>,
    id: String,
) -> Result<String, String>;

#[tauri::command]
pub async fn validate_workflow_template(
    template: WorkflowTemplateInput,
) -> Result<ValidationResult, String>;

#[tauri::command]
pub async fn get_preset_templates(
    app_state: State<'_, AppState>,
) -> Result<Vec<WorkflowTemplateResponse>, String>;
```

### 4.2 请求/响应结构

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTemplateInput {
    pub name: String,
    pub description: String,
    pub icon: String,
    pub tags: Vec<String>,
    pub trigger_config: TriggerConfig,
    pub nodes: Vec<WorkflowNode>,
    pub edges: Vec<WorkflowEdge>,
    pub input_schema: Option<JsonSchema>,
    pub output_schema: Option<JsonSchema>,
    pub variables: Vec<Variable>,
    pub error_config: Option<ErrorConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTemplateResponse {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub tags: Vec<String>,
    pub version: i32,
    pub is_preset: bool,
    pub is_editable: bool,
    pub is_public: bool,
    pub trigger_config: TriggerConfig,
    pub nodes: Vec<WorkflowNode>,
    pub edges: Vec<WorkflowEdge>,
    pub input_schema: Option<JsonSchema>,
    pub output_schema: Option<JsonSchema>,
    pub variables: Vec<Variable>,
    pub error_config: Option<ErrorConfig>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateFilter {
    pub is_preset: Option<bool>,
    pub tags: Option<Vec<String>>,
    pub search: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub error_type: String,
    pub node_id: Option<String>,
    pub message: String,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    pub warning_type: String,
    pub node_id: Option<String>,
    pub message: String,
}
```

### 4.3 AI 辅助 Commands

```rust
// src-tauri/src/commands/ai_assist.rs

#[tauri::command]
pub async fn generate_workflow_from_prompt(
    app_state: State<'_, AppState>,
    prompt: String,
) -> Result<WorkflowTemplateResponse, String>;

#[tauri::command]
pub async fn optimize_agent_prompt(
    app_state: State<'_, AppState>,
    current_prompt: String,
    context: Option<String>,
) -> Result<String, String>;

#[tauri::command]
pub async fn recommend_nodes(
    app_state: State<'_, AppState>,
    workflow_description: String,
    existing_nodes: Vec<WorkflowNode>,
) -> Result<NodeRecommendations, String>;

#[tauri::command]
pub async fn analyze_workflow(
    app_state: State<'_, AppState>,
    template: WorkflowTemplateInput,
) -> Result<WorkflowAnalysis, String>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeRecommendations {
    pub suggested_nodes: Vec<SuggestedNode>,
    pub suggested_connections: Vec<SuggestedConnection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedNode {
    pub node_type: String,
    pub reason: String,
    pub position: Position,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedConnection {
    pub from_node_id: String,
    pub to_node_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowAnalysis {
    pub issues: Vec<WorkflowIssue>,
    pub suggestions: Vec<String>,
    pub complexity_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowIssue {
    pub severity: String,  // "error", "warning", "info"
    pub node_id: Option<String>,
    pub message: String,
}
```

---

## 5. 前端架构设计

### 5.1 目录结构

```
src/components/workflow/
├── index.ts                              # 导出入口
├── WorkflowEditor.tsx                     # 主编辑器组件
├── types/
│   ├── index.ts                           # 类型导出
│   ├── workflow.types.ts                  # 核心类型定义
│   ├── node.types.ts                      # 节点类型定义
│   └── api.types.ts                       # API 类型定义
│
├── Canvas/
│   ├── DAGCanvas.tsx                     # 画布主组件
│   ├── CanvasControls.tsx                 # 控制按钮
│   └── CanvasBackground.tsx               # 背景网格
│
├── Nodes/
│   ├── BaseNode.tsx                       # 基础节点
│   ├── TriggerNode.tsx                    # 触发器节点
│   ├── AgentNode.tsx                      # Agent 节点
│   ├── LLMNode.tsx                        # LLM 节点
│   ├── ConditionNode.tsx                  # 条件分支节点
│   ├── ParallelNode.tsx                  # 并行分支节点
│   ├── LoopNode.tsx                      # 循环节点
│   ├── MergeNode.tsx                     # 合并节点
│   ├── DelayNode.tsx                     # 延迟节点
│   ├── ToolNode.tsx                      # 工具节点
│   ├── CodeNode.tsx                      # 代码节点
│   ├── SubWorkflowNode.tsx               # 子工作流节点
│   └── EndNode.tsx                       # 结束节点
│
├── Edges/
│   ├── BaseEdge.tsx                       # 基础边
│   ├── ConditionEdge.tsx                  # 条件边
│   ├── LoopBackEdge.tsx                 # 循环回边
│   └── ErrorEdge.tsx                    # 错误边
│
├── Panels/
│   ├── LeftPanel.tsx                      # 左侧面板
│   ├── NodePalette.tsx                   # 节点调色板
│   ├── TemplateList.tsx                  # 模板列表
│   ├── SettingsPanel.tsx                 # 设置面板
│   ├── RightPanel.tsx                    # 右侧面板
│   ├── PropertiesPanel.tsx               # 属性面板
│   └── AIPanel.tsx                       # AI 助手面板
│
├── Header/
│   ├── EditorHeader.tsx                   # 编辑器头部
│   └── TemplateNameInput.tsx             # 模板名称输入
│
├── StatusBar/
│   └── EditorStatusBar.tsx               # 状态栏
│
├── Modals/
│   ├── VariableModal.tsx                  # 变量编辑弹窗
│   ├── SchemaModal.tsx                   # Schema 编辑弹窗
│   └── ImportExportModal.tsx             # 导入导出弹窗
│
└── hooks/
    ├── useWorkflowEditor.ts               # 编辑器主 hook
    ├── useWorkflowValidation.ts           # 验证 hook
    ├── useAutoLayout.ts                  # 自动布局 hook
    └── useWorkflowPersistence.ts         # 持久化 hook
```

### 5.2 布局结构

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  EditorHeader (56px)                                                              │
│  [←] [模板名称 (可编辑)]                    [预览] [保存] [发布] [AI]              │
├────────┬───────────────────────────────────────────────────────────┬──────────────┤
│        │                                                           │              │
│ Left   │                      DAGCanvas                            │   Right      │
│ Panel  │                                                           │   Panel      │
│ (280px)│   ┌──────┐    ┌──────┐    ┌──────┐                       │   (360px)    │
│        │   │ Start│───▶│Agent │───▶│ LLM  │                       │              │
│ ┌────┐ │   └──────┘    └──────┘    └───┬──┘                       │ ┌──────────┐ │
│ │Node│ │                             │                           │ │ Node     │ │
│ │Palt│ │              ┌──────────────┴──────────────┐              │ │ Props    │ │
│ │ette│ │              ▼                             ▼              │ │ Panel    │ │
│ └────┘ │         ┌────────┐                   ┌────────┐           │ │          │ │
│        │         │Cond T  │                   │Cond F  │           │ └──────────┘ │
│ ┌────┐ │         └───┬────┘                   └────────┘           │              │
│ │Temp│ │             │                                               │ ┌──────────┐ │
│ │late│ │       ┌─────┴─────┐                                        │ │ AI       │ │
│ │List│ │       ▼           ▼                                        │ │ Assistant│ │
│ └────┘ │   ┌────────┐   ┌────────┐                                   │ │ Panel    │ │
│        │   │ Step A │   │ Step B │                                   │ │ (可折叠) │ │
│ ┌────┐ │   └────────┘   └────────┘                                   │ └──────────┘ │
│ │Sett│ │        │           │                                        │              │
│ │ings│ │        └─────┬─────┘                                        │              │
│ └────┘ │              ▼                                               │              │
│        │         ┌────────┐                                           │              │
│        │         │  End   │                                           │              │
│        │         └────────┘                                           │              │
│        │                                                           │              │
├────────┴───────────────────────────────────────────────────────────┴──────────────┤
│  EditorStatusBar (32px):  节点数: 5  |  边数: 4  |  最后保存: 2分钟前  |  ● 已连接 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

## 6. 节点类型详细设计

### 6.1 节点类型列表

| 类别 | 节点类型 | 图标 | 功能描述 |
|------|----------|------|----------|
| **触发器** | Manual | `Hand` | 手动触发启动 |
| | Schedule | `Clock` | 定时触发（Cron） |
| | Webhook | `Webhook` | HTTP Webhook 触发 |
| | Event | `Zap` | 事件触发 |
| **Agent** | Agent | `Bot` | 多角色 Agent 执行 |
| **LLM** | LLM Call | `MessageSquare` | LLM 对话调用 |
| | Doc Summarize | `FileText` | 文档摘要 |
| | Vector Search | `Search` | 向量检索 |
| **工具** | Tool | `Wrench` | 工具调用 |
| | Code | `Code` | 代码执行 |
| **流程控制** | Condition | `GitBranch` | 条件分支 |
| | Parallel | `GitMerge` | 并行分支 |
| | Loop | `Repeat` | 循环迭代 |
| | Merge | `CircleDot` | 合并节点 |
| | Delay | `Timer` | 延迟等待 |
| **子工作流** | SubWorkflow | `Package` | 调用子工作流 |
| **终止** | End | `Flag` | 结束节点 |

### 6.2 节点视觉规范

**节点尺寸：**
- 最小宽度：200px
- 最大宽度：320px
- 高度：自适应内容

**节点结构：**
```
┌─────────────────────────────────────┐
│ ⋮⋮ 🤖 Agent              [⋮] [─] [×] │  ← 头部：拖拽手柄、图标、操作按钮
├─────────────────────────────────────┤
│ Code Review Agent                   │  ← 标题
│ Role: Reviewer                      │  ← 副标题
├─────────────────────────────────────┤
│ ┌─────────────────────────────┐     │
│ │ System Prompt Preview...     │     │  ← 内容预览
│ └─────────────────────────────┘     │
├─────────────────────────────────────┤
│ 📥 context: 2  🔧 tools: 3         │  ← 状态栏
├─────────────────────────────────────┤
│ ● ─────────────────────────▶        │  ← 连接点
└─────────────────────────────────────┘
```

**节点状态样式：**

| 状态 | 边框颜色 | 背景色 | 动画 |
|------|----------|--------|------|
| 默认 | `#d9d9d9` | `#ffffff` | - |
| 选中 | `#1890ff` | `#e6f7ff` | - |
| 运行中 | `#52c41a` | `#f6ffed` | 脉冲动画 |
| 完成 | `#8c8c8c` | `#f5f5f5` | - |
| 失败 | `#ff4d4f` | `#fff2f0` | - |
| 禁用 | `#d9d9d9` | `#f5f5f5` | 50% 透明度 |

### 6.3 节点类型配色

| 节点类型 | 背景色 | 边框色 |
|----------|--------|--------|
| Trigger | `#fff7e6` | `#faad14` |
| Agent | `#f6ffed` | `#52c41a` |
| LLM | `#fff0f0` | `#ff4d4f` |
| Condition | `#fff7e6` | `#fa8c16` |
| Parallel | `#f9f0ff` | `#722ed1` |
| Loop | `#fff0f0` | `#eb2f96` |
| Tool | `#e6f7ff` | `#1890ff` |
| End | `#f5f5f5` | `#8c8c8c` |

---

## 7. 状态管理设计

### 7.1 Zustand Store

```typescript
// src/stores/workflowEditorStore.ts
import { create } from 'zustand';
import { devtools, subscribeWithSelector } from 'zustand/middleware';
import type {
  WorkflowTemplate,
  WorkflowNode,
  WorkflowEdge,
  ValidationResult,
  Variable,
} from '@/components/workflow/types';

interface WorkflowEditorState {
  // 模板数据
  template: WorkflowTemplate | null;
  isDirty: boolean;
  isSaving: boolean;
  lastSavedAt: number | null;

  // DAG 数据
  nodes: Node[];
  edges: Edge[];

  // UI 状态
  selectedNodeId: string | null;
  selectedEdgeId: string | null;
  showAIPanel: boolean;
  leftPanelTab: 'nodes' | 'templates' | 'settings';
  rightPanelTab: 'properties' | 'ai';

  // 验证
  validationResult: ValidationResult | null;

  // 历史记录
  history: { nodes: Node[]; edges: Edge[] }[];
  historyIndex: number;

  // Actions
  loadTemplate: (id: string) => Promise<void>;
  setTemplate: (template: WorkflowTemplate) => void;
  updateTemplateMeta: (meta: Partial<WorkflowTemplate>) => void;

  setNodes: (nodes: Node[]) => void;
  setEdges: (edges: Edge[]) => void;
  addNode: (type: string, position: { x: number; y: number }) => void;
  updateNode: (id: string, data: Partial<WorkflowNode>) => void;
  deleteNode: (id: string) => void;
  selectNode: (id: string | null) => void;

  addEdge: (edge: Edge) => void;
  updateEdge: (id: string, data: Partial<WorkflowEdge>) => void;
  deleteEdge: (id: string) => void;
  selectEdge: (id: string | null) => void;

  addVariable: (variable: Variable) => void;
  updateVariable: (name: string, variable: Variable) => void;
  deleteVariable: (name: string) => void;

  validate: () => ValidationResult;
  save: () => Promise<void>;

  undo: () => void;
  redo: () => void;
  pushHistory: () => void;
}

export const useWorkflowEditorStore = create<WorkflowEditorState>()(
  devtools(
    subscribeWithSelector((set, get) => ({
      // Initial state
      template: null,
      isDirty: false,
      isSaving: false,
      lastSavedAt: null,
      nodes: [],
      edges: [],
      selectedNodeId: null,
      selectedEdgeId: null,
      showAIPanel: false,
      leftPanelTab: 'nodes',
      rightPanelTab: 'properties',
      validationResult: null,
      history: [],
      historyIndex: -1,

      // Implementation...
    })),
    { name: 'workflow-editor' }
  )
);
```

---

## 8. API 集成

### 8.1 Tauri Invoke 调用

```typescript
// src/lib/workflowApi.ts
import { invoke } from '@tauri-apps/api/core';
import type {
  WorkflowTemplateInput,
  WorkflowTemplateResponse,
  TemplateFilter,
  ValidationResult,
} from '@/components/workflow/types';

export const workflowApi = {
  async listTemplates(filter?: TemplateFilter): Promise<WorkflowTemplateResponse[]> {
    return invoke('list_workflow_templates', { filter });
  },

  async getTemplate(id: string): Promise<WorkflowTemplateResponse> {
    return invoke('get_workflow_template', { id });
  },

  async createTemplate(input: WorkflowTemplateInput): Promise<WorkflowTemplateResponse> {
    return invoke('create_workflow_template', { input });
  },

  async updateTemplate(id: string, input: WorkflowTemplateInput): Promise<WorkflowTemplateResponse> {
    return invoke('update_workflow_template', { id, input });
  },

  async deleteTemplate(id: string): Promise<void> {
    return invoke('delete_workflow_template', { id });
  },

  async duplicateTemplate(id: string, newName: string): Promise<WorkflowTemplateResponse> {
    return invoke('duplicate_workflow_template', { id, newName });
  },

  async importTemplate(jsonContent: string): Promise<WorkflowTemplateResponse> {
    return invoke('import_workflow_template', { jsonContent });
  },

  async exportTemplate(id: string): Promise<string> {
    return invoke('export_workflow_template', { id });
  },

  async validateTemplate(input: WorkflowTemplateInput): Promise<ValidationResult> {
    return invoke('validate_workflow_template', { input });
  },

  async getPresetTemplates(): Promise<WorkflowTemplateResponse[]> {
    return invoke('get_preset_templates');
  },
};
```

---

## 9. 样式规范

### 9.1 CSS 变量

```css
/* src/styles/workflow-editor.css */

:root {
  /* 画布 */
  --wf-canvas-bg: #f5f5f5;
  --wf-canvas-grid: #e0e0e0;
  --wf-canvas-grid-size: 20px;

  /* 节点 */
  --wf-node-bg: #ffffff;
  --wf-node-border: #d9d9d9;
  --wf-node-border-selected: #1890ff;
  --wf-node-bg-selected: #e6f7ff;

  /* 节点类型颜色 */
  --wf-node-trigger-bg: #fff7e6;
  --wf-node-trigger-border: #faad14;
  --wf-node-agent-bg: #f6ffed;
  --wf-node-agent-border: #52c41a;
  --wf-node-llm-bg: #fff0f0;
  --wf-node-llm-border: #ff4d4f;
  --wf-node-condition-bg: #fff7e6;
  --wf-node-condition-border: #fa8c16;
  --wf-node-parallel-bg: #f9f0ff;
  --wf-node-parallel-border: #722ed1;
  --wf-node-loop-bg: #fff0f0;
  --wf-node-loop-border: #eb2f96;
  --wf-node-tool-bg: #e6f7ff;
  --wf-node-tool-border: #1890ff;
  --wf-node-end-bg: #f5f5f5;
  --wf-node-end-border: #8c8c8c;

  /* 边颜色 */
  --wf-edge-default: #1890ff;
  --wf-edge-true: #52c41a;
  --wf-edge-false: #ff4d4f;
  --wf-edge-loop: #faad14;
  --wf-edge-error: #ff4d4f;

  /* 阴影 */
  --wf-node-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
  --wf-node-shadow-selected: 0 4px 16px rgba(24, 144, 255, 0.3);
  --wf-node-shadow-hover: 0 4px 12px rgba(0, 0, 0, 0.12);

  /* 尺寸 */
  --wf-panel-width-left: 280px;
  --wf-panel-width-right: 360px;
  --wf-header-height: 56px;
  --wf-status-bar-height: 32px;
  --wf-node-min-width: 200px;
  --wf-node-max-width: 320px;

  /* 圆角 */
  --wf-node-radius: 8px;
  --wf-panel-radius: 12px;

  /* 过渡 */
  --wf-transition-fast: 150ms ease;
  --wf-transition-normal: 250ms ease;
}

[data-theme="dark"] {
  --wf-canvas-bg: #1a1a2e;
  --wf-canvas-grid: #2d2d44;
  --wf-node-bg: #252538;
  --wf-node-border: #404060;
}
```

---

## 10. 实施计划

### 10.1 阶段划分

| 阶段 | 名称 | 工作内容 | 预估时间 |
|------|------|----------|----------|
| **Phase 1** | 数据库与后端基础设施 | 数据库 Schema、Repository、Commands | 1 周 |
| **Phase 2** | 前端基础框架 | 类型定义、Store、基础组件结构 | 1 周 |
| **Phase 3** | DAG 画布实现 | React Flow 集成、节点渲染、连线 | 1 周 |
| **Phase 4** | 节点组件开发 | 13 种节点组件实现 | 2 周 |
| **Phase 5** | 属性面板 | 属性配置表单、验证逻辑 | 1 周 |
| **Phase 6** | 模板管理 | CRUD、导入导出、预设模板迁移 | 1 周 |
| **Phase 7** | AI 辅助功能 | 自然语言生成、Prompt 优化 | 1 周 |
| **Phase 8** | 测试与优化 | 集成测试、性能优化、Bug 修复 | 1 周 |

**总工期：约 8 周**

### 10.2 Phase 1 详细任务 - 数据库与后端基础设施

| 任务 | 描述 | 负责 |
|------|------|------|
| 1.1 | 创建数据库迁移脚本 `m20260426_000001_add_workflow_templates.rs` | Backend |
| 1.2 | 实现 `WorkflowTemplate` 实体 | Backend |
| 1.3 | 实现 `WorkflowNode` 类型定义 | Backend |
| 1.4 | 实现 `WorkflowEdge` 类型定义 | Backend |
| 1.5 | 实现 `WorkflowTemplateRepository` | Backend |
| 1.6 | 实现 `list_workflow_templates` Command | Backend |
| 1.7 | 实现 `get_workflow_template` Command | Backend |
| 1.8 | 实现 `create_workflow_template` Command | Backend |
| 1.9 | 实现 `update_workflow_template` Command | Backend |
| 1.10 | 实现 `delete_workflow_template` Command | Backend |
| 1.11 | 实现 `validate_workflow_template` Command | Backend |
| 1.12 | 实现预设模板数据迁移脚本 | Backend |
| 1.13 | 单元测试覆盖 | Backend |

### 10.3 Phase 2 详细任务 - 前端基础框架

| 任务 | 描述 | 负责 |
|------|------|------|
| 2.1 | 添加 npm 依赖 (@xyflow/react, dagre, uuid) | Frontend |
| 2.2 | 创建类型定义 `workflow.types.ts` | Frontend |
| 2.3 | 创建类型定义 `node.types.ts` | Frontend |
| 2.4 | 创建 Zustand Store `workflowEditorStore.ts` | Frontend |
| 2.5 | 创建 `workflowApi.ts` API 调用层 | Frontend |
| 2.6 | 创建 `WorkflowEditor` 主组件骨架 | Frontend |
| 2.7 | 创建 `DAGCanvas` 画布组件 | Frontend |
| 2.8 | 创建 `LeftPanel` 左侧面板组件 | Frontend |
| 2.9 | 创建 `RightPanel` 右侧面板组件 | Frontend |
| 2.10 | 创建 `EditorHeader` 头部组件 | Frontend |
| 2.11 | 创建 `EditorStatusBar` 状态栏组件 | Frontend |

### 10.4 Phase 3 详细任务 - DAG 画布实现

| 任务 | 描述 | 负责 |
|------|------|------|
| 3.1 | 集成 React Flow 配置 | Frontend |
| 3.2 | 实现自定义节点渲染 | Frontend |
| 3.3 | 实现自定义边渲染 | Frontend |
| 3.4 | 实现拖拽创建节点 | Frontend |
| 3.5 | 实现节点连线 | Frontend |
| 3.6 | 实现节点选择与编辑 | Frontend |
| 3.7 | 实现画布缩放与平移 | Frontend |
| 3.8 | 实现 MiniMap 小地图 | Frontend |
| 3.9 | 实现 Controls 控制按钮 | Frontend |
| 3.10 | 实现背景网格 | Frontend |
| 3.11 | 实现对齐线 | Frontend |

### 10.5 Phase 4 详细任务 - 节点组件开发

| 任务 | 描述 | 负责 |
|------|------|------|
| 4.1 | 实现 `BaseNode` 基础组件 | Frontend |
| 4.2 | 实现 `TriggerNode` 触发器节点 | Frontend |
| 4.3 | 实现 `AgentNode` Agent 节点 | Frontend |
| 4.4 | 实现 `LLMNode` LLM 节点 | Frontend |
| 4.5 | 实现 `ConditionNode` 条件分支节点 | Frontend |
| 4.6 | 实现 `ParallelNode` 并行分支节点 | Frontend |
| 4.7 | 实现 `LoopNode` 循环节点 | Frontend |
| 4.8 | 实现 `MergeNode` 合并节点 | Frontend |
| 4.9 | 实现 `DelayNode` 延迟节点 | Frontend |
| 4.10 | 实现 `ToolNode` 工具节点 | Frontend |
| 4.11 | 实现 `CodeNode` 代码节点 | Frontend |
| 4.12 | 实现 `SubWorkflowNode` 子工作流节点 | Frontend |
| 4.13 | 实现 `EndNode` 结束节点 | Frontend |

### 10.6 Phase 5 详细任务 - 属性面板

| 任务 | 描述 | 负责 |
|------|------|------|
| 5.1 | 实现 `PropertiesPanel` 基础面板 | Frontend |
| 5.2 | 实现 `AgentProperties` Agent 配置表单 | Frontend |
| 5.3 | 实现 `ConditionProperties` 条件配置表单 | Frontend |
| 5.4 | 实现 `ParallelProperties` 并行配置表单 | Frontend |
| 5.5 | 实现 `LoopProperties` 循环配置表单 | Frontend |
| 5.6 | 实现 `VariableModal` 变量编辑弹窗 | Frontend |
| 5.7 | 实现 `SchemaModal` Schema 编辑弹窗 | Frontend |
| 5.8 | 实现实时验证反馈 | Frontend |

### 10.7 Phase 6 详细任务 - 模板管理

| 任务 | 描述 | 负责 |
|------|------|------|
| 6.1 | 实现 `TemplateList` 模板列表 | Frontend |
| 6.2 | 实现模板选择功能 | Frontend |
| 6.3 | 实现 `ImportExportModal` 导入导出 | Frontend |
| 6.4 | 迁移预设模板到数据库 | Backend |
| 6.5 | 实现模板复制功能 | Frontend |
| 6.6 | 实现模板版本管理 | Backend |

### 10.8 Phase 7 详细任务 - AI 辅助功能

| 任务 | 描述 | 负责 |
|------|------|------|
| 7.1 | 实现 `AIPanel` AI 面板 | Frontend |
| 7.2 | 实现 `generate_workflow_from_prompt` Command | Backend |
| 7.3 | 实现 `optimize_agent_prompt` Command | Backend |
| 7.4 | 实现 `recommend_nodes` Command | Backend |
| 7.5 | 实现自然语言生成工作流 UI | Frontend |
| 7.6 | 实现 Prompt 优化 UI | Frontend |
| 7.7 | 实现节点推荐 UI | Frontend |

### 10.9 Phase 8 详细任务 - 测试与优化

| 任务 | 描述 | 负责 |
|------|------|------|
| 8.1 | 集成测试 | QA |
| 8.2 | E2E 测试 | QA |
| 8.3 | 性能优化 | Frontend |
| 8.4 | Bug 修复 | All |
| 8.5 | 文档完善 | All |

---

## 11. 技术依赖

### 11.1 新增 npm 依赖

```json
{
  "dependencies": {
    "@xyflow/react": "^12.0.0",
    "reactflow": "^11.11.0",
    "dagre": "^0.8.5",
    "@dagrejs/dagre": "^1.1.0",
    "@monaco-editor/react": "^4.6.0",
    "uuid": "^9.0.0",
    "framer-motion": "^11.0.0"
  },
  "devDependencies": {
    "@types/dagre": "^0.7.52",
    "@types/uuid": "^9.0.0"
  }
}
```

### 11.2 现有依赖使用

| 依赖 | 用途 |
|------|------|
| `@ant-design/x` | AI 相关组件 |
| `antd` | UI 组件库 |
| `@dnd-kit/core` | 拖拽功能 |
| `zustand` | 状态管理 |
| `lucide-react` | 图标 |
| `@lobehub/icons` | Agent 图标 |

---

## 12. 风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| React Flow 版本升级 | 破坏性变更 | 锁定版本，评估后升级 |
| DAG 复杂性能问题 | 用户体验 | 虚拟化、懒加载 |
| 预设模板迁移数据丢失 | 业务影响 | 充分测试，保留回滚方案 |
| AI 生成质量不稳定 | 用户体验 | 提供编辑功能，用户可修正 |
| 循环依赖检测遗漏 | 功能缺陷 | 充分测试覆盖 |

---

## 13. 验收标准

### 13.1 功能验收

- [ ] 用户可以创建、编辑、删除自定义模板
- [ ] 用户可以拖拽节点到画布创建工作流
- [ ] 用户可以连接节点形成 DAG
- [ ] 13 种节点类型全部可用
- [ ] 预设模板正常迁移且可编辑
- [ ] AI 辅助生成工作流可用
- [ ] 模板可以导入导出

### 13.2 性能验收

- [ ] 100 节点以内工作流编辑流畅
- [ ] 页面加载时间 < 2s
- [ ] 保存操作 < 500ms

### 13.3 测试验收

- [ ] 单元测试覆盖率 > 70%
- [ ] 集成测试覆盖核心流程
- [ ] E2E 测试覆盖 CRUD 操作
