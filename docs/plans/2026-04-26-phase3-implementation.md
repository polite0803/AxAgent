# Phase 3: 深度推理与规划 - 详细实施计划

> 阶段: Phase 3
> 时间: 2026-04-26 起（2-3 月）
> 前置: Phase 2 模块 1 已完成（计算机控制能力）
> 目标: 实现类似 o1/Claude Thinking 的深度推理能力，使 AxAgent 具备自主问题解决能力
> 基线审计: 2026-04-26 基于实际代码分析更新

---

## 现有代码基线（Phase 2 完成后）

### Agent 模块结构

| 模块 | 状态 | 实际位置 | 说明 |
|------|------|---------|------|
| Agent 主入口 | ✅ | `crates/agent/src/lib.rs` | Agent 核心入口 |
| Provider 适配器 | ✅ | `crates/providers/` | 12 个 provider，支持多模态 |
| 图像生成 | ✅ | `crates/providers/src/image_gen.rs` | Flux + DallE |
| 图表生成 | ✅ | `commands/chart_generator.rs` | LLM → ECharts |
| 计算机控制 | ✅ | `crates/core/src/computer_control.rs` | 屏幕捕获、UI 自动化 |
| 屏幕截图 | ✅ | `crates/core/src/screen_capture.rs` | xcap 跨平台 |
| UI 自动化 | ✅ | `crates/core/src/ui_automation.rs` | 元素检测与交互 |
| 操作审计 | ✅ | `crates/core/src/operation_audit.rs` | 风险级别与确认 |
| 内置工具系统 | ✅ | `builtin_tools_registry.rs` + `builtin_tools.rs` | 工具注册与分发 |

### Agent 执行流程（当前）

```
用户输入 → ChatPanel → Agent → LLM Provider → 工具调用 → 结果返回
```

**当前局限**:
- LLM 一次性生成回复，无多轮推理
- 无思维链展示，用户无法理解推理过程
- 无法处理复杂多步骤任务
- 错误处理简单，无智能恢复

---

## Phase 3 目标架构

```
用户输入 → ReAct 引擎 → 任务分解 → 多步执行 → 结果验证 → 响应
                ↓
         思维链可视化（Streaming）
```

### 核心能力提升

1. **ReAct 推理**: 推理与执行循环，类 o1 的思考过程
2. **任务分解**: 复杂任务自动拆分为可执行子任务
3. **自我验证**: 执行结果自动校验，错误自动恢复
4. **思维可视化**: 用户可见推理过程，增强信任

---

## 模块 1: ReAct（Reasoning + Acting）引擎（Week 1-3）

### 1.1 架构设计

```
用户输入
    ↓
推理状态机（ReasoningState）
    ↓
┌─────────────────────────────────────────────┐
│  ReasoningState::Thinking                     │
│    - 生成下一步推理                          │
│    - 调用 LLM 分析现状                       │
├─────────────────────────────────────────────┤
│  ReasoningState::Planning                    │
│    - 决定下一步动作                          │
│    - 选择工具或子任务                        │
├─────────────────────────────────────────────┤
│  ReasoningState::Acting                      │
│    - 执行工具调用                            │
│    - 记录动作结果                            │
├─────────────────────────────────────────────┤
│  ReasoningState::Observing                  │
│    - 验证动作结果                            │
│    - 判断是否继续或完成                       │
└─────────────────────────────────────────────┘
    ↓
验证通过 → 最终回复
验证失败 → 回溯重试（最多 N 次）
```

### 1.2 新增文件

**Rust 后端**:
```
src-tauri/crates/agent/src/
├── react_engine.rs      # ReAct 核心引擎
├── reasoning_state.rs   # 推理状态定义
├── thought_chain.rs     # 思维链管理
├── action_executor.rs   # 动作执行器
└── self_verifier.rs     # 结果验证器
```

### 1.3 核心实现

#### ReasoningState 枚举

```rust
pub enum ReasoningState {
    Thinking,    // LLM 推理分析
    Planning,    // 动作规划
    Acting,      // 执行动作
    Observing,   // 验证结果
    Finished,    // 完成
    Failed,      // 失败
}
```

#### ReActEngine 结构

```rust
pub struct ReActEngine {
    max_iterations: usize,
    max_depth: usize,
    verification_enabled: bool,
    thought_chain: Vec<ThoughtStep>,
}

pub struct ThoughtStep {
    pub state: ReasoningState,
    pub reasoning: String,
    pub action: Option<ToolCall>,
    pub observation: Option<String>,
    pub result: Option<Value>,
    pub is_verified: bool,
}
```

#### 核心流程

```rust
impl ReActEngine {
    pub async fn run(&self, input: &str) -> Result<ReActResult> {
        let mut state = ReasoningState::Thinking;
        let mut iteration = 0;

        while iteration < self.max_iterations {
            match state {
                ReasoningState::Thinking => {
                    let reasoning = self.think(input).await?;
                    state = ReasoningState::Planning;
                }
                ReasoningState::Planning => {
                    let action = self.plan().await?;
                    state = ReasoningState::Acting(action);
                }
                ReasoningState::Acting(action) => {
                    let result = self.execute(action).await?;
                    state = ReasoningState::Observing(result);
                }
                ReasoningState::Observing(result) => {
                    if self.verify(&result)? {
                        state = ReasoningState::Finished;
                    } else {
                        state = ReasoningState::Thinking;
                    }
                }
                ReasoningState::Finished => break,
                ReasoningState::Failed => return Err(...),
            }
            iteration += 1;
        }
    }
}
```

### 1.4 思维链可视化

**前端组件**: `src/components/chat/ThoughtChainPanel.tsx`

```tsx
interface ThoughtStepProps {
  step: ThoughtStep;
  isActive: boolean;
}

export function ThoughtChainPanel({ steps, activeIndex }: Props) {
  return (
    <div className="thought-chain">
      {steps.map((step, i) => (
        <ThoughtStepView
          key={i}
          step={step}
          isActive={i === activeIndex}
        />
      ))}
    </div>
  );
}
```

**流式输出**: 使用 `ReadableStream` 实时推送思维步骤

---

## 模块 2: 智能任务分解（Week 3-5）

### 2.1 架构设计

```
复杂任务
    ↓
意图分析（LLM）
    ↓
任务分解树
    ↓
依赖拓扑排序
    ↓
并行/串行执行
```

### 2.2 新增文件

```
src-tauri/crates/agent/src/
├── task_decomposer.rs    # 任务分解核心
├── task_graph.rs         # 任务依赖图
└── task_executor.rs      # 任务执行器

src/components/chat/
├── TaskDecompositionView.tsx   # 分解可视化
└── TaskExecutionView.tsx       # 执行进度
```

### 2.3 核心实现

#### TaskNode 结构

```rust
pub struct TaskNode {
    pub id: String,
    pub description: String,
    pub task_type: TaskType,
    pub dependencies: Vec<String>,
    pub status: TaskStatus,
    pub result: Option<Value>,
}

pub enum TaskType {
    ToolCall(String),     // 工具调用
    Reasoning(String),     // 推理步骤
    Query(String),        // 用户查询
    Validation(String),   // 验证步骤
}
```

#### 分解 prompt 模板

```
你是一个任务分解专家。将以下复杂任务分解为可执行的子任务。

规则：
1. 每个子任务应该是原子的、明确的
2. 标注任务间的依赖关系
3. 识别可以并行执行的任务
4. 包含验证步骤确保任务正确完成

输入: {user_input}

输出格式（JSON）:
{{
  "tasks": [
    {{
      "id": "1",
      "description": "...",
      "type": "tool_call|reasoning|query|validation",
      "dependencies": []
    }}
  ],
  "parallel_groups": [[1, 2], [3], [4, 5]]
}}
```

### 2.4 前端可视化

**任务分解视图**:
- 树形结构显示任务层级
- 依赖连线
- 状态着色（待执行/执行中/完成/失败）
- 可折叠/展开

**执行进度**:
- 实时更新执行状态
- 当前任务高亮
- 完成百分比

---

## 模块 3: 智能错误恢复（Week 5-7）

### 3.1 架构设计

```
执行失败
    ↓
错误分类
    ↓
┌─────────────────────────────────────┐
│ ErrorType                           │
├─────────────────────────────────────┤
│ Transient（临时）: 重试可解决         │
│   - 网络超时                         │
│   - 服务暂时不可用                   │
├─────────────────────────────────────┤
│ Recoverable（可恢复）: 调整参数       │
│   - 资源不足                         │
│   - 权限问题                         │
├─────────────────────────────────────┤
│ Unrecoverable（不可恢复）: 放弃      │
│   - 语法错误                         │
│   - 逻辑错误                         │
└─────────────────────────────────────┘
    ↓
策略选择 → 执行恢复 → 验证
```

### 3.2 新增文件

```
src-tauri/crates/agent/src/
├── error_classifier.rs    # 错误分类器
├── recovery_strategies.rs # 恢复策略库
└── retry_policy.rs       # 重试策略
```

### 3.3 核心实现

#### ErrorClassifier

```rust
pub struct ErrorClassifier;

impl ErrorClassifier {
    pub fn classify(&self, error: &AgentError) -> ErrorType {
        match error {
            AgentError::Network(_) => ErrorType::Transient,
            AgentError::Timeout(_) => ErrorType::Transient,
            AgentError::PermissionDenied(_) => ErrorType::Recoverable,
            AgentError::ResourceExhausted(_) => ErrorType::Recoverable,
            AgentError::InvalidInput(_) => ErrorType::Unrecoverable,
            _ => ErrorType::Unknown,
        }
    }

    pub fn get_recovery_strategy(&self, error: &AgentError) -> RecoveryStrategy {
        match self.classify(error) {
            ErrorType::Transient => RecoveryStrategy::Retry { max_attempts: 3 },
            ErrorType::Recoverable => RecoveryStrategy::AdjustAndRetry,
            ErrorType::Unrecoverable => RecoveryStrategy::Fail,
            ErrorType::Unknown => RecoveryStrategy::Fail,
        }
    }
}
```

#### RetryPolicy

```rust
pub struct RetryPolicy {
    pub max_attempts: usize,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub exponential_backoff: bool,
}

impl RetryPolicy {
    pub fn should_retry(&self, attempt: usize, error: &AgentError) -> bool {
        if attempt >= self.max_attempts {
            return false;
        }
        self.classify(error) == ErrorType::Transient
    }

    pub fn next_delay(&self, attempt: usize) -> Duration {
        if self.exponential_backoff {
            let delay = self.base_delay * 2u32.pow(attempt as u32);
            delay.min(self.max_delay)
        } else {
            self.base_delay
        }
    }
}
```

---

## 模块 4: 反思与自改进（Week 7-9）

### 4.1 架构设计

```
任务完成
    ↓
反思分析（LLM）
    ↓
┌─────────────────────────────────────┐
│ 反思维度                              │
├─────────────────────────────────────┤
│ 1. 结果质量评估                       │
│ 2. 执行效率分析                       │
│ 3. 错误模式识别                       │
│ 4. 知识巩固建议                       │
└─────────────────────────────────────┘
    ↓
生成改进建议 → 更新记忆/知识库
```

### 4.2 新增文件

```
src-tauri/crates/agent/src/
├── reflector.rs           # 反思引擎
└── insight_generator.rs  # 洞察生成器
```

### 4.3 核心实现

#### Reflector

```rust
pub struct Reflector {
    llm: LLMClient,
}

impl Reflector {
    pub async fn reflect(&self, task: &Task, result: &TaskResult) -> Result<Reflection> {
        let prompt = format!(
            r#"
分析以下任务执行过程，提供反思和改进建议。

任务: {}
执行结果: {}
思维链: {:?}
工具使用: {:?}
执行时间: {}ms

请从以下维度分析：
1. 结果质量（1-10）及改进空间
2. 执行效率问题
3. 发现的错误模式
4. 可复用的模式
5. 知识巩固建议
"#
        );

        let response = self.llm.complete(&prompt).await?;
        self.parse_reflection(&response)
    }
}
```

---

## 模块 5: Agent 配置与调优（Week 9-10）

### 5.1 可配置参数

```rust
pub struct AgentConfig {
    // ReAct 引擎
    pub max_iterations: usize,           // 最大推理迭代次数
    pub max_depth: usize,               // 最大任务分解深度
    pub verification_enabled: bool,     // 是否启用结果验证

    // 任务分解
    pub decomposition_threshold: usize, // 触发分解的子任务数
    pub parallel_execution: bool,       // 是否允许并行执行

    // 错误恢复
    pub retry_enabled: bool,            // 是否启用重试
    pub max_retry_attempts: usize,     // 最大重试次数
    pub retry_delay_ms: u64,           // 重试延迟

    // 反思
    pub reflection_enabled: bool,       // 是否启用反思
    pub insight_storage_enabled: bool,  // 是否保存洞察
}
```

### 5.2 前端配置 UI

**组件**: `src/components/settings/AgentSettings.tsx`

- ReAct 参数配置
- 错误恢复策略
- 反思开关
- 调试模式（显示完整思维链）

---

## 技术依赖

### Rust 依赖

```toml
# src-tauri/crates/agent/Cargo.toml
[dependencies]
# 已有
axagent-core = { path = "../core" }
tokio = { workspace = true }
serde = { workspace = true }
anyhow = { workspace = true }

# 新增
```

### 前端依赖

```json
// package.json
{
  "dependencies": {
    // 已有
    "zustand": "^4.x",
    "@tanstack/react-query": "^5.x",

    // 新增
    "@xyflow/react": "^12.x"  // 任务图可视化
  }
}
```

---

## 实施顺序

| 周数 | 模块 | 任务 |
|------|------|------|
| Week 1 | 模块 1 | ReAct 引擎核心实现 |
| Week 2 | 模块 1 | 思维链可视化 + 流式输出 |
| Week 3 | 模块 1+2 | 状态机完善 + 任务分解基础 |
| Week 4 | 模块 2 | 任务分解完整实现 |
| Week 5 | 模块 2+3 | 任务执行器 + 错误分类 |
| Week 6 | 模块 3 | 恢复策略 + 重试机制 |
| Week 7 | 模块 4 | 反思引擎 |
| Week 8 | 模块 4 | 洞察生成 + 知识积累 |
| Week 9 | 模块 5 | 配置系统 + 前端 UI |
| Week 10 | 集成 | 端到端测试 + 优化 |

---

## 风险与备选方案

| 风险 | 影响 | 备选方案 |
|------|------|---------|
| LLM 推理延迟高 | 体验下降 | 异步处理 + 逐步显示 |
| 任务分解质量不稳定 | 执行失败 | 限制复杂度 + 人工干预 |
| 循环推理无法终止 | 系统卡死 | 硬性迭代限制 + 超时 |
| 自我验证误判 | 错误结果 | 保守策略 + 用户确认 |

---

## 成功标准

1. **ReAct 引擎**: 复杂多步骤任务成功率 > 80%
2. **任务分解**: 分解准确性 > 85%（人工评估）
3. **错误恢复**: 可恢复错误恢复率 > 70%
4. **思维可视化**: 用户可见推理过程，无明显延迟
5. **反思能力**: 洞察生成质量可接受

---

## 附录：相关文件

### 现有 Agent 相关文件

```
src-tauri/crates/agent/src/
├── lib.rs                  # Agent 主入口
├── session_manager.rs       # 会话管理
├── tool_registry.rs         # 工具注册
├── provider_adapter.rs      # Provider 适配
└── event_emitter.rs         # 事件发射

src/components/chat/
├── ChatPanel.tsx           # 主聊天面板
└── ComputerControlPanel.tsx # 计算机控制面板
```

### Phase 2 遗留项（可后续处理）

- ArtifactPanel 完整工作区 UI
- MarkdownPreview markstream-react 升级
- Playwright 浏览器自动化支持
