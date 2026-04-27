# AxAgent 功能路线图

> 本文档基于项目代码库实际调研，专注于真实差距分析和可实施的功能增强。

## 一、Phase 1：工作流与 RAG（2 个月）

### 1.1 现有代码库分析

#### 已实现功能

| 组件 | 路径 | 状态 |
|------|------|------|
| 工作流编辑器前端 | `src/components/workflow/` | ✅ React Flow 拖拽编辑器，15+ 节点类型 |
| 工作流类型定义 | `src-tauri/crates/core/src/workflow_types.rs` | ✅ 完整节点类型、配置、变量定义 |
| 工作流执行引擎 | `src-tauri/crates/runtime/src/work_engine/` | ✅ DAG 执行、拓扑排序、重试机制 |
| 节点执行器 | `src-tauri/crates/runtime/src/work_engine/node_executor.rs` | ✅ AtomicSkill/Agent/LLM 节点执行 |
| RAG 文档解析 | `src-tauri/crates/core/src/document_parser.rs` | ✅ PDF/DOCX/XLSX/PPTX |
| 混合检索 | `src-tauri/crates/core/src/hybrid_search.rs` | ✅ 向量 + BM25 混合检索 |
| 重排序 | `src-tauri/crates/core/src/reranker.rs` | ✅ 多因子评分排序 |

#### 真实差距

| 差距 | 当前状态 | 目标状态 |
|------|---------|---------|
| NodeExecutor trait | struct 过程式设计 | trait-based 可扩展设计 |
| Prometheus Metrics | 仅有内存收集器 | 添加 /metrics HTTP 端点 |
| 工作流版本控制 | 仅支持模板版本 | 工作流定义版本化 |
| 前端撤销/重做 | 缺失 | 基于命令模式的撤销/重做 |

### 1.2 NodeExecutor Trait 重构

**目标**: 将 `NodeExecutor` 从 struct 重构为 trait，支持动态节点扩展

```rust
// 目标接口
#[async_trait]
pub trait NodeExecutor: Send + Sync {
    async fn execute(
        &self,
        context: &WorkflowContext,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, NodeError>;

    fn node_type(&self) -> &'static str;
}

pub struct WorkflowContext {
    pool: Pool<Sqlite>,
    variables: RwLock<HashMap<String, serde_json::Value>>,
    state: ExecutionState,
}
```

**工作量**: 8 人天

### 1.3 前端工作流编辑器增强

**目标**: 提升编辑效率和用户体验

| 功能 | 说明 | 优先级 |
|------|------|--------|
| 节点搜索/过滤 | 快速定位节点 | P1 |
| 撤销/重做 | 命令模式实现 | P1 |
| 节点分组/折叠 | 复杂流程简化 | P2 |
| 快捷键优化 | 提升操作效率 | P2 |

**工作量**: 4 人天

### 1.4 Prometheus Metrics 端点

**目标**: 添加可观测性支持

```rust
// 新增文件: src-tauri/crates/gateway/src/metrics.rs
pub async fn metrics_handler() -> impl Responder {
    let metrics = collect_metrics();
    HttpResponse::Ok()
        .content_type("text/plain; charset=utf-8")
        .body(format!("# HELP workflow_executions_total\n{}", metrics))
}
```

**工作量**: 2 人天

### 1.5 文件变更清单

| 操作 | 文件路径 |
|------|---------|
| 新增 | `src-tauri/crates/runtime/src/work_engine/node_executor_trait.rs` |
| 新增 | `src-tauri/crates/runtime/src/work_engine/executors/` |
| 修改 | `src-tauri/crates/gateway/src/routes.rs` |
| 新增 | `src-tauri/crates/gateway/src/metrics.rs` |
| 修改 | `src/components/workflow/WorkflowEditor.tsx` |
| 修改 | `src/stores/workflowStore.ts` |

**总工作量**: 14 人天

---

## 二、Phase 2：LLMOps 与可观测性（2 个月）

### 2.1 指标数据模型

**新增文件**: `src-tauri/crates/telemetry/src/metrics.rs`

```rust
pub struct AppMetrics {
    pub app_id: Uuid,
    pub date: DateTime<Utc>,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub total_tokens: u64,
    pub latency_ms: u64,
}
```

### 2.2 Prompt 模板管理

**目标**: 支持版本化的 Prompt 模板

| 功能 | 说明 |
|------|------|
| 模板版本控制 | 模板历史记录和回滚 |
| 模板变量验证 | JSON Schema 校验 |
| A/B 测试 | 多版本模板对比 |

### 2.3 工作流版本控制

**目标**: 支持工作流定义的版本化管理

```rust
pub struct WorkflowVersion {
    pub id: Uuid,
    pub workflow_id: Uuid,
    pub version: u32,
    pub definition: WorkflowDefinition,
    pub created_at: DateTime<Utc>,
    pub is_active: bool,
}
```

### 2.4 文件变更清单

| 操作 | 文件路径 |
|------|---------|
| 新增 | `src-tauri/crates/telemetry/src/metrics.rs` |
| 新增 | `src-tauri/crates/core/src/prompt_template.rs` |
| 修改 | `src-tauri/crates/core/src/entity/workflow_template.rs` |
| 新增 | `src/pages/PromptTemplatesPage.tsx` |

**总工作量**: 15 人天

---

## 三、Phase 3：高级特性（2 个月）

### 3.1 子工作流调用

- 支持异步子工作流调用
- 子工作流错误处理和超时控制
- 调用结果缓存

### 3.2 工作流市场

- 模板发布和发现
- 模板评分和评论
- 一键导入/导出

### 3.3 高级调试功能

- 执行历史可视化
- 变量状态快照
- 性能分析

### 3.4 文件变更清单

| 操作 | 文件路径 |
|------|---------|
| 修改 | `src-tauri/crates/runtime/src/work_engine/engine.rs` |
| 新增 | `src/pages/WorkflowMarketplace.tsx` |
| 新增 | `src/components/workflow/DebugPanel.tsx` |

**总工作量**: 20 人天

---

## 四、技术债务清理

| 项目 | 说明 | 优先级 |
|------|------|--------|
| 错误处理标准化 | 统一错误类型和传播 | P1 |
| 文档完善 | API 文档和使用指南 | P2 |
| 单元测试覆盖 | 关键模块测试覆盖率达到 80% | P2 |

---

## 五、总工作量估算

| Phase | 内容 | 人天 |
|-------|------|------|
| Phase 1 | 工作流与 RAG | 14 |
| Phase 2 | LLMOps 与可观测性 | 15 |
| Phase 3 | 高级特性 | 20 |
| 技术债务 | 代码清理和测试 | 10 |
| **总计** | | **59** |
