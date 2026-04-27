# Phase 4: 研究型 Agent 实施计划

> 规划日期: 2026-04-26
> 阶段目标: 实现 Perplexity/GPT Research 级别的深度研究能力
> 规划依据: Phase 2 计算机控制能力 + Phase 3 深度推理能力
> 执行周期: 2-3 月

---

## 一、当前项目基线

### 1.1 已完成模块

| 阶段 | 模块 | 状态 | 关键文件 |
|------|------|------|----------|
| Phase 2 | 屏幕感知与计算机控制 | ✅ 完成 | screen_capture.rs, computer_control.rs, ui_automation.rs |
| Phase 3 | ReAct 推理引擎 | ✅ 完成 | react_engine.rs, reasoning_state.rs, thought_chain.rs |
| Phase 3 | 智能任务分解 | ✅ 完成 | task.rs, task_decomposer.rs, task_executor.rs |
| Phase 3 | 智能错误恢复 | ✅ 完成 | error_classifier.rs, recovery_strategies.rs, retry_policy.rs, error_recovery_engine.rs |
| Phase 3 | 反思与自改进 | ✅ 完成 | reflector.rs, insight_generator.rs |
| Phase 3 | Agent 配置与调优 | ✅ 完成 | agent_config.rs, ConfigManager |

### 1.2 技术栈现状

**Rust 后端**:
- `axagent-core`: 屏幕捕获、UI 自动化、计算机控制、操作审计
- `axagent-agent`: ReAct 引擎、任务分解、错误恢复、反思引擎、配置管理
- `axagent-runtime`: 工作流引擎
- `axagent-trajectory`: 轨迹记录

**前端**:
- React + TypeScript + Ant Design
- Tauri IPC 通信
- 状态管理: Zustand

### 1.3 可复用基础设施

- **任务系统**: task.rs, TaskGraph, TaskExecutor 可扩展支持研究任务
- **错误恢复**: error_recovery_engine.rs 可用于研究过程中的错误处理
- **反思引擎**: reflector.rs 可用于研究结果的质量评估
- **配置系统**: agent_config.rs 可扩展支持研究参数配置
- **前端面板**: React 组件架构可复用

---

## 二、目标架构

### 2.1 研究型 Agent 架构

```
用户输入（研究主题）
        ↓
研究规划器（Research Planner）
        ↓
┌─────────────────────────────────────────────────────────────┐
│                    研究阶段                                  │
├─────────────────────────────────────────────────────────────┤
│  1. 搜索计划生成                                            │
│      ↓                                                      │
│  2. 并行搜索执行 ─────────────────────────────────────→ 搜索结果 │
│      ↓                                                      │
│  3. 信息提取与可信度评估                                     │
│      ↓                                                      │
│  4. 综合分析                                                │
│      ↓                                                      │
│  5. 报告生成                                                │
└─────────────────────────────────────────────────────────────┘
        ↓
引用追踪系统 ←────────────────────────────
        ↓
研究报告输出（Markdown/HTML）
```

### 2.2 核心组件

| 组件 | 职责 | 依赖 |
|------|------|------|
| ResearchAgent | 研究主控制器 | ReAct Engine, TaskExecutor |
| SearchOrchestrator | 搜索计划与执行 | Phase 2 ComputerControl, 搜索引擎 API |
| CredibilityEvaluator | 信息可信度评估 | LLM 分析 |
| CitationTracker | 引用追踪管理 | Trajectory 系统 |
| ReportGenerator | 报告生成 | LLM, 反思引擎 |

---

## 三、模块实施计划

### 模块 1: 研究智能体核心（Week 1-2）

#### 1.1 新增文件

```
src-tauri/crates/agent/src/
├── research_agent.rs      # 研究智能体主控制器
├── research_state.rs      # 研究状态管理
├── search_planner.rs      # 搜索规划器
└── search_orchestrator.rs  # 搜索编排器
```

#### 1.2 核心实现

**research_state.rs** - 研究状态定义:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchState {
    pub id: String,
    pub topic: String,
    pub status: ResearchStatus,
    pub current_phase: ResearchPhase,
    pub search_results: Vec<SearchResult>,
    pub citations: Vec<Citation>,
    pub progress: ResearchProgress,
    pub config: ResearchConfig,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ResearchPhase {
    Planning,
    Searching,
    Extracting,
    Analyzing,
    Synthesizing,
    Reporting,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ResearchStatus {
    Pending,
    InProgress,
    Paused,
    Completed,
    Failed,
}
```

**research_agent.rs** - 研究智能体:

```rust
pub struct ResearchAgent {
    planner: SearchPlanner,
    orchestrator: SearchOrchestrator,
    credibility: CredibilityEvaluator,
    citation_tracker: CitationTracker,
    report_generator: ReportGenerator,
    state: Arc<RwLock<ResearchState>>,
    event_emitter: broadcast::Sender<ResearchEvent>,
}
```

#### 1.3 前端组件

```
src/components/chat/
├── ResearchPanel.tsx           # 研究主面板
├── ResearchProgress.tsx        # 研究进度展示
└── ResearchReport.tsx          # 报告预览
```

### 模块 2: 搜索编排系统（Week 2-3）

#### 2.1 新增文件

```
src-tauri/crates/agent/src/
├── search_provider.rs    # 搜索提供者抽象
├── web_search.rs         # Web 搜索实现
├── academic_search.rs    # 学术搜索实现
└── source_validator.rs   # 来源验证器
```

#### 2.2 核心实现

**搜索提供者接口**:

```rust
#[async_trait]
pub trait SearchProvider: Send + Sync {
    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, SearchError>;
    async fn extract_content(&self, url: &str) -> Result<ExtractedContent, ExtractError>;
    fn source_type(&self) -> SourceType;
}
```

**SearchOrchestrator** 并行搜索:

```rust
pub struct SearchOrchestrator {
    providers: Vec<Box<dyn SearchProvider>>,
    max_concurrent: usize,
}

impl SearchOrchestrator {
    pub async fn search(&self, plan: &SearchPlan) -> Result<Vec<SearchResult>, OrchestratorError> {
        // 1. 按 source_type 分组查询
        // 2. 并行执行搜索
        // 3. 去重和排序
        // 4. 返回结果
    }
}
```

#### 2.3 搜索源集成

| 搜索源 | 类型 | 用途 |
|--------|------|------|
| Web Search API | 通用 | 常规网页搜索 |
| Academic Search | 学术 | 论文、学术资料 |
| Wikipedia API | 百科 | 基础概念确认 |
| GitHub API | 代码 | 代码片段搜索 |

### 模块 3: 可信度评估系统（Week 3-4）

#### 3.1 新增文件

```
src-tauri/crates/agent/src/
├── credibility_evaluator.rs    # 可信度评估器
├── source_classifier.rs        # 来源分类器
└── fact_checker.rs             # 事实核查
```

#### 3.2 核心实现

**CredibilityEvaluator**:

```rust
pub struct CredibilityEvaluator {
    source_weights: HashMap<SourceType, f32>,
    llm_client: LLMClient,
}

impl CredibilityEvaluator {
    pub async fn evaluate(&self, source: &Source) -> CredibilityScore {
        let authority = self.evaluate_authority(source).await;
        let consistency = self.evaluate_consistency(source).await;
        let recency = self.evaluate_recency(source);
        let objectivity = self.evaluate_objectivity(source).await;

        CredibilityScore {
            overall: Self::weighted_score(authority, consistency, recency, objectivity),
            authority,
            consistency,
            recency,
            objectivity,
        }
    }
}
```

**评估维度**:

| 维度 | 权重 | 评估方法 |
|------|------|----------|
| Authority | 30% | 来源权威性（官方 > 媒体 > 个人博客） |
| Consistency | 25% | 多源交叉验证 |
| Recency | 20% | 信息时效性 |
| Objectivity | 25% | 主观性分析（事实 vs 观点） |

### 模块 4: 引用追踪系统（Week 4-5）

#### 4.1 新增文件

```
src-tauri/crates/agent/src/
├── citation.rs           # 引用结构定义
├── citation_tracker.rs   # 引用追踪器
└── reference_builder.rs  # 参考文献构建器
```

#### 4.2 核心实现

**Citation 结构**:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    pub id: String,
    pub source_url: String,
    pub source_title: String,
    pub accessed_at: DateTime<Utc>,
    pub quoted_text: Option<String>,
    pub page_number: Option<u32>,
    pub credibility: CredibilityScore,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitationTracker {
    citations: Arc<RwLock<HashMap<String, Citation>>>,
    usage_records: Arc<RwLock<HashMap<String, Vec<CitationUsage>>>>,
}
```

### 模块 5: 报告生成系统（Week 5-6）

#### 5.1 新增文件

```
src-tauri/crates/agent/src/
├── report_generator.rs   # 报告生成器
├── outline_builder.rs   # 大纲构建器
└── content_synthesizer.rs # 内容综合
```

#### 5.2 核心实现

**ReportGenerator**:

```rust
pub struct ReportGenerator {
    llm_client: LLMClient,
    reflector: Arc<Reflector>,
    citation_tracker: Arc<CitationTracker>,
}

impl ReportGenerator {
    pub async fn generate(&self, state: &ResearchState) -> Result<ResearchReport, ReportError> {
        // 1. 构建报告大纲
        let outline = self.build_outline(state).await?;

        // 2. 分节生成内容
        let sections = self.generate_sections(&outline, state).await?;

        // 3. 综合引用
        let references = self.build_references(state).await?;

        // 4. 生成摘要
        let summary = self.generate_summary(&sections).await?;

        Ok(ResearchReport { outline, sections, references, summary })
    }
}
```

**报告格式支持**:

| 格式 | 用途 | 生成方式 |
|------|------|----------|
| Markdown | 默认输出 | 直接生成 |
| HTML | 预览展示 | Markdown 转换 |
| JSON | API 输出 | 结构化数据 |

### 模块 6: 前端集成与 UI（Week 6-7）

#### 6.1 组件清单

```
src/components/chat/
├── ResearchPanel.tsx           # 研究主面板
├── ResearchProgress.tsx        # 研究进度
├── ResearchSources.tsx        # 来源列表
├── CitationManager.tsx         # 引用管理
├── CredibilityBadge.tsx       # 可信度标签
└── ReportViewer.tsx           # 报告查看器
```

#### 6.2 UI 设计

```
┌─────────────────────────────────────────────────────────────────┐
│  研究型 Agent                                                  │
├─────────────────────────────────────────────────────────────────┤
│  研究主题: [________________________________]  [开始研究]         │
├─────────────────────────────────────────────────────────────────┤
│  进度面板                                                       │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │ ○ 规划  ● 搜索  ○ 提取  ○ 分析  ○ 综合  ○ 报告           ││
│  │ ████████████░░░░░░░░░░░░░░░░░░░░░░░ 45%                 ││
│  └─────────────────────────────────────────────────────────────┘│
├─────────────────────────────────────────────────────────────────┤
│  搜索结果                                                       │
│  ┌───────────────────────────┬─────────────────────────────────┐│
│  │ 来源列表                   │ 详细信息                        ││
│  │ ★★★☆☆ Wikipedia         │ 标题: xxx                       ││
│  │ ★★★★★ 官方文档           │ URL: xxx                        ││
│  │ ★★☆☆☆ 博客              │ 可信度: 中                      ││
│  │                          │ 引用: [添加到报告]               ││
│  └───────────────────────────┴─────────────────────────────────┘│
├─────────────────────────────────────────────────────────────────┤
│  生成的报告                                          [复制] [导出]│
│  ┌─────────────────────────────────────────────────────────────┐│
│  │ # 研究报告: xxx                                             ││
│  │                                                            ││
│  │ ## 摘要                                                     ││
│  │ ...                                                         ││
│  │                                                            ││
│  │ ## 参考资料                                                  ││
│  │ [1] xxx                                                    ││
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
```

---

## 四、前端 Hook

### 4.1 useResearch Hook

```typescript
interface UseResearchOptions {
  topic: string;
  onProgress?: (phase: ResearchPhase, progress: number) => void;
  onSourceFound?: (source: SearchResult) => void;
  onReportGenerated?: (report: ResearchReport) => void;
}

export function useResearch(options: UseResearchOptions) {
  const [state, setState] = useState<ResearchState | null>(null);
  const [isResearching, setIsResearching] = useState(false);

  const startResearch = async () => { /* ... */ };
  const pauseResearch = () => { /* ... */ };
  const resumeResearch = () => { /* ... */ };
  const stopResearch = () => { /* ... */ };

  return {
    state,
    isResearching,
    progress: calculateProgress(state),
    startResearch,
    pauseResearch,
    resumeResearch,
    stopResearch,
  };
}
```

---

## 五、技术依赖

### 5.1 Rust 依赖

```toml
# src-tauri/crates/agent/Cargo.toml
[dependencies]
# 已有
axagent-core = { path = "../core" }
axagent-runtime = { path = "../runtime" }
axagent-trajectory = { path = "../trajectory" }
tokio = { workspace = true }
serde = { workspace = true }
thiserror = "1.0"
chrono = { version = "0.4", features = ["serde"] }

# 新增
scraper = "0.21"           # HTML 解析
select = "0.8"              # DOM 选择器
url = "2.5"                 # URL 解析
```

### 5.2 前端依赖

无需新增依赖，复用现有组件库。

---

## 六、测试计划

### 6.1 单元测试

| 模块 | 测试内容 |
|------|----------|
| SearchOrchestrator | 并行搜索、去重、排序 |
| CredibilityEvaluator | 各维度评分计算 |
| CitationTracker | 引用添加、去重、查找 |
| ReportGenerator | 大纲生成、内容综合 |

### 6.2 集成测试

| 测试场景 | 描述 |
|----------|------|
| 完整研究流程 | 主题 → 搜索 → 评估 → 报告 |
| 多源搜索 | Web + Academic + Wikipedia 并行 |
| 引用追踪 | 添加引用 → 生成报告 → 导出 |

---

## 七、风险与备选方案

| 风险 | 影响 | 备选方案 |
|------|------|----------|
| 搜索 API 限制 | 研究质量下降 | 降级为单一搜索源 |
| LLM 生成报告质量不稳定 | 报告不可用 | 提供模板选择 + 人工编辑 |
| 可信度评估误判 | 引用不可靠来源 | 提示用户确认关键引用 |
| 研究过程过长 | 用户体验差 | 支持暂停/恢复 + 增量保存 |

---

## 八、里程碑

| 周数 | 里程碑 | 交付物 |
|------|--------|--------|
| Week 1-2 | 研究智能体核心 | research_agent.rs, research_state.rs |
| Week 2-3 | 搜索编排系统 | search_orchestrator.rs, 搜索源集成 |
| Week 3-4 | 可信度评估 | credibility_evaluator.rs |
| Week 4-5 | 引用追踪 | citation_tracker.rs |
| Week 5-6 | 报告生成 | report_generator.rs |
| Week 6-7 | 前端集成 | ResearchPanel.tsx, useResearch hook |
| Week 8 | 端到端测试 | 完整研究流程测试 |

---

## 九、文件变更清单

### 9.1 新增 Rust 文件

```
src-tauri/crates/agent/src/
├── research_agent.rs           # 研究智能体主控制器
├── research_state.rs           # 研究状态管理
├── search_planner.rs           # 搜索规划器
├── search_orchestrator.rs      # 搜索编排器
├── search_provider.rs           # 搜索提供者接口
├── web_search.rs               # Web 搜索实现
├── academic_search.rs          # 学术搜索实现
├── source_validator.rs         # 来源验证器
├── credibility_evaluator.rs    # 可信度评估器
├── source_classifier.rs        # 来源分类器
├── fact_checker.rs             # 事实核查
├── citation.rs                 # 引用结构定义
├── citation_tracker.rs         # 引用追踪器
├── reference_builder.rs         # 参考文献构建器
├── report_generator.rs         # 报告生成器
├── outline_builder.rs          # 大纲构建器
└── content_synthesizer.rs      # 内容综合
```

### 9.2 新增 TypeScript 文件

```
src/components/chat/
├── ResearchPanel.tsx           # 研究主面板
├── ResearchProgress.tsx        # 研究进度展示
├── ResearchSources.tsx         # 来源列表
├── CitationManager.tsx         # 引用管理
├── CredibilityBadge.tsx       # 可信度标签
└── ReportViewer.tsx            # 报告查看器

src/hooks/
└── useResearch.ts              # 研究 hook
```

### 9.3 修改文件

```
src-tauri/crates/agent/src/lib.rs       # 添加模块导出
src-tauri/crates/agent/Cargo.toml      # 添加依赖
```

---

## 十、验收标准

1. ✅ 支持多源并行搜索（Web, Academic, Wikipedia）
2. ✅ 搜索结果可实时展示并显示可信度评分
3. ✅ 支持添加和管理引用
4. ✅ 自动生成结构化研究报告
5. ✅ 报告支持 Markdown/HTML 导出
6. ✅ 研究过程支持暂停/恢复
7. ✅ 前端 UI 可实时显示研究进度
