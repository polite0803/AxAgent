# LLM Wiki 功能借鉴参考文档

> 基于 [nashsu/llm_wiki](https://github.com/nashsu/llm_wiki) 项目分析
> 创建时间：2026-04-29

---

## 一、背景

AxAgent 定位为 **Karpathy LLM Wiki 模式的完整实现**。[nashsu/llm_wiki](https://github.com/nashsu/llm_wiki) 是该模式的优秀参考实现，提供了丰富的功能增强。本文档梳理可借鉴的功能点，为后续开发提供依据。

---

## 二、可借鉴功能清单

### 2.1 高优先级（核心体验提升）

#### 1. Two-Step Chain-of-Thought Ingest
**现状**：AxAgent 有 IngestPipeline，但为单步处理
**借鉴内容**：
- Step 1 (Analysis)：LLM 先分析源文件，产出结构化分析（实体、概念、关联、矛盾）
- Step 2 (Generation)：基于分析结果生成 Wiki 页面
- SHA256 增量缓存：源文件内容哈希，变化才处理

**实现位置**：`agent/src/ingest_pipeline.rs` → 扩展为两步

---

#### 2. Persistent Ingest Queue
**现状**：未实现
**借鉴内容**：
- 串行处理队列，防止并发 LLM 调用
- 队列持久化到磁盘，崩溃可恢复
- 失败任务自动重试（最多3次）
- 进度可视化（Activity Panel）

**实现位置**：新增 `agent/src/ingest_queue.rs`

---

#### 3. Purpose.md
**现状**：未实现
**借鉴内容**：
- 每个 Wiki 的 `purpose.md` 定义目标、关键问题、研究范围
- LLM 在 ingest 和 query 时都读取作为上下文
- Schema 是结构规则，Purpose 是方向意图

**文件位置**：`{vault_path}/purpose.md`

---

### 2.2 中优先级（知识图谱增强）

#### 4. 4-Signal Knowledge Graph
**现状**：AxAgent 有 LinkGraph（双向链接解析）
**借鉴内容**：

| Signal | Weight | Description |
|--------|--------|-------------|
| Direct link | ×3.0 | 通过 `[[wikilinks]]` 直接链接 |
| Source overlap | ×4.0 | 共享同一 raw source（通过 frontmatter `sources[]`） |
| Adamic-Adar | ×1.5 | 共享邻居节点（按邻居度数加权） |
| Type affinity | ×1.0 | 同类型页面bonus（entity↔entity, concept↔concept） |

**实现位置**：`core/src/repo/note_graph.rs` → 扩展 relevance 模型

---

#### 5. Louvain Community Detection
**现状**：未实现
**借鉴内容**：
- 使用 Louvain 算法自动发现知识聚类
- 聚类 cohesion scoring（低 cohesion < 0.15 警告）
- 支持按 type / community 切换着色

**依赖库**：`graphology-communities-louvain`（前端）或 Rust 图算法库（后端）

---

#### 6. Graph Insights
**现状**：未实现
**借鉴内容**：
- **Surprising Connections**：跨社区边、跨类型链接、边缘↔中心耦合
- **Knowledge Gaps**：
  - 孤立页面（度数 ≤ 1）
  - 稀疏聚类（cohesion < 0.15，≥3页）
  - 桥接节点（连接3+聚类）
- 一键 Deep Research

**实现位置**：前端 `GraphView.tsx` + 后端 `query_engine.rs`

---

### 2.3 低优先级（扩展功能）

#### 7. Deep Research
**现状**：未实现
**借鉴内容**：
- LLM 优化搜索主题（读取 overview.md + purpose.md）
- 多查询网页搜索
- 自动 ingest 搜索结果到 wiki
- 可编辑的确认对话框

**实现复杂度**：高，需要搜索引擎集成

---

#### 8. Async Review System
**现状**：未实现
**借鉴内容**：
- LLM 标记需要人类判断的事项
- 预定义操作选项
- 预生成搜索查询

**可简化为**：作为 Lint 流程的一部分

---

#### 9. Folder Import
**现状**：未实现
**借鉴内容**：
- 递归文件夹导入，保留目录结构
- 文件夹路径作为 LLM 分类上下文

---

#### 10. Chrome Web Clipper
**现状**：未实现
**借鉴内容**：
- 浏览器扩展，一键网页捕获
- 自动 ingest 到知识库

**实现参考**：`llm_wiki/extension` 目录

---

#### 11. Multi-Conversation Chat
**现状**：AxAgent 有 Chat，是 Agent 系统的一部分
**借鉴内容**：
- 独立的 Chat 会话管理
- 会话侧边栏快速切换
- 每会话持久化到 `.llm-wiki/chats/{id}.json`

**注意**：需与现有 Agent Chat 系统协调

---

## 三、功能优先级汇总

| 优先级 | 功能 | 工作量估计 | 依赖 |
|--------|------|-----------|------|
| P0 | Two-Step CoT Ingest | 中 | 现有 IngestPipeline 扩展 |
| P0 | SHA256 增量缓存 | 低 | 现有 IngestPipeline 扩展 |
| P1 | Persistent Ingest Queue | 中 | 新增 ingest_queue.rs |
| P1 | Purpose.md | 低 | 新增文件模板 |
| P2 | 4-Signal Knowledge Graph | 中 | LinkGraph 扩展 |
| P2 | Louvain Community Detection | 中 | 引入图算法库 |
| P2 | Graph Insights | 中 | 与 Knowledge Graph 配合 |
| P3 | Deep Research | 高 | 搜索引擎集成 |
| P3 | Folder Import | 低 | 扩展文件导入功能 |
| P3 | Chrome Web Clipper | 中 | 新增 extension 目录 |
| P4 | Async Review System | 低 | 可集成到 Lint 流程 |

---

## 四、关键文件对应关系

| llm_wiki 模块 | AxAgent 现有文件 | 备注 |
|--------------|-----------------|------|
| Two-Step Ingest | `agent/src/ingest_pipeline.rs` | 需重构为两步 |
| Ingest Queue | `agent/src/ingest_queue.rs` | 新增 |
| WikiCompiler | `agent/src/wiki_compiler.rs` | 已有，可扩展 |
| QueryEngine | `agent/src/query_engine.rs` | 已有，可扩展 |
| LintChecker | `agent/src/lint_checker.rs` | 已有 |
| Knowledge Graph | `core/src/repo/note_graph.rs` | 已有，可增强 |
| Schema Manager | `agent/src/schema_manager.rs` | 检查是否存在 |

---

## 五、附录：llm_wiki 核心技术点

### 5.1 Query Pipeline 流程
```
Phase 1: Tokenized Search
  → 英文：word splitting + stop word removal
  → 中文：CJK bigram tokenization

Phase 1.5: Vector Semantic Search (optional)
  → LanceDB 存储
  → OpenAI-compatible /v1/embeddings endpoint

Phase 2: Graph Expansion
  → 4-signal relevance model
  → 2-hop traversal with decay

Phase 3: Budget Control
  → 可配置 context window (4K → 1M tokens)
  → 按比例分配：60% wiki pages, 20% chat history, 5% index, 15% system

Phase 4: Context Assembly
  → 编号页面 + 完整内容
  → LLM 按编号引用 [1], [2]
```

### 5.2 Ingest 输出结构
```yaml
---
type: source-summary  # 或 entity/concept/index/overview
title: "源文件摘要"
sources: ["raw/paper.pdf"]  # 源文件追溯
created_at: timestamp
---

内容正文...
```

### 5.3 Graph Insights 计算
- **Surprise Score** = cross_community × 2 + cross_type × 1.5 + peripheral_hub × 1
- **Cohesion** = intra_edges / possible_edges
- **Bridge Detection** = 节点连接的聚类数 ≥ 3
