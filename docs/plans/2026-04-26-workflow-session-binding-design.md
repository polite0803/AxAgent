# 工作流执行与 Agent 会话深度绑定设计方案

**日期**: 2026-04-26
**版本**: v1.0
**状态**: 草稿

---

## 1. 问题分析

### 1.1 当前架构

```
┌─────────────────────────────────────────────────────────────────┐
│                        当前架构                                   │
└─────────────────────────────────────────────────────────────────┘

会话路径 (agent_query):
┌──────────┐     ┌──────────────┐     ┌─────────────────┐
│ Frontend │────▶│  agent_query  │────▶│  Agent 处理      │────▶ 返回结果
└──────────┘     └──────────────┘     └─────────────────┘
                                                │
                    ┌───────────────────────────┤
                    │  emit "agent-stream-text"  │  (流式文本)
                    │  emit "agent-done"         │  (完成)
                    ▼                           ▼

工作流路径 (workflow_execute):
┌──────────┐     ┌──────────────────┐     ┌─────────────────┐
│ Frontend │────▶│ workflow_execute │────▶│ WorkflowRunner  │ (后台异步)
└──────────┘     └──────────────────┘     └─────────────────┘
                                                │
                    ┌───────────────────────────┤
                    │  emit "workflow:step-*"   │ (工作流事件)
                    │  emit "workflow:done"     │
                    ▼                           ▼
              不与任何会话关联                独立执行
```

### 1.2 问题

1. **工作流执行与会话分离** - `workflow_execute` 是独立的 Tauri 命令
2. **结果不返回给用户** - 工作流执行在后台，结果不发送给用户
3. **无流式反馈** - 步骤执行结果不会实时流式显示给用户
4. **两条路径不互通** - 用户在工作流执行时看不到 Agent 思考过程

---

## 2. 设计目标

### 2.1 核心目标

1. **工作流步骤结果实时流式返回** - 用户在对话中看到每一步的执行
2. **与 Agent 会话无缝结合** - 工作流执行成为 Agent 响应的一部分
3. **步骤状态可追踪** - 用户能看到当前执行到哪一步
4. **保持兼容性** - 不破坏现有的独立工作流执行功能

### 2.2 非目标

- 不改变现有的 UI 交互
- 不修改数据库 schema
- 不改变工作流模板存储结构

---

## 3. 架构设计

### 3.1 目标架构

```
┌─────────────────────────────────────────────────────────────────┐
│                        目标架构                                   │
└─────────────────────────────────────────────────────────────────┘

┌──────────┐     ┌──────────────────────────────────────────┐
│ Frontend │────▶│  agent_query (统一入口)                   │
└──────────┘     └──────────────────────────────────────────┘
                                                │
                    ┌───────────────────────────┼───────────────────┐
                    │                           │                   │
                    ▼                           ▼                   ▼
          ┌─────────────────┐      ┌─────────────────┐    ┌─────────────────┐
          │  直接 LLM 对话   │      │   技能执行       │    │  工作流执行     │
          │  (Q&A Mode)     │      │   (Skill Mode)  │    │  (Workflow Mode)│
          └─────────────────┘      └─────────────────┘    └─────────────────┘
                    │                           │                   │
                    │                           │                   │
                    ▼                           │                   │
          ┌─────────────────┐                    │                   │
          │  emit "agent-   │◀───────────────────┼───────────────────┘
          │   stream-text"  │     (统一的流式事件通道)
          │  emit "agent-   │
          │   done"         │
          └─────────────────┘
                    ▲
                    │  workflow_execute_with_session
                    │  内部调用 WorkflowRunner
                    │  每步执行后 emit "agent-stream-text"
                    │  执行完成 emit "agent-done"
                    │
┌───────────────────┴─────────────────────────────────────────────┐
│                 WorkflowRunner + SessionCallback                  │
│                                                                  │
│  每步执行时:                                                      │
│    1. 执行步骤 LLM 调用                                           │
│    2. 获取结果                                                    │
│    3. 调用 session_callback.on_step_result(step, result)        │
│    4. Frontend 收到 "agent-stream-text" 更新显示                 │
└──────────────────────────────────────────────────────────────────┘
```

### 3.2 核心组件设计

#### 3.2.1 SessionWorkflowExecutor

创建一个新的 `StepExecutor` 实现，包装 `LlmStepExecutor`，在每个步骤执行后通过回调通知会话。

```rust
// 位置: runtime/src/workflow_executor.rs

/// 带会话回调的工作流步骤执行器
pub struct SessionWorkflowExecutor {
    inner: Arc<dyn StepExecutor>,
    session_callback: Arc<dyn SessionCallback>,
}

impl SessionWorkflowExecutor {
    pub fn new(
        inner: Arc<dyn StepExecutor>,
        session_callback: Arc<dyn SessionCallback>,
    ) -> Self {
        Self { inner, session_callback }
    }
}

impl StepExecutor for SessionWorkflowExecutor {
    fn execute(
        &self,
        step: WorkflowStep,
        deps_results: HashMap<String, String>,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send>> {
        let callback = self.session_callback.clone();
        let step_clone = step.clone();

        Box::pin(async move {
            // 执行实际步骤
            let result = self.inner.execute(step_clone.clone(), deps_results).await;

            // 通知会话
            callback.on_step_result(&step_clone, result.as_ref().ok());

            result
        })
    }
}

/// 会话回调 trait
pub trait SessionCallback: Send + Sync {
    fn on_step_start(&self, step: &WorkflowStep);
    fn on_step_result(&self, step: &WorkflowStep, result: Result<&str, &str>);
    fn on_step_error(&self, step: &WorkflowStep, error: &str);
    fn on_workflow_complete(&self, workflow_id: &str);
}
```

#### 3.2.2 Tauri 事件发射 trait

```rust
// 位置: runtime/src/workflow_executor.rs

pub struct TauriSessionCallback {
    app: AppHandle,
    conversation_id: String,
    message_id: String,
}

impl TauriSessionCallback {
    pub fn new(app: AppHandle, conversation_id: String, message_id: String) -> Self {
        Self { app, conversation_id, message_id }
    }
}

impl SessionCallback for TauriSessionCallback {
    fn on_step_start(&self, step: &WorkflowStep) {
        let _ = self.app.emit("agent-stream-text", serde_json::json!({
            "conversation_id": self.conversation_id,
            "assistant_message_id": self.message_id,
            "type": "workflow_step_start",
            "step_id": step.id,
            "step_goal": step.goal,
            "agent_role": format!("{:?}", step.agent_role),
        }));
    }

    fn on_step_result(&self, step: &WorkflowStep, result: Result<&str, &str>) {
        match result {
            Ok(text) => {
                let _ = self.app.emit("agent-stream-text", serde_json::json!({
                    "conversation_id": self.conversation_id,
                    "assistant_message_id": self.message_id,
                    "type": "workflow_step_complete",
                    "step_id": step.id,
                    "step_goal": step.goal,
                    "result": text,
                }));
            }
            Err(e) => {
                let _ = self.app.emit("agent-stream-text", serde_json::json!({
                    "conversation_id": self.conversation_id,
                    "assistant_message_id": self.message_id,
                    "type": "workflow_step_error",
                    "step_id": step.id,
                    "error": e,
                }));
            }
        }
    }

    fn on_workflow_complete(&self, workflow_id: &str) {
        let _ = self.app.emit("agent-done", serde_json::json!({
            "conversation_id": self.conversation_id,
            "assistant_message_id": self.message_id,
            "workflow_id": workflow_id,
        }));
    }
}
```

---

## 4. 接口设计

### 4.1 新增 Tauri 命令

```rust
/// 带会话绑定的工作流执行命令
#[tauri::command]
pub async fn workflow_execute_with_session(
    app: AppHandle,
    app_state: State<'_, AppState>,
    workflow_id: String,
    conversation_id: String,
    streaming_message_id: String,
    provider_id: String,
) -> Result<(), String> {
    // 1. 获取 Provider 和 API Key
    let prov = provider::get_provider(&app_state.sea_db, &provider_id)
        .await
        .map_err(|e| e.to_string())?;

    let key = prov.keys.iter().find(|k| k.enabled)
        .ok_or_else(|| "No active API key for provider".to_string())?;

    let api_key = crypto::decrypt_key(&key.key_encrypted, &app_state.master_key)
        .map_err(|e| e.to_string())?;

    // 2. 创建 Adapter
    let adapter: Arc<dyn ProviderAdapter> = create_adapter(&prov)?;

    // 3. 创建 LlmStepExecutor
    let llm_executor = create_llm_step_executor(
        adapter,
        key.id.clone(),
        api_key,
        prov.id.clone(),
        resolve_base_url_for_type(&prov.api_host, &prov.provider_type),
    );

    // 4. 创建会话回调
    let session_callback = Arc::new(TauriSessionCallback::new(
        app.clone(),
        conversation_id.clone(),
        streaming_message_id.clone(),
    ));

    // 5. 包装为 SessionWorkflowExecutor
    let executor = Arc::new(SessionWorkflowExecutor::new(llm_executor, session_callback));

    // 6. 创建 Runner 并执行
    let runner = WorkflowRunner::new(app_state.workflow_engine.clone(), executor);

    // 后台异步执行
    let wid = workflow_id.clone();
    tokio::spawn(async move {
        if let Err(e) = runner.run(&wid).await {
            tracing::error!("[workflow] Execution failed: {}", e);
        }
    });

    Ok(())
}
```

### 4.2 修改现有的 skill 执行

在 `execute_skill_async` 中，当检测到 `workflow` 模式时：

```rust
match execution_mode.as_str() {
    "workflow" => {
        if let Some(ref skill_steps) = steps {
            let workflow_steps = skill_steps_to_workflow_steps(skill_steps.clone());

            // 不再创建临时 Runner
            // 而是返回给调用者，让调用者决定如何执行

            // 返回步骤信息，让 agent_query 处理执行
            return Ok(SkillExecutionResult {
                execution_mode: "workflow".to_string(),
                content: format!(
                    "开始执行工作流，共 {} 个步骤",
                    skill_steps.len()
                ),
                steps: Some(skill_steps.clone()),
                workflow_id: Some(format!("skill_workflow_{}", skill_name)),
                ..
            });
        }
    }
    // ...
}
```

### 4.3 修改 agent_query

```rust
// 在 agent_query 中，检测到 skill 执行结果为 workflow 模式时
if skill_result.execution_mode == "workflow" {
    if let Some(ref workflow_id) = skill_result.workflow_id {
        // 调用带会话的工作流执行
        workflow_execute_with_session(
            app.clone(),
            app_state,
            workflow_id.clone(),
            conversation_id.clone(),
            streaming_message_id.clone(),
            request.provider_id.clone(),
        ).await?;
    }
}
```

---

## 5. 前端事件处理

### 5.1 新增事件类型

```typescript
// agent-stream-text 事件新增 type
type WorkflowStepStartEvent = {
  type: "workflow_step_start";
  step_id: string;
  step_goal: string;
  agent_role: string;
};

type WorkflowStepCompleteEvent = {
  type: "workflow_step_complete";
  step_id: string;
  step_goal: string;
  result: string;
};

type WorkflowStepErrorEvent = {
  type: "workflow_step_error";
  step_id: string;
  error: string;
};
```

### 5.2 前端处理

```typescript
// conversationStore.ts 中
setupEventListeners() {
  // ...

  // 工作流步骤开始
  const unlistenStepStart = await listen("agent-stream-text", (event) => {
    const payload = event.payload as WorkflowStepStartEvent;
    if (payload.type === "workflow_step_start") {
      appendMessageChunk(payload.step_goal, "agent");
    }
  });

  // 工作流步骤完成
  const unlistenStepComplete = await listen("agent-stream-text", (event) => {
    const payload = event.payload as WorkflowStepCompleteEvent;
    if (payload.type === "workflow_step_complete") {
      appendMessageChunk(`\n[步骤 ${payload.step_id} 完成]\n${payload.result}`, "agent");
    }
  });
}
```

---

## 6. 执行流程

### 6.1 完整执行流程

```
1. 用户发送消息
         │
         ▼
2. agent_query 被调用
         │
         ▼
3. Agent 检测到需要执行技能
         │
         ▼
4. execute_skill_async 执行
         │
         ▼
5. detect_skill_execution_mode 返回 "workflow"
         │
         ▼
6. 创建工作流，创建 SkillExecutionResult (包含 workflow_id)
         │
         ▼
7. 返回结果给 agent_query
         │
         ▼
8. agent_query 检测到 workflow 模式
         │
         ▼
9. workflow_execute_with_session 被调用
         │
         ▼
10. TauriSessionCallback 创建
         │
         ▼
11. WorkflowRunner.run() 开始执行
         │
         ├── 每步开始 → emit "workflow_step_start"
         │                 │
         │                 ▼
         │              Frontend 显示 "正在执行: [role] - goal"
         │
         ├── 每步完成 → emit "workflow_step_complete"
         │                 │
         │                 ▼
         │              Frontend 显示 "[step_id] 完成: result"
         │
         └── 全部完成 → emit "agent-done"
                           │
                           ▼
                        Frontend 显示最终结果
```

---

## 7. 兼容性考虑

### 7.1 独立工作流执行

保持现有的 `workflow_execute` 命令不变，用于：
- 前端工作流编辑器中的"启动"按钮
- 不需要与会话绑定的场景

### 7.2 事件命名空间

使用不同的事件前缀区分：
- `agent-stream-text` - Agent 会话相关
- `workflow:*` - 独立工作流相关

---

## 8. 实施计划

### Phase 1: 核心实现
1. 创建 `SessionCallback` trait
2. 创建 `TauriSessionCallback` 实现
3. 创建 `SessionWorkflowExecutor`
4. 创建 `workflow_execute_with_session` 命令

### Phase 2: 集成
1. 修改 `execute_skill_async` 返回 workflow 信息
2. 修改 `agent_query` 调用 `workflow_execute_with_session`
3. 前端事件监听处理

### Phase 3: 测试
1. 独立工作流执行测试
2. 会话内工作流执行测试
3. 事件流测试
