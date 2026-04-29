# LLM Wiki 实现计划

> 基于 llm_wiki 借鉴参考文档
> 创建时间：2026-04-29

---

## 一、Phase 划分

| Phase | 周期 | 目标 |
|-------|------|------|
| **Phase 1** | P0-P1 | 核心：Two-Step CoT Ingest + 增量缓存 + Ingest Queue + Purpose.md |
| **Phase 2** | P2 | 图谱：4-Signal KG + Louvain + Graph Insights |
| **Phase 3** | P3-P4 | 扩展：Deep Research + Web Clipper + Folder Import |

---

## 二、Phase 1 详细实现

### 2.1 Two-Step Chain-of-Thought Ingest

**目标**：将现有单步 ingest 重构为两步：
1. Step 1 - Analysis：LLM 分析源文件，产出结构化分析
2. Step 2 - Generation：基于分析生成 Wiki 页面

#### Step 1: Analysis LLM Call

**新增文件**：`agent/src/ingest_pipeline.rs` → 新增 `analyze_source` 方法

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceAnalysis {
    pub entities: Vec<EntityMention>,      // 关键实体
    pub concepts: Vec<ConceptMention>,    // 核心概念
    pub arguments: Vec<Argument>,         // 主要论点
    pub connections: Vec<ConnectionHint>, // 与现有wiki的关联建议
    pub contradictions: Vec<Contradiction>, // 与现有知识的矛盾点
    pub suggested_structure: Vec<PageSuggestion>, // 建议的页面类型和标题
    pub review_items: Vec<ReviewItem>,    // 需要人工判断的事项
    pub search_queries: Vec<String>,      // 建议的搜索查询
}

pub struct IngestPipeline {
    db: Arc<DatabaseConnection>,
    llm_adapter: Arc<dyn ProviderAdapter>,  // 新增
    // ...
}

impl IngestPipeline {
    pub async fn analyze_source(
        &self,
        wiki_id: &str,
        content: &str,
        purpose: Option<&str>,
    ) -> Result<SourceAnalysis, String> {
        let prompt = self.build_analysis_prompt(content, purpose);
        let response = self.llm_adapter.complete(&prompt).await?;
        self.parse_analysis_response(&response)
    }

    fn build_analysis_prompt(&self, content: &str, purpose: Option<&str>) -> String {
        let purpose_context = purpose
            .map(|p| format!("\n\nWiki Purpose:\n{}", p))
            .unwrap_or_default();

        format!(
            r#"作为LLM Wiki的分析助手，请分析以下源内容并产出结构化分析。

{purpose_context}

## 源内容
```
{content}
```

## 分析要求
请以JSON格式输出，包含：
1. **entities**: 识别关键实体（名称、类型、描述）
2. **concepts**: 核心概念及其关系
3. **arguments**: 主要论点或观点
4. **connections**: 与现有wiki内容的关联建议（如果没有现有内容可留空）
5. **contradictions**: 与现有知识的矛盾或张力
6. **suggested_structure**: 建议生成的wiki页面（类型：entity/concept/source-summary等）
7. **review_items**: 需要人工判断的事项
8. **search_queries**: 为深入研究所需的搜索查询

请确保分析深入且准确，JSON格式正确。"#
        )
    }
}
```

#### Step 2: Generation LLM Call

**新增方法**：`generate_wiki_pages` 在 `ingest_pipeline.rs`

```rust
impl IngestPipeline {
    pub async fn generate_wiki_pages(
        &self,
        wiki_id: &str,
        source_id: &str,
        analysis: &SourceAnalysis,
        raw_path: &str,
    ) -> Result<Vec<GeneratedPage>, String> {
        let prompt = self.build_generation_prompt(analysis, raw_path);
        let response = self.llm_adapter.complete(&prompt).await?;
        self.parse_generation_response(&response, source_id)
    }

    fn build_generation_prompt(&self, analysis: &SourceAnalysis, raw_path: &str) -> String {
        // 构建包含analysis结果的prompt，生成具体的wiki页面
        format!(
            r#"基于以下分析结果，生成Wiki页面。

## 分析结果
{analysis_json}

## 原始文件路径
{raw_path}

## 生成要求
1. 为每个suggested_structure生成对应的wiki页面
2. 页面格式遵循Obsidian风格，包含YAML frontmatter
3. 使用[[wikilinks]]建立页面间链接
4. 每个页面的frontmatter包含：
   - type: 页面类型
   - title: 页面标题
   - sources: [raw_path]
   - created_at: 时间戳
5. 生成index.md的更新内容

请以JSON格式输出生成的页面列表，每个页面包含filename和content。"#
        )
    }
}
```

#### 主入口重构

```rust
impl IngestPipeline {
    pub async fn ingest(
        &self,
        wiki_id: &str,
        source: IngestSource,
    ) -> Result<IngestResult, String> {
        // 1. 检查增量缓存
        if let Some(cached) = self.check_cache(wiki_id, &source).await? {
            return Ok(cached);
        }

        // 2. 解析源文件
        let parsed = self.parse_source(&source).await?;

        // 3. Step 1: Analysis
        let purpose = self.load_purpose(wiki_id).await.ok();
        let analysis = self.analyze_source(wiki_id, &parsed, purpose.as_deref()).await?;

        // 4. 保存原始文件
        let raw_path = self.save_to_raw(wiki_id, &source, &parsed).await?;
        let source_record = self.save_source_record(wiki_id, &raw_path, &source, &parsed).await?;

        // 5. Step 2: Generation
        let pages = self.generate_wiki_pages(wiki_id, &source_record.id, &analysis, &raw_path).await?;

        // 6. 保存生成的页面
        self.save_generated_pages(wiki_id, pages).await?;

        // 7. 更新缓存
        self.update_cache(wiki_id, &source, &source_record.id).await?;

        Ok(IngestResult {
            source_id: source_record.id,
            raw_path,
            title: source_record.title,
        })
    }
}
```

#### 文件变更

| 文件 | 变更 |
|------|------|
| `agent/src/ingest_pipeline.rs` | 扩展，添加 `analyze_source`, `generate_wiki_pages`, 增量缓存逻辑 |

---

### 2.2 SHA256 增量缓存

**目标**：源文件内容哈希，变化才处理

```rust
// 在 ingest_pipeline.rs 中添加

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestCache {
    pub source_path: String,
    pub content_hash: String,
    pub source_id: String,
    pub processed_at: i64,
}

impl IngestPipeline {
    async fn check_cache(&self, wiki_id: &str, source: &IngestSource) -> Result<Option<IngestResult>, String> {
        let content = self.parse_source(source).await?;
        let hash = self.compute_sha256(&content).await;

        let cache_path = format!("~/axagent-notes/{}/.cache/ingest_cache.json", wiki_id);
        if let Ok(cache_data) = tokio::fs::read_to_string(&cache_path).await {
            let caches: Vec<IngestCache> = serde_json::from_str(&cache_data).unwrap_or_default();
            if let Some(cached) = caches.iter().find(|c| c.content_hash == hash) {
                return Ok(Some(IngestResult {
                    source_id: cached.source_id.clone(),
                    raw_path: format!("~/axagent-notes/{}/raw/{}", wiki_id, cached.source_id),
                    title: "Cached".to_string(),
                }));
            }
        }
        Ok(None)
    }

    async fn update_cache(&self, wiki_id: &str, source: &IngestSource, source_id: &str) -> Result<(), String> {
        let content = self.parse_source(source).await?;
        let hash = self.compute_sha256(&content).await;

        let cache_path = format!("~/axagent-notes/{}/.cache/ingest_cache.json", wiki_id);
        let mut caches: Vec<IngestCache> = tokio::fs::read_to_string(&cache_path)
            .await
            .map(|d| serde_json::from_str(&d).unwrap_or_default())
            .unwrap_or_default();

        caches.retain(|c| c.source_path != source.path);
        caches.push(IngestCache {
            source_path: source.path.clone(),
            content_hash: hash,
            source_id: source_id.to_string(),
            processed_at: chrono::Utc::now().timestamp(),
        });

        let cache_dir = PathBuf::from(&cache_path).parent().unwrap().to_path_buf();
        tokio::fs::create_dir_all(&cache_dir).await.map_err(|e| e.to_string())?;
        tokio::fs::write(&cache_path, serde_json::to_string_pretty(&caches).map_err(|e| e.to_string())?)
            .await.map_err(|e| e.to_string())?;

        Ok(())
    }

    async fn compute_sha256(&self, content: &str) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}
```

---

### 2.3 Persistent Ingest Queue

**新增文件**：`agent/src/ingest_queue.rs`

```rust
use std::sync::Arc;
use tokio::sync::Mutex;
use serde::{Deserialize, Serialize};
use axagent_core::utils::gen_id;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedIngestTask {
    pub id: String,
    pub wiki_id: String,
    pub source: super::ingest_pipeline::IngestSource,
    pub status: TaskStatus,
    pub retry_count: u32,
    pub error_message: Option<String>,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

pub struct IngestQueue {
    tasks: Arc<Mutex<Vec<QueuedIngestTask>>>,
    pipeline: Arc<super::ingest_pipeline::IngestPipeline>,
    queue_path: String,
}

impl IngestQueue {
    pub fn new(
        pipeline: Arc<super::ingest_pipeline::IngestPipeline>,
        queue_path: String,
    ) -> Self {
        Self {
            tasks: Arc::new(Mutex::new(Vec::new())),
            pipeline,
            queue_path,
        }
    }

    pub async fn load_from_disk(&self) -> Result<(), String> {
        if let Ok(data) = tokio::fs::read_to_string(&self.queue_path).await {
            let tasks: Vec<QueuedIngestTask> = serde_json::from_str(&data).unwrap_or_default();
            *self.tasks.lock().await = tasks;
        }
        Ok(())
    }

    async fn save_to_disk(&self) -> Result<(), String> {
        let tasks = self.tasks.lock().await;
        tokio::fs::write(
            &self.queue_path,
            serde_json::to_string_pretty(&*tasks).map_err(|e| e.to_string())?,
        )
        .await
        .map_err(|e| e.to_string())
    }

    pub async fn enqueue(&self, wiki_id: &str, source: super::ingest_pipeline::IngestSource) -> String {
        let task = QueuedIngestTask {
            id: gen_id(),
            wiki_id: wiki_id.to_string(),
            source,
            status: TaskStatus::Pending,
            retry_count: 0,
            error_message: None,
            created_at: chrono::Utc::now().timestamp(),
            started_at: None,
            completed_at: None,
        };

        self.tasks.lock().await.push(task);
        self.save_to_disk().await.ok();
        task.id
    }

    pub async fn process_queue(&self) -> Result<(), String> {
        loop {
            let task_id = {
                let mut tasks = self.tasks.lock().await;
                if let Some(idx) = tasks.iter().position(|t| t.status == TaskStatus::Pending) {
                    tasks[idx].status = TaskStatus::Processing;
                    tasks[idx].started_at = Some(chrono::Utc::now().timestamp());
                    tasks[idx].id.clone()
                } else {
                    break;
                }
            };

            let result = self.process_task(&task_id).await;

            {
                let mut tasks = self.tasks.lock().await;
                if let Some(task) = tasks.iter_mut().find(|t| t.id == task_id) {
                    match result {
                        Ok(_) => {
                            task.status = TaskStatus::Completed;
                            task.completed_at = Some(chrono::Utc::now().timestamp());
                        }
                        Err(e) => {
                            task.retry_count += 1;
                            if task.retry_count >= 3 {
                                task.status = TaskStatus::Failed;
                                task.error_message = Some(e.clone());
                            } else {
                                task.status = TaskStatus::Pending;
                                task.error_message = Some(e);
                            }
                        }
                    }
                }
            }

            self.save_to_disk().await.ok();
        }
        Ok(())
    }

    async fn process_task(&self, task_id: &str) -> Result<(), String> {
        let task = self.tasks.lock().await
            .iter()
            .find(|t| t.id == task_id)
            .cloned()
            .ok_or("Task not found")?;

        self.pipeline.ingest(&task.wiki_id, task.source).await?;
        Ok(())
    }

    pub async fn cancel_task(&self, task_id: &str) -> Result<(), String> {
        let mut tasks = self.tasks.lock().await;
        if let Some(task) = tasks.iter_mut().find(|t| t.id == task_id) {
            if task.status == TaskStatus::Pending {
                task.status = TaskStatus::Failed;
                task.error_message = Some("Cancelled by user".to_string());
            }
        }
        self.save_to_disk().await
    }

    pub async fn retry_task(&self, task_id: &str) -> Result<(), String> {
        let mut tasks = self.tasks.lock().await;
        if let Some(task) = tasks.iter_mut().find(|t| t.id == task_id) {
            task.status = TaskStatus::Pending;
            task.error_message = None;
        }
        self.save_to_disk().await
    }
}
```

---

### 2.4 Purpose.md 支持

**目标**：每个 Wiki 的 `purpose.md` 定义目标、方向

#### 新增文件模板

```markdown
# {wiki_name}

## Purpose
[描述这个Wiki存在的目的和意义]

## Key Questions
- [关键问题1]
- [关键问题2]
- [关键问题3]

## Research Scope
[定义研究范围和边界]

## Thesis
[随着知识积累，期望形成的核心观点或结论]

## Evolving Notes
[记录Purpose的演进历史]
- YYYY-MM-DD: [更新内容]
```

#### 加载 Purpose

```rust
// 在 wiki.rs 或新增 purpose_manager.rs

pub async fn load_purpose(wiki_id: &str) -> Result<String, String> {
    let purpose_path = format!("~/axagent-notes/{}/purpose.md", wiki_id);
    tokio::fs::read_to_string(&purpose_path)
        .await
        .map_err(|e| format!("Purpose not found: {}", e))
}

pub async fn save_purpose(wiki_id: &str, content: &str) -> Result<(), String> {
    let purpose_path = format!("~/axagent-notes/{}/purpose.md", wiki_id);
    tokio::fs::write(&purpose_path, content)
        .await
        .map_err(|e| e.to_string())
}
```

#### 新增 Tauri Command

```rust
// src-tauri/src/commands/llm_wiki.rs

#[tauri::command]
pub async fn llm_wiki_get_purpose(wiki_id: String) -> Result<String, String> {
    purpose_manager::load_purpose(&wiki_id).await
}

#[tauri::command]
pub async fn llm_wiki_update_purpose(wiki_id: String, content: String) -> Result<(), String> {
    purpose_manager::save_purpose(&wiki_id, &content).await
}
```

---

## 三、Phase 2 详细实现

### 3.1 4-Signal Knowledge Graph

**目标**：扩展现有 LinkGraph，支持多信号 relevance 计算

#### 新增 Relevance Signal 定义

```rust
// core/src/repo/note_graph.rs

#[derive(Debug, Clone)]
pub struct RelevanceSignal {
    pub direct_link: f64,      // ×3.0
    pub source_overlap: f64,   // ×4.0
    pub adamic_adar: f64,      // ×1.5
    pub type_affinity: f64,    // ×1.0
}

impl RelevanceSignal {
    pub fn total_score(&self) -> f64 {
        self.direct_link * 3.0 +
        self.source_overlap * 4.0 +
        self.adamic_adar * 1.5 +
        self.type_affinity * 1.0
    }
}
```

#### 扩展 LinkGraph

```rust
impl LinkGraph {
    pub fn compute_relevance(
        &self,
        page_a: &str,
        page_b: &str,
        source_map: &HashMap<String, Vec<String>>,
        type_map: &HashMap<String, PageType>,
    ) -> RelevanceSignal {
        RelevanceSignal {
            direct_link: self.has_direct_link(page_a, page_b) as f64,
            source_overlap: self.compute_source_overlap(page_a, page_b, source_map),
            adamic_adar: self.compute_adamic_adar(page_a, page_b),
            type_affinity: self.compute_type_affinity(page_a, page_b, type_map),
        }
    }

    fn compute_source_overlap(
        &self,
        page_a: &str,
        page_b: &str,
        source_map: &HashMap<String, Vec<String>>,
    ) -> f64 {
        let sources_a: HashSet<_> = source_map.get(page_a).cloned().unwrap_or_default().into_iter().collect();
        let sources_b: HashSet<_> = source_map.get(page_b).cloned().unwrap_or_default().into_iter().collect();
        if sources_a.is_empty() || sources_b.is_empty() {
            return 0.0;
        }
        let intersection = sources_a.intersection(&sources_b).count() as f64;
        let union = sources_a.union(&sources_b).count() as f64;
        intersection / union
    }

    fn compute_adamic_adar(&self, page_a: &str, page_b: &str) -> f64 {
        let neighbors_a: HashSet<_> = self.get_neighbors(page_a).into_iter().collect();
        let neighbors_b: HashSet<_> = self.get_neighbors(page_b).into_iter().collect();
        let common_neighbors: Vec<_> = neighbors_a.intersection(&neighbors_b).collect();

        common_neighbors.iter().map(|n| {
            let degree = self.get_degree(n) as f64;
            if degree > 1.0 {
                1.0 / (degree - 1.0).ln()
            } else {
                0.0
            }
        }).sum()
    }

    fn compute_type_affinity(
        &self,
        page_a: &str,
        page_b: &str,
        type_map: &HashMap<String, PageType>,
    ) -> f64 {
        match (type_map.get(page_a), type_map.get(page_b)) {
            (Some(t1), Some(t2)) if t1 == t2 => 1.0,
            (Some(PageType::Entity), Some(PageType::Entity)) => 1.0,
            (Some(PageType::Concept), Some(PageType::Concept)) => 1.0,
            _ => 0.0,
        }
    }
}
```

---

### 3.2 Louvain Community Detection

**新增文件**：`core/src/graph/louvain.rs`

```rust
use std::collections::HashMap;

pub struct LouvainCommunity {
    pub communities: HashMap<String, i32>,
    pub cohesion_scores: HashMap<i32, f64>,
}

impl LouvainCommunity {
    pub fn detect(graph: &LinkGraph) -> Self {
        // 实现 Louvain 算法
        // 1. 初始化每个节点为独立社区
        // 2. 迭代移动节点到能最大提升modularity的社区
        // 3. 合并社区构建新图
        // 4. 重复直到收敛

        let mut communities = HashMap::new();
        let mut cohesion_scores = HashMap::new();

        // ... 实现细节

        Self { communities, cohesion_scores }
    }

    pub fn get_cohesion(&self, community_id: i32) -> f64 {
        self.cohesion_scores.get(&community_id).copied().unwrap_or(0.0)
    }

    pub fn is_low_cohesion(&self, community_id: i32) -> bool {
        self.get_cohesion(community_id) < 0.15
    }
}
```

---

### 3.3 Graph Insights

**新增文件**：`agent/src/graph_insights.rs`

```rust
use crate::louvain::LouvainCommunity;
use crate::note_graph::LinkGraph;

pub struct GraphInsights {
    pub surprising_connections: Vec<SurprisingConnection>,
    pub knowledge_gaps: Vec<KnowledgeGap>,
    pub bridge_nodes: Vec<BridgeNode>,
}

pub struct SurprisingConnection {
    pub page_a: String,
    pub page_b: String,
    pub surprise_score: f64,
    pub dismissed: bool,
}

pub struct KnowledgeGap {
    pub gap_type: GapType,
    pub nodes: Vec<String>,
    pub description: String,
}

pub struct BridgeNode {
    pub page_id: String,
    pub connected_communities: Vec<i32>,
}

impl GraphInsights {
    pub fn detect(
        graph: &LinkGraph,
        communities: &LouvainCommunity,
        types: &HashMap<String, PageType>,
    ) -> Self {
        Self {
            surprising_connections: Self::find_surprising_connections(graph, communities, types),
            knowledge_gaps: Self::find_knowledge_gaps(graph, communities),
            bridge_nodes: Self::find_bridge_nodes(graph, communities),
        }
    }

    fn find_surprising_connections(
        graph: &LinkGraph,
        communities: &LouvainCommunity,
        types: &HashMap<String, PageType>,
    ) -> Vec<SurprisingConnection> {
        let mut connections = Vec::new();

        for edge in graph.get_edges() {
            let (a, b) = (&edge.source, &edge.target);
            let community_a = communities.communities.get(a);
            let community_b = communities.communities.get(b);
            let type_a = types.get(a);
            let type_b = types.get(b);

            let mut surprise_score = 0.0;

            // Cross-community edge bonus
            if community_a != community_b {
                surprise_score += 2.0;
            }

            // Cross-type link bonus
            if let (Some(t1), Some(t2)) = (type_a, type_b) {
                if !std::mem::discriminant(t1, t2) {
                    surprise_score += 1.5;
                }
            }

            // Peripheral-hub coupling
            let degree_a = graph.get_degree(a);
            let degree_b = graph.get_degree(b);
            if (degree_a <= 2 && degree_b >= 10) || (degree_b <= 2 && degree_a >= 10) {
                surprise_score += 1.0;
            }

            if surprise_score > 2.0 {
                connections.push(SurprisingConnection {
                    page_a: a.clone(),
                    page_b: b.clone(),
                    surprise_score,
                    dismissed: false,
                });
            }
        }

        connections.sort_by(|a, b| b.surprise_score.partial_cmp(&a.surprise_score).unwrap());
        connections
    }

    fn find_knowledge_gaps(graph: &LinkGraph, communities: &LouvainCommunity) -> Vec<KnowledgeGap> {
        let mut gaps = Vec::new();

        // Isolated pages (degree <= 1)
        let isolated: Vec<_> = graph.get_nodes()
            .filter(|n| graph.get_degree(n) <= 1)
            .cloned()
            .collect();
        if !isolated.is_empty() {
            gaps.push(KnowledgeGap {
                gap_type: GapType::IsolatedPages,
                nodes: isolated,
                description: "Pages with few or no connections".to_string(),
            });
        }

        // Sparse communities
        for (community_id, cohesion) in &communities.cohesion_scores {
            if *cohesion < 0.15 {
                let members: Vec<_> = communities.communities.iter()
                    .filter(|(_, c)| *c == *community_id)
                    .map(|(n, _)| n.clone())
                    .collect();
                if members.len() >= 3 {
                    gaps.push(KnowledgeGap {
                        gap_type: GapType::SparseCommunity,
                        nodes: members,
                        description: format!("Community {} with low cohesion: {:.2}", community_id, cohesion),
                    });
                }
            }
        }

        gaps
    }

    fn find_bridge_nodes(graph: &LinkGraph, communities: &LouvainCommunity) -> Vec<BridgeNode> {
        graph.get_nodes()
            .filter(|n| {
                let connected: HashSet<_> = communities.communities.iter()
                    .filter(|(node, _)| graph.is_connected(n, node))
                    .map(|(_, c)| *c)
                    .collect();
                connected.len() >= 3
            })
            .map(|n| BridgeNode {
                page_id: n.clone(),
                connected_communities: communities.communities.iter()
                    .filter(|(node, _)| graph.is_connected(n, node))
                    .map(|(_, c)| *c)
                    .collect(),
            })
            .collect()
    }
}
```

---

## 四、Phase 3 概要

### 4.1 Deep Research

| 步骤 | 说明 |
|------|------|
| 1 | Graph Insights 触发，输入 topic |
| 2 | 读取 overview.md + purpose.md 构建上下文 |
| 3 | LLM 生成多个搜索查询 |
| 4 | 多查询并发执行 web search |
| 5 | 结果自动 ingest 到 wiki |

**新增文件**：`agent/src/deep_research.rs`

---

### 4.2 Chrome Web Clipper

| 组件 | 说明 |
|------|------|
| `extension/` | Browser extension (Manifest V3) |
| `extension/background.js` | 捕获页面内容，发送到 Tauri backend |
| `extension/content.js` | 提取页面正文 |

**参考**：`llm_wiki/extension` 目录

---

### 4.3 Folder Import

```rust
pub async fn import_folder(
    &self,
    wiki_id: &str,
    folder_path: &str,
) -> Result<Vec<String>, String> {
    let mut task_ids = Vec::new();
    self.walk_dir(wiki_id, folder_path, &mut task_ids).await?;
    Ok(task_ids)
}

async fn walk_dir(
    &self,
    wiki_id: &str,
    dir: &str,
    task_ids: &mut Vec<String>,
) -> Result<(), String> {
    let mut entries = tokio::fs::read_dir(dir).await.map_err(|e| e.to_string())?;
    while let Some(entry) = entries.next_entry().await.map_err(|e| e.to_string())? {
        let path = entry.path();
        if path.is_dir() {
            self.walk_dir(wiki_id, &path.to_string_lossy(), task_ids).await?;
        } else {
            // 构建 folder_context 如 "papers/energy"
            let relative = Self::get_relative_path(dir, &path);
            let folder_context = relative.parent()
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or_default();

            let source = IngestSource {
                source_type: Self::infer_type(&path),
                path: path.to_string_lossy().to_string(),
                url: None,
                title: None,
            };

            let task_id = self.queue.enqueue(wiki_id, source).await;
            task_ids.push(task_id);
        }
    }
    Ok(())
}
```

---

## 五、文件变更汇总

| Phase | 文件 | 变更类型 |
|-------|------|----------|
| 1 | `agent/src/ingest_pipeline.rs` | 扩展 |
| 1 | `agent/src/ingest_queue.rs` | 新增 |
| 1 | `agent/src/purpose_manager.rs` | 新增 |
| 1 | `core/src/repo/note_graph.rs` | 扩展 |
| 2 | `core/src/graph/louvain.rs` | 新增 |
| 2 | `core/src/graph/mod.rs` | 新增 |
| 2 | `agent/src/graph_insights.rs` | 新增 |
| 2 | `agent/src/relevance.rs` | 新增 |
| 3 | `agent/src/deep_research.rs` | 新增 |
| 3 | `extension/` | 新增目录 |

---

## 六、测试计划

| 模块 | 测试类型 |
|------|----------|
| IngestPipeline | Unit + Integration |
| IngestQueue | Unit (状态机) + Integration |
| LinkGraph (4-signal) | Unit |
| Louvain | Unit (合成图) |
| GraphInsights | Unit |
| End-to-end | 完整 ingest → graph → insights 流程 |

---

## 七、依赖更新

```toml
# Cargo.toml (agent)

[dependencies]
sha2 = "0.10"  # SHA256 计算
# Louvain 算法可考虑使用 Rust 图算法库或 WASM 绑定
```

---

## 八、风险与缓解

| 风险 | 缓解方案 |
|------|----------|
| Two-Step Ingest LLM 调用成本翻倍 | 增量缓存 + Step 1 可选（简单文档跳过分析） |
| Louvain 算法性能 | 限制节点数量，大图采样 |
| Graph Insights 误报 | 提供 dismiss 机制 |
