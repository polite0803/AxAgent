# Batch 3: 多智能体协作 + DAG 工作流鲁棒性 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** SharedBlackboard 解决多 Agent 上下文碎片化 + 冲突投票仲裁；DAG 工作流补齐降级节点、持久化、差异化超时和 Schema 校验

**Architecture:** 新增 `shared_blackboard` 模块提供全局工作记忆和简单多数投票冲突解决；扩展 `WorkflowStep` 结构体增加降级/超时/Schema 字段；利用已有 Sea-ORM 框架新增 `workflow_snapshots` 表持久化工作流状态

**Tech Stack:** Rust 2021, Sea-ORM, serde_json, jsonschema

**Spec:** `docs/superpowers/specs/2026-05-13-batch3-multiagent-workflow-design.md`

---

## 文件结构总览

```
新增:
  src-tauri/crates/agent/src/shared_blackboard.rs
  src-tauri/crates/core/src/entity/workflow_snapshots.rs
  src-tauri/crates/migration/src/m20250513_000001_workflow_snapshots.rs

修改:
  src-tauri/crates/agent/src/lib.rs
  src-tauri/crates/agent/src/session_manager.rs
  src-tauri/crates/rt-messaging/src/message_gateway.rs
  src-tauri/crates/rt-workflow/src/workflow_engine.rs
  src-tauri/crates/migration/src/lib.rs
```

---

### Task 1: 创建 SharedBlackboard 模块

**Files:**
- Create: `src-tauri/crates/agent/src/shared_blackboard.rs`

- [ ] **Step 1: 编写 shared_blackboard.rs**

```rust
//! 多 Agent 协作的全局工作记忆（Blackboard 模式）。
//!
//! 提供共享状态、决策记录、冲突仲裁和消息广播功能。

use std::collections::HashMap;

/// Agent 的一次决策
#[derive(Debug, Clone)]
pub struct AgentDecision {
    pub agent_id: String,
    pub timestamp_ms: u64,
    pub task_id: String,
    pub field: String,
    pub value: String,
}

/// Blackboard 消息
#[derive(Debug, Clone)]
pub struct BlackboardMessage {
    pub from: String,
    pub to: Option<String>,
    pub content: String,
    pub timestamp_ms: u64,
}

/// 冲突解决结果
#[derive(Debug, Clone)]
pub enum ConflictResolution {
    MajorityVote { winner: String, vote_count: usize },
    TieBreak { chosen: String, reason: String },
}

/// 冲突记录
#[derive(Debug, Clone)]
pub struct ConflictRecord {
    pub task_id: String,
    pub field: String,
    pub conflicting_decisions: Vec<AgentDecision>,
    pub resolution: ConflictResolution,
}

/// 多 Agent 协作的全局工作记忆
pub struct SharedBlackboard {
    pub task_id: String,
    pub goal: String,
    pub shared_state: HashMap<String, String>,
    pub decisions: Vec<AgentDecision>,
    pub messages: Vec<BlackboardMessage>,
    pub conflicts: Vec<ConflictRecord>,
}

impl SharedBlackboard {
    /// 创建新的 Blackboard
    pub fn new(task_id: impl Into<String>, goal: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
            goal: goal.into(),
            shared_state: HashMap::new(),
            decisions: Vec::new(),
            messages: Vec::new(),
            conflicts: Vec::new(),
        }
    }

    /// 记录 Agent 决策
    pub fn record_decision(
        &mut self,
        agent_id: &str,
        task_id: &str,
        field: &str,
        value: &str,
    ) {
        let decision = AgentDecision {
            agent_id: agent_id.to_string(),
            timestamp_ms: now_ms(),
            task_id: task_id.to_string(),
            field: field.to_string(),
            value: value.to_string(),
        };
        self.decisions.push(decision);
    }

    /// 设置共享状态
    pub fn set_state(&mut self, key: &str, value: &str) {
        self.shared_state.insert(key.to_string(), value.to_string());
    }

    /// 读取共享状态
    pub fn get_state(&self, key: &str) -> Option<&String> {
        self.shared_state.get(key)
    }

    /// 获取所有 Agent 对某个 field 的共识值
    pub fn get_consensus(&self, field: &str) -> Option<String> {
        let relevant: Vec<&AgentDecision> = self
            .decisions
            .iter()
            .filter(|d| d.field == field)
            .collect();
        if relevant.is_empty() {
            return None;
        }
        // 找出现次数最多的值
        let mut votes: HashMap<&str, usize> = HashMap::new();
        for d in &relevant {
            *votes.entry(&d.value).or_default() += 1;
        }
        votes
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(value, _)| value.to_string())
    }

    /// 检测并解决冲突
    pub fn resolve_conflicts(&mut self) -> Vec<ConflictRecord> {
        let mut records = Vec::new();
        let mut groups: HashMap<(String, String), Vec<&AgentDecision>> = HashMap::new();
        for d in &self.decisions {
            groups
                .entry((d.task_id.clone(), d.field.clone()))
                .or_default()
                .push(d);
        }
        for ((task_id, field), decisions) in groups {
            if decisions.len() < 2 {
                continue;
            }
            let mut votes: HashMap<&str, usize> = HashMap::new();
            for d in &decisions {
                *votes.entry(&d.value).or_default() += 1;
            }
            let max_votes = votes.values().max().copied().unwrap_or(0);
            let winners: Vec<&&str> = votes
                .iter()
                .filter(|(_, &c)| c == max_votes)
                .map(|(v, _)| v)
                .collect();
            let resolution = if winners.len() == 1 {
                ConflictResolution::MajorityVote {
                    winner: winners[0].to_string(),
                    vote_count: max_votes,
                }
            } else {
                let first = decisions
                    .iter()
                    .min_by_key(|d| d.timestamp_ms)
                    .unwrap();
                ConflictResolution::TieBreak {
                    chosen: first.value.clone(),
                    reason: "平局，选择首个完成者的决策".to_string(),
                }
            };
            records.push(ConflictRecord {
                task_id,
                field,
                conflicting_decisions: decisions.iter().map(|&d| d.clone()).collect(),
                resolution,
            });
        }
        self.conflicts.extend(records.clone());
        records
    }

    /// 广播消息到所有 Agent
    pub fn broadcast(&mut self, from: &str, content: &str) {
        self.messages.push(BlackboardMessage {
            from: from.to_string(),
            to: None,
            content: content.to_string(),
            timestamp_ms: now_ms(),
        });
    }

    /// 获取发给特定 Agent 的消息（含广播）
    pub fn get_messages_for(&self, agent_id: &str) -> Vec<&BlackboardMessage> {
        self.messages
            .iter()
            .filter(|m| m.to.is_none() || m.to.as_deref() == Some(agent_id))
            .collect()
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_and_read_decision() {
        let mut bb = SharedBlackboard::new("task-1", "test goal");
        bb.record_decision("agent-a", "task-1", "next_action", "call_api");
        assert_eq!(bb.decisions.len(), 1);
        assert_eq!(bb.decisions[0].value, "call_api");
    }

    #[test]
    fn consensus_returns_majority_value() {
        let mut bb = SharedBlackboard::new("task-1", "test");
        bb.record_decision("a", "task-1", "result", "A");
        bb.record_decision("b", "task-1", "result", "A");
        bb.record_decision("c", "task-1", "result", "B");
        assert_eq!(bb.get_consensus("result"), Some("A".to_string()));
    }

    #[test]
    fn conflict_majority_vote_wins() {
        let mut bb = SharedBlackboard::new("task-1", "test");
        bb.record_decision("a", "task-1", "action", "deploy");
        bb.record_decision("b", "task-1", "action", "deploy");
        bb.record_decision("c", "task-1", "action", "rollback");
        let records = bb.resolve_conflicts();
        assert_eq!(records.len(), 1);
        match &records[0].resolution {
            ConflictResolution::MajorityVote { winner, vote_count } => {
                assert_eq!(winner, "deploy");
                assert_eq!(*vote_count, 2);
            },
            _ => panic!("expected MajorityVote"),
        }
    }

    #[test]
    fn conflict_tiebreak_chooses_first() {
        let mut bb = SharedBlackboard::new("task-1", "test");
        bb.record_decision("a", "task-1", "action", "X");
        std::thread::sleep(std::time::Duration::from_millis(2));
        bb.record_decision("b", "task-1", "action", "Y");
        let records = bb.resolve_conflicts();
        assert_eq!(records.len(), 1);
        match &records[0].resolution {
            ConflictResolution::TieBreak { chosen, .. } => {
                assert_eq!(chosen, "X"); // a 先决策
            },
            _ => panic!("expected TieBreak"),
        }
    }

    #[test]
    fn broadcast_and_receive() {
        let mut bb = SharedBlackboard::new("task-1", "test");
        bb.broadcast("agent-a", "hello all");
        let msgs = bb.get_messages_for("agent-b");
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].content, "hello all");
    }

    #[test]
    fn shared_state_read_write() {
        let mut bb = SharedBlackboard::new("task-1", "test");
        bb.set_state("status", "in_progress");
        assert_eq!(bb.get_state("status"), Some(&"in_progress".to_string()));
    }
}
```

- [ ] **Step 2: 编译 + 测试**

Run: `cargo test -p axagent-agent -- shared_blackboard`
Expected: 6 tests PASS

- [ ] **Step 3: Commit**

```bash
git add src-tauri/crates/agent/src/shared_blackboard.rs
git commit -m "feat: 新增 SharedBlackboard 多 Agent 全局工作记忆模块"
```

---

### Task 2: 注册 Blackboard + AgentSession 集成

**Files:**
- Modify: `src-tauri/crates/agent/src/lib.rs`
- Modify: `src-tauri/crates/agent/src/session_manager.rs`

- [ ] **Step 1: lib.rs 添加模块声明**

在 `src-tauri/crates/agent/src/lib.rs` 中找到 `pub mod` 区域，添加：
```rust
pub mod shared_blackboard;
```
并添加 re-export：
```rust
pub use shared_blackboard::SharedBlackboard;
```

- [ ] **Step 2: session_manager.rs 附加 Blackboard**

在 `AgentSession` 结构体中添加字段：
```rust
use std::sync::{Arc, RwLock};
use crate::shared_blackboard::SharedBlackboard;

pub struct AgentSession {
    // ... 现有字段 ...
    pub blackboard: Option<Arc<RwLock<SharedBlackboard>>>,
}
```

在 `AgentSession::new` 中初始化为 `None`：
```rust
blackboard: None,
```

添加方法：
```rust
impl AgentSession {
    /// 为此会话附加一个协作 Blackboard
    pub fn with_blackboard(mut self, bb: Arc<RwLock<SharedBlackboard>>) -> Self {
        self.blackboard = Some(bb);
        self
    }

    /// 记录此 Agent 的决策到 Blackboard
    pub fn record_to_blackboard(&self, field: &str, value: &str) {
        if let Some(ref bb) = self.blackboard {
            if let Ok(mut board) = bb.write() {
                board.record_decision(
                    &self.axagent_session_id.clone().unwrap_or_default(),
                    &self.conversation_id,
                    field,
                    value,
                );
            }
        }
    }
}
```

- [ ] **Step 3: 编译验证**

Run: `cargo check -p axagent-agent`
Expected: 编译成功

- [ ] **Step 4: Commit**

```bash
git add src-tauri/crates/agent/src/lib.rs src-tauri/crates/agent/src/session_manager.rs
git commit -m "feat: AgentSession 集成 SharedBlackboard 引用 + with_blackboard 方法"
```

---

### Task 3: MessageGateway 增加 BlackboardSync 消息类型

**Files:**
- Modify: `src-tauri/crates/rt-messaging/src/message_gateway.rs`

- [ ] **Step 1: 在 MessagePayload 枚举中添加 BlackboardSync**

在 `MessagePayload` enum 中添加新变体（在 `Error` 变体之后）：
```rust
    /// Blackboard 状态同步消息
    BlackboardSync {
        task_id: String,
        shared_state: HashMap<String, String>,
        from_agent: String,
    },
```

- [ ] **Step 2: 编译验证**

Run: `cargo check -p axagent-rt-messaging`
Expected: 编译成功

- [ ] **Step 3: Commit**

```bash
git add src-tauri/crates/rt-messaging/src/message_gateway.rs
git commit -m "feat: MessagePayload 增加 BlackboardSync 消息类型"
```

---

### Task 4: WorkflowStep 增加降级 + 超时 + Schema 字段

**Files:**
- Modify: `src-tauri/crates/rt-workflow/src/workflow_engine.rs`

- [ ] **Step 1: 在 WorkflowStep 结构体中添加三个新字段**

在 `WorkflowStep` 结构体最后（`agent_role_override` 之后）添加：
```rust
    /// 降级步骤 ID：当前步骤失败后可自动执行的替代步骤
    #[serde(default)]
    pub fallback_step_id: Option<String>,
    /// 步骤超时（秒），覆盖全局 step_timeout
    #[serde(default)]
    pub timeout_secs: Option<u64>,
    /// 输出 JSON Schema，用于校验步骤输出格式
    #[serde(default)]
    pub expected_output_schema: Option<String>,
```

- [ ] **Step 2: 更新 Default impl**

在 `Default for WorkflowStep` 中添加：
```rust
fallback_step_id: None,
timeout_secs: None,
expected_output_schema: None,
```

- [ ] **Step 3: 编译验证**

Run: `cargo check -p axagent-rt-workflow`
Expected: 编译成功

- [ ] **Step 4: Commit**

```bash
git add src-tauri/crates/rt-workflow/src/workflow_engine.rs
git commit -m "feat: WorkflowStep 增加 fallback_step_id + timeout_secs + expected_output_schema"
```

---

### Task 5: 降级节点 + 差异化超时逻辑

**Files:**
- Modify: `src-tauri/crates/rt-workflow/src/workflow_engine.rs`

- [ ] **Step 1: 修改 WorkflowRunner.run() — 差异化超时**

找到步骤超时的代码块（约 line 768），将：
```rust
let step_timeout = self.step_timeout;
```
改为：
```rust
let step_timeout = step.timeout_secs
    .map(Duration::from_secs)
    .unwrap_or(self.step_timeout);
```
注意需要先获取 step 后再读取 timeout_secs。

- [ ] **Step 2: 修改 Abort 分支 — 降级节点**

在 `WorkflowRunner::run()` 中，找到 `OnStepFailure::Abort` 分支（约 line 962），替换为：
```rust
OnStepFailure::Abort => {
    let fallback_id = {
        let workflows = self.engine.workflows.read().ok();
        workflows.and_then(|w| {
            w.get(workflow_id).and_then(|wf| {
                wf.steps.iter().find(|s| s.id == outcome.step_id)
                    .and_then(|s| s.fallback_step_id.clone())
            })
        })
    };
    if let Some(ref fb_id) = fallback_id {
        // 激活降级节点
        self.engine.update_step_status(
            workflow_id, fb_id, StepStatus::Ready, None, None,
        ).ok();
        self.engine.update_step_status(
            workflow_id, &outcome.step_id,
            StepStatus::Skipped, None, Some(e),
        ).ok();
    } else {
        self.engine.update_step_status(
            workflow_id, &outcome.step_id,
            StepStatus::Failed, None, Some(e),
        ).ok();
    }
},
```

- [ ] **Step 3: 运行测试**

Run: `cargo test -p axagent-rt-workflow`
Expected: 所有测试 PASS

- [ ] **Step 4: Commit**

```bash
git add src-tauri/crates/rt-workflow/src/workflow_engine.rs
git commit -m "feat: 降级节点 fallback_step_id + 差异化步骤超时 timeout_secs"
```

---

### Task 6: Workflow 快照持久化

**Files:**
- Create: `src-tauri/crates/core/src/entity/workflow_snapshots.rs`
- Create: `src-tauri/crates/migration/src/m20250513_000001_workflow_snapshots.rs`
- Modify: `src-tauri/crates/migration/src/lib.rs`

- [ ] **Step 1: 创建 entity**

```rust
// crates/core/src/entity/workflow_snapshots.rs
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "workflow_snapshots")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub workflow_id: String,
    pub snapshot_json: String,
    pub created_at: i64,
    pub step_id: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
```

在 `src-tauri/crates/core/src/entity/mod.rs` 中添加 `pub mod workflow_snapshots;`。

- [ ] **Step 2: 创建 migration**

```rust
// crates/migration/src/m20250513_000001_workflow_snapshots.rs
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(WorkflowSnapshots::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(WorkflowSnapshots::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(WorkflowSnapshots::WorkflowId).string().not_null())
                    .col(ColumnDef::new(WorkflowSnapshots::SnapshotJson).text().not_null())
                    .col(ColumnDef::new(WorkflowSnapshots::CreatedAt).big_integer().not_null())
                    .col(ColumnDef::new(WorkflowSnapshots::StepId).string().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(WorkflowSnapshots::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum WorkflowSnapshots {
    Table,
    Id,
    WorkflowId,
    SnapshotJson,
    CreatedAt,
    StepId,
}
```

- [ ] **Step 3: 注册 migration**

在 `migration/src/lib.rs` 中：
1. 添加 `mod m20250513_000001_workflow_snapshots;`
2. 在 `migrations()` 方法的 vec 中添加 `Box::new(m20250513_000001_workflow_snapshots::Migration),`

- [ ] **Step 4: 在 WorkflowEngine 中添加 checkpoint 方法**

在 `workflow_engine.rs` 的 `WorkflowEngine` impl 中：

```rust
/// 将当前工作流状态序列化为 JSON
pub fn serialize_workflow(&self, workflow_id: &str) -> Result<String, WorkflowError> {
    let workflows = self.workflows.read().map_err(|_| WorkflowError::LockError)?;
    let wf = workflows.get(workflow_id).ok_or(WorkflowError::WorkflowNotFound)?;
    serde_json::to_string(wf).map_err(|e| WorkflowError::SerializationError(e.to_string()))
}
```

在 `WorkflowError` 中添加：
```rust
SerializationError(String),
```

- [ ] **Step 5: 编译 + 测试**

Run: `cargo check` from `src-tauri/`
Run: `cargo test -p axagent-migration`
Expected: 编译成功，migration 测试 PASS

- [ ] **Step 6: Commit**

```bash
git add src-tauri/crates/core/src/entity/workflow_snapshots.rs \
        src-tauri/crates/core/src/entity/mod.rs \
        src-tauri/crates/migration/src/m20250513_000001_workflow_snapshots.rs \
        src-tauri/crates/migration/src/lib.rs \
        src-tauri/crates/rt-workflow/src/workflow_engine.rs
git commit -m "feat: workflow_snapshots 持久化表 + WorkflowEngine checkpoint 方法"
```

---

### Task 7: 全量编译 + 测试 + lint 验证

- [ ] **Step 1: 全量编译**

Run: `cargo check --all-targets` from `src-tauri/`

- [ ] **Step 2: 运行所有相关测试**

```
cargo test -p axagent-agent -- shared_blackboard
cargo test -p axagent-rt-workflow
cargo test -p axagent-rt-messaging
cargo test -p axagent-migration
```

- [ ] **Step 3: cargo fmt + clippy**

```
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
```

- [ ] **Step 4: 修复并提交**

```bash
git add -A
git commit -m "chore: Batch 3 全量编译 + 测试 + lint 验证通过"
```
