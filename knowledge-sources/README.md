# 知识源接入 PoC（开源股票知识库 → AxInvest Wiki 冷启动）

> 用途：把**开源投资知识库（实体-关系图谱）**作为 AxInvest 的 Wiki 冷启动种子，
> 直接激活 `graph_insights` 的社区发现 / 桥节点识别 / 意外关联，对应知识层价值定位 #3。

## 链路闭环

```
知识库 CSV (nodes.csv + edges.csv)
        │  scripts/kg_to_linkgraph.py
        ▼
  link_graph.json                 ← 对齐 harness::graph_dtos::GraphData（可直喂 LinkGraph）
  wiki_pages/<id>.md              ← 每实体一页，frontmatter + [[wikilink]] 互链（可走 RawMarkdown 直接 ingest 建图，无需 LLM）
        │
        ├─► LinkGraph::from_graph_data → dao::repo::louvain::detect_communities → agent::graph_insights::GraphInsightAnalyzer::analyze()
        │       → 产出 communities / bridge_nodes / surprising_connections / knowledge_gaps / isolated_pages
        │
        └─► ingest_pipeline（IngestSourceType::RawMarkdown）→ wiki_compiler → Wiki 节点/边持久化
```

## 验证（端到端测试）

```bash
cd src-tauri && cargo test -p axagent-runtime --test kg_graph_insights
```

测试落在 `runtime` crate（wiring 层），因为 `GraphInsightAnalyzer`(agent/consumer)
需要外部传入 `LouvainResult`，而 `detect_communities` 在 `dao`(implementor)——
按 harness 分层，组装这步只能在 runtime，不能在 agent 测试里引 dao。

## 关键实现发现

- `agent/src/graph_insights.rs` 的 `find_bridge_nodes` 门槛是 **连接 ≥3 个不同社区**
  （`community_count >= 3`，且排除节点自身所在社区），不是"连接 2 个社区"。
  真实知识库里连接 ≥3 社区的枢纽很少，做图洞察调参 / 构造样例时务必注意。
- 样例图若含 `related_sector` 这类"跨子群"边（如 `银行业→保险业`），Louvain 会把
  两个金融子群合并成单一社区，导致没有任何节点能连满 3 社区 → 桥节点为空。
  PoC 样例已删此类边，并让「中国平安」显式连接银行/保险/半导体/人物 4 个社区，
  保证其必然成为桥节点。

## 用真实知识库接入

1. 把任意知识库的节点/关系导出为 `knowledge-sources/<name>/{nodes.csv,edges.csv}`：
   - `nodes.csv`: `id,title,type,tags`（type ∈ company/industry/concept/person/...；tags 用 `|` 分隔）
   - `edges.csv`: `source,target,type`
2. 跑 `python scripts/kg_to_linkgraph.py knowledge-sources/<name>` 生成 `link_graph.json` + `wiki_pages/`
3. 复用 PoC 测试验证图洞察产出；或直接把 `wiki_pages/` 喂 `ingest_pipeline` 建 Wiki

## 已接入的 A 股知识库

### lemonhu/stock-knowledge-graph（✅ 已就绪）

**A 股专属结构化知识图谱**，数据源来自**同花顺（个股页 + Tushare 行业/概念分类）**。

| 指标              | 数值              |
| ----------------- | ----------------- |
| 股票 node         | 3,188             |
| 行业 node         | 49（同花顺分类）  |
| 概念 node         | 163（同花顺概念） |
| 人物 node（高管） | 20,872            |
| stock→行业 边     | 2,908             |
| stock→概念 边     | 9,442             |
| 人物→stock 边     | 24,775            |
| 总节点            | 24,272            |
| 总边              | 37,125            |

**已生成的产物：**

- `knowledge-sources/lemonhu/raw/` — 原始 Neo4j 格式 CSV（7 文件，GitHub raw 直拉）
- `knowledge-sources/lemonhu/nodes.csv` + `edges.csv` — 标准格式（经 `scripts/lemonhu_to_standard.py` 转换）
- `knowledge-sources/lemonhu/link_graph.json` — 对齐 harness GraphData 的图谱（11.5MB，24k 节点 + 37k 边）
- `knowledge-sources/lemonhu/wiki_pages/` — 24,271 个 A 股 Wiki 页面（Markdown + [[wikilink]] 互链）

**生成的 Rust 种子代码：**

- `concept_index.rs::seed_ashare_ontology()` — 注册 49 行业 + 163 概念的**本体对齐层**（MD5 hash 为 ID，中文名 + hash 为别名）
- 编译验证通过（`cargo check -p axagent-stock-analysis`）

**Wiki 冷启动链路：** `link_graph.json` → `LinkGraph::from_graph_data` → Louvain 社区发现 → `GraphInsightAnalyzer::analyze()`
或 `wiki_pages/` → `ingest_pipeline(RawMarkdown)` → Wiki 持久化

### 其他候选源

| 库                               | 备注                                              |
| -------------------------------- | ------------------------------------------------- |
| `shinezai/QASystemOnFinancialKG` | 概念 1121/行业 66/高管/指数，CSV 就绪，可后续接入 |
| `FinReflectKG`                   | 英文/国际，CC-BY-NC-4.0 非商业，暂排除            |

> 授权提醒：代码 MIT，底层数据来自 Tushare/同花顺，商用需审数据源授权。

## #1 选股主题维度升级（已 PoC 验证）

> 让 `screener` 从纯行情指标升级为「**概念 / 行业 / 产业链主题 + 量化指标**」组合筛选。
> 核心思路：**先按主题收窄候选宇宙，再叠加量化打分**。

### 新增模块

- `crates/stock-analysis/src/concept_index.rs`
  - `ConceptIndex`：概念/行业 → 成员股票 的倒排索引；
    `from_graph_edges` / `from_edge_csv` 从知识图谱边构建；
    `theme_universe(queries, require_all)` 计算 OR / AND 主题宇宙
  - `ConceptNode` + `seed_sample_ontology`：**本体对齐层**——
    把用户查询词 / 同花顺 / 东方财富 / 问财 命名统一映射到规范概念 id
    （如 `AI` / `人工智能` / `ai概念` → `concept_ai`）
- `crates/stock-analysis/src/screener.rs`
  - `ScreenCriteria` 新增 `concepts` / `industries` / `industry_chains` / `require_all_themes` / `max_pe` / `max_pb`
  - `ScreenResult` 新增 `matched_themes: Option<Vec<String>>`
    （命中的概念/行业显示名，前端可展示「为什么入选」）
  - 纯函数 `StockScreener::screen_snapshots(snapshots, criteria, index)`：
    网络无关、便于单测；先主题收窄再量化打分
  - 网络版 `StockScreener::screen_by_theme(client, universe, criteria, index)`：
    拉行情拼快照后调纯函数，可直接接 live 数据

### 验证

```bash
cd src-tauri && cargo test -p axagent-stock-analysis screener::tests concept_index::tests
```

覆盖：AI 概念 + 低 PE(≤20) 应得 002415/601318 且带 `matched_themes`；纯量化退化（无主题时不受收窄）；行业筛选用成员关系。

### 真实接入要点（本体对齐）

- **成员数据**：生产环境 `ConceptIndex` 的成员应由 `astock-data`（vendor 概念板块）填充，
  知识图谱仅作种子 / 补全（实时性）。
- **本体种子**：`seed_ashare_ontology` 已注册 49 行业 + 163 概念（来自 lemonhu/同花顺），
  别名包含中文名 + MD5 hash。新增 vendor 命名时在 `with_aliases` 追加即可。

## #4 本地 RAG 投研问答（走标准 knowledge_base 路径）

> 把**开源金融语料（OmniEval-KnowledgeCorpus 等）**灌入 AxInvest 已有的知识库体系，
> 由对话页「启用知识库」开关 / 工作流 `KnowledgeRetrievalNode` 直接复用，
> 提供「可溯源、复用应用既有向量模型」的投研问答 / 概念语义检索，对应价值定位 #4。

### 边界澄清（重要）

- `iwencai` 在 AxInvest 里是 **`astock-data` 的一个 `StockVendor`**（同花顺问财），

> 提供的是**概念板块 / 一致预期 / 选股搜索**这类语义数据（`astock-data/src/vendors/iwencai.rs`），
> **不是聊天问答 LLM**。

- 所以本地 RAG 的准确边界是：**补充 iwencai 的概念语义**（与 #1 选股主题维度呼应）

> - **提供投研问答**（用户问"什么是半导体概念"之类，基于本地知识库作答）。
>   它**不替代** iwencai 的实时行情 / 选股搜索能力。

### 项目已有完整 RAG 链路（2026-07-13 探明并纠正）

- 数据层（`crates/search` + `src/indexing`）：`knowledge_bases` / `knowledge_documents` / `chunks`(向量) / `vec_collections` / `retrieval_hits`(溯源) 表齐全
- `indexing::index_knowledge_base`：文档 → `rag::prepare_chunks` → `generate_embeddings(provider)` → `rag::index` 落向量库
- `indexing::search_knowledge`：向量检索；`axagent_search::rag::KnowledgeContainer::from_knowledge_base` 从 DB 加载
- **两个现成入口**（无需新写）：
  - 对话页：`streaming.rs` 每次对话 `collect_rag_context` 注入 LLM，用户经 `enabled_knowledge_base_ids` 指定
  - 工作流：`KnowledgeRetrievalNode.tsx` + `VectorRetrievePropertyPanel.tsx` 直接检索
- 白嫖能力：溯源 `retrieval_hits` + citation、`prompt-guard` 的 `RagKnowledgeBase` 信任标签、token budget、`<retrieved-context>` 注入

### 检索后端（复用应用既有 embedding 模型，无需单独配置）

- 金融知识库的向量化**完全复用应用已有的 provider 配置**：建 KB 时在 `EmbeddingModelSelect` 下拉里挑一个你已开启的 Embedding 类型模型（来自 `useProviderStore`），存为 `providerId::model_id` 进 `knowledge_bases.embedding_provider`
- 灌库（`indexing.rs:457` 读 KB 配置）与检索（`knowledge.rs:178` 读同一字段）都自动用它，**无任何额外配置步骤**
- 唯一前提：应用里至少已有一个开启 Embedding 类型的模型——这是用任何知识库/Wiki/Memory 的共性前提，非金融语料特有
- 质量建议（非依赖）：中文金融语料优先挑中文能力强的 embedding（如 bge-m3）
- **不再维护独立内存检索内核**：`knowledge_retrieval.rs` 已收敛删除，分块/检索全走 `axagent_search::rag` + `text_chunker`

### 语料 → 知识库文档（ingest 脚本）

- 已删除 OmniEval（英文通用金融 QA 语料，非 A 股专用）。A 股知识直接从 lemonhu 知识图谱获取。
- 后续如需本地 RAG 语料，可：从 lemonhu wiki_pages 直接导入知识库；或接 `shinezai/QASystemOnFinancialKG` 的金融 QA/概念语料。

## 价值定位（映射到股票业务）

- **#3 Wiki 冷启动 + 图洞察激活**（本 PoC 已验证）：喂数据即生效，`ingest`/`wiki`/`graph_insights` 已就绪
- **#1 选股主题维度**（本 PoC 已验证）：`screener` 升级为「主题 + 量化」组合，先主题收窄再打分；本体对齐层已落地模板
- **#4 本地 RAG 投研问答**（走标准 knowledge_base 路径）：金融语料灌入知识库，对话页/工作流节点直接复用，补充 iwencai 概念语义 + 提供投研问答（不替代实时行情）
- **#2 可解释归因**：用知识源做买卖信号的自然语言解释 + 引用溯源（可借 `matched_themes` 起步）

> 知识库补的是**语义/领域知识层**，替代不了 `astock-data`(数据) 与 `stock-analysis`/`quant`(量化)。
> 它让决策"有依据、可追溯、可关联"。
