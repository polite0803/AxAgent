# Phase 3 实施计划：高级特性

> 时间周期：2 个月 | 总工作量：20 人天

## 3.1 任务概览

| 任务 | 工作量 | 优先级 | 依赖 |
|------|--------|--------|------|
| 子工作流调用 | 8 人天 | P0 | Phase 1 NodeExecutor Trait |
| 工作流市场 | 6 人天 | P1 | Phase 2 Prompt 模板管理 |
| 高级调试功能 | 6 人天 | P2 | Phase 1 工作流编辑器 |

---

## 3.2 子工作流调用

### 目标
支持在父工作流中异步调用子工作流，包含错误处理、超时控制和结果缓存。

### 详细步骤

#### Step 1: 定义 SubWorkflowNode 类型
**文件**: `src-tauri/crates/runtime/src/work_engine/executors/subworkflow_executor.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubWorkflowNodeConfig {
    pub workflow_id: String,
    pub workflow_version: Option<i32>,
    pub input_mapping: HashMap<String, String>,
    pub output_mapping: HashMap<String, String>,
    pub timeout_secs: Option<u64>,
    pub retry_on_failure: bool,
    pub max_retries: u32,
    pub cache_enabled: bool,
    pub cache_ttl_secs: u64,
}
```

#### Step 2: 实现 SubWorkflowExecutor
**文件**: `src-tauri/crates/runtime/src/work_engine/executors/subworkflow_executor.rs`

```rust
pub struct SubWorkflowExecutor {
    cache: Arc<dyn CacheLayer>,
    http_client: reqwest::Client,
}

#[async_trait]
impl NodeExecutorTrait for SubWorkflowExecutor {
    fn node_type(&self) -> &'static str {
        "subworkflow"
    }

    async fn execute(
        &self,
        node: &WorkflowNode,
        context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let config: SubWorkflowNodeConfig = serde_json::from_value(node.config.clone())
            .map_err(|e| NodeError::ConfigParse(e.to_string()))?;

        let cache_key = self.compute_cache_key(&config, context);
        if config.cache_enabled {
            if let Some(cached) = self.cache.get(&cache_key).await {
                return Ok(NodeOutput::Cached(cached));
            }
        }

        let timeout = Duration::from_secs(config.timeout_secs.unwrap_or(300));
        let result = tokio::time::timeout(
            timeout,
            self.execute_subworkflow(&config, context)
        ).await??;

        if config.cache_enabled {
            self.cache.set(&cache_key, &result, config.cache_ttl_secs).await;
        }

        Ok(NodeOutput::SubWorkflowResult(result))
    }
}
```

#### Step 3: 添加调用结果缓存层
**文件**: `src-tauri/crates/runtime/src/work_engine/cache_layer.rs`

```rust
pub trait CacheLayer: Send + Sync {
    async fn get(&self, key: &str) -> Option<Vec<u8>>;
    async fn set(&self, key: &str, value: &[u8], ttl_secs: u64) -> Result<(), CacheError>;
    async fn delete(&self, key: &str) -> Result<(), CacheError>;
}

pub struct InMemoryCache {
    store: RwLock<HashMap<String, (Vec<u8>, Instant)>>,
    ttl: Duration,
}
```

#### Step 4: 注册执行器到调度器
**文件**: `src-tauri/crates/runtime/src/work_engine/dispatcher.rs`

```rust
impl NodeDispatcher {
    pub fn new() -> Self {
        let mut dispatcher = Self {
            executors: HashMap::new(),
        };
        dispatcher.register(AtomicSkillExecutor::new());
        dispatcher.register(AgentExecutor::new());
        dispatcher.register(LlmExecutor::new());
        dispatcher.register(SubWorkflowExecutor::new());
        dispatcher
    }
}
```

### 验收标准
- [ ] 子工作流可异步调用
- [ ] 支持超时控制
- [ ] 支持错误重试
- [ ] 调用结果可缓存

---

## 3.3 工作流市场

### 目标
实现模板发布、发现、评分和导入/导出功能。

### 详细步骤

#### Step 1: 创建 Marketplace Entity
**文件**: `src-tauri/crates/core/src/entity/workflow_marketplace.rs`

```rust
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "workflow_marketplace")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub template_id: String,
    pub author_id: String,
    pub name: String,
    pub description: Option<String>,
    pub category: String,
    pub tags: Option<String>,
    pub downloads: i64,
    pub rating_average: f32,
    pub rating_count: i32,
    pub is_featured: bool,
    pub is_verified: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "workflow_marketplace_reviews")]
pub struct ReviewModel {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub marketplace_id: String,
    pub user_id: String,
    pub rating: i32,
    pub comment: Option<String>,
    pub created_at: i64,
}
```

#### Step 2: 创建导入/导出服务
**文件**: `src-tauri/crates/core/src/marketplace.rs`

```rust
pub struct MarketplaceService {
    db: DatabaseConnection,
    storage: Arc<dyn FileStorage>,
}

impl MarketplaceService {
    pub async fn export_template(&self, template_id: &str) -> Result<WorkflowExport, Error> {
        let template = self.get_template(template_id).await?;
        let versions = self.get_versions(template_id).await?;

        Ok(WorkflowExport {
            template,
            versions,
            exported_at: Utc::now(),
            version: "1.0".to_string(),
        })
    }

    pub async fn import_template(&self, export: WorkflowExport) -> Result<String, Error> {
        let mut template = export.template;
        template.id = Uuid::new_v4().to_string();
        self.create_template(template).await?;
        Ok(template.id)
    }

    pub async fn publish_template(&self, template_id: &str, category: &str) -> Result<String, Error> {
        let marketplace_id = Uuid::new_v4().to_string();
        // Publish logic here
        Ok(marketplace_id)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkflowExport {
    pub template: workflow_template::Model,
    pub versions: Vec<workflow_template_version::Model>,
    pub exported_at: DateTime<Utc>,
    pub version: String,
}
```

#### Step 3: 创建前端页面
**文件**: `src/pages/WorkflowMarketplace.tsx`

```typescript
interface MarketplaceTemplate {
  id: string;
  name: string;
  description?: string;
  category: string;
  author: string;
  downloads: number;
  rating: number;
  isFeatured: boolean;
}

export function WorkflowMarketplace() {
  const [templates, setTemplates] = useState<MarketplaceTemplate[]>([]);
  const [categories] = useState(['Productivity', 'Development', 'Data', 'Automation']);
  const [selectedCategory, setSelectedCategory] = useState<string | null>(null);

  const filteredTemplates = selectedCategory
    ? templates.filter(t => t.category === selectedCategory)
    : templates;

  return (
    <div className="flex h-full">
      <aside className="w-56 border-r">
        <h3>Categories</h3>
        <ul>
          <li onClick={() => setSelectedCategory(null)}>All</li>
          {categories.map(cat => (
            <li key={cat} onClick={() => setSelectedCategory(cat)}>{cat}</li>
          ))}
        </ul>
      </aside>
      <main className="flex-1 p-6">
        <div className="grid grid-cols-3 gap-4">
          {filteredTemplates.map(template => (
            <TemplateCard key={template.id} template={template} />
          ))}
        </div>
      </main>
    </div>
  );
}
```

### 验收标准
- [ ] 模板可发布到市场
- [ ] 支持分类浏览和搜索
- [ ] 支持评分和评论
- [ ] 支持导入/导出

---

## 3.4 高级调试功能

### 目标
提供执行历史可视化、变量状态快照和性能分析。

### 详细步骤

#### Step 1: 扩展 WorkflowExecution Entity
**文件**: `src-tauri/crates/core/src/entity/workflow_executions.rs`

```rust
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "workflow_executions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub workflow_id: String,
    pub status: String,
    pub input: String,
    pub output: Option<String>,
    pub error: Option<String>,
    pub started_at: i64,
    pub completed_at: Option<i64>,
    pub node_executions: Option<String>,
    pub variable_snapshots: Option<String>,
    pub performance_metrics: Option<String>,
}
```

#### Step 2: 创建调试面板组件
**文件**: `src/components/workflow/DebugPanel.tsx`

```typescript
interface ExecutionHistory {
  id: string;
  status: 'success' | 'failed' | 'running';
  startedAt: number;
  completedAt?: number;
  nodeExecutions: NodeExecution[];
  variableSnapshots: VariableSnapshot[];
  performanceMetrics: PerformanceMetrics;
}

interface NodeExecution {
  nodeId: string;
  nodeName: string;
  nodeType: string;
  status: string;
  startTime: number;
  endTime?: number;
  duration?: number;
  input: Record<string, any>;
  output?: Record<string, any>;
  error?: string;
}

export function DebugPanel({ executionId }: { executionId: string }) {
  const [history, setHistory] = useState<ExecutionHistory | null>(null);
  const [activeTab, setActiveTab] = useState<'history' | 'variables' | 'performance'>('history');

  return (
    <div className="flex flex-col h-full border-l">
      <Tabs activeKey={activeTab} onChange={(k) => setActiveTab(k as any)}>
        <Tabs.TabPane tab="Execution History" key="history">
          <ExecutionTimeline history={history} />
        </Tabs.TabPane>
        <Tabs.TabPane tab="Variable Snapshots" key="variables">
          <VariableInspector snapshots={history?.variableSnapshots} />
        </Tabs.TabPane>
        <Tabs.TabPane tab="Performance" key="performance">
          <PerformancePanel metrics={history?.performanceMetrics} />
        </Tabs.TabPane>
      </Tabs>
    </div>
  );
}
```

#### Step 3: 实现变量快照功能
**文件**: `src-tauri/crates/runtime/src/work_engine/variable_snapshot.rs`

```rust
pub struct VariableSnapshot {
    pub node_id: String,
    pub timestamp: DateTime<Utc>,
    pub variables: HashMap<String, serde_json::Value>,
}

pub struct VariableTracker {
    snapshots: Vec<VariableSnapshot>,
}

impl VariableTracker {
    pub fn new() -> Self {
        Self { snapshots: Vec::new() }
    }

    pub fn snapshot(&mut self, node_id: &str, vars: HashMap<String, serde_json::Value>) {
        self.snapshots.push(VariableSnapshot {
            node_id: node_id.to_string(),
            timestamp: Utc::now(),
            variables: vars,
        });
    }

    pub fn get_snapshots(&self) -> &[VariableSnapshot] {
        &self.snapshots
    }

    pub fn get_snapshot_at(&self, index: usize) -> Option<&VariableSnapshot> {
        self.snapshots.get(index)
    }
}
```

#### Step 4: 实现性能分析
**文件**: `src-tauri/crates/runtime/src/work_engine/performance_analyzer.rs`

```rust
pub struct PerformanceMetrics {
    pub total_duration_ms: u64,
    pub node_durations: HashMap<String, u64>,
    pub node_wait_times: HashMap<String, u64>,
    pub token_usage: TokenUsage,
    pub memory_peak_mb: f64,
}

pub struct PerformanceAnalyzer;

impl PerformanceAnalyzer {
    pub fn analyze(execution: &WorkflowExecution) -> PerformanceMetrics {
        let mut node_durations = HashMap::new();
        let mut node_wait_times = HashMap::new();

        if let Some(node_executions) = &execution.node_executions {
            let executions: Vec<NodeExecutionRecord> = serde_json::from_str(node_executions).unwrap();
            for exec in &executions {
                let duration = exec.end_time_ms.saturating_sub(exec.start_time_ms);
                node_durations.insert(exec.node_id.clone(), duration);
            }
        }

        PerformanceMetrics {
            total_duration_ms: execution.total_duration_ms(),
            node_durations,
            node_wait_times,
            token_usage: execution.token_usage().unwrap_or_default(),
            memory_peak_mb: execution.memory_peak_mb().unwrap_or(0.0),
        }
    }

    pub fn generate_report(&self, metrics: &PerformanceMetrics) -> String {
        let slowest_node = metrics.node_durations.iter()
            .max_by_key(|(_, v)| *v)
            .map(|(k, _)| k);

        format!(
            "Total Duration: {}ms\nSlowest Node: {:?}\nMemory Peak: {:.2}MB",
            metrics.total_duration_ms,
            slowest_node,
            metrics.memory_peak_mb
        )
    }
}
```

### 验收标准
- [ ] 可视化执行历史
- [ ] 可查看变量状态快照
- [ ] 显示性能分析数据

---

## 3.5 里程碑

| 里程碑 | 日期 | 完成内容 |
|--------|------|----------|
| M1 | 第 1-2 周 | 子工作流调用核心实现 |
| M2 | 第 3 周 | 子工作流缓存和错误处理 |
| M3 | 第 4 周 | 工作流市场实体和服务 |
| M4 | 第 5 周 | 工作流市场前端 |
| M5 | 第 6-7 周 | 高级调试功能 |
| M8 | 第 8 周 | 测试和集成 |

---

## 3.6 文件变更清单

```
src-tauri/crates/runtime/src/work_engine/
├── executors/
│   ├── subworkflow_executor.rs    # 新增
│   └── mod.rs                     # 修改: 导出 SubWorkflowExecutor
├── cache_layer.rs                 # 新增
├── variable_snapshot.rs           # 新增
├── performance_analyzer.rs        # 新增
└── dispatcher.rs                  # 修改: 注册 SubWorkflowExecutor

src-tauri/crates/core/src/
├── entity/
│   ├── workflow_marketplace.rs    # 新增
│   └── workflow_marketplace_reviews.rs  # 新增
├── marketplace.rs                  # 新增
└── entity/mod.rs                  # 修改: 导出新实体

src/pages/
└── WorkflowMarketplace.tsx        # 新增

src/components/workflow/
└── DebugPanel.tsx                # 新增
```

---

## 3.7 测试计划

| 测试类型 | 覆盖率目标 | 工具 |
|---------|-----------|------|
| 单元测试 | 80% | `cargo test` |
| 集成测试 | 关键路径 | `cargo test --test` |
| E2E 测试 | 核心流程 | Playwright |

---

## 3.8 风险评估

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| 子工作流循环依赖 | 高 | 图循环检测，阻止发布 |
| 市场导入安全 | 中 | 沙箱验证，模板签名 |
| 调试性能开销 | 低 | 按需启用，快照压缩 |
