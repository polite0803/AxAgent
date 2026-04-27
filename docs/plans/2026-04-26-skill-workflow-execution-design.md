# 技能与工作流执行流程重构设计方案

**日期**: 2026-04-26
**版本**: v1.0
**状态**: 草稿

---

## 1. 背景与问题分析

### 1.1 当前架构问题

经过代码分析，发现以下关键问题：

| 问题 | 位置 | 描述 |
|-----|------|------|
| **技能执行不调用工作流** | `agent.rs:1996-2008` | `workflow` 模式的技能只是返回步骤给 LLM，并未真正执行工作流 |
| **Mock 执行器** | `workflow_engine.rs:597-607` | `run_workflow` 方法中的 `StepExecutor` 只是打印日志并返回固定字符串 |
| **执行路径未打通** | 整体架构 | 技能分解产生的工作流定义未连接到执行引擎 |

### 1.2 当前执行流程图

```
技能执行流程 (当前):
┌─────────────┐     ┌──────────────────────────────┐
│   Agent     │────▶│  detect_skill_execution_mode │────┐
└─────────────┘     └──────────────────────────────┘    │
                            │                            │
              ┌─────────────┼─────────────┐              │
              ▼             ▼             ▼              │
         "workflow"     "mcp"       "content"           │
              │             │             │              │
              ▼             ▼             │              │
    步骤返回给LLM    执行MCP工具      返回内容给LLM       │
                                          │              │
                                          └──────────────┘
                                                    │
                                                    ▼
                                            返回给用户

工作流执行流程 (当前):
┌──────────────────┐     ┌──────────────────────────────┐
│ Frontend 选择模板 │────▶│    workflow_create           │
└──────────────────┘     └──────────────────────────────┘
                                          │
                                          ▼
                               ┌────────────────────────┐
                               │   workflow_engine       │
                               │   (内存 HashMap 存储)    │
                               └────────────────────────┘
                                          │
                                          ▼
                               ┌────────────────────────┐
                               │  start_workflow         │
                               │  (创建执行记录)         │
                               └────────────────────────┘
                                          │
                                          ▼
                               ┌────────────────────────┐
                               │  run_workflow (Mock!)   │
                               │  不调用 LLM             │
                               └────────────────────────┘
```

---

## 2. 设计目标

### 2.1 核心目标

1. **技能执行真正调用工作流引擎**
   - 当技能内容包含 `workflow` 模式时，应创建并执行真正的工作流

2. **实现真实的 StepExecutor**
   - 每个工作流步骤应调用 LLM/Agent 执行
   - 支持角色分配 (researcher/reviewer/synthesizer)

3. **打通技能分解到工作流执行的完整链路**
   - 技能分解产生的工作流定义应能存储和执行

### 2.2 非目标 (YAGNI)

- 不改变现有的 `atomic_skills` 表结构
- 不改变 Frontend 的 UI 交互
- 不实现复杂的多 Agent 通信协议

---

## 3. 架构设计

### 3.1 整体架构

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           用户交互层                                      │
│  ┌─────────────┐    ┌──────────────────┐    ┌─────────────────────┐  │
│  │ 技能工具调用 │    │ 工作流模板选择    │    │ 直接发送消息        │  │
│  └──────┬──────┘    └────────┬─────────┘    └──────────┬──────────┘  │
└─────────┼─────────────────────┼─────────────────────────┼──────────────┘
          │                     │                         │
          ▼                     ▼                         ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                           Agent 层                                       │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                      SkillExecutor                                │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐ │   │
│  │  │  workflow   │  │    mcp      │  │       content           │ │   │
│  │  │   mode      │  │    mode     │  │        mode             │ │   │
│  │  └──────┬──────┘  └──────┬──────┘  └───────────┬─────────────┘ │   │
│  └─────────┼─────────────────┼────────────────────┼───────────────┘   │
└────────────┼─────────────────┼────────────────────┼───────────────────┘
             │                 │                    │
             ▼                 │                    │
┌────────────────────────────┐ │                    │
│    WorkflowEngine          │ │                    │
│  ┌──────────────────────┐  │ │                    │
│  │  create_workflow()   │◀─┘                    │
│  │  run_workflow()      │────────────────────────┘
│  │  (真实 StepExecutor) │                         │
│  └──────────────────────┘                          │
└─────────────────────────────────────────────────────┘
             │
             ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                         LLM/Agent 层                                    │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │                    StepExecutor 实现                             │    │
│  │  对每个 WorkflowStep:                                            │    │
│  │  1. 获取步骤 goal 和 agent_role                                  │    │
│  │  2. 构建 Agent 请求 (包含依赖结果)                               │    │
│  │  3. 调用 LLM 执行                                                 │    │
│  │  4. 返回执行结果                                                  │    │
│  └─────────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────────┘
```

### 3.2 核心组件设计

#### 3.2.1 StepExecutor 实现

```rust
// 位置: runtime/src/workflow_executor.rs

/// 真实的工作流步骤执行器
pub struct LlmStepExecutor {
    adapter: Arc<dyn AxAgentAdapter>,
    db: Arc<DatabaseConnection>,
}

impl LlmStepExecutor {
    pub fn new(
        adapter: Arc<dyn AxAgentAdapter>,
        db: Arc<DatabaseConnection>,
    ) -> Self {
        Self { adapter, db }
    }
}

impl StepExecutor for LlmStepExecutor {
    fn execute(
        &self,
        step: WorkflowStep,
        deps_results: HashMap<String, String>,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send>> {
        Box::pin(async move {
            // 1. 构建 Agent 请求
            let system_prompt = build_role_system_prompt(&step.agent_role);
            let user_message = build_step_user_message(&step.goal, &deps_results);

            // 2. 调用 LLM
            let response = self
                .adapter
                .complete(&system_prompt, &user_message)
                .await
                .map_err(|e| e.to_string())?;

            // 3. 返回结果
            Ok(response)
        })
    }
}
```

#### 3.2.2 SkillExecutor 修改

```rust
// 位置: agent.rs

async fn execute_skill_async(
    // ...
) -> Result<String, String> {
    let (execution_mode, steps, mcp_tool_call) = detect_skill_execution_mode(skill_content);

    match execution_mode.as_str() {
        "workflow" => {
            // 构建工作流步骤
            let workflow_steps = build_workflow_steps_from_skill(&skill_name, &steps, &goal, &constraints);

            // 创建并执行工作流
            let workflow_id = ctx.create_and_run_workflow(&skill_name, workflow_steps).await?;
            Ok(format!("Workflow '{}' started with ID: {}", skill_name, workflow_id))
        }
        "mcp" => { /* 保持不变 */ }
        _ => { /* 保持不变 */ }
    }
}
```

#### 3.2.3 SkillExecutionContext 扩展

```rust
// 位置: agent.rs

impl SkillExecutionContext {
    pub async fn create_and_run_workflow(
        &self,
        name: &str,
        steps: Vec<WorkflowStep>,
    ) -> Result<String, String> {
        // 1. 创建工作流
        let workflow = self.work_engine.create_workflow(name, steps)
            .map_err(|e| e.to_string())?;

        // 2. 获取执行器
        let executor = LlmStepExecutor::new(
            self.adapter.clone(),
            self.db.clone(),
        );

        // 3. 创建 Runner 并执行
        let runner = WorkflowRunner::new(
            Arc::new(self.workflow_engine.clone()),
            Arc::new(executor),
        );

        // 4. 启动执行 (异步)
        let workflow_id = workflow.id.clone();
        tokio::spawn(async move {
            if let Err(e) = runner.run(&workflow_id).await {
                tracing::error!("[workflow] Execution failed: {}", e);
            }
        });

        Ok(workflow_id)
    }
}
```

---

## 4. 执行流程对比

### 4.1 修改后的技能执行流程

```
┌─────────────────────────────────────────────────────────────────┐
│                    技能执行流程 (修改后)                          │
└─────────────────────────────────────────────────────────────────┘

1. LLM 调用 skill_tool("skill_name", input)
                              │
                              ▼
2. execute_skill_async(skill_name, skill_content, input)
                              │
                              ▼
3. detect_skill_execution_mode(skill_content)
                              │
              ┌───────────────┼───────────────┐
              ▼               ▼               ▼
         "workflow"        "mcp"          "content"
              │               │               │
              ▼               │               │
4. 构建 WorkflowStep    执行 MCP 工具      返回内容
   列表从 steps         │
              │          │
              ▼          │
5. ctx.create_and_run_workflow(name, steps)
              │
              ▼
6. work_engine.create_workflow() ──▶ 创建工作流 DAG
              │
              ▼
7. 创建 LlmStepExecutor + WorkflowRunner
              │
              ▼
8. runner.run(workflow_id) ──▶ 异步执行步骤
              │
              ▼
9. 每个步骤调用 LLM 执行
   │
   ├── researcher: 探索和研究
   ├── reviewer:   审查和分析
   ├── synthesizer: 汇总结果
   │
   ▼
10. 返回 execution_id 给 LLM
```

### 4.2 修改后的工作流执行流程

```
┌─────────────────────────────────────────────────────────────────┐
│                   工作流执行流程 (修改后)                         │
└─────────────────────────────────────────────────────────────────┘

1. Frontend: 用户选择工作流模板
                              │
                              ▼
2. workflow_create(name, steps)
   (steps 包含 role, goal, needs)
                              │
                              ▼
3. workflow_engine.create_workflow()
   ──▶ 验证 DAG (无环、无重复 ID)
   ──▶ 存储到 HashMap
                              │
                              ▼
4. 用户输入 → Agent 处理
   或
   start_workflow_execution(workflow_id, input)
                              │
                              ▼
5. 创建真实的 LlmStepExecutor
   (包含 Adapter, DB 连接)
                              │
                              ▼
6. runner.run(workflow_id)
   │
   ├── Pipeline 并发执行
   ├── 依赖满足时调度步骤
   ├── 每个步骤调用 LLM
   │
   ▼
7. 步骤结果存入 workflow.results
   状态更新: Pending → Running → Completed/Failed/Skipped
```

---

## 5. 数据流设计

### 5.1 技能到工作流的转换

```
Marketplace 技能内容
       │
       ▼
┌─────────────────────────────────────────────────────────────┐
│                    SkillDecomposer                           │
│  输入: CompositeSkillData { content, ... }                   │
│  输出: DecompositionResult {                                │
│    atomic_skills: [...],                                    │
│    workflow_nodes: [...],  ◀─── 关键输出                    │
│    workflow_edges: [...],                                    │
│  }                                                          │
└─────────────────────────────────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────────────────────────┐
│                 workflow_nodes → WorkflowStep               │
│                                                             │
│  workflow_nodes[i]:                                         │
│  {                                                         │
│    "id": "step_1",                                         │
│    "goal": "探索代码结构",                                  │
│    "role": "researcher",                                    │
│    "needs": []                                              │
│  }                                                         │
│                              ↓                              │
│  WorkflowStep {                                            │
│    id: "step_1",                                          │
│    goal: "探索代码结构",                                    │
│    agent_role: AgentRole::Researcher,                      │
│    needs: vec![],                                          │
│    ...                                                     │
│  }                                                         │
└─────────────────────────────────────────────────────────────┘
```

### 5.2 步骤执行的数据流

```
WorkflowRunner.run()
       │
       ▼
┌─────────────────────────────────────────────────────────────┐
│                    步骤执行循环                              │
│                                                             │
│  对于每个就绪的步骤 (无未完成依赖):                          │
│       │                                                     │
│       ▼                                                     │
│  LlmStepExecutor.execute(step, deps_results)                │
│       │                                                     │
│       ▼                                                     │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ 1. 构建系统提示 (基于 agent_role)                   │   │
│  │    - researcher: "你是一个研究员，专注于探索..."     │   │
│  │    - reviewer: "你是一个审查员，专注于分析..."      │   │
│  │    - synthesizer: "你是一个综合员，专注于汇总..."   │   │
│  └─────────────────────────────────────────────────────┘   │
│       │                                                     │
│       ▼                                                     │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ 2. 构建用户消息                                     │   │
│  │    - 当前步骤目标                                   │   │
│  │    - 依赖步骤结果 (如果 needs 非空)                 │   │
│  └─────────────────────────────────────────────────────┘   │
│       │                                                     │
│       ▼                                                     │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ 3. 调用 LLM                                         │   │
│  │    adapter.complete(system_prompt, user_message)   │   │
│  └─────────────────────────────────────────────────────┘   │
│       │                                                     │
│       ▼                                                     │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ 4. 返回结果                                          │   │
│  │    Ok(result_string) 或 Err(error_string)           │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

---

## 6. 接口设计

### 6.1 新增 Tauri 命令

```rust
// 位置: agent.rs

#[tauri::command]
pub async fn workflow_execute(
    state: State<'_, AppState>,
    workflow_id: String,
    input: serde_json::Value,
) -> Result<String, String> {
    // 1. 获取工作流定义
    let workflow = state
        .workflow_engine
        .get_workflow(&workflow_id)
        .map_err(|e| e.to_string())?
        .ok_or("Workflow not found")?;

    // 2. 创建执行器
    let adapter = create_adapter_from_state(&state).await?;
    let executor = LlmStepExecutor::new(
        Arc::new(adapter),
        state.sea_db.clone(),
    );

    // 3. 创建 Runner
    let runner = WorkflowRunner::new(
        Arc::new(state.workflow_engine.clone()),
        Arc::new(executor),
    );

    // 4. 执行 (异步)
    let wid = workflow_id.clone();
    tokio::spawn(async move {
        if let Err(e) = runner.run(&wid).await {
            tracing::error!("[workflow] Execution failed: {}", e);
        }
    });

    Ok(workflow_id)
}
```

### 6.2 SkillExecutionContext 扩展

```rust
// 新增方法
impl SkillExecutionContext {
    /// 从技能内容创建并执行工作流
    pub async fn execute_skill_as_workflow(
        &self,
        skill_name: &str,
        skill_content: &str,
        input: &SkillInput,
    ) -> Result<String, String> {
        // 1. 解析技能输入
        let task = &input.input.task;
        let goal = input.input.context.as_ref().and_then(|c| c.goal.clone());
        let constraints = input.input.context.as_ref().and_then(|c| c.constraints.clone());

        // 2. 提取步骤
        let (_, steps, _) = detect_skill_execution_mode(skill_content);
        let steps = steps.ok_or("No steps found in workflow mode")?;

        // 3. 构建 WorkflowStep 列表
        let workflow_steps = steps
            .into_iter()
            .enumerate()
            .map(|(i, s)| {
                let role = infer_role_from_action(&s.action);
                WorkflowStep {
                    id: format!("{}_step_{}", skill_name, i + 1),
                    goal: s.description,
                    agent_role: role,
                    needs: vec![],  // 串行执行
                    context: constraints.clone(),
                    result: None,
                    status: StepStatus::Pending,
                    attempts: 0,
                    error: None,
                    max_retries: 2,
                    on_failure: OnStepFailure::Abort,
                    retry_policy: RetryPolicy::default(),
                    circuit_breaker: CircuitBreaker::default(),
                }
            })
            .collect();

        // 4. 创建并执行工作流
        self.create_and_run_workflow(skill_name, workflow_steps).await
    }

    /// 创建并执行工作流
    async fn create_and_run_workflow(
        &self,
        name: &str,
        steps: Vec<WorkflowStep>,
    ) -> Result<String, String> {
        // 创建工作流
        let workflow = self
            .work_engine
            .create_workflow(name, steps)
            .map_err(|e| e.to_string())?;

        let workflow_id = workflow.id.clone();

        // 获取执行器
        let executor = LlmStepExecutor::new(
            self.adapter.clone(),
            self.db.clone(),
        );

        // 创建 Runner
        let runner = WorkflowRunner::new(
            Arc::new(self.work_engine.clone()),
            Arc::new(executor),
        );

        // 异步执行
        let wid = workflow_id.clone();
        tokio::spawn(async move {
            if let Err(e) = runner.run(&wid).await {
                tracing::error!("[workflow] Execution failed: {}", e);
            }
        });

        Ok(workflow_id)
    }
}
```

---

## 7. 错误处理

### 7.1 步骤执行失败

```rust
match step.on_failure {
    OnStepFailure::Abort => {
        // 更新步骤状态为 Failed
        // 终止整个工作流
        return Err(format!("Step '{}' failed: {}", step.id, error));
    }
    OnStepFailure::Skip => {
        // 更新步骤状态为 Skipped
        // 继续执行独立步骤
    }
}
```

### 7.2 LLM 调用失败

```rust
// 重试逻辑由 WorkflowRunner 处理
// 超时: step_timeout (默认 300s)
```

---

## 8. 实现计划

### Phase 1: 基础架构 (预计工作量: 小)

1. [ ] 创建 `LlmStepExecutor` 结构体
2. [ ] 实现 `StepExecutor` trait
3. [ ] 在 `WorkflowEngine` 中添加创建 Runner 的方法

### Phase 2: 技能执行集成 (预计工作量: 中)

1. [ ] 修改 `SkillExecutionContext` 添加 `create_and_run_workflow`
2. [ ] 修改 `execute_skill_async` 的 `workflow` 模式分支
3. [ ] 添加 `workflow_execute` Tauri 命令

### Phase 3: 测试与优化 (预计工作量: 中)

1. [ ] 单元测试: `LlmStepExecutor`
2. [ ] 集成测试: 技能 → 工作流执行
3. [ ] 性能优化: 并发执行调优

---

## 9. 风险与注意事项

| 风险 | 缓解措施 |
|-----|--------|
| LLM 调用超时 | 配置合理的 `step_timeout` |
| 循环依赖 | DAG 验证已在 `create_workflow` 中实现 |
| 步骤执行失败 | 支持 `on_failure: Skip` 策略 |
| 资源竞争 | `WorkflowRunner` 有 `max_concurrent` 限制 |

---

## 10. 附录

### A. 相关文件列表

- `src-tauri/src/commands/agent.rs` - 技能执行入口
- `src-tauri/crates/runtime/src/workflow_engine.rs` - 工作流引擎
- `src-tauri/crates/runtime/src/work_engine/engine.rs` - 执行管理
- `src-tauri/crates/trajectory/src/skill_decomposition/` - 技能分解

### B. 术语表

| 术语 | 定义 |
|-----|------|
| StepExecutor | 工作流步骤执行器的类型别名 |
| WorkflowEngine |管理工作流 DAG 的内存存储 |
| WorkEngine | 管理工作流执行状态的引擎 |
| WorkflowRunner | 负责执行工作流步骤的运行器 |
| AgentRole | 代理角色 (Researcher/Reviewer/Synthesizer/Executor) |
