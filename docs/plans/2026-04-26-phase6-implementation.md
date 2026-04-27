# Phase 6: 开发者生态 - 实施计划

> 文档版本: 1.0
> 创建日期: 2026-04-26
> 阶段周期: 2-3 个月
> 基于版本: AxAgent 当前代码库基线

---

## 一、概述

### 1.1 阶段目标

打造开放的开发者生态系统，提供调试追踪和性能评估能力。

### 1.2 核心模块

| 模块 | 描述 | 优先级 |
|------|------|--------|
| 模块 1 | 可视化执行追踪 | P0 |
| 模块 2 | 评估框架 | P1 |

### 1.3 技术挑战

1. **追踪性能开销**：在不影响主流程性能的情况下记录完整调用链
2. **评估基准标准化**：建立客观可重复的评估指标

---

## 二、模块 1: 可视化执行追踪

### 2.1 概述

构建类似 LangSmith 的调试和追踪工具，完整记录 Agent 执行过程中的调用链路、耗时分析和成本追踪。

### 2.2 当前基线

现有 `telemetry` crate 仅包含 API 请求配置文件，无执行追踪能力。
- `src-tauri/crates/telemetry/src/lib.rs` - ClientIdentity, AnthropicRequestProfile
- 无调用链追踪
- 无工具执行记录
- 无成本追踪

### 2.3 架构设计

#### 2.3.1 Rust 后端模块

扩展 `src-tauri/crates/telemetry/` crate：

```
src-tauri/crates/telemetry/src/
├── lib.rs                      # 现有入口
├── tracer.rs                   # 追踪器核心
├── span.rs                     # 调用跨度
├── event.rs                    # 追踪事件
├── storage.rs                  # 追踪存储
├── exporter.rs                 # 导出器
└── metrics.rs                  # 指标收集
```

#### 2.3.2 核心数据结构

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trace {
    pub id: String,
    pub trace_id: String,
    pub parent_span_id: Option<String>,
    pub span_id: String,
    pub name: String,
    pub span_type: SpanType,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
    pub status: SpanStatus,
    pub attributes: HashMap<String, serde_json::Value>,
    pub events: Vec<SpanEvent>,
    pub inputs: Option<serde_json::Value>,
    pub outputs: Option<serde_json::Value>,
    pub errors: Vec<SpanError>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpanType {
    Agent,
    Tool,
    LlmCall,
    Task,
    SubTask,
    Reflection,
    Reasoning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpanStatus {
    Ok,
    Error,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanEvent {
    pub name: String,
    pub timestamp: DateTime<Utc>,
    pub attributes: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanError {
    pub error_type: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub stack_trace: Option<String>,
}
```

#### 2.3.3 追踪器接口

```rust
pub trait Tracer: Send + Sync {
    fn start_span(&self, name: &str, span_type: SpanType) -> SpanId;
    fn end_span(&self, span_id: SpanId, status: SpanStatus);
    fn add_event(&self, span_id: SpanId, event: SpanEvent);
    fn record_error(&self, span_id: SpanId, error: SpanError);
    fn set_attribute(&self, span_id: SpanId, key: &str, value: serde_json::Value);
    fn get_trace(&self, trace_id: &str) -> Option<Trace>;
    fn export(&self) -> impl Future<Output = Result<Vec<u8>, TracerError>>;
}
```

### 2.4 实现细节

#### 2.4.1 Span 管理 (span.rs)

```rust
pub struct Span {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub name: String,
    pub span_type: SpanType,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub status: SpanStatus,
    pub attributes: HashMap<String, serde_json::Value>,
    pub events: Vec<SpanEvent>,
    pub inputs: Option<serde_json::Value>,
    pub outputs: Option<serde_json::Value>,
    pub errors: Vec<SpanError>,
}

impl Span {
    pub fn new(name: String, span_type: SpanType, parent: Option<&Span>) -> Self {
        let trace_id = parent
            .map(|p| p.trace_id.clone())
            .unwrap_or_else(|| Ulid::new().to_string());
        let parent_span_id = parent.map(|p| p.span_id.clone());

        Self {
            trace_id,
            span_id: Ulid::new().to_string(),
            parent_span_id,
            name,
            span_type,
            start_time: Utc::now(),
            end_time: None,
            status: SpanStatus::Ok,
            attributes: HashMap::new(),
            events: Vec::new(),
            inputs: None,
            outputs: None,
            errors: Vec::new(),
        }
    }

    pub fn duration_ms(&self) -> Option<u64> {
        self.end_time.map(|end| (end - self.start_time).num_milliseconds() as u64)
    }
}
```

#### 2.4.2 事件追踪 (event.rs)

```rust
#[derive(Debug, Clone)]
pub struct TraceEvent {
    pub event_type: EventType,
    pub timestamp: DateTime<Utc>,
    pub span_id: String,
    pub data: EventData,
}

#[derive(Debug, Clone)]
pub enum EventType {
    SpanStarted,
    SpanEnded,
    LlmCallStarted,
    LlmCallEnded,
    ToolCallStarted,
    ToolCallEnded,
    ErrorOccurred,
    UserFeedback,
}

#[derive(Debug, Clone)]
pub enum EventData {
    LlmCall { model: String, prompt_tokens: u32, completion_tokens: u32 },
    ToolCall { tool_name: String, arguments: serde_json::Value },
    Error { error_type: String, message: String },
    UserFeedback { feedback_type: String, content: String },
}
```

#### 2.4.3 指标收集 (metrics.rs)

```rust
#[derive(Debug, Clone, Default)]
pub struct MetricsCollector {
    spans_count: AtomicU64,
    errors_count: AtomicU64,
    total_duration_ms: AtomicU64,
    llm_tokens_total: AtomicU64,
    llm_cost_total: AtomicF64,
}

impl MetricsCollector {
    pub fn record_span(&self, duration_ms: u64, span_type: SpanType) {
        self.spans_count.fetch_add(1, Ordering::Relaxed);
        self.total_duration_ms.fetch_add(duration_ms, Ordering::Relaxed);
    }

    pub fn record_llm_usage(&self, tokens: u64, cost: f64) {
        self.llm_tokens_total.fetch_add(tokens, Ordering::Relaxed);
        self.llm_cost_total.fetch_add(cost, Ordering::Relaxed);
    }

    pub fn record_error(&self) {
        self.errors_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_summary(&self) -> MetricsSummary {
        MetricsSummary {
            total_spans: self.spans_count.load(Ordering::Relaxed),
            total_errors: self.errors_count.load(Ordering::Relaxed),
            total_duration_ms: self.total_duration_ms.load(Ordering::Relaxed),
            llm_tokens_total: self.llm_tokens_total.load(Ordering::Relaxed),
            llm_cost_total: self.llm_cost_total.load(Ordering::Relaxed),
        }
    }
}
```

#### 2.4.4 存储与导出 (storage.rs, exporter.rs)

```rust
pub trait TraceStorage: Send + Sync {
    async fn save(&self, trace: &Trace) -> Result<(), TracerError>;
    async fn load(&self, trace_id: &str) -> Result<Option<Trace>, TracerError>;
    async fn list(&self, filter: TraceFilter) -> Result<Vec<TraceSummary>, TracerError>;
    async fn delete(&self, trace_id: &str) -> Result<(), TracerError>;
}

pub trait TraceExporter: Send + Sync {
    fn export(&self, traces: &[Trace]) -> impl Future<Output = Result<Vec<u8>, TracerError>>;
    fn content_type(&self) -> &str;
}

pub struct JsonExporter;
pub struct CsvExporter;
```

### 2.5 前端集成

#### 2.5.1 新增页面和组件

```
src/pages/DevTools/TraceExplorer.tsx    # 追踪浏览器页面
src/components/devtools/
├── TraceTimeline.tsx          # 时间线视图
├── TraceList.tsx              # 追踪列表
├── SpanDetail.tsx             # Span 详情
├── MetricsPanel.tsx           # 指标面板
└── TraceFilters.tsx           # 筛选器
```

#### 2.5.2 TraceExplorer.tsx

```tsx
export function TraceExplorer() {
  const { traces, selectedTrace, loading, loadTraces, selectTrace } = useTracerStore();

  return (
    <div className="h-full flex">
      <div className="w-1/3 border-r overflow-auto">
        <TraceFilters />
        <TraceList
          traces={traces}
          selectedId={selectedTrace?.id}
          onSelect={selectTrace}
        />
      </div>
      <div className="flex-1 overflow-auto">
        {selectedTrace ? (
          <>
            <TraceTimeline trace={selectedTrace} />
            <SpanDetail span={selectedTrace.rootSpan} />
            <MetricsPanel trace={selectedTrace} />
          </>
        ) : (
          <Empty description="选择一个追踪记录" />
        )}
      </div>
    </div>
  );
}
```

### 2.6 文件清单

| 文件路径 | 描述 | 操作 |
|---------|------|------|
| `src-tauri/crates/telemetry/src/tracer.rs` | 追踪器核心 | 新建 |
| `src-tauri/crates/telemetry/src/span.rs` | 调用跨度 | 新建 |
| `src-tauri/crates/telemetry/src/event.rs` | 追踪事件 | 新建 |
| `src-tauri/crates/telemetry/src/storage.rs` | 存储接口 | 新建 |
| `src-tauri/crates/telemetry/src/exporter.rs` | 导出器 | 新建 |
| `src-tauri/crates/telemetry/src/metrics.rs` | 指标收集 | 新建 |
| `src-tauri/crates/telemetry/src/lib.rs` | 模块入口 | 修改 |
| `src-tauri/src/commands/tracer.rs` | Tauri 命令 | 新建 |
| `src/pages/DevTools/TraceExplorer.tsx` | 追踪浏览器 | 新建 |
| `src/components/devtools/TraceTimeline.tsx` | 时间线视图 | 新建 |
| `src/components/devtools/TraceList.tsx` | 追踪列表 | 新建 |
| `src/components/devtools/SpanDetail.tsx` | Span 详情 | 新建 |
| `src/components/devtools/MetricsPanel.tsx` | 指标面板 | 新建 |
| `src/components/devtools/TraceFilters.tsx` | 筛选器 | 新建 |
| `src/stores/devtools/tracerStore.ts` | 追踪状态管理 | 新建 |
| `src/types/tracer.ts` | TypeScript 类型 | 新建 |

### 2.7 验收标准

- [ ] 完整的调用链追踪
- [ ] LLM 调用成本记录
- [ ] 工具执行时间分析
- [ ] 追踪数据导出（JSON/CSV）
- [ ] 前端追踪浏览器
- [ ] 性能指标仪表盘

---

## 三、模块 2: 评估框架

### 3.1 概述

内置基准测试系统，支持自动化评估 Agent 在各种任务上的表现。

### 3.2 当前基线

现有 `agent` crate 无评估能力。

### 3.3 架构设计

#### 3.3.1 Rust 后端模块

```
src-tauri/crates/agent/src/evaluator/
├── mod.rs                      # 模块入口
├── benchmark.rs                # 基准测试定义
├── dataset.rs                  # 数据集管理
├── runner.rs                   # 评估运行器
├── metrics.rs                  # 评估指标
└── reporter.rs                 # 报告生成
```

#### 3.3.2 核心数据结构

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Benchmark {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: BenchmarkCategory,
    pub tasks: Vec<BenchmarkTask>,
    pub metadata: BenchmarkMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkTask {
    pub id: String,
    pub name: String,
    pub description: String,
    pub input: TaskInput,
    pub expected_output: Option<TaskOutput>,
    pub evaluation_criteria: Vec<EvaluationCriteria>,
    pub difficulty: Difficulty,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvaluationMetric {
    ExactMatch,
    Contains,
    SemanticSimilarity,
    LevenshteinSimilarity,
    ToolCorrectness,
    StateCorrectness,
    Performance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationCriteria {
    pub name: String,
    pub metric: EvaluationMetric,
    pub weight: f32,
    pub threshold: Option<f32>,
}
```

#### 3.3.3 评估运行器

```rust
pub struct EvaluationRunner {
    config: RunnerConfig,
    metrics_calculator: MetricsCalculator,
}

pub struct RunnerConfig {
    pub max_concurrency: usize,
    pub timeout_ms: u64,
    pub max_difficulty: Option<Difficulty>,
    pub include_traces: bool,
}

impl EvaluationRunner {
    pub async fn run_benchmark(&self, benchmark: &Benchmark) -> BenchmarkResult {
        let mut task_results = Vec::new();

        for task in &benchmark.tasks {
            let result = self.run_task(task).await;
            task_results.push(result);
        }

        let aggregate = self.aggregate_results(&task_results);

        BenchmarkResult {
            benchmark_id: benchmark.id.clone(),
            run_at: Utc::now(),
            task_results,
            aggregate,
        }
    }

    async fn run_task(&self, task: &BenchmarkTask) -> TaskResult {
        let response = self.execute_task(task).await;
        let scores = self.evaluate_task(task, &response);
        let overall_score = scores.values().sum::<f32>() / scores.len() as f32;

        TaskResult {
            task_id: task.id.clone(),
            success: overall_score >= 0.5,
            duration_ms: 0,
            response: Some(response),
            scores,
        }
    }
}
```

#### 3.3.4 指标计算

```rust
pub struct MetricsCalculator {
    threshold: f32,
}

impl MetricsCalculator {
    pub fn new() -> Self {
        Self { threshold: 0.5 }
    }

    pub async fn evaluate(&self, task: &BenchmarkTask, response: &AgentResponse) -> HashMap<String, f32> {
        let mut scores = HashMap::new();

        for criteria in &task.evaluation_criteria {
            let score = match criteria.metric {
                EvaluationMetric::ExactMatch => {
                    self.eval_exact_match(task, response)
                }
                EvaluationMetric::Contains => {
                    self.eval_contains(task, response)
                }
                EvaluationMetric::SemanticSimilarity => {
                    self.eval_semantic_similarity(task, response).await
                }
                EvaluationMetric::ToolCorrectness => {
                    self.eval_tool_correctness(task, response)
                }
                EvaluationMetric::Performance => {
                    self.eval_performance(task, response)
                }
                _ => 0.0,
            };
            scores.insert(criteria.name.clone(), score);
        }

        scores
    }

    fn eval_exact_match(&self, task: &BenchmarkTask, response: &Result<AgentResponse, AgentError>) -> f32 {
        let expected = task.expected_output.as_ref().map(|o| o.content.as_str()).unwrap_or("");
        let actual = response.as_ref().ok().map(|r| r.content.as_str()).unwrap_or("");

        if expected.trim() == actual.trim() {
            1.0
        } else {
            0.0
        }
    }

    fn aggregate_results(&self, results: &[TaskResult]) -> AggregateMetrics {
        let total = results.len() as f32;
        let passed = results.iter().filter(|r| r.success).count() as f32;
        let avg_duration: f32 = results.iter().map(|r| r.duration_ms as f32).sum::<f32>() / total;

        let avg_scores: HashMap<String, f32> = if results.is_empty() {
            HashMap::new()
        } else {
            let mut sums = HashMap::new();
            let mut counts = HashMap::new();

            for result in results {
                for (name, score) in &result.scores {
                    *sums.entry(name.clone()).or_insert(0.0) += score;
                    *counts.entry(name.clone()).or_insert(0) += 1;
                }
            }

            sums.into_iter()
                .map(|(k, v)| (k, v / counts.get(&k).copied().unwrap_or(1) as f32))
                .collect()
        };

        AggregateMetrics {
            pass_rate: passed / total,
            avg_duration_ms: avg_duration,
            avg_scores,
            overall_score: avg_scores.values().sum::<f32>() / avg_scores.len().max(1) as f32,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub benchmark_id: String,
    pub run_at: DateTime<Utc>,
    pub task_results: Vec<TaskResult>,
    pub aggregate: AggregateMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub success: bool,
    pub duration_ms: u64,
    pub response: Option<AgentResponse>,
    pub error: Option<String>,
    pub scores: HashMap<String, f32>,
    pub trace_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateMetrics {
    pub pass_rate: f32,
    pub avg_duration_ms: f32,
    pub avg_scores: HashMap<String, f32>,
    pub overall_score: f32,
}
```

### 3.5 前端集成

#### 3.5.1 新增页面和组件

```
src/pages/DevTools/BenchmarkRunner.tsx   # 基准测试运行页面
src/components/benchmark/
├── BenchmarkSelector.tsx    # 基准测试选择器
├── TaskList.tsx             # 任务列表
├── TaskResult.tsx           # 单个任务结果
├── ScoreCard.tsx            # 评分卡片
├── MetricsChart.tsx         # 指标图表
└── ReportGenerator.tsx     # 报告生成
```

#### 3.5.2 BenchmarkRunner.tsx

```tsx
export function BenchmarkRunner() {
  const [selectedBenchmark, setSelectedBenchmark] = useState<string | null>(null);
  const [results, setResults] = useState<BenchmarkResult | null>(null);
  const [isRunning, setIsRunning] = useState(false);
  const [config, setConfig] = useState<RunnerConfig>({
    maxConcurrency: 3,
    timeoutMs: 60000,
    maxDifficulty: undefined,
  });

  const runBenchmark = async () => {
    if (!selectedBenchmark) return;

    setIsRunning(true);
    try {
      const result = await invoke<BenchmarkResult>('run_benchmark', {
        benchmarkId: selectedBenchmark,
        config,
      });
      setResults(result);
    } catch (error) {
      message.error(`基准测试失败: ${error}`);
    } finally {
      setIsRunning(false);
    }
  };

  return (
    <div className="p-6">
      <BenchmarkSelector
        selected={selectedBenchmark}
        onChange={setSelectedBenchmark}
      />

      <ConfigPanel config={config} onChange={setConfig} />

      <Button
        type="primary"
        onClick={runBenchmark}
        loading={isRunning}
        disabled={!selectedBenchmark}
      >
        运行基准测试
      </Button>

      {results && (
        <BenchmarkReport result={results} />
      )}
    </div>
  );
}
```

### 3.6 文件清单

| 文件路径 | 描述 | 操作 |
|---------|------|------|
| `src-tauri/crates/agent/src/evaluator/mod.rs` | 模块入口 | 新建 |
| `src-tauri/crates/agent/src/evaluator/benchmark.rs` | 基准测试定义 | 新建 |
| `src-tauri/crates/agent/src/evaluator/dataset.rs` | 数据集管理 | 新建 |
| `src-tauri/crates/agent/src/evaluator/runner.rs` | 评估运行器 | 新建 |
| `src-tauri/crates/agent/src/evaluator/metrics.rs` | 评估指标 | 新建 |
| `src-tauri/crates/agent/src/evaluator/reporter.rs` | 报告生成 | 新建 |
| `src-tauri/crates/agent/src/lib.rs` | 模块导出更新 | 修改 |
| `src-tauri/src/commands/evaluator.rs` | Tauri 命令 | 新建 |
| `src/pages/DevTools/BenchmarkRunner.tsx` | 基准测试页面 | 新建 |
| `src/components/benchmark/BenchmarkSelector.tsx` | 测试选择器 | 新建 |
| `src/components/benchmark/TaskList.tsx` | 任务列表 | 新建 |
| `src/components/benchmark/TaskResult.tsx` | 任务结果 | 新建 |
| `src/components/benchmark/ScoreCard.tsx` | 评分卡片 | 新建 |
| `src/components/benchmark/MetricsChart.tsx` | 指标图表 | 新建 |
| `src/components/benchmark/ReportGenerator.tsx` | 报告生成 | 新建 |
| `src/types/evaluator.ts` | TypeScript 类型 | 新建 |
| `src/stores/devtools/evaluatorStore.ts` | 评估状态管理 | 新建 |

### 3.7 验收标准

- [ ] 内置多种基准测试数据集
- [ ] 支持自定义基准测试导入
- [ ] 自动化评估流程
- [ ] 生成详细的性能报告
- [ ] 支持评估结果历史对比

---

## 四、风险与缓解

| 风险 | 影响 | 缓解策略 |
|------|------|---------|
| 追踪性能开销 | 中 | 异步记录、采样策略、可关闭 |
| 评估主观性 | 中 | 多维度指标、用户可配置权重 |
| 解耦复杂度 | 高 | 渐进式重构、先抽象后实现 |

---

## 五、附录

### 5.1 术语表

| 术语 | 定义 |
|------|------|
| Trace | 一次完整的 Agent 执行追踪记录 |
| Span | 调用链中的单个操作单元 |
| Benchmark | 基准测试套件 |
| Task | 基准测试中的单个评估任务 |
| AgentCore | 与框架无关的核心 Agent 抽象 |

### 5.2 参考文档

- [Phase 5 实施文档](./2026-04-26-phase5-implementation.md)
- [AxAgent 升级路线图](./2026-04-26-axagent-upgrade-roadmap.md)
- 现有 `telemetry` crate
- 现有 `agent` crate
- 现有 `trajectory` crate
