# Phase 4 实施计划：技术债务清理

> 时间周期：1 个月 | 总工作量：10 人天

## 4.1 任务概览

| 任务 | 工作量 | 优先级 | 依赖 |
|------|--------|--------|------|
| 错误处理标准化 | 4 人天 | P1 | - |
| 文档完善 | 3 人天 | P2 | - |
| 单元测试覆盖 | 3 人天 | P2 | - |

---

## 4.2 错误处理标准化

### 目标
统一错误类型定义和传播机制，建立清晰的错误层次结构。

### 现状分析

通过代码审查发现以下问题：

#### 1. 多处分散的错误类型定义

| 模块 | 文件 | 错误类型 |
|------|------|---------|
| core | `error.rs` | `AxAgentError`, `HealthCheckError` |
| runtime | `work_engine/node_executor_trait.rs` | `NodeError` |
| runtime | `lane_events.rs` | `LaneFailureClass` |
| agent | `error_classifier.rs` | `ErrorType`, `ClassifiedError` |
| agent | `error_recovery_engine.rs` | `RecoveryEvent`, `RecoveryContext` |
| agent | `retry_policy.rs` | 多处 Result<String, T> |
| trajectory | `atomic_skill/types.rs` | `AtomicSkillError` |
| telemetry | `span.rs` | `SpanError` |

#### 2. 问题清单

**问题 1: 错误信息不统一**
```rust
// 分散的错误定义方式
pub struct NodeError {
    pub error_type: String,
    pub message: String,
}

pub struct AtomicSkillError {
    pub error_type: String,
    pub message: String,
}

pub struct SpanError {
    pub error_type: String,
    pub message: String,
}
```

**问题 2: Result 类型不统一**
```rust
// 有的用 String 作为错误
async fn execute(...) -> Result<NodeOutput, String>

// 有的用自定义错误
async fn execute(...) -> Result<NodeOutput, NodeError>

// 有的用 thiserror
async fn execute(...) -> Result<T, AxAgentError>

// 有的用 anyhow
async fn execute(...) -> Result<T, anyhow::Error>
```

**问题 3: 错误传播链路不清晰**
- 很多地方使用 `?` 操作符直接传播
- 缺少错误上下文信息（哪个模块、哪个函数）
- 没有统一的错误日志记录

### 详细步骤

#### Step 1: 创建统一错误层次结构
**文件**: `src-tauri/crates/core/src/error.rs`

```rust
use thiserror::Error;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Error, Serialize, Deserialize)]
pub enum AxAgentError {
    #[error("Database error: {0}")]
    Database(#[from] sea_orm::DbErr),

    #[error("Provider error: {0}")]
    Provider(String),

    #[error("Gateway error: {0}")]
    Gateway(String),

    #[error("Crypto error: {0}")]
    Crypto(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Workflow error: {context}")]
    Workflow {
        #[source]
        source: Option<Box<AxAgentError>>,
        context: String,
    },

    #[error("Agent error: {context}")]
    Agent {
        #[source]
        source: Option<Box<AxAgentError>>,
        context: String,
    },

    #[error("Execution error: {context}")]
    Execution {
        #[source]
        source: Option<Box<AxAgentError>>,
        context: String,
    },

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Timeout: {0}")]
    Timeout(String),
}

pub type Result<T> = std::result::Result<T, AxAgentError>;

// Helper traits for error context
pub trait WithContext<T, E> {
    fn context<C: Into<String>>(self, ctx: C) -> Result<T>;
    fn with_context<F, C: Into<String>>(self, f: F) -> Result<T>
    where F: FnOnce() -> C;
}

impl<T, E: Into<AxAgentError>> WithContext<T, E> for Result<T, E> {
    fn context<C: Into<String>>(self, ctx: C) -> Result<T> {
        self.map_err(|e| {
            let err: AxAgentError = e.into();
            err.context(ctx)
        })
    }

    fn with_context<F, C: Into<String>>(self, f: F) -> Result<T>
    where F: FnOnce() -> C {
        self.map_err(|e| {
            let err: AxAgentError = e.into();
            err.context(f())
        })
    }
}
```

#### Step 2: 迁移 NodeError 到统一错误类型
**文件**: `src-tauri/crates/runtime/src/work_engine/node_executor_trait.rs`

```rust
// 旧代码
#[derive(Debug, Clone)]
pub struct NodeError {
    pub error_type: String,
    pub message: String,
}

impl NodeError {
    pub fn unsupported_node_type(msg: String) -> Self { ... }
    pub fn execution_failed(msg: String) -> Self { ... }
    pub fn timeout(msg: String) -> Self { ... }
}

// 新代码 - 使用 thiserror
#[derive(Debug, thiserror::Error)]
pub enum NodeError {
    #[error("Unsupported node type: {0}")]
    UnsupportedNodeType(String),

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Invalid node type: expected {expected}, got {got}")]
    InvalidNodeType { expected: String, got: String },

    #[error("Variable not found: {0}")]
    VariableNotFound(String),

    #[error("Workflow error: {0}")]
    Workflow(String),
}

pub type Result<T> = std::result::Result<T, NodeError>;
```

#### Step 3: 添加错误传播辅助函数
**文件**: `src-tauri/crates/core/src/error.rs`

```rust
impl AxAgentError {
    pub fn workflow<S: Into<String>>(context: S) -> Self {
        AxAgentError::Workflow {
            source: None,
            context: context.into(),
        }
    }

    pub fn workflow_with_source<E: Into<AxAgentError>>(context: String, source: E) -> Self {
        AxAgentError::Workflow {
            source: Some(Box::new(source.into())),
            context,
        }
    }

    pub fn agent<S: Into<String>>(context: S) -> Self {
        AxAgentError::Agent {
            source: None,
            context: context.into(),
        }
    }

    pub fn execution<S: Into<String>>(context: S) -> Self {
        AxAgentError::Execution {
            source: None,
            context: context.into(),
        }
    }

    pub fn add_context(mut self, ctx: String) -> Self {
        match self {
            AxAgentError::Workflow { source, context } => {
                AxAgentError::Workflow {
                    source,
                    context: format!("{}: {}", ctx, context),
                }
            }
            AxAgentError::Agent { source, context } => {
                AxAgentError::Agent {
                    source,
                    context: format!("{}: {}", ctx, context),
                }
            }
            AxAgentError::Execution { source, context } => {
                AxAgentError::Execution {
                    source,
                    context: format!("{}: {}", ctx, context),
                }
            }
            _ => self,
        }
    }
}
```

#### Step 4: 创建错误日志记录宏
**文件**: `src-tauri/crates/core/src/error.rs`

```rust
#[macro_export]
macro_rules! error_context {
    ($expr:expr, $ctx:expr) => {
        $expr.map_err(|e| {
            tracing::error!(error = %e, context = $ctx, "Error occurred");
            e
        })
    };
    ($expr:expr, $ctx:expr,) => { error_context!($expr, $ctx) };
}

#[macro_export]
macro_rules! anyhow_to_error {
    ($expr:expr) => {
        $expr.map_err(|e| AxAgentError::Execution {
            source: Some(Box::new(e.into())),
            context: String::new(),
        })
    };
}
```

### 验收标准
- [ ] 统一错误类型 `AxAgentError` 覆盖所有模块
- [ ] 关键函数添加错误上下文
- [ ] 消除所有 `Result<T, String>` 用法
- [ ] 添加错误日志记录

---

## 4.3 文档完善

### 目标
为公共 API 添加文档注释，建立代码文档生成流程。

### 现状分析

| 类别 | 当前状态 |
|------|---------|
| 公共 API 文档 | 无 `///` 注释 |
| 模块级文档 | 极少 |
| 示例代码 | 无 |
| API 文档生成 | 未配置 |

### 详细步骤

#### Step 1: 为核心模块添加文档
**文件**: `src-tauri/crates/core/src/lib.rs`

```rust
//! AxAgent Core Library
//!
//! This crate provides the core data types and functionality for the AxAgent application.
//!
//! # Main Modules
//!
//! - [`workflow_types`] - Workflow definition and execution types
//! - [`error`] - Unified error handling
//! - [`entity`] - Database entity definitions
//! - [`prompt_template`] - Prompt template management
//!
//! # Usage
//!
//! ```rust
//! use axagent_core::{Workflow, AxAgentError};
//!
//! fn example() -> Result<(), AxAgentError> {
//!     Ok(())
//! }
//! ```

pub mod error;
pub mod workflow_types;
// ...
```

#### Step 2: 为公共 API 添加文档
**文件**: `src-tauri/crates/core/src/workflow_types.rs`

```rust
/// Represents a node in a workflow graph.
///
/// # Examples
///
/// ```rust
/// use axagent_core::workflow_types::{WorkflowNode, LlmNode};
///
/// let node = WorkflowNode::Llm(LlmNode {
///     id: "llm_1".to_string(),
///     name: "Chat LLM".to_string(),
///     config: Default::default(),
/// });
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkflowNode {
    /// Trigger node that starts the workflow
    Trigger(TriggerNode),
    /// LLM node for AI interactions
    Llm(LlmNode),
    // ...
}
```

#### Step 3: 配置文档生成
**文件**: `src-tauri/crates/core/Cargo.toml`

```toml
[package]
name = "axagent-core"
version = "1.0.0"
edition = "2021"
description = "Core library for AxAgent"
repository = "https://github.com/your-org/axagent"
documentation = "https://docs.axagent.dev/core"

[package.metadata.docs.rs]
all-features = true
rustdoc-args = ["--cfg", "docsrs"]
```

#### Step 4: 创建文档构建脚本
**文件**: `docs/build.sh`

```bash
#!/bin/bash
set -e

echo "Building documentation..."

# Build Rust documentation
cd src-tauri
cargo doc --no-deps --all-features

# Copy to docs folder
cp -r target/doc ../docs/rust-api/

echo "Documentation built successfully!"
```

### 验收标准
- [ ] 核心模块添加模块级文档
- [ ] 公共 API 添加函数文档
- [ ] 配置 cargo doc 生成
- [ ] 创建文档构建脚本

---

## 4.4 单元测试覆盖

### 目标
提高关键模块的测试覆盖率至 80%，建立持续集成测试流程。

### 现状分析

| 类别 | 当前状态 |
|------|---------|
| 测试模块数量 | 100 个 `#[cfg(test)]` |
| 集成测试 | 1 个文件 |
| 前端测试 | ~30 个测试文件 |
| 覆盖率工具 | 未配置 |
| CI 测试 | 基础 cargo test |

### 详细步骤

#### Step 1: 配置覆盖率工具
**文件**: `src-tauri/.github/workflows/test.yml` (新建)

```yaml
name: Tests

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Setup Rust cache
        uses: Swatinem/rust-cache@v2

      - name: Run tests with coverage
        uses: actions-rs/tarpaulin@v0.1
        with:
          version: "0.30.0"
          args: "--workspace --out Xml --ignored"

      - name: Upload coverage to Codecov
        uses: codecov/codecov-action@v3
        with:
          files: ./cobertura.xml
          flags: unittests
```

#### Step 2: 添加核心模块测试
**文件**: `src-tauri/crates/core/src/workflow_types.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_node_serialization() {
        let node = WorkflowNode::Llm(LlmNode {
            id: "test".to_string(),
            name: "Test LLM".to_string(),
            config: LlmNodeConfig::default(),
        });

        let json = serde_json::to_string(&node).unwrap();
        let deserialized: WorkflowNode = serde_json::from_str(&json).unwrap();

        match deserialized {
            WorkflowNode::Llm(llm) => assert_eq!(llm.id, "test"),
            _ => panic!("Expected Llm node"),
        }
    }

    #[test]
    fn test_workflow_validation() {
        let workflow = WorkflowTemplateData {
            id: "wf-1".to_string(),
            // ... set up valid workflow
            nodes: vec![],
            edges: vec![],
            // ...
        };

        assert!(workflow.validate().is_ok());
    }
}
```

#### Step 3: 添加错误处理测试
**文件**: `src-tauri/crates/core/src/error.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_context() {
        let err = AxAgentError::NotFound("User not found".to_string());
        let err_with_ctx = err.add_context("get_user".to_string());

        assert!(err_with_ctx.to_string().contains("get_user"));
    }

    #[test]
    fn test_workflow_error() {
        let err = AxAgentError::workflow("Invalid node configuration");
        assert!(matches!(err, AxAgentError::Workflow { .. }));
    }

    #[test]
    fn test_error_serialization() {
        let err = AxAgentError::Validation("Field is required".to_string());
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("validation"));
    }
}
```

#### Step 4: 添加 executor 测试
**文件**: `src-tauri/crates/runtime/src/work_engine/executors/subworkflow_executor.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_key_computation() {
        let executor = SubWorkflowExecutor::new(SubWorkflowExecutorConfig::default());
        let node = SubWorkflowNode {
            id: "sub-1".to_string(),
            name: "Test SubWorkflow".to_string(),
            config: SubWorkflowNodeConfig {
                workflow_id: "wf-123".to_string(),
                input_mapping: HashMap::new(),
                output_mapping: HashMap::new(),
                timeout_secs: Some(300),
                retry_on_failure: true,
                max_retries: 3,
                cache_enabled: true,
                cache_ttl_secs: 3600,
            },
        };
        let context = ExecutionState::new("exec-1".to_string());

        let key = executor.compute_cache_key(&node, &context);
        assert!(!key.is_empty());
        assert!(key.contains("wf-123"));
    }

    #[tokio::test]
    async fn test_input_mapping() {
        let executor = SubWorkflowExecutor::new(SubWorkflowExecutorConfig::default());
        let node = SubWorkflowNode {
            id: "sub-1".to_string(),
            name: "Test".to_string(),
            config: SubWorkflowNodeConfig {
                workflow_id: "wf-1".to_string(),
                input_mapping: HashMap::from([
                    ("target_var".to_string(), "source_var".to_string()),
                ]),
                output_mapping: HashMap::new(),
                timeout_secs: None,
                retry_on_failure: false,
                max_retries: 0,
                cache_enabled: false,
                cache_ttl_secs: 0,
            },
        };

        let mut variables = HashMap::new();
        variables.insert("source_var".to_string(), json!("value"));

        let context = ExecutionState {
            variables,
            ..Default::default()
        };

        let result = executor.map_inputs(&node, &context);
        assert!(result.is_ok());
        let mapped = result.unwrap();
        assert!(mapped.contains_key("target_var"));
    }
}
```

#### Step 5: 添加前端组件测试
**文件**: `src/components/workflow/__tests__/DebugPanel.test.tsx`

```typescript
import { render, screen } from '@testing-library/react';
import { DebugPanel } from '../DebugPanel';

describe('DebugPanel', () => {
  const mockTrace = {
    id: 'trace-1',
    workflowId: 'wf-1',
    workflowName: 'Test Workflow',
    status: 'completed' as const,
    startTime: Date.now() - 5000,
    endTime: Date.now(),
    duration: 5000,
    nodeExecutions: [],
    variableSnapshots: [],
    triggerType: 'manual',
  };

  it('renders debug panel header', () => {
    render(<DebugPanel trace={mockTrace} />);
    expect(screen.getByText('Debug Panel')).toBeInTheDocument();
  });

  it('shows execution status', () => {
    render(<DebugPanel trace={mockTrace} />);
    expect(screen.getByText('completed')).toBeInTheDocument();
  });
});
```

### 验收标准
- [ ] 配置 tarpaulin 覆盖率工具
- [ ] 核心模块测试覆盖率 > 80%
- [ ] 添加 10+ 个新的测试用例
- [ ] 前端组件测试覆盖新增组件

---

## 4.5 风险评估

| 风险 | 影响 | 缓解措施 |
|------|------|---------|
| 错误类型重构影响范围大 | 高 | 分模块逐步迁移，保留旧类型别名 |
| 文档编写耗时 | 中 | 优先文档化公共 API |
| 测试覆盖率目标过高 | 中 | 分阶段目标，优先核心模块 |

---

## 4.6 验收清单

### 错误处理标准化
- [ ] `AxAgentError` 扩展支持 Workflow/Agent/Execution 错误
- [ ] `NodeError` 迁移到 thiserror
- [ ] 添加 `WithContext` trait
- [ ] 添加错误传播宏
- [ ] 消除 `Result<T, String>` 用法

### 文档完善
- [ ] 核心模块添加 `//!` 文档
- [ ] 公共函数添加 `///` 文档
- [ ] 配置 `cargo doc`
- [ ] 创建文档构建脚本

### 单元测试覆盖
- [ ] 配置 tarpaulin 覆盖率跟踪
- [ ] 添加 error.rs 测试
- [ ] 添加 workflow_types.rs 测试
- [ ] 添加 subworkflow_executor.rs 测试
- [ ] 添加前端 DebugPanel 测试
- [ ] 覆盖率报告生成
