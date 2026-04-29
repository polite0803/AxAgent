# LLM Wiki (Obsidian + Karpathy LLM Wiki) 设计方案

> 日期：2026-04-29
> 版本：v2.0

---

## 一、项目概述与目标

### 1.1 项目背景

AxAgent 目前已具备成熟的**知识库（Knowledge Base）**系统，支持 RAG 文档检索。本项目旨在构建一个**双模式知识管理系统**，融合两种范式：

| 模式 | 代表 | 特点 |
|------|------|------|
| **Obsidian 风格** | 双链笔记、个人知识管理 | 用户主导、`[[双链]]`语法、知识图谱 |
| **Karpathy LLM Wiki** | LLM 主动编译知识 | Agent 主导、`ingest/query/lint`循环、增量积累 |

### 1.2 核心目标

| 目标 | 说明 |
|------|------|
| **Obsidian 风格笔记** | 支持 `[[双链]]` 语法、本地 Markdown 存储、快速搜索 |
| **知识图谱可视化** | 笔记关系网络图展示双向链接 |
| **LLM Wiki 编译引擎** | Agent 从原始资料（网页/文章）增量编译知识到 wiki |
| **Ingest/Query/Lint 循环** | 维护 Wiki 生命周期的核心操作 |
| **工作流编排** | 用工作流实现笔记自动化处理 |
| **RAG 检索增强** | 笔记内容可被 RAG 问答系统引用 |
| **本地 + 云端存储** | 本地文件系统 + S3/WebDAV 云同步 |

### 1.3 与现有 KB 的关系

**三系统并存 + Vault ↔ RAG 一对一架构**：
- **Wiki Vault 系统**：每个 Vault 是独立的知识单元，拥有自己的 RAG 向量库（`vec_wiki_{vault_id}`）
- **LLM Wiki 编译系统**：面向 Agent 驱动的知识积累，从原始资料编译到 Wiki Vault
- **知识库 KB**：面向结构化文档管理，支持 PDF/DOCX 等文件解析、RAG 检索（独立于 Wiki）
- **共享层**：Wiki Vault 的 RAG 与 KB RAG 平行运作，通过统一检索接口调用

---

## 二、系统架构

### 2.1 核心设计原则

| 原则 | 说明 |
|------|------|
| **多 Vault 架构** | 每个 Wiki 是独立的 Vault，支持多主题/多项目隔离管理 |
| **Vault ↔ RAG 一对一** | 每个 Vault 拥有独立的 RAG Source，便于针对性检索 |
| **一套数据结构** | 同一实体同时服务 Wiki 浏览和 RAG 检索，无需重复存储 |
| **Obsidian 兼容** | 文件格式与 Obsidian vault 完全兼容 |
| **Wiki 与 RAG 同步** | Wiki 文件系统 ↔ RAG 向量库实时同步，两者是同一数据结构的两面 |

### 2.2 Wiki Vault 目录结构

LLM Wiki 遵循 Karpathy LLM Wiki 模式，采用统一目录结构：

```
~/axagent-notes/{vault_id}/
├── notes/                   ← 统一笔记目录
│   ├── user_note.md       ← 用户笔记 (author: "user")
│   └── llm_*.md           ← LLM 编译产出 (author: "llm")
├── raw/                    ← 原始资料（LLM 素材，仅供 LLM 阅读，不索引）
│   ├── document1.pdf
│   ├── article.html
│   └── notes.docx
├── SCHEMA.md                ← LLM Wiki 的 Schema 定义
└── .obsidian/              ← Obsidian 配置
```

**统一目录 + 元数据区分**：

| 字段 | 值 | 说明 |
|------|-----|------|
| `author: user` | 用户笔记 | 用户亲自编写，长期保存 |
| `author: llm` | LLM 编译产出 | LLM 从 raw/ 编译生成，可能被覆盖 |
| `source` | 素材路径 | LLM 编译产出的来源（如 `raw/doc.pdf`） |

**frontmatter 示例**：

```yaml
# user_note.md
---
title: 我的笔记
author: user
created: 2026-04-29
tags: [笔记, 收藏]
---
```

```yaml
# llm_summary.md
---
title: 论文摘要：Attention Is All You Need
author: llm
source: raw/papers/attention.pdf
compiled_at: 2026-04-29
tags: [论文, AI, Transformer]
---
```

**LLM Wiki 的工作方式**：
```
raw/ (LLM 阅读素材)  ──►  LLM 理解/编译  ──►  notes/llm_*.md (产出的知识)
                                                        │
                                                        ▼
                                                  RAG 索引检索
```

**核心理解**：所有笔记统一放在 `notes/` 目录，通过 frontmatter 区分来源。raw/ 中的文件只是 LLM 的"原材料"，不参与 RAG。

### 2.3 Wiki ↔ RAG 同步机制

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    Wiki Vault = 文件系统 + RAG 向量库                        │
│                                                                             │
│   ~/axagent-notes/{vault_id}/                                                │
│   │                                                                           │
│   ├── notes/              ←─── 统一笔记目录                                  │
│   │   ├── *.md (user)  ←─── 用户编辑 ─────────────────────┐               │
│   │   └── llm_*.md   ←─── LLM 编译产出 ──────────────────┼───► 同步      │
│   │                                                         │               │
│   └── raw/            ←─── 原始文件（LLM 素材）           │               │
│                                    │                     ▼               │
│                                    ▼            ┌─────────────────┐       │
│                         ┌─────────────────────┐ │ vec_wiki_{vid} │       │
│                         │   IngestPipeline    │ │  (RAG 向量库)  │       │
│                         │  LLM 读取 raw/     │ │               │       │
│                         │  编译到 notes/     │ │  仅索引 .md   │       │
│                         └─────────────────────┘ └─────────────────┘       │
│                                                                             │
│   操作逻辑：                                                                 │
│   ┌─────────────┐    ┌─────────────┐    ┌─────────────────────────────┐    │
│   │ 用户编辑笔记 │───►│ 保存文件系统 │───►│ 自动更新向量库 (同步)       │    │
│   └─────────────┘    └─────────────┘    └─────────────────────────────┘    │
│                                                                             │
│   LLM Wiki Ingest 流程（raw/ 不直接参与 RAG）：                              │
│   ┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌───────────┐   │
│   │ raw/ 素材   │───►│ LLM 阅读    │───►│ LLM 编译    │───►│ notes/   │   │
│   └─────────────┘    └─────────────┘    └─────────────┘    └───────────┘   │
│                                                            │               │
│                                                            ▼               │
│                                                     同步到向量库            │
│                                                                             │
│   检索逻辑：                                                                 │
│   ┌─────────────┐    ┌─────────────┐    ┌─────────────────────────────┐    │
│   │ RAG 问答    │───►│ 查向量库     │───►│ 返回 notes/ 位置          │    │
│   └─────────────┘    └─────────────┘    └─────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────────────┘
```

**核心原则**：
- **同一份文件**：Wiki 浏览和 RAG 检索读取的是同一份 Markdown 文件
- **raw/ 不参与 RAG**：raw/ 中的 PDF/Office 等文件只是 LLM 的素材，LLM 编译产出到 notes/ 后才参与 RAG
- **双向同步**：用户编辑 notes/ 中的 user 笔记或 LLM 编译生成 llm 笔记 → 自动更新向量库

### 2.4 整体架构图

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                              AxAgent 前端 (React)                                 │
│  ┌────────────────┐  ┌────────────────┐  ┌────────────────┐  ┌──────────────┐ │
│  │ Wiki 笔记 UI    │  │ LLM Wiki UI    │  │ KB 文档 UI     │  │ RAG 问答入口 │ │
│  │ · 双链编辑器   │  │ · 编译状态     │  │ · 文档管理     │  │ · 指定 Wiki  │ │
│  │ · 知识图谱     │  │ · Schema 视图  │  │ · 上传解析     │  │ · 问答助手   │ │
│  │ · Command      │  │ · Ingest 日志  │  │ · 检索         │  │              │ │
│  │   Palette      │  │ · Query 界面   │  │                │  │              │ │
│  └───────┬────────┘  └───────┬────────┘  └───────┬────────┘  └──────┬───────┘ │
│          │                   │                    │                   │          │
│          └───────────────────┼────────────────────┴───────────────────┘          │
│                              ▼                                                    │
│  ┌─────────────────────────────────────────────────────────────────────────────┐ │
│  │                     Tauri Command API (统一入口)                                │ │
│  │  wiki_notes_*  │  llm_wiki_*  │  knowledge_*  │  rag_search  │  workflow_*   │ │
│  └─────────────────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────────┐
│                            AxAgent Core (Rust)                                    │
│                                                                                  │
│  ┌──────────────────────┐    ┌──────────────────────┐    ┌────────────────────┐  │
│  │   Wiki 笔记核心       │    │   LLM Wiki 编译引擎   │    │   Knowledge Base   │  │
│  │   · NoteRepo         │    │   · WikiCompiler     │    │   · KnowledgeRepo  │  │
│  │   · LinkGraph        │    │   · IngestPipeline  │◄──│   · DocumentParser │  │
│  │   · MarkdownParser   │    │   · QueryEngine      │    │                    │  │
│  └─────────┬────────────┘    │   · LintChecker      │    └─────────┬──────────┘  │
│            │                 └─────────┬────────────┘              │              │
│            │                           │                           │              │
│            └───────────────────────────┼───────────────────────────┘              │
│                                        ▼                                          │
│  ┌─────────────────────────────────────────────────────────────────────────────┐ │
│  │                    统一 RAG 检索层 (Hybrid Search)                            │ │
│  │                                                                             │ │
│  │   WikiVaultRAG(wiki_id) ◄────► Vault(wiki_id) ◄────► VectorCollection      │ │
│  │         │                           │                      (per-vault)    │ │
│  │         │                           ├─ notes 表 (vault_id)                   │ │
│  │         │                           ├─ note_links 表 (vault_id)              │ │
│  │         │                           ├─ wiki_sources 表 (vault_id)            │ │
│  │         │                           ├─ wiki_pages 表 (vault_id)             │ │
│  │         │                           └─ wiki_operations 表 (vault_id)         │ │
│  │         │                                                                   │ │
│  │         └───────────────────────► vec_wiki_{vault_id} (独立向量库)            │ │
│  │                                                                             │ │
│  │   KnowledgeRAG + MemoryRAG (原有 KB 系统，平行运作)                         │ │
│  └─────────────────────────────────────────────────────────────────────────────┘ │
│                                                                                  │
│  ┌────────────────────┐  ┌────────────────────┐  ┌──────────────────────────┐  │
│  │   Agent 引擎       │  │   工作流引擎        │  │   存储适配器              │  │
│  │   · 摘要/标签      │  │   · 笔记整理流程   │  │   · FileStore           │  │
│  │   · Wiki 编译      │  │   · 定时任务       │  │   · S3Client            │  │
│  │   · 知识归纳       │  │   · 自动化触发     │  │   · WebDavClient        │  │
│  └────────────────────┘  └────────────────────┘  └──────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────────┘
                                       │
                    ┌─────────────────┴─────────────────┐
                    ▼                                       ▼
          ┌─────────────────────┐               ┌─────────────────────┐
          │   本地文件系统        │               │   云存储              │
          │   ~/axagent-notes   │               │   S3 / WebDAV        │
          │                     │               │                     │
          │   Vault 1 ─────────┼──► RAG 1      │                     │
          │   ├── notes/       │               │                     │
          │   ├── raw/         │               │                     │
          │   ├── SCHEMA.md    │               │                     │
          │   └── .obsidian/   │               │                     │
          │                     │               │                     │
          │   Vault 2 ─────────┼──► RAG 2      │                     │
          │   └── ...          │               │                     │
          │                     │               │                     │
          │   Vault N ─────────┼──► RAG N      │                     │
          └─────────────────────┘
```

### 2.5 Vault 与 RAG 一对一映射

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         Vault ↔ RAG 一对一映射                                │
│                                                                             │
│  ┌─────────────┐     ┌─────────────────┐     ┌─────────────────────────┐   │
│  │  Vault 1    │────►│  WikiVaultRAG   │────►│  vec_wiki_1            │   │
│  │  (笔记+LLM) │     │  (RAG Source)   │     │  (独立向量库)            │   │
│  └─────────────┘     └─────────────────┘     └─────────────────────────┘   │
│           │                                                                  │
│           ├─ notes 表 (vault_id = 1)                                         │
│           ├─ note_links 表 (vault_id = 1)                                    │
│           ├─ wiki_sources 表 (vault_id = 1)                                  │
│           ├─ wiki_pages 表 (vault_id = 1)                                    │
│           └─ wiki_operations 表 (vault_id = 1)                               │
│                                                                             │
│  ┌─────────────┐     ┌─────────────────┐     ┌─────────────────────────┐   │
│  │  Vault 2    │────►│  WikiVaultRAG   │────►│  vec_wiki_2            │   │
│  │  (独立主题) │     │  (RAG Source)   │     │  (独立向量库)            │   │
│  └─────────────┘     └─────────────────┘     └─────────────────────────┘   │
│                                                                             │
│  每个 Vault 拥有独立的：                                                     │
│  · 文件夹（~/axagent-notes/{vault-id}/）                                    │
│  · 数据库记录（通过 vault_id 隔离）                                          │
│  · 向量库（vec_wiki_{vault_id}）                                            │
│  · RAG 检索上下文                                                           │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 2.6 核心模块职责

| 模块 | 路径 | 职责 |
|------|------|------|
| **NoteRepo** | `core/src/repo/note.rs` | 笔记 CRUD、文件系统读写、元数据管理（按 vault_id 隔离） |
| **LinkGraph** | `core/src/repo/note_graph.rs` | 双向链接解析、Backlinks 计算 |
| **MarkdownParser** | `core/src/markdown_parser.rs` | `[[双链]]` 语法解析、frontmatter 提取 |
| **WikiCompiler** | `agent/src/wiki_compiler.rs` | LLM Wiki 核心编译引擎 |
| **IngestPipeline** | `agent/src/ingest_pipeline.rs` | 原始资料解析 → 知识条目 |
| **QueryEngine** | `agent/src/query_engine.rs` | Wiki 内容检索与问答 |
| **LintChecker** | `agent/src/lint_checker.rs` | Wiki 结构完整性与一致性检查 |
| **WikiVaultRAG** | `core/src/rag.rs` | Vault 级别的 RAG 检索，按 wiki_id 隔离向量库 |
| **WikiWorkflow** | `runtime/src/work_engine/` | 笔记自动化工作流节点 |

---

## 三、数据模型

### 3.1 Wiki 笔记实体（Obsidian 模式）

#### 3.1.1 notes 表

```rust
// src-tauri/crates/core/src/entity/notes.rs

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "notes")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub title: String,
    pub file_path: String,
    pub content_hash: String,
    pub doc_xml: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub is_deleted: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::note_links::Entity")]
    NoteLink,
    #[sea_orm(has_many = "super::note_backlinks::Entity")]
    Backlink,
}
```

#### 3.1.2 note_links 表

```rust
// src-tauri/crates/core/src/entity/note_links.rs

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "note_links")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub source_note_id: String,
    pub target_note_id: String,
    pub link_text: String,
    pub context_snippet: String,
}
```

### 3.2 LLM Wiki 实体（Karpathy 模式）

#### 3.2.1 wiki_sources 表（原始资料）

```rust
// src-tauri/crates/core/src/entity/wiki_sources.rs

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "wiki_sources")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub wiki_id: String,              // 所属 wiki 命名空间
    pub source_path: String,           // 相对路径 raw/
    pub source_type: String,          // "web_article", "paper", "book", "raw_markdown"
    pub title: String,
    pub url: Option<String>,           // 如果是网页
    pub content_hash: String,         // SHA256，内容变化检测
    pub last_ingested_at: Option<i64>,
    pub created_at: i64,
}
```

#### 3.2.2 wiki_pages 表（编译后的知识页面）

```rust
// src-tauri/crates/core/src/entity/wiki_pages.rs

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "wiki_pages")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub wiki_id: String,
    pub page_type: String,           // "concept", "entity", "comparison", "source_summary", "index", "log", "overview"
    pub title: String,
    pub file_path: String,            // 相对路径 notes/
    pub content: String,
    pub source_refs: Json,           // ["source_id_1", "source_id_2"]
    pub related_pages: Json,        // ["page_id_1", "page_id_2"]
    pub quality_score: f64,          // Lint 评分
    pub last_linted_at: Option<i64>,
    pub last_modified_at: i64,
    pub created_at: i64,
}
```

#### 3.2.3 wiki_operations 表（操作日志）

```rust
// src-tauri/crates/core/src/entity/wiki_operations.rs

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "wiki_operations")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub wiki_id: String,
    pub operation_type: String,      // "ingest", "query", "lint", "compile"
    pub status: String,               // "pending", "running", "completed", "failed"
    pub input_refs: Json,            // 相关的 source_ids 或 query
    pub output_refs: Json,           // 生成的 page_ids
    pub error_message: Option<String>,
    pub started_at: i64,
    pub completed_at: Option<i64>,
}
```

### 3.3 TypeScript 类型

```typescript
// src/types/note.ts

export type Note = {
  id: string;
  title: string;
  filePath: string;
  content: string;
  contentHash: string;
  preview: string;
  createdAt: number;
  updatedAt: number;
  tags: string[];
  links: NoteLink[];
  backlinks: NoteLink[];
  isDeleted: boolean;
};

export type NoteLink = {
  id: string;
  targetNoteId: string;
  targetTitle: string;
  linkText: string;
  contextSnippet: string;
};

export type NoteGraph = {
  nodes: GraphNode[];
  edges: GraphEdge[];
};

// LLM Wiki Types
export type WikiSource = {
  id: string;
  wikiId: string;
  sourcePath: string;
  sourceType: 'web_article' | 'paper' | 'book' | 'raw_markdown';
  title: string;
  url?: string;
  contentHash: string;
  lastIngestedAt?: number;
  createdAt: number;
};

export type WikiPage = {
  id: string;
  wikiId: string;
  pageType: 'concept' | 'entity' | 'comparison' | 'source_summary' | 'index' | 'log' | 'overview';
  title: string;
  filePath: string;
  content: string;
  sourceRefs: string[];
  relatedPages: string[];
  qualityScore: number;
  lastLintedAt?: number;
  lastModifiedAt: number;
  createdAt: number;
};

export type WikiOperation = {
  id: string;
  wikiId: string;
  operationType: 'ingest' | 'query' | 'lint' | 'compile';
  status: 'pending' | 'running' | 'completed' | 'failed';
  inputRefs: string[];
  outputRefs: string[];
  errorMessage?: string;
  startedAt: number;
  completedAt?: number;
};

export type LlmWiki = {
  id: string;
  name: string;
  schemaContent: string;           // SCHEMA.md 内容
  rootPath: string;                //  ~/axagent-notes/{vault-id}/
  embeddingProvider?: string;
  createdAt: number;
  updatedAt: number;
};

export type CreateLlmWikiInput = {
  name: string;
  schemaContent?: string;          // 可选，使用默认 Schema 模板
};

export type IngestInput = {
  wikiId: string;
  sourcePaths: string[];            // raw/ 下的文件路径
  sourceType: WikiSource['sourceType'];
};

export type QueryInput = {
  wikiId: string;
  query: string;
  includeSources?: boolean;
};

export type LintInput = {
  wikiId: string;
  pageIds?: string[];             // 可选，指定页面；空则全量
};
```

---

## 四、LLM Wiki 核心：Ingest / Query / Lint 循环

### 4.1 三循环概述

```
                    ┌─────────────────────────────────────────────┐
                    │           LLM Wiki 生命周期                    │
                    │                                             │
    ┌─────────┐     │   ┌─────────┐    ┌─────────┐    ┌─────────┐ │
    │  raw/   │────►│   │ INGEST  │───►│ COMPILE │───►│ notes/ │ │
    │ (原始)  │     │   └─────────┘    └─────────┘    └─────────┘ │
    └─────────┘     │       │                            │       │
                    │       ▼                            │       │
                    │   ┌─────────┐                      │       │
                    │   │  QUERY  │◄─────────────────────┘       │
                    │   └─────────┘    (检索已编译知识)            │
                    │       │                                    │
                    │       ▼                                    │
                    │   ┌─────────┐                              │
                    └──►│  LINT   │──────────────────────────────┘
                        └─────────┘    (检查完整性、一致性)
```

### 4.2 INGEST - 原始资料摄入

**目的**：将外部资料（网页文章、PDF、Markdown）转换为结构化条目，写入 `raw/` 目录。

```rust
// agent/src/ingest_pipeline.rs

pub struct IngestPipeline {
    parser: Arc<dyn SourceParser>,
    embedder: Arc<dyn Embedder>,
}

impl IngestPipeline {
    /// 从 URL 或文件路径提取内容
    pub async fn ingest(
        &self,
        wiki_id: &str,
        source: &IngestSource,
    ) -> Result<IngestResult> {
        // 1. 解析原始内容
        let parsed = self.parser.parse(source).await?;

        // 2. 提取关键信息（标题、摘要、关键段落）
        let extracted = self.extract_metadata(&parsed).await?;

        // 3. 写入 raw/ 目录
        let raw_path = self.save_to_raw(wiki_id, &parsed).await?;

        // 4. 生成嵌入向量（用于后续检索）
        let embedding = self.embedder.embed(&parsed.content).await?;

        // 5. 记录到 wiki_sources 表
        let source_record = self.save_source_record(wiki_id, &raw_path, &extracted).await?;

        Ok(IngestResult {
            source_id: source_record.id,
            raw_path,
            embedding,
        })
    }
}
```

**支持的源类型**：

| 类型 | 解析方式 |
|------|----------|
| `web_article` | HTTP GET + HTML 解析（提取正文） |
| `paper` | PDF 解析 + 关键段落提取 |
| `book` | 长文本分块处理 |
| `raw_markdown` | 直接读取 Markdown 文件 |

### 4.3 COMPILE - 知识编译

**目的**：调用 LLM 将 `raw/` 中的原始资料编译成结构化的 `notes/` 页面。

```rust
// agent/src/wiki_compiler.rs

pub struct WikiCompiler {
    agent: Arc<AgentRuntime>,
}

impl WikiCompiler {
    /// 增量编译：将新 source 编译成 wiki 页面
    pub async fn compile(
        &self,
        wiki: &LlmWiki,
        source_ids: Vec<String>,
    ) -> Result<CompileResult> {
        // 1. 读取 SCHEMA.md（操作契约）
        let schema = self.read_schema(&wiki.root_path).await?;

        // 2. 读取原始资料
        let sources = self.load_sources(&wiki.root_path, &source_ids).await?;

        // 3. 调用 LLM 编译
        let pages = self.llm_compile(&schema, &sources).await?;

        // 4. 写入 notes/ 目录
        let page_paths = self.save_pages(&wiki.root_path, pages).await?;

        // 5. 更新 wiki_pages 表
        let page_records = self.save_page_records(&wiki.id, &page_paths).await?;

        // 6. 更新 index.md, log.md, overview.md
        self.update_index(&wiki.root_path).await?;

        Ok(CompileResult {
            new_pages: page_records,
        })
    }

    async fn llm_compile(
        &self,
        schema: &str,
        sources: &[WikiSource],
    ) -> Result<Vec<CompiledPage>> {
        let prompt = format!(
            r#"你是知识工程师。根据以下 SCHEMA 和原始资料，编译成结构化 wiki 页面。

SCHEMA:
{}

原始资料:
{}

输出要求：
1. 为每个独立概念/实体创建页面
2. 创建源摘要页面
3. 更新 index.md 索引
4. 更新 log.md 操作日志
"#,
            schema,
            sources.iter().map(|s| format!("## {}\n{}", s.title, s.content)).join("\n\n")
        );

        let response = self.agent.complete(&prompt).await?;
        self.parse_compiled_output(&response)
    }
}
```

### 4.4 QUERY - 知识检索

**目的**：在已编译的 wiki 中检索知识，作为 Agent 的长期记忆。

```rust
// agent/src/query_engine.rs

pub struct QueryEngine {
    rag: Arc<dyn RAGSource>,
    embedder: Arc<dyn Embedder>,
}

impl QueryEngine {
    /// 检索与 query 相关的 wiki 页面
    pub async fn query(
        &self,
        wiki: &LlmWiki,
        user_query: &str,
        include_sources: bool,
    ) -> Result<QueryResult> {
        // 1. 向量化 query
        let query_embedding = self.embedder.embed(user_query).await?;

        // 2. 向量检索
        let pages = self.rag.search(&wiki.id, query_embedding, top_k).await?;

        // 3. 可选：同时检索原始资料
        let sources = if include_sources {
            self.rag.search_raw(&wiki.id, query_embedding, top_k).await?
        } else {
            vec![]
        };

        Ok(QueryResult { pages, sources })
    }

    /// 直接用自然语言查询 wiki（LLM 综述）
    pub async fn ask(
        &self,
        wiki: &LlmWiki,
        question: &str,
    ) -> Result<String> {
        // 1. 检索相关页面
        let relevant = self.query(wiki, question, true).await?;

        // 2. 构建上下文
        let context = self.build_context(&relevant);

        // 3. LLM 生成答案
        let prompt = format!(
            "基于以下 wiki 内容回答问题。如果信息不足，说明不知道。\n\n问题：{}\n\n{}",
            question, context
        );

        self.agent.complete(&prompt).await
    }
}
```

### 4.5 LINT - 结构检查

**目的**：检查 wiki 完整性，修复断链、缺失索引等问题。

```rust
// agent/src/lint_checker.rs

pub struct LintChecker {
    agent: Arc<AgentRuntime>,
}

#[derive(Debug)]
pub struct LintResult {
    pub issues: Vec<LintIssue>,
    pub score: f64,           // 0.0 ~ 1.0 质量分
}

pub enum LintIssue {
    BrokenLink { page: String, link: String },
    MissingIndexEntry { page: String },
    OrphanPage { page: String },           // 没有被任何页面引用
    StaleOverview,
    IncompleteSourceSummary { source: String },
}

impl LintChecker {
    /// 检查 wiki 结构完整性
    pub async fn lint(
        &self,
        wiki: &LlmWiki,
        page_ids: Option<Vec<String>>,
    ) -> Result<LintResult> {
        // 1. 解析所有页面的 [[链接]]
        let links = self.extract_all_links(&wiki.root_path).await?;

        // 2. 检查每个链接是否有效
        let broken_links = self.find_broken_links(&links).await?;

        // 3. 检查 index.md 是否包含所有页面
        let missing_index = self.find_missing_index_entries().await?;

        // 4. 查找孤儿页面（没有被任何页面引用）
        let orphans = self.find_orphan_pages(&links).await?;

        // 5. 综合评分
        let score = self.calculate_score(&broken_links, &missing_index, &orphans);

        Ok(LintResult {
            issues: vec![broken_links, missing_index, orphans].concat(),
            score,
        })
    }

    /// 自动修复可修复的问题
    pub async fn auto_fix(&self, wiki: &LlmWiki, issues: &[LintIssue]) -> Result<()> {
        for issue in issues {
            match issue {
                LintIssue::BrokenLink { page, link } => {
                    self.fix_broken_link(wiki, page, link).await?;
                }
                LintIssue::MissingIndexEntry { page } => {
                    self.add_to_index(wiki, page).await?;
                }
                LintIssue::OrphanPage { page } => {
                    self.add_references(wiki, page).await?;
                }
                _ => {} // 其他问题需要人工介入
            }
        }
        Ok(())
    }
}
```

---

## 五、LLM Wiki 目录结构

### 5.1 整体结构

```
~/axagent-notes/
└── {vault-id}/                     # 每个 Vault 是一个独立 Wiki
    ├── notes/                      # 统一笔记目录
    │   ├── index.md               # 知识索引
    │   ├── log.md                 # 操作日志（时间线）
    │   ├── overview.md            # 总览
    │   │
    │   ├── user/                  # 用户笔记（author: user）
    │   │   ├── work/
    │   │   │   └── project-notes.md
    │   │   └── daily/
    │   │       └── 2026-04-29.md
    │   │
    │   ├── concepts/              # 概念页面（author: llm）
    │   │   ├── machine-learning.md
    │   │   └── transformer.md
    │   │
    │   ├── entities/              # 实体页面（author: llm）
    │   │   ├── GPT-4.md
    │   │   └── Claude-3.md
    │   │
    │   ├── comparisons/           # 对比页面（author: llm）
    │   │   └── GPT-4-vs-Claude-3.md
    │   │
    │   └── sources/               # 源摘要（author: llm）
    │       └── article-001-summary.md
    │
    ├── raw/                       # 不可变的原始资料（LLM 素材）
    │   ├── article-001.md         # 网页文章
    │   ├── paper-002.pdf          # 论文
    │   └── book-003.md            # 书籍笔记
    │
    ├── SCHEMA.md                  # 唯一真实来源（操作契约）
    └── .obsidian/                 # Obsidian 配置
```

### 5.2 SCHEMA.md 示例

```markdown
# LLM Wiki Schema

这是本 Wiki 的唯一真实来源。所有 Agent 都应遵循此 Schema。

## 目录结构

- `raw/` - 不可变的原始资料（网页、论文、书籍等）
- `notes/` - 统一笔记目录，通过 frontmatter 的 author 字段区分来源

## frontmatter 字段

| 字段 | 值 | 说明 |
|------|-----|------|
| `author` | `user` | 用户亲自编写的笔记 |
| `author` | `llm` | LLM 编译生成的页面 |
| `source` | 路径 | llm 编译页面的来源（如 `raw/article-001.md`） |

## 笔记目录结构

### notes/user/
用户主导的笔记目录。格式：
- 标题：用户定义
- frontmatter：author = "user"

### notes/concepts/
概念定义页。格式：
- 标题：概念名称
- frontmatter：author = "llm", page_type = "concept"

### notes/entities/
实体页（人、产品、公司等）。格式：
- 标题：实体名称
- frontmatter：author = "llm", page_type = "entity"

### notes/comparisons/
对比分析页。格式：
- 标题：`A vs B`
- frontmatter：author = "llm", page_type = "comparison"

### notes/sources/
源摘要页。格式：
- 标题：源文件名称
- frontmatter：author = "llm", page_type = "source", source = "raw/xxx"

## 操作规则

### INGEST
1. 原始资料存入 `raw/`，文件名格式：`{type}-{nnn}.{ext}`
2. 更新 `notes/sources/{filename}-summary.md`

### COMPILE
1. 基于 `raw/` 内容编译新页面到 `notes/`
2. 每个概念/实体至少一个独立页面
3. 更新 `notes/index.md` 索引
4. 更新 `notes/log.md` 添加操作记录

### LINT
1. 所有 `[[链接]]` 必须有效
2. `index.md` 必须包含所有页面
3. 没有孤儿页面（所有页面至少被一个其他页面引用）
4. `overview.md` 必须是最新的总览

## 质量标准
- 每个页面至少 3 句话
- 概念页必须包含与其他概念的关系
- 源摘要必须标注来源 URL
```

---

## 六、API 设计

### 6.1 Wiki 笔记 Commands（Obsidian 模式）

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `wiki_notes_list` | `vault_path?: string` | `Note[]` | 列出笔记 |
| `wiki_notes_get` | `id: string` | `Note` | 获取单条笔记 |
| `wiki_notes_create` | `input: CreateNoteInput` | `Note` | 创建笔记 |
| `wiki_notes_update` | `id: string, input: UpdateNoteInput` | `Note` | 更新笔记 |
| `wiki_notes_delete` | `id: string` | `void` | 软删除笔记 |
| `wiki_notes_search` | `query: string, top_k: number` | `NoteSearchResult[]` | 全文检索 |
| `wiki_notes_get_graph` | `note_id?: string` | `NoteGraph` | 获取关系图 |
| `wiki_notes_sync` | `direction` | `SyncResult` | 同步 |

### 6.2 LLM Wiki Commands

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `llm_wiki_list` | - | `LlmWiki[]` | 列出所有 wiki |
| `llm_wiki_create` | `input: CreateLlmWikiInput` | `LlmWiki` | 创建 wiki |
| `llm_wiki_get` | `id: string` | `LlmWiki` | 获取 wiki 详情 |
| `llm_wiki_delete` | `id: string` | `void` | 删除 wiki |
| `llm_wiki_ingest` | `input: IngestInput` | `IngestResult` | 摄入原始资料 |
| `llm_wiki_compile` | `wiki_id: string, source_ids: string[]` | `CompileResult` | 编译知识 |
| `llm_wiki_query` | `input: QueryInput` | `QueryResult` | 检索 wiki |
| `llm_wiki_ask` | `wiki_id: string, question: string` | `string` | 问答 |
| `llm_wiki_lint` | `input: LintInput` | `LintResult` | 检查结构 |
| `llm_wiki_fix` | `wiki_id: string, issues: LintIssue[]` | `void` | 自动修复 |
| `llm_wiki_get_operations` | `wiki_id: string` | `WikiOperation[]` | 操作历史 |
| `llm_wiki_import_obsidian` | `vault_path: string` | `LlmWiki` | 从 Obsidian vault 导入 |

### 6.3 Agent Commands

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `wiki_agent_summarize` | `note_id: string` | `string` | 笔记摘要 |
| `wiki_agent_tag` | `note_id: string` | `string[]` | 自动打标签 |
| `wiki_agent_organize` | `note_ids: string[]` | `OrganizeResult` | 批量整理 |

---

## 七、前端组件设计

### 7.1 页面路由

| 路由 | 组件 | 说明 |
|------|------|------|
| `/wiki` | `WikiPage.tsx` | Wiki 主页 |
| `/wiki/:noteId` | `WikiEditorPage.tsx` | 笔记编辑器 |
| `/wiki/graph` | `WikiGraphPage.tsx` | 知识图谱 |
| `/wiki/settings` | `WikiSettingsPage.tsx` | 设置页 |
| `/llm-wiki` | `LlmWikiPage.tsx` | LLM Wiki 列表 |
| `/llm-wiki/:wikiId` | `LlmWikiEditorPage.tsx` | LLM Wiki 编辑器 |
| `/llm-wiki/:wikiId/ingest` | `IngestPage.tsx` | 资料摄入页 |
| `/llm-wiki/:wikiId/log` | `WikiLogPage.tsx` | 操作日志页 |
| `/llm-wiki/:wikiId/graph` | `WikiGraphPage.tsx` | LLM Wiki 知识图谱 |

### 7.2 核心组件

```
src/
├── pages/
│   ├── WikiPage.tsx              # Obsidian 风格笔记主页
│   ├── WikiEditorPage.tsx        # 双链编辑器
│   ├── WikiGraphPage.tsx         # 知识图谱
│   ├── LlmWikiPage.tsx           # LLM Wiki 列表
│   ├── LlmWikiEditorPage.tsx     # LLM Wiki 详情
│   ├── IngestPage.tsx            # 资料摄入
│   └── WikiLogPage.tsx           # 操作日志
├── components/
│   ├── wiki/                     # Wiki 相关组件
│   │   ├── Sidebar.tsx           # 侧边栏
│   │   ├── NoteEditor.tsx        # Markdown 编辑器
│   │   ├── CommandPalette.tsx    # Ctrl+K 快速切换
│   │   ├── BacklinksPanel.tsx    # 反向链接面板
│   │   └── GraphView.tsx         # D3.js 知识图谱
│   └── llm-wiki/                 # LLM Wiki 组件
│       ├── SchemaEditor.tsx      # SCHEMA.md 编辑器
│       ├── IngestPanel.tsx       # 资料摄入面板
│       ├── CompileStatus.tsx     # 编译状态
│       ├── WikiTreeView.tsx      # Wiki 目录树
│       ├── LintReport.tsx        # Lint 报告
│       └── OperationTimeline.tsx  # 操作时间线
├── stores/
│   └── feature/
│       ├── wikiStore.ts          # Obsidian 笔记状态
│       └── llmWikiStore.ts       # LLM Wiki 状态
└── types/
    ├── note.ts                   # 笔记类型
    └── llmWiki.ts                # LLM Wiki 类型
```

## 八、动态知识图谱（Obsidian 风格）

### 7.1 图谱特性

参考 Obsidian 的图谱视图，实现以下交互能力：

| 特性 | 说明 |
|------|------|
| **力导向布局** | D3.js force-directed graph，节点自动排列 |
| **缩放/平移** | 鼠标滚轮缩放，拖拽画布平移 |
| **节点筛选** | 按标签、文件夹、笔记类型筛选 |
| **局部视图** | 点击节点高亮其一跳范围内的关联节点 |
| **点击跳转** | 点击节点跳转至对应笔记页面 |
| **悬停预览** | 悬停显示笔记标题、摘要、链接数 |
| **边类型区分** | 普通链接（灰色）、双向链接（蓝色粗线） |
| **性能优化** | 超过 500 节点时启用 Canvas 渲染 + WebWorker 计算 |

### 7.2 图谱数据结构

```typescript
// GraphView.tsx

export type GraphNode = {
  id: string;
  title: string;
  type: 'note' | 'concept' | 'entity' | 'source';
  tags: string[];
  linkCount: number;          // 出链数量
  backlinkCount: number;      // 入链数量
  path: string;              // 文件路径
  x?: number;                // D3 力导向计算后的坐标
  y?: number;
};

export type GraphEdge = {
  source: string;            // 源节点 ID
  target: string;            // 目标节点 ID
  type: 'link' | 'backlink';
};

export type GraphData = {
  nodes: GraphNode[];
  edges: GraphEdge[];
};
```

### 7.3 图谱组件实现

```tsx
// components/wiki/GraphView.tsx

import { useEffect, useRef, useState } from 'react';
import * as d3 from 'd3';

interface GraphViewProps {
  data: GraphData;
  onNodeClick?: (nodeId: string) => void;
  onNodeHover?: (nodeId: string | null) => void;
  filters?: {
    tags?: string[];
    pathPrefix?: string;
    types?: GraphNode['type'][];
  };
}

export function GraphView({ data, onNodeClick, filters, onNodeHover }: GraphViewProps) {
  const svgRef = useRef<SVGSVGElement>(null);
  const [dimensions, setDimensions] = useState({ width: 800, height: 600 });

  useEffect(() => {
    const svg = d3.select(svgRef.current);

    // 筛选节点
    const filteredNodes = data.nodes.filter(node => {
      if (filters?.tags?.length && !node.tags.some(t => filters.tags!.includes(t))) return false;
      if (filters?.pathPrefix && !node.path.startsWith(filters.pathPrefix)) return false;
      if (filters?.types?.length && !filters.types.includes(node.type)) return false;
      return true;
    });

    const nodeIds = new Set(filteredNodes.map(n => n.id));
    const filteredEdges = data.edges.filter(
      e => nodeIds.has(e.source) && nodeIds.has(e.target)
    );

    // 力导向模拟
    const simulation = d3.forceSimulation(filteredNodes)
      .force('link', d3.forceLink(filteredEdges).id((d: any) => d.id).distance(100))
      .force('charge', d3.forceManyBody().strength(-300))
      .force('center', d3.forceCenter(dimensions.width / 2, dimensions.height / 2))
      .force('collision', d3.forceCollide().radius(30));

    // 渲染节点和边
    // ... (完整实现见代码)

    return () => simulation.stop();
  }, [data, dimensions, filters]);

  return (
    <div className="graph-container">
      <svg ref={svgRef} width={dimensions.width} height={dimensions.height} />
      <div className="graph-controls">
        <button onClick={() => zoomIn()}>+</button>
        <button onClick={() => zoomOut()}>-</button>
        <button onClick={() => resetView()}>重置</button>
      </div>
    </div>
  );
}
```

### 7.4 图谱样式

```css
/* Obsidian 风格图谱样式 */
.graph-container {
  background: var(--bg-primary);
  border-radius: 8px;
  overflow: hidden;
}

.graph-container .node {
  cursor: pointer;
  transition: all 0.2s ease;
}

.graph-container .node:hover {
  filter: brightness(1.2);
  transform: scale(1.1);
}

.graph-container .node--note {
  fill: var(--accent-blue);
}

.graph-container .node--concept {
  fill: var(--accent-green);
}

.graph-container .node--entity {
  fill: var(--accent-orange);
}

.graph-container .link {
  stroke: var(--text-muted);
  stroke-opacity: 0.4;
  transition: all 0.2s ease;
}

.graph-container .link--bidirectional {
  stroke: var(--accent-blue);
  stroke-width: 2px;
}

.graph-container .node-label {
  font-size: 10px;
  fill: var(--text-primary);
  pointer-events: none;
}
```

---

## 九、存储与同步

### 9.1 本地文件系统

- **Wiki Vault**：`~/axagent-notes/{vault-id}/`

Vault 目录结构完全兼容 Obsidian vault 格式。

### 9.2 云端同步

与 Obsidian 风格笔记共享同一套 S3/WebDAV 同步机制：
- 上传：打包为 ZIP（含 `notes/` 和 `raw/`）
- 冲突处理：以本地为准，云端冲突文件加 `.conflict-{timestamp}` 后缀

---

## 十、支持的文件格式

### 9.1 现有文档解析器

项目已有 `document_parser.rs` 支持以下格式，可直接复用：

| 格式 | MIME Type | 解析方式 | 状态 |
|------|-----------|----------|------|
| **Markdown** | `text/markdown` | 直接读取 | ✅ 已支持 |
| **纯文本** | `text/plain` | 直接读取 | ✅ 已支持 |
| **HTML** | `text/html` | 直接读取 | ✅ 已支持 |
| **CSV** | `text/csv` | 直接读取 | ✅ 已支持 |
| **JSON** | `application/json` | 直接读取 | ✅ 已支持 |
| **XML** | `application/xml` | 直接读取 | ✅ 已支持 |
| **PDF** | `application/pdf` | `pdf-extract` crate | ✅ 已支持 |
| **Word (DOCX)** | `application/vnd.openxmlformats-officedocument.wordprocessingml.document` | ZIP → XML 解析 | ✅ 已支持 |
| **Excel (XLSX)** | `application/vnd.openxmlformats-officedocument.spreadsheetml.sheet` | ZIP → XML 解析 | ✅ 已支持 |
| **PowerPoint (PPTX)** | `application/vnd.openxmlformats-officedocument.presentationml.presentation` | ZIP → XML 解析 | ✅ 已支持 |

### 9.2 新增格式支持（如需扩展）

| 格式 | 建议方式 | 优先级 |
|------|----------|--------|
| **EPUB** | `epub` crate | P2 |
| **ODT** | ZIP → XML 解析 | P2 |
| **RTF** | `unrtf` 或第三方库 | P3 |

### 9.3 IngestPipeline 中的格式处理

```rust
// agent/src/ingest_pipeline.rs

pub enum SourceType {
    WebArticle,
    Paper,
    Book,
    RawMarkdown,
    Docx,
    Pdf,
    Xlsx,
    Pptx,
}

impl SourceType {
    pub fn from_mime(mime: &str) -> Option<Self> {
        match mime {
            "application/pdf" => Some(SourceType::Paper),
            "text/markdown" | "text/plain" => Some(SourceType::RawMarkdown),
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => Some(SourceType::Docx),
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => Some(SourceType::Xlsx),
            "application/vnd.openxmlformats-officedocument.presentationml.presentation" => Some(SourceType::Pptx),
            _ => None,
        }
    }
}
```

### 9.4 前端上传组件

```tsx
// components/llm-wiki/UploadPanel.tsx

const SUPPORTED_EXTENSIONS = [
  // Markdown & Text
  { ext: '.md', mime: 'text/markdown', label: 'Markdown' },
  { ext: '.txt', mime: 'text/plain', label: '纯文本' },
  { ext: '.csv', mime: 'text/csv', label: 'CSV' },
  { ext: '.html', mime: 'text/html', label: 'HTML' },
  { ext: '.json', mime: 'application/json', label: 'JSON' },

  // Office
  { ext: '.docx', mime: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document', label: 'Word' },
  { ext: '.xlsx', mime: 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet', label: 'Excel' },
  { ext: '.pptx', mime: 'application/vnd.openxmlformats-officedocument.presentationml.presentation', label: 'PowerPoint' },

  // PDF
  { ext: '.pdf', mime: 'application/pdf', label: 'PDF' },
];

export function UploadPanel() {
  return (
    <div className="upload-panel">
      <FileDropzone
        accept={SUPPORTED_EXTENSIONS.map(e => e.ext).join(',')}
        onDrop={handleFileUpload}
      />
      <div className="supported-formats">
        <span>支持格式：</span>
        {SUPPORTED_EXTENSIONS.map(f => (
          <Tag key={f.ext}>{f.label}</Tag>
        ))}
      </div>
    </div>
  );
}
```

---

## 十一、实现计划

### 9.1 Phase 1：Obsidian 核心（3 人天）

| 任务 | 文件变更 | 工作量 |
|------|----------|--------|
| 数据模型 | 新增 `entity/notes.rs`, `entity/note_links.rs` | 0.5 人天 |
| Repo 层 | 新增 `repo/note.rs` | 0.5 人天 |
| Tauri Commands | 新增 `wiki_notes_*` 命令 | 0.5 人天 |
| 前端页面 | `WikiPage.tsx`, `NoteEditor.tsx` | 1 人天 |
| 双链解析 | `markdown_parser.rs` | 0.5 人天 |

### 9.2 Phase 2：LLM Wiki 核心（4 人天）

| 任务 | 文件变更 | 工作量 |
|------|----------|--------|
| 数据模型 | 新增 `entity/wiki_sources.rs`, `entity/wiki_pages.rs`, `entity/wiki_operations.rs` | 0.5 人天 |
| WikiCompiler | 新增 `agent/src/wiki_compiler.rs` | 1 人天 |
| IngestPipeline | 新增 `agent/src/ingest_pipeline.rs` | 1 人天 |
| QueryEngine | 新增 `agent/src/query_engine.rs` | 0.5 人天 |
| LintChecker | 新增 `agent/src/lint_checker.rs` | 0.5 人天 |
| LLM Wiki Commands | 新增 `llm_wiki_*` 命令 | 0.5 人天 |

### 9.3 Phase 3：LLM Wiki 前端（3 人天）

| 任务 | 文件变更 | 工作量 |
|------|----------|--------|
| 页面组件 | `LlmWikiPage.tsx`, `LlmWikiEditorPage.tsx` | 1 人天 |
| Ingest 界面 | `IngestPanel.tsx`, `IngestPage.tsx` | 0.5 人天 |
| Schema 编辑器 | `SchemaEditor.tsx` | 0.5 人天 |
| Lint 报告 | `LintReport.tsx` | 0.5 人天 |
| 操作日志 | `OperationTimeline.tsx` | 0.5 人天 |

### 9.4 Phase 4：RAG 与同步（2 人天）

| 任务 | 文件变更 | 工作量 |
|------|----------|--------|
| WikiRAG | 扩展 `rag.rs`，新增 `WikiRAG` | 0.5 人天 |
| 知识图谱 | `GraphView.tsx` (D3.js) | 1 人天 |
| 云同步 | 复用现有 S3/WebDAV | 0.5 人天 |

### 9.5 文件变更清单

| 操作 | 文件路径 |
|------|---------|
| **新增** | `src-tauri/crates/core/src/entity/notes.rs` |
| **新增** | `src-tauri/crates/core/src/entity/note_links.rs` |
| **新增** | `src-tauri/crates/core/src/entity/wiki_sources.rs` |
| **新增** | `src-tauri/crates/core/src/entity/wiki_pages.rs` |
| **新增** | `src-tauri/crates/core/src/entity/wiki_operations.rs` |
| **新增** | `src-tauri/crates/core/src/repo/note.rs` |
| **新增** | `src-tauri/crates/core/src/repo/wiki.rs` |
| **新增** | `src-tauri/crates/core/src/markdown_parser.rs` |
| **新增** | `src-tauri/crates/agent/src/wiki_compiler.rs` |
| **新增** | `src-tauri/crates/agent/src/ingest_pipeline.rs` |
| **新增** | `src-tauri/crates/agent/src/query_engine.rs` |
| **新增** | `src-tauri/crates/agent/src/lint_checker.rs` |
| **修改** | `src-tauri/crates/core/src/rag.rs` (新增 WikiRAG) |
| **修改** | `src-tauri/src/commands/mod.rs` (新增命令) |
| **新增** | `src/pages/WikiPage.tsx` |
| **新增** | `src/pages/WikiEditorPage.tsx` |
| **新增** | `src/pages/WikiGraphPage.tsx` |
| **新增** | `src/pages/LlmWikiPage.tsx` |
| **新增** | `src/pages/LlmWikiEditorPage.tsx` |
| **新增** | `src/pages/IngestPage.tsx` |
| **新增** | `src/pages/WikiLogPage.tsx` |
| **新增** | `src/components/wiki/*` |
| **新增** | `src/components/llm-wiki/*` |
| **新增** | `src/stores/feature/wikiStore.ts` |
| **新增** | `src/stores/feature/llmWikiStore.ts` |
| **新增** | `src/types/note.ts` |
| **新增** | `src/types/llmWiki.ts` |
| **新增** | 数据库迁移 `m20260429_000001_add_wiki_tables.rs` |

---

## 十二、测试计划

| 测试类型 | 覆盖范围 |
|----------|----------|
| 单元测试 | `markdown_parser` 双链解析、`lint_checker` 结构检查 |
| 集成测试 | `wiki_compiler` 编译流程、`ingest_pipeline` 摄入流程 |
| E2E 测试 | `wiki.spec.ts`：创建笔记、双链跳转、LLM Wiki 编译 |
| LLM 评估 | 编译质量、Lint 评分准确性 |

---

## 附录

### A. 与 Obsidian 的兼容性

- **vault 格式兼容**：目录结构与 Obsidian 相同
- **双链语法兼容**：`[[笔记名]]`、`[[笔记名|显示文本]]`
- **Frontmatter 兼容**：YAML frontmatter 完全支持
- **社区插件**：暂不支持（未来可扩展）

### B. 参考资料

- [Karpathy LLM Wiki Bootstrap Skill](https://github.com/nanzhipro/Karpathy-llm-wiki-bootstrap-skill)
- [Obsidian 官方文档](https://help.obsidian.md/)
- 本项目现有 RAG 实现：`src-tauri/crates/core/src/rag.rs`
- 本项目 Obsidian 工具：`src-tauri/crates/core/src/builtin_tools.rs`
