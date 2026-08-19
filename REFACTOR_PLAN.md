# 实质性重构方案：从“外围防御”到“核心改造”

> **背景**：上一轮“类型驱动设计”的 5 个阶段（DTO 测试、静态断言、构建期检查、IPC 同步、Typestate）大多是**外围防御**或**形式主义**，没有真正触及“运行时业务错误”的核心痛点。
>
> **核心痛点**：
>
> 1. **魔法字符串 (Magic Strings)**：用字符串代表状态或类型，拼写错误或重构时容易出错。
> 2. **隐式状态机散落**：到处都是 `if node.status == "pending"`，容易漏写分支。
> 3. **潜在崩溃 (Panic)**：`.unwrap()` 和 `.expect()` 在生产环境是定时炸弹。
> 4. **隐式协议**：依赖 LLM 输出特定 JSON 字符串，微调就炸。
>
> **目标**：从“外围防御”转向“核心改造”，通过实质性的类型系统改造解决上述痛点。

---

## 阶段 A：消灭魔法字符串 (Magic Strings → Strong Types)

**目标**：消除因字符串拼写错误或重构导致的业务逻辑失效。

### 进度：✅ A1 已完成 | ✅ A2 已完成

### 步骤 A1：重构 `business_rules.rs` ✅

- **位置**：`src-tauri/crates/rt-workflow/src/business_rules.rs`
- **问题**：
  ```rust
  matches!(node_type, "fileOperation" | "tool")
  ```
- **已完成的改造**：
  1. 将 `BusinessRule` 和 `BusinessRuleEvaluator` 的 `node_type: &str` 参数改为 `kind: &NodeKind`。
  2. 定义了 `NodeKind` 枚举（在 `harness/workflow_types.rs` 中），包含：`Input`, `Output`, `Tool`, `Agent`, `Condition`, `Loop`, `Container`, `Storage`。
  3. 所有调用方改用 `NodeKind` 枚举值（`NodeKind::Tool`、`NodeKind::Agent` 等）。
  4. 在 `node_executor_trait.rs` 中添加 `node_kind()` 函数，将 `WorkflowNode` 映射到 `NodeKind`。
  5. 修复了 `BusinessRuleInterceptor` 中的 bug（原来错误地使用 `node_id` 作为 `node_type`）。
  6. 更新了 `InterceptorContext`，添加 `node_kind: Option<NodeKind>` 字段。
  7. 所有 11 个测试通过。

### 步骤 A2：重构 `plan_compiler.rs` ✅

- **位置**：`src-tauri/crates/harness/src/plan_compiler.rs`
- **问题**：
  ```rust
  match action_type.as_str() {
      "tool" => WorkflowNode::Tool(...),
      "llm" => WorkflowNode::Llm(...),
      // ...
      _ => WorkflowNode::Agent(...),
  }
  ```
- **已完成的改造**：
  1. 在 `plan_types.rs` 中定义 `ActionType` 枚举（`Tool`, `Llm`, `Agent`），实现 `FromStr` 和 `Display`。
  2. 将 `PlannedTask.action_type` 从 `String` 改为 `ActionType`。
  3. `plan_compiler.rs` 中的 `compile_plan_to_dag` 和 `dag_to_plan` 改用枚举匹配。
  4. `hierarchical_planner.rs` 的 `TaskBuilder` 改用 `ActionType` 参数，添加便捷构造器 `new_tool()`, `new_llm()`, `new_agent()`。
  5. `commands/plan.rs` 改用 `ActionType::from_str()` 解析输入。
  6. 所有测试代码更新为使用 `ActionType` 枚举值。
  7. 全部 2009 个测试通过。

### 预期收益

- ✅ 编译器在拼写错误或重构时会直接报错。
- ✅ IDE 可以自动补全。
- ✅ 代码更清晰，意图更明确。

---

## 阶段 B：激活 Typestate (从死代码到活逻辑)

**目标**：将孤立的 Typestate 实现集成到核心执行逻辑中，消除运行时状态检查。

### 进度：✅ B1 已完成 | ✅ B2 已完成

### 步骤 B1：改造 `DagStore` ✅

- **位置**：`src-tauri/crates/rt-workflow/src/work_engine/engine/dag_store.rs`
- **问题**：
  ```rust
  // 散落的运行时状态检查
  if matches!(s.status, NodeStatus::Pending | NodeStatus::Ready) { ... }
  ```
- **已完成的改造**：
  1. 在 `dag_store.rs` 中添加 Typestate 辅助方法：
     - `to_typestate_map()`: 从 Workflow 创建 Typestate 节点集合
     - `compute_ready_nodes_typed()`: 返回类型安全的 `Vec<ReadyNode>`
     - `sync_typestate_to_workflow()`: 将 Typestate 状态同步回 Workflow
     - `mark_ready_to_running()`: Ready → Running 状态转移
     - `mark_running_to_completed()`: Running → Completed 状态转移
     - `mark_running_to_failed()`: Running → Failed 状态转移
     - `mark_ready_to_skipped()`: Ready → Skipped 状态转移
     - `mark_failed_to_ready()`: Failed → Ready 状态转移
  2. 所有方法都有运行时状态校验，确保只有合法的状态转移被执行。

### 步骤 B2：改造 `WorkEngine` ✅

- **位置**：`src-tauri/crates/rt-workflow/src/work_engine/engine/mod.rs`
- **问题**：
  ```rust
  // 散落的状态迁移逻辑
  if node.status == NodeStatus::Pending { ... }
  else if node.status == NodeStatus::Ready { ... }
  ```
- **已完成的改造**：
  1. 在 `apply_node_status_update` 中集成 Typestate 状态转移验证和执行：
     - 添加 `validate_typestate_transition()`: 使用 Typestate 验证状态转移合法性
     - 添加 `can_transition_via_typestate()`: 判断转移是否在 Typestate 规则内
     - 在状态更新时调用对应的 Typestate 方法（如 `mark_ready_to_running`）
  2. 在 `get_ready_steps` 和 `get_ready_steps_for_execution` 中同时使用现有方法和 Typestate 方法计算就绪节点，验证结果一致性。
  3. 添加 Typestate 同步逻辑，确保 Typestate 状态与 Workflow 状态保持一致。
  4. 全部 113 个测试通过，`cargo clippy -- -D warnings` 零警告。

### 预期收益

- ✅ 非法的状态转移（如 `Pending → Running`）在运行时被阻止，并产生清晰的警告日志。
- ✅ 状态机逻辑集中在 Typestate 实现中，不再散落各处。
- ✅ 代码更安全，更易维护。
- ✅ 为后续完全移除运行时状态检查奠定基础。

---

## 阶段 C：硬化错误处理 (Panic → Result)

**目标**：消除核心逻辑中的 `unwrap()` 和 `expect()`，改为显式的错误处理。

### 进度：✅ C1 已完成 | ✅ C2 已完成

### 步骤 C1：重构 `Dispatcher` ✅

- **位置**：`src-tauri/crates/rt-workflow/src/work_engine/dispatcher.rs`
- **问题**：
  ```rust
  .expect("FallbackExecutor must be registered")
  ```
- **已完成的改造**：
  1. `register_arc` 中的 `.expect("checked above")` 改为 `unwrap_or_else()` + `unreachable!()`，添加错误日志
  2. `dispatch` 方法中的 `.expect("FallbackExecutor must be registered")` 改为：
     - 先尝试获取指定类型的执行器
     - 找不到时回退到 fallback 执行器
     - fallback 也找不到时返回 `NodeError::UNSUPPORTED_NODE_TYPE` 错误，而不是 panic
  3. 全部 113 个测试通过，`cargo clippy -- -D warnings` 零警告

### 步骤 C2：审查 `AgentExecutor` ✅

- **位置**：`src-tauri/crates/rt-workflow/src/work_engine/executors/agent_executor.rs`
- **问题**：
  ```rust
  serde_json::to_string(...).unwrap_or_default()
  ```
- **已完成的改造**：
  1. **缓存查询** (L450)：将 `.expect("缓存应命中")` 改为 `match` 分支，缓存未命中时优雅降级查询数据库
  2. **Profile 加载**：重构缓存加载逻辑，消除 TOCTOU 竞态，简化代码结构
  3. **序列化容错**：将 `serde_json::to_value().unwrap_or_default()` 改为 `match` 分支，失败时记录警告日志并使用 `serde_json::Value::Null` 占位
  4. **参数序列化**：将 `serde_json::to_string().unwrap_or_default()` 改为 `match` 分支，失败时记录警告日志并使用空字符串
  5. **错误信息提取**：将 `.unwrap_or_default()` 改为 `.unwrap_or_default()` 配中文注释，说明空字符串在此处是语义正确的默认值
  6. 关键业务路径（如 Profile 查询失败）返回明确的 `NodeError`，而非静默降级

### 预期收益

- ✅ 生产环境不再因意外状况而 panic。
- ✅ 错误处理更明确，更易追踪。
- ✅ 系统更健壮。

---

## 阶段 D：锁定隐式协议 (JSON Strings → Structured Enum)

**目标**：让依赖 LLM 输出的解析更健壮，不再脆弱。

### 进度：✅ D1 已完成 | ✅ D2 已完成

### 步骤 D1：定义状态枚举 ✅

- **位置**：`src-tauri/crates/harness/src/node_output_status.rs` (新建)
- **问题**：
  ```rust
  if output["status"] == "pending" { ... }  // 脆弱的字符串比较
  ```
- **已完成的改造**：
  1. 定义 `NodeOutputStatus` 枚举，使用 serde tag 功能：
     ```rust
     #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
     #[serde(tag = "status", rename_all = "snake_case")]
     pub enum NodeOutputStatus {
         WaitingForApproval { approval_request: Option<Value>, ... },
         Paused { reason: Option<String> },
         Skipped { reason: Option<String> },
         NeedsIntervention { reason: Option<String>, required_info: Option<Vec<String>> },
         Custom { custom_status: String },
     }
     ```
  2. 实现解析函数 `NodeOutputStatus::from_json(value: &Value) -> Result<Self, String>`
  3. 实现便捷方法：`is_pending()`、`should_trigger_interrupt()`、`status_str()`
  4. 添加 7 个单元测试覆盖解析、序列化、状态判断等场景

### 步骤 D2：严格解析 ✅

- **位置**：`loop_executor.rs` 和 `approval_executor.rs`
- **已完成的改造**：
  1. **`detect_interrupt` 函数**：
     - 优先使用 `NodeOutputStatus::from_json()` 解析状态
     - 通过 `status.is_pending()` 判断是否触发中断
     - 解析失败时降级到旧的字符串比较（向后兼容）
     - 添加详细的 debug 日志记录状态判断过程
  2. **`ApprovalExecutor`**：
     - 使用 `NodeOutputStatus::waiting_for_approval()` 构造状态
     - 通过 `serde_json::to_value(&status)` 序列化输出
     - 添加额外字段（node_id, pause_reason）
  3. **修复 `NodeKind`**：添加 `PartialEq` derive，修复已有测试错误

### 预期收益

- ✅ 对 LLM 输出的微小变化更健壮。
- ✅ 解析错误能被正确捕获和处理。
- ✅ 类型安全。
- ✅ 全部 113 + 7 个测试通过，`cargo clippy -- -D warnings` 零警告。

---

## 执行顺序

- [x] **阶段 A1**：消灭魔法字符串 — business_rules.rs ✅ 已完成
- [x] **阶段 A2**：消灭魔法字符串 — plan_compiler.rs ✅ 已完成
- [x] **阶段 B**：激活 Typestate ✅ 已完成
- [x] **阶段 C**：硬化错误处理 ✅ 已完成
- [x] **阶段 D**：锁定隐式协议 ✅ 已完成

---

## 核心原则

1. **类型优先**：能用类型表达的，不要用字符串。
2. **编译期安全**：能在编译时阻止的错误，不要留到运行时。
3. **显式优于隐式**：显式的 `Result` 优于隐式的 `unwrap()`。
4. **集中优于散落**：状态机逻辑应集中实现，不应散落各处。
5. **小步快跑**：每个阶段独立完成并验证，避免大爆炸式重构。
