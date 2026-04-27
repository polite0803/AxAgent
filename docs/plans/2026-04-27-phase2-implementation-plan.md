# Phase 2 实施计划：LLMOps 与可观测性

> 时间周期：2 个月 | 总工作量：15 人天
>
> **状态**: ✅ 已完成

## 2.1 任务概览

| 任务 | 工作量 | 优先级 | 状态 |
|------|--------|--------|------|
| 指标数据模型完善 | 5 人天 | P0 | ✅ 已完成 |
| Prompt 模板管理 | 5 人天 | P1 | ✅ 已完成 |
| 工作流版本控制 | 5 人天 | P2 | ✅ 已完成 |

---

## 2.2 指标数据模型完善

### 状态: ✅ 已完成

#### 新增/修改文件

| 文件路径 | 说明 |
|---------|------|
| `src-tauri/crates/telemetry/src/metrics.rs` | 修改: 添加 `AppMetrics` 和 `WorkflowMetrics` 结构 |
| `src-tauri/crates/telemetry/src/collector.rs` | 新增: `MetricsCollector` 实现 |
| `src-tauri/crates/telemetry/src/lib.rs` | 修改: 导出新模块 |
| `src-tauri/crates/telemetry/Cargo.toml` | 修改: 添加 `prometheus` 和 `tokio` 依赖 |

#### 核心结构

**AppMetrics**:
```rust
pub struct AppMetrics {
    pub app_id: String,
    pub date: DateTime<Utc>,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub total_tokens: u64,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_cost_usd: f64,
    pub latency_ms: u64,
}
```

**WorkflowMetrics**:
```rust
pub struct WorkflowMetrics {
    pub workflow_id: String,
    pub execution_count: u64,
    pub success_count: u64,
    pub failure_count: u64,
    pub total_duration_ms: u64,
    pub avg_duration_ms: u64,
}
```

**MetricsCollector**:
```rust
pub struct MetricsCollector {
    registry: Registry,
    workflow_executions_total: IntCounterVec,
    workflow_execution_duration: Histogram,
    llm_requests_total: IntCounterVec,
    rag_retrieval_total: IntCounterVec,
    active_workflows: IntGauge,
}
```

---

## 2.3 Prompt 模板管理

### 状态: ✅ 已完成

#### 新增文件

| 文件路径 | 说明 |
|---------|------|
| `src-tauri/crates/core/src/entity/prompt_template.rs` | `PromptTemplate` Entity 定义 |
| `src-tauri/crates/core/src/entity/prompt_template_version.rs` | `PromptTemplateVersion` Entity 定义 |
| `src-tauri/crates/core/src/prompt_template.rs` | `PromptTemplateRenderer` 模板渲染器 |
| `src-tauri/crates/core/src/lib.rs` | 修改: 导出新模块 |
| `src/pages/PromptTemplatesPage.tsx` | 前端页面: 模板列表、编辑器、版本历史 |

#### 核心功能

- **PromptTemplate Entity**: 支持版本化、A/B 测试、变量 Schema
- **PromptTemplateRenderer**: 模板变量提取、渲染、验证
- **前端页面**: 模板 CRUD、版本历史、回滚

---

## 2.4 工作流版本控制

### 状态: ✅ 已完成

#### 新增/修改文件

| 文件路径 | 说明 |
|---------|------|
| `src-tauri/crates/core/src/workflow_version.rs` | 新增: `VersionDiff` 和 `WorkflowVersionComparator` |
| `src-tauri/crates/migration/src/m20260501_000001_add_workflow_version_fields.rs` | 新增: 添加 `is_active` 和 `changelog` 字段 |
| `src-tauri/crates/core/src/entity/mod.rs` | 修改: 导出 `workflow_template_version` 模块 |
| `src-tauri/crates/core/src/lib.rs` | 修改: 导出 `workflow_version` 模块 |

#### 核心功能

**VersionDiff**:
```rust
pub struct VersionDiff {
    pub name_changed: bool,
    pub description_changed: bool,
    pub nodes_changed: bool,
    pub edges_changed: bool,
    pub variables_changed: bool,
    // ... more fields
}
```

**WorkflowVersionComparator**:
```rust
pub struct WorkflowVersionComparator;

impl WorkflowVersionComparator {
    pub fn compare(v1: &Model, v2: &Model) -> VersionDiff;
}
```

---

## 2.5 里程碑

| 里程碑 | 日期 | 完成内容 |
|--------|------|----------|
| M1 | 第 1-2 周 | ✅ 指标数据模型完善 |
| M2 | 第 3 周 | ✅ Prompt 模板实体和 CRUD |
| M3 | 第 4 周 | ✅ Prompt 模板版本管理 |
| M4 | 第 5 周 | ✅ 工作流版本控制增强 |
| M5 | 第 6-8 周 | (预留测试/Bug修复时间) |

---

## 2.6 文件变更清单

```
src-tauri/crates/telemetry/src/
├── metrics.rs              # 修改: 添加 AppMetrics, WorkflowMetrics
├── collector.rs           # 新增: MetricsCollector
└── lib.rs                 # 修改: 导出新模块

src-tauri/crates/core/src/
├── entity/
│   ├── prompt_template.rs           # 新增
│   ├── prompt_template_version.rs    # 新增
│   └── mod.rs                        # 修改: 导出新模块
├── prompt_template.rs               # 新增: PromptTemplateRenderer
├── workflow_version.rs              # 新增: VersionDiff, WorkflowVersionComparator
└── lib.rs                           # 修改: 导出新模块

src-tauri/crates/migration/src/
└── m20260501_000001_add_workflow_version_fields.rs  # 新增

src/pages/
└── PromptTemplatesPage.tsx          # 新增
```
