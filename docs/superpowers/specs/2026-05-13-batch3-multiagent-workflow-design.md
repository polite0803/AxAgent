# Batch 3: 多智能体协作 + DAG 工作流鲁棒性 设计文档

> SharedBlackboard + 冲突投票 + 降级节点 + 持久化 + Schema 校验
> 日期：2026-05-13 | 状态：待实现 | 批次：3/3

## 1. 背景与目标

审计发现两个子系统的架构缺陷：

**多智能体协作：**
- 上下文碎片化：Agent 独立上下文，信息手动传递，极易丢失关键状态
- 决策冲突：分工边界模糊时多 Agent 对同一任务做出矛盾操作
- 通信协议简陋：无标准化消息总线，全靠 LLM 自然语言协商
- 长任务上下文衰减：缺少全局工作记忆

**DAG 工作流鲁棒性：**
- 无降级/fallback 节点：步骤失败只能 Abort/Skip
- 无分支回滚机制
- 状态仅在内存 HashMap，进程重启丢失
- 步骤超时全局统一（300s），不可按步骤差异化
- 无输出格式校验

目标：用最小改动补齐关键缺口，不引入重量级框架。

## 2. Part A: 多智能体协作

### 2.1 SharedBlackboard — 全局工作记忆

新增 `crates/agent/src/shared_blackboard.rs`：

```rust
/// 多 Agent 协作的全局工作记忆
pub struct SharedBlackboard {
    /// 协作任务 ID
    pub task_id: String,
    /// 原始任务目标
    pub goal: String,
    /// 共享键值状态（Agent 可读写）
    pub shared_state: HashMap<String, String>,
    /// 所有 Agent 的决策记录
    pub decisions: Vec<AgentDecision>,
    /// Agent 间消息
    pub messages: Vec<BlackboardMessage>,
    /// 冲突与仲裁记录
    pub conflicts: Vec<ConflictRecord>,
}

/// Agent 的一次决策
pub struct AgentDecision {
    pub agent_id: String,
    pub timestamp_ms: u64,
    pub task_id: String,
    pub field: String,    // 决策域（如 "next_action", "output"）
    pub value: String,    // 决策值
}

/// Blackboard 消息
pub struct BlackboardMessage {
    pub from: String,
    pub to: Option<String>,  // None = 广播
    pub content: String,
    pub timestamp_ms: u64,
}

/// 冲突记录
pub struct ConflictRecord {
    pub task_id: String,
    pub field: String,
    pub conflicting_decisions: Vec<AgentDecision>,
    pub resolution: ConflictResolution,
}

pub enum ConflictResolution {
    /// 多数票获胜
    MajorityVote { winner: String, vote_count: usize },
    /// 平局，选择首个完成者
    TieBreak { chosen: String, reason: String },
}

impl SharedBlackboard {
    /// 记录 Agent 决策
    pub fn record_decision(&mut self, agent_id: &str, task_id: &str, field: &str, value: &str);

    /// 检测并解决冲突（简单多数投票）
    pub fn resolve_conflicts(&mut self) -> Vec<ConflictRecord>;

    /// 获取共识值（所有 Agent 同意的值）
    pub fn get_consensus(&self, field: &str) -> Option<String>;

    /// 广播消息到所有 Agent
    pub fn broadcast(&mut self, from: &str, content: &str);

    /// 获取发给特定 Agent 的消息
    pub fn get_messages_for(&self, agent_id: &str) -> Vec<&BlackboardMessage>;
}
```

### 2.2 冲突解决：简单多数投票

```rust
pub fn resolve_conflicts(&mut self) -> Vec<ConflictRecord> {
    let mut records = Vec::new();
    // 按 (task_id, field) 分组所有决策
    let mut groups: HashMap<(String, String), Vec<&AgentDecision>> = HashMap::new();
    for d in &self.decisions {
        groups.entry((d.task_id.clone(), d.field.clone())).or_default().push(d);
    }
    for ((task_id, field), decisions) in groups {
        if decisions.len() < 2 { continue; }
        // 统计投票
        let mut votes: HashMap<&str, usize> = HashMap::new();
        for d in &decisions {
            *votes.entry(&d.value).or_default() += 1;
        }
        let max_votes = votes.values().max().copied().unwrap_or(0);
        let winners: Vec<&&str> = votes.iter()
            .filter(|(_, &c)| c == max_votes)
            .map(|(v, _)| v).collect();
        let resolution = if winners.len() == 1 {
            ConflictResolution::MajorityVote {
                winner: winners[0].to_string(),
                vote_count: max_votes,
            }
        } else {
            // 平局 → 选第一个做决策的 Agent 的方案
            let first = decisions.iter().min_by_key(|d| d.timestamp_ms).unwrap();
            ConflictResolution::TieBreak {
                chosen: first.value.clone(),
                reason: "平局，选择首个完成者的决策".to_string(),
            }
        };
        records.push(ConflictRecord {
            task_id, field,
            conflicting_decisions: decisions.iter().map(|&d| d.clone()).collect(),
            resolution,
        });
    }
    records
}
```

### 2.3 集成点

| 文件 | 变更 |
|------|------|
| `crates/agent/src/shared_blackboard.rs` (新增) | Blackboard 实现 |
| `crates/agent/src/session_manager.rs` | AgentSession 增加 `blackboard: Option<Arc<RwLock<SharedBlackboard>>>` |
| `crates/rt-messaging/src/message_gateway.rs` | AgentMessage 增加 `BlackboardSync` 消息类型 |
| `crates/agent/src/lib.rs` | `pub mod shared_blackboard` |

## 3. Part B: DAG 工作流鲁棒性

### 3.1 降级节点

在 `WorkflowStep` 中增加：

```rust
pub struct WorkflowStep {
    // ... 现有字段 ...
    /// 降级步骤 ID：当前步骤失败后自动执行的替代步骤
    pub fallback_step_id: Option<String>,
    /// 步骤超时（秒），覆盖全局 step_timeout
    pub timeout_secs: Option<u64>,
    /// 输出 JSON Schema（用于校验步骤输出）
    pub expected_output_schema: Option<String>,
}
```

在 `WorkflowRunner::run()` 中，当步骤失败且 `on_failure == Abort` 时，检查 `fallback_step_id`。如果存在，激活降级步骤而非直接终止：

```rust
OnStepFailure::Abort => {
    if let Some(ref fallback_id) = step.fallback_step_id {
        // 激活降级节点：将其状态设为 Ready
        engine.update_step_status(wf_id, fallback_id, StepStatus::Ready, None, None);
        // 当前步骤标记为 Skipped（降级接管）
        engine.update_step_status(wf_id, &sid, StepStatus::Skipped, None, Some(e));
    } else {
        engine.update_step_status(wf_id, &sid, StepStatus::Failed, None, Some(e));
    }
}
```

### 3.2 状态持久化

利用已有的 Sea-ORM + SQLite 框架，新增 `workflow_snapshots` 表：

```sql
CREATE TABLE workflow_snapshots (
    id TEXT PRIMARY KEY,
    workflow_id TEXT NOT NULL,
    snapshot_json TEXT NOT NULL,  -- Workflow 的 JSON 序列化
    created_at INTEGER NOT NULL,
    step_id TEXT,                 -- 触发 checkpoint 的步骤 ID
    FOREIGN KEY (workflow_id) REFERENCES workflows(id)
);
```

`WorkflowEngine` 增加 `save_checkpoint()` / `load_checkpoint()` 方法。每次步骤完成或失败后自动保存快照。

### 3.3 差异化超时

`WorkflowRunner::run()` 在启动步骤执行时读取 `step.timeout_secs`，覆盖默认 300s：

```rust
let effective_timeout = step.timeout_secs
    .map(Duration::from_secs)
    .unwrap_or(self.step_timeout);
```

### 3.4 输出 Schema 校验

步骤完成后，如果配置了 `expected_output_schema`，用 `jsonschema` crate 校验输出：

```rust
if let Some(ref schema_str) = step.expected_output_schema {
    if let Ok(schema) = serde_json::from_str::<serde_json::Value>(schema_str) {
        if let Ok(output) = serde_json::from_str::<serde_json::Value>(&result) {
            if let Err(validation_errors) = jsonschema::validate(&schema, &output) {
                // Schema 校验失败 → 自动重试一次
                return Err(format!("输出格式校验失败: {:?}", validation_errors));
            }
        }
    }
}
```

## 4. 修改点总览

| # | 文件 | 变更 |
|---|------|------|
| 1 | `crates/agent/src/shared_blackboard.rs` (新增) | Blackboard + 冲突解决 |
| 2 | `crates/agent/src/lib.rs` | pub mod shared_blackboard |
| 3 | `crates/agent/src/session_manager.rs` | AgentSession 附加 Blackboard |
| 4 | `crates/rt-messaging/src/message_gateway.rs` | BlackboardSync 消息类型 |
| 5 | `crates/rt-workflow/src/workflow_engine.rs` | 降级节点 + 差异化超时 + Schema 校验 + 持久化 |
| 6 | `crates/core/src/entity/workflow_snapshots.rs` (新增) | 持久化 entity |
| 7 | `crates/migration/src/m20250513_000001_workflow_snapshots.rs` (新增) | DB migration |
| 8 | `crates/migration/src/lib.rs` | 注册 migration |

## 5. 测试计划

| 测试 | 预期 |
|------|------|
| blackboard_record_decision | 决策被正确记录 |
| blackboard_consensus | 所有 Agent 同意的值被正确返回 |
| blackboard_conflict_majority | 多数票获胜 |
| blackboard_conflict_tiebreak | 平局时选首个完成者 |
| blackboard_broadcast | 广播消息对所有 Agent 可见 |
| workflow_fallback_step | 主步骤失败 → 降级步骤自动激活 |
| workflow_checkpoint_save_load | 保存 → 进程重启 → 加载恢复 |
| workflow_per_step_timeout | 步骤自定义超时覆盖全局默认 |
| workflow_schema_validation | 格式错误输出被拒绝并重试 |

## 6. 风险

| 风险 | 缓解 |
|------|------|
| Blackboard 并发写入 | Arc<RwLock<>> 保证线程安全 |
| 持久化性能 | 仅在步骤完成时 checkpoint，不频繁写入 |
| jsonschema 增加编译时间 | 使用 feature gate，可按需启用 |
| 降级节点形成循环 | 创建时检测循环依赖（已有 Kahn 算法） |
