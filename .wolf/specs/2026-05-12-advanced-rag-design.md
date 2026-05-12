# 高级 RAG 扩展设计方案

> 日期：2026-05-12 | 作者：Claude | 状态：已确认

## 一、背景与目标

AxAgent 当前 RAG 实现处于中级偏上水平（6.5/10），具备统一 RAG 抽象层、sqlite-vec 向量存储、混合搜索（向量+BM25）、语义缓存、基础重排序等能力。但在现代高级 RAG 的关键能力上存在多项缺失。

**目标**：按 P0 → P1 → P2 优先级，分三期将 RAG 能力升级至高级水平。

---

## 二、技术决策

| 决策项 | 选择 | 原因 |
|--------|------|------|
| 嵌入模型策略 | 混合方案（A） | 通用检索用云端 API，rerank/分块/裁判用本地 Ollama 模型 |
| 本地模型部署 | 部署后按需下载 | 不打包进安装包，通过 ModelDownloader 按需拉取 |
| 重排序集成方式 | 透明管线层（A） | 在现有管线中插入 RerankStage，对上层透明 |
| Self-RAG 裁判模型 | 专用小模型（B） | 本地 Ollama 运行 qwen2.5:0.5b 做 relevance 判断 |

---

## 三、架构总览

```
                        ┌────────────────────────────┐
                        │     RAG Entry Point        │
                        │  collect_rag_context()     │
                        └──────────┬─────────────────┘
                                   │
                        ┌──────────▼─────────────────┐
                        │   Query Enhancement (P0)   │
                        │  HyDE / Multi-Query / Decomp│
                        └──────────┬─────────────────┘
                                   │
                        ┌──────────▼─────────────────┐
                        │  Adaptive Router (P2)      │
                        │  决定检索策略 & top-k       │
                        └──────────┬─────────────────┘
                                   │
              ┌────────────────────┼────────────────────┐
              │                    │                    │
     ┌────────▼───────┐  ┌────────▼───────┐  ┌────────▼───────┐
     │ Vector Search  │  │ BM25 / FTS5   │  │ Graph Search   │
     │ (sqlite-vec)   │  │ (hybrid)      │  │ (KG + backlink)│
     └────────┬───────┘  └────────┬───────┘  └────────┬───────┘
              │                    │                    │
              └────────────────────┼────────────────────┘
                                   │
                        ┌──────────▼─────────────────┐
                        │  Metadata Filter (P1)      │
                        │  标签/时间/类型 过滤         │
                        └──────────┬─────────────────┘
                                   │
                        ┌──────────▼─────────────────┐
                        │  Cross-Encoder Rerank (P0) │
                        │  BGE-Reranker-v2-m3        │
                        └──────────┬─────────────────┘
                                   │
                        ┌──────────▼─────────────────┐
                        │  Self-RAG Gate (P0)        │
                        │  相关性评估 + 质量门控       │
                        └──────────┬─────────────────┘
                                   │
                        ┌──────────▼─────────────────┐
                        │  Context Assembly          │
                        │  (现有 + 父子块扩展)         │
                        └────────────────────────────┘
```

**核心原则**：
- 每一级都是可插拔的 Stage，通过配置启用/禁用
- 默认行为保持与当前一致（新 stage 默认关闭）
- 新代码放在 `crates/core/src/` 下，遵循现有模块命名风格

---

## 四、P0 模块设计

### 4.1 模型下载管理器

**文件**：`crates/core/src/model_downloader.rs`（新增）

```rust
struct ModelDownloader { cache_dir: PathBuf }
// fn ensure_model(name, url, expected_hash) -> Result<PathBuf>
//   - 检查 ~/.axagent/models/{name} 是否已下载
//   - 未下载时通过 HTTP(S) 断点续传下载
//   - 下载后校验 SHA256
// fn list_local_models() -> Vec<ModelInfo>
```

### 4.2 查询增强

**文件**：`crates/core/src/query_enhancement.rs`（新增）

三个策略：

| 策略 | 触发条件 | 行为 |
|------|---------|------|
| **HyDE** | 查询包含概念性词汇 | LLM 生成假设理想答案，用答案嵌入替代问题嵌入 |
| **Multi-Query** | 查询长度 > 15 字或多方面 | 从 3-5 个视角改写查询，并行检索合并 |
| **Query Decomposition** | 包含复合关系 | 拆分为 2-4 个子问题，逐个检索后融合 |

**关键设计**：
- HyDE 和 Multi-Query 在一次 LLM 调用中完成（结构化 JSON 输出）
- 多查询通过 `tokio::join!` 并行检索
- 结果合并复用现有 `deduplicate_cross_source`

### 4.3 跨编码器重排序

**文件**：`crates/core/src/reranker.rs`（重构现有）

```rust
trait RerankBackend {                          // 可插拔后端
    async fn rerank(query, chunks) -> Vec<(chunk_id, score)>
}
struct RuleReranker;                           // 现有规则 → 实现 RerankBackend
struct CrossEncoderReranker {                  // Ollama 调用 bge-reranker-v2-m3
    model_name: String,
    ollama_endpoint: String,
}
struct RerankPipeline {                        // 两级编排
    stages: Vec<Box<dyn RerankBackend>>,
}
```

**两级管线**：RuleReranker(初筛 Top-30→15, ~0ms) → CrossEncoderReranker(精排 Top-15→5, ~200ms)

### 4.4 自省式 RAG

**文件**：`crates/core/src/self_rag.rs`（新增）

工作流：
```
检索结果 Top-5 → 批量相关性判断（本地 qwen2.5:0.5b）
    ├── Good (>60%相关) → 直接注入
    ├── Partial (30-60%) → 过滤无关项后注入
    └── Poor (<30%) → 纠正循环(最多2轮):
        1. 分析原因 → 2. 生成精炼查询 → 3. 重新检索 → 4. 再判断
```

---

## 五、P1 模块设计

### 5.1 语义分块 + 父子块

**文件**：`crates/core/src/chunking/`（新目录）

- `semantic_chunker.rs`：基于句子嵌入余弦相似度找语义断点
- `parent_child.rs`：child_chunk(200t) 用于索引检索，parent_chunk(800t) 用于返回上下文
- 现有 `text_chunker.rs` 迁移为 `sliding_window.rs`

**ChunkStrategy 扩展**：
```rust
pub enum ChunkStrategy {
    // 现有保留
    ParseAndChunk { ... }, Direct, FromText { ... },
    // 新增
    Semantic { similarity_threshold: f32 },
    ParentChild { child_size: usize, parent_size: usize, overlap: usize },
}
```

### 5.2 元数据过滤

**文件**：`crates/core/src/metadata_filter.rs`（新增）

- `MetadataFilter` + `FilterGroup` 结构体支持复杂条件（And/Or 嵌套）
- 编译为 SQL WHERE 子句追加到向量检索查询
- `vec_xxx_meta` 表新增 `file_type`, `created_at`, `tags` 字段
- 数据库迁移：`m20240101_000006_rag_metadata.rs`

### 5.3 Graph RAG 双通道

**文件**：`crates/core/src/graph_rag.rs`（新增）

双通道并行检索：
- 通道 1：向量检索（现有）
- 通道 2：LLM 抽取实体 → KG 精确匹配 → 1-hop 扩展关联实体
- 两路结果融合后注入上下文

关键约束：图遍历默认仅 1-hop，避免爆炸。

---

## 六、P2 模块设计

### 6.1 多模态 RAG

**文件**：`crates/core/src/multimodal_rag.rs`（新增）

- PDF/PPTX 图片提取 → 视觉 LLM 描述 → 嵌入描述文本
- 图片描述缓存（hash → description）
- 仅支持 PDF 和 PPTX，图片缓存到 `~/.axagent/images/`

### 6.2 RAG 评估框架

**文件**：`crates/core/src/rag_evaluation.rs`（新增）

RAGAS 标准指标：
- Context Relevance / Context Recall
- Answer Faithfulness / Answer Relevance
- 支持离线评估 + 在线采样（5% 流量）

### 6.3 自适应检索

**文件**：`crates/core/src/adaptive_retrieval.rs`（新增）

```rust
enum QueryComplexity { Simple, Moderate, Complex }
enum RetrievalStrategy {
    NoRetrieval, LightRetrieval, StandardRetrieval, DeepRetrieval,
}
```

路由规则：
- Simple(10%) → NoRetrieval 或 Light
- Moderate(60%) → Standard
- Complex(30%) → Deep（multi-query + rerank + self-RAG）

### 6.4 增量索引

**文件**：`crates/core/src/incremental_index.rs`（新增）

- `ChangeTracker`：记录每文档的 SHA256 + 最后索引时间
- `compute_delta()`：对比得出 new/modified/deleted/unchanged
- 触发：手动全量 / 文件监听增量 / 定时扫描

---

## 七、与现有代码的集成点

| 现有文件 | 改动类型 | 说明 |
|---------|---------|------|
| `rag.rs` | 修改 | 在 `collect_rag_context()` 插入 query_enhancement / rerank / self_rag 阶段 |
| `reranker.rs` | 重构 | 改为 trait-based 后端模式 |
| `text_chunker.rs` | 迁移 | 移至 `chunking/sliding_window.rs` |
| `hybrid_search.rs` | 修改 | 增加 metadata filter 支持 |
| `types.rs` | 扩展 | 增加新的配置类型和枚举 |
| `source` 配置 | 扩展 | 每个数据源可配置 rerank/self_rag/graph 策略 |
| `lib.rs` (core) | 修改 | 注册新模块 |
| migration | 新增 | `m20240101_000006_rag_metadata.rs` |

---

## 八、前端变更

| 组件 | 改动 |
|------|------|
| `KnowledgeSettings.tsx` | 增加 rerank/self_rag/query_enhancement 开关配置 |
| `KnowledgePage.tsx` | 增加元数据过滤 UI（文件类型、日期、标签筛选） |
| `SettingsPage.tsx` | 新增 "本地模型管理" 选项卡（下载/删除模型） |
| 新增 DevTools 面板 | RAG 评估指标展示（命中率、延迟、相关性分布） |

---

## 九、实施分期

### 第一期：检索质量质变（P0，预计 2-3 周）

| 任务 | 涉及文件 | 工作量 |
|------|---------|--------|
| 模型下载管理器 | `model_downloader.rs`（新） | 2d |
| 查询增强 HyDE + Multi-Query | `query_enhancement.rs`（新） | 3d |
| 跨编码器重排序 | `reranker.rs`（重构） | 3d |
| Self-RAG 质检门控 | `self_rag.rs`（新） | 3d |
| 管线集成 + 配置扩展 | `rag.rs` 修改 | 2d |
| 前端知识库设置页适配 | `KnowledgeSettings.tsx` | 1d |
| 测试 + 文档 | 各模块单测 | 2d |

### 第二期：体系化增强（P1，预计 2-3 周）

| 任务 | 涉及文件 | 工作量 |
|------|---------|--------|
| 语义分块器 | `chunking/semantic_chunker.rs`（新） | 2d |
| 父子块索引 + 检索 | `chunking/parent_child.rs`（新） | 3d |
| 元数据过滤 | `metadata_filter.rs`（新）+ migration | 2d |
| Graph RAG 双通道融合 | `graph_rag.rs`（新） | 3d |
| 前端过滤 UI | `KnowledgePage.tsx` | 1d |
| 测试 | 各模块单测 + 集成测试 | 2d |

### 第三期：完善与优化（P2，预计 3-4 周）

| 任务 | 涉及文件 | 工作量 |
|------|---------|--------|
| PDF/PPTX 图片提取 + 描述 | `multimodal_rag.rs`（新） | 3d |
| RAGAS 评估框架 | `rag_evaluation.rs`（新） | 3d |
| 自适应检索路由 | `adaptive_retrieval.rs`（新） | 2d |
| 增量索引 | `incremental_index.rs`（新） | 3d |
| DevTools 评估面板 | 前端 `stores/devtools/` | 2d |
| 模型下载前端 UI | 设置页新增 | 1d |
| 端到端测试 | Playwright E2E | 2d |
