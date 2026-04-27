# Phase 1 实施计划：工作流与 RAG

> 时间周期：2 个月 | 总工作量：14 人天
>
> **状态**: ✅ 已完成

## 1.1 任务概览

| 任务 | 工作量 | 优先级 | 状态 |
|------|--------|--------|------|
| NodeExecutor Trait 重构 | 8 人天 | P0 | ✅ 已完成 |
| 前端工作流编辑器增强 | 4 人天 | P1 | ✅ 已完成 |
| Prometheus Metrics 端点 | 2 人天 | P2 | ✅ 已完成 |

---

## 1.2 NodeExecutor Trait 重构

### 状态: ✅ 已完成

#### 新增文件

| 文件路径 | 说明 |
|---------|------|
| `src-tauri/crates/runtime/src/work_engine/node_executor_trait.rs` | Trait 定义，包含 `NodeExecutorTrait`、`NodeOutput`、`NodeError` |
| `src-tauri/crates/runtime/src/work_engine/dispatcher.rs` | 节点调度器，管理所有执行器 |
| `src-tauri/crates/runtime/src/work_engine/executors/mod.rs` | 执行器模块入口 |
| `src-tauri/crates/runtime/src/work_engine/executors/llm_executor.rs` | LLM 节点执行器 |
| `src-tauri/crates/runtime/src/work_engine/executors/agent_executor.rs` | Agent 节点执行器 |
| `src-tauri/crates/runtime/src/work_engine/executors/atomic_skill_executor.rs` | AtomicSkill 节点执行器 |

#### 修改文件

| 文件路径 | 说明 |
|---------|------|
| `src-tauri/crates/runtime/src/work_engine/mod.rs` | 导出新模块 |
| `src-tauri/crates/runtime/src/work_engine/node_executor.rs` | 改为使用 trait-based 设计 |
| `src-tauri/crates/runtime/Cargo.toml` | 添加 `thiserror` 依赖 |

#### 核心设计

```rust
#[async_trait]
pub trait NodeExecutorTrait: Send + Sync {
    fn node_type(&self) -> &'static str;
    async fn execute(
        &self,
        node: &WorkflowNode,
        context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError>;
}
```

---

## 1.3 前端工作流编辑器增强

### 状态: ✅ 已完成

#### 新增文件

| 文件路径 | 说明 |
|---------|------|
| `src/hooks/useWorkflowShortcuts.ts` | 键盘快捷键管理 Hook |
| `src/hooks/useUndoRedo.ts` | 撤销/重做功能 Hook |

#### 快捷键支持

| 快捷键 | 功能 |
|--------|------|
| Ctrl+Z | 撤销 |
| Ctrl+Y / Ctrl+Shift+Z | 重做 |
| Ctrl+F | 搜索节点 |
| Delete / Backspace | 删除选中 |
| Ctrl+C / Ctrl+V | 复制/粘贴 |
| Ctrl+A | 全选 |

---

## 1.4 Prometheus Metrics 端点

### 状态: ✅ 已完成

#### 新增文件

| 文件路径 | 说明 |
|---------|------|
| `src-tauri/crates/gateway/src/metrics.rs` | Metrics 处理函数 |

#### 修改文件

| 文件路径 | 说明 |
|---------|------|
| `src-tauri/crates/gateway/src/lib.rs` | 添加 `metrics` 模块 |
| `src-tauri/crates/gateway/src/routes.rs` | 添加 `/metrics` 路由 |
| `src-tauri/crates/gateway/Cargo.toml` | 添加 `prometheus = "0.13"` 依赖 |

#### 端点

| 端点 | 方法 | 说明 |
|------|------|------|
| `/metrics` | GET | Prometheus 格式指标 |

---

## 1.5 里程碑

| 里程碑 | 日期 | 完成内容 |
|--------|------|----------|
| M1 | 第 1 周 | ✅ NodeExecutor Trait 定义和 WorkflowContext |
| M2 | 第 2-3 周 | ✅ 各节点执行器实现，迁移现有代码 |
| M3 | 第 4 周 | ✅ 前端搜索/撤销/重做功能 |
| M4 | 第 5 周 | ✅ Prometheus Metrics 端点 |
| M5 | 第 6-8 周 | (预留测试/Bug修复时间) |

---

## 1.6 文件变更清单

```
src-tauri/crates/runtime/src/work_engine/
├── node_executor_trait.rs    # 新增: Trait 定义
├── dispatcher.rs             # 新增: 调度器
├── executors/
│   ├── mod.rs               # 新增: 模块入口
│   ├── llm_executor.rs      # 新增
│   ├── agent_executor.rs    # 新增
│   └── atomic_skill_executor.rs  # 新增
└── node_executor.rs         # 修改: 改为使用 trait

src-tauri/crates/gateway/src/
├── metrics.rs               # 新增
└── lib.rs                   # 修改

src/hooks/
├── useWorkflowShortcuts.ts  # 新增
└── useUndoRedo.ts          # 新增
```
