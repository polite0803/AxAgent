# AxAgent

[![Release](https://img.shields.io/github/v/release/polite0803/AxAgent?style=flat-square)](https://github.com/polite0803/AxAgent/releases)
[![Build](https://img.shields.io/github/actions/workflow/status/polite0803/AxAgent/release.yml?style=flat-square)](https://github.com/polite0803/AxAgent/actions)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux%20%7C%20Android%20%7C%20iOS-blue?style=flat-square)
![License](https://img.shields.io/badge/license-AGPL--3.0-green?style=flat-square)

**AxAgent** 是一款開源的跨平台 AI 助理桌面用戶端，支援 **Windows / macOS / Linux / Android / iOS** 五大平台。它不只是聊天介面——整合了 ReAct 智慧體引擎、視覺化工作流編排、本機 RAG 知識庫、MCP 協定擴充、多模型統一閘道、瀏覽器自動化、電腦控制等能力，可作為日常開發、研究、知識管理與自動化工作的 AI 工作站。

> **語言版本**: [简体中文](./README.md) | [English](./README-EN.md) | [繁體中文](./README-ZH-TW.md) | [日本語](./README-JA.md) | [한국어](./README-KO.md) | [Français](./README-FR.md) | [Deutsch](./README-DE.md) | [Español](./README-ES.md) | [Русский](./README-RU.md) | [हिन्दी](./README-HI.md) | [العربية](./README-AR.md)

---

## 專案定位

AxAgent 解決了三個核心問題：

1. **多模型統一調度**：在單一介面中同時使用 OpenAI、Anthropic Claude、Google Gemini、Ollama 本機模型及任何 OpenAI 相容 API，支援多 Key 輪換、智慧模型路由、串流對比
2. **AI 能力工具化**：將 AI 從「對話」擴展到「執行」——透過 47+ 內建工具、視覺化工作流、MCP 擴充、瀏覽器自動化與電腦控制，讓 AI 直接操作檔案、執行程式碼、管理 Git、排程任務
3. **本機優先的資料主權**：AI 對話、知識庫、記憶、設定檔均儲存於本機 SQLite 資料庫中，API Key 使用 AES-256-GCM 加密，無需第三方雲端服務即可執行核心功能

---

## 核心能力

### 多模型引擎

- **9 種提供者配接器**：OpenAI (Chat Completions + Responses + Realtime)、Anthropic Claude、Google Gemini、Ollama (含 GGUF 管理)、OpenClaw、Hermes，以及所有 OpenAI 相容 API
- **多 Key 輪換**：為同一提供者設定多個 API Key，依配額自動輪換，避免單一 Key 限流中斷
- **智慧路由**：依任務類型（程式碼審查 / 摘要 / 翻譯 / 通用）自動選擇最合適的模型，支援自訂路由規則
- **提供者健康監控**：即時追蹤各提供者的成功率、延遲與可用狀態，支援分層級自動降級 (ProviderTier)
- **AI 影像生成**：DALL-E 3 與 Flux (Replicate) 多尺寸預設
- **即時語音**：基於 OpenAI Realtime API 的 WebSocket 語音對話，支援打斷與串流轉寫

### 智慧體系統

整個智慧體系統建構在 **ReAct (Reasoning + Acting) 引擎** 之上，包含以下實際實作的子系統：

- **層級規劃器** (`hierarchical_planner`)：將複雜任務分解為帶依賴關係的 Phase → Task 結構化計畫，編譯為 DAG 拓撲執行
- **深度研究** (`deep_research`)：多源搜尋編排，包含搜尋計畫 (`search_planner`)、搜尋執行 (`search_orchestrator`)、內容綜合 (`content_synthesizer`)、引用追蹤 (`citation_tracker`)
- **事實核查** (`fact_checker`)：AI 驅動的事實驗證，包含來源分類器 (`source_classifier`)、來源驗證器 (`source_validator`)、可信度評估 (`credibility_evaluator`)
- **思維樹** (`tree_of_thoughts`)：多路徑推理探索，分支評估與回溯
- **反思器** (`reflector`)：任務執行後的自我評估與改進建議生成
- **自驗證** (`self_verifier`)：推理結果的自動校驗，循環偵測 (`cycle_detector`) 避免無限推理
- **錯誤恢復** (`error_recovery_engine`)：分類錯誤類型 → 選擇恢復策略 → 自動重試或調整計畫，支援指數退避
- **A/B 測試** (`ab_testing`)：不同推理策略的對比評估
- **評估系統** (`evaluator`)：內建基準測試框架，支援資料集、指標、報告生成
- **LoRA 微調** (`fine_tune`)：內建訓練管線，支援 LoRA 配接器管理
- **RL 最佳化器** (`rl_optimizer`)：基於經驗回饋的策略強化學習，包含經驗重播、策略梯度
- **工具推薦** (`tool_recommender`)：基於上下文的工具使用模式分析與推薦

**多智慧體協作**：

- 主從協調架構 (`coordinator`)，子智慧體平行執行，依賴感知排程
- 共享黑板 (`shared_blackboard`) 用於智慧體間資訊交換
- 對抗性辯論模式，Pro/Con 輪次與論點強度評分
- Swarm 叢集模式，多處理程序智慧體叢集支援權限同步與自動重連
- 主動模式 (`proactive_mode`)：智慧體可主動發起建議與操作

**電腦控制**：AI 驅動的滑鼠點擊、鍵盤輸入、螢幕捲動，三級權限（預設 / 接受編輯 / 完全存取），沙箱路徑隔離

**瀏覽器自動化**：透過 CDP 協定控制瀏覽器，支援導覽、截圖、點擊、表單填寫、文字擷取、頁面狀態監控

### 技能系統

- **技能市集**：瀏覽與安裝社群技能
- **AI 輔助建立**：從自然語言提案自動建立技能結構
- **技能演化** (`evolution_engine`)：基於執行回饋自動分析並改進技能
- **語意匹配** (`skill`)：根據對話上下文語意匹配相關技能，自動推薦
- **技能分解** (`skill_decomposition`)：將複雜任務自動分解為原子技能組合
- **生成工具** (`generated_tool`)：AI 生成並註冊新工具
- **沙箱執行** (`sandbox`)：技能在隔離的沙箱環境中安全執行

### 視覺化工作流

基於 ReactFlow 12 的拖放式 DAG 工作流編輯器：

- **17 種節點類型**：觸發器、智慧體、LLM 呼叫、條件分支、平行分叉、迴圈、合併、延遲、工具呼叫、程式碼執行、子工作流、向量檢索、文件解析、驗證、結束、業務規則、Agent 角色
- **Kahn 拓撲排序執行**：自動偵測循環依賴，平行管線排程
- **內建範本**：程式碼審查、Bug 修復、文件生成、測試、重構、探索、效能分析、安全審查、功能開發
- **YAML 序列化**：工作流定義支援 YAML 格式匯入匯出
- **版本管理**：工作流範本版本控制
- **AI 輔助**：AI 輔助工作流設計與節點推薦

### 知識管理

- **多知識庫 RAG**：文件上傳 → 自動解析（PDF/DOCX/XLSX/PPTX/TXT）→ 分塊 → 向量索引
- **混合檢索**：向量相似度（sqlite-vec + candle 本機嵌入）+ BM25 全文檢索（FTS5），混合排序
- **Self-RAG**：自檢索增強生成，檢索結果自動反思與驗證
- **重排序**：Cross-encoder 結果重排序提升精度
- **知識圖譜**：實體擷取 (`EntityExtractor`) → 關係建構 → 視覺化圖譜
- **檔案監聽**：基於 `notify` 的即時檔案變更監聽，自動增量索引
- **LLM Wiki**：AI 輔助的 Wiki 編譯器與驗證器，支援 Wiki 裁剪瀏覽器擴充

### 記憶系統

- **多命名空間記憶**：依專案/主題隔離，支援手動錄入與 AI 自動擷取
- **持久化整合**：Honcho 與 Mem0 閉環記憶
- **使用者畫像** (`user_profile` / `profile`)：自動學習程式碼風格（縮排/命名/註解）、技術堆疊偏好、溝通風格
- **風格遷移** (`style`)：擷取程式碼風格特徵 → 套用到 AI 生成程式碼
- **夢境整合** (`dream`)：背景自動整合記憶碎片與行為模式，生成結構化知識
- **專案記憶** (`project_memory`)：依專案維度的上下文持久化

### API 閘道

內建基於 `axum` 的 HTTP + WebSocket 閘道伺服器：

- **相容端點**：OpenAI `/v1/chat/completions`、Claude Messages API、Gemini API，以及 OpenAI Responses 與 Realtime WebSocket
- **Key 管理**：生成、撤銷、啟用/停用存取金鑰，支援過期時間設定
- **用量追蹤**：依 Key、提供者、日期的請求量與 token 消耗統計，Prometheus 指標匯出
- **速率限制**：基於 `governor` 的令牌桶演算法，可設定的速率限制策略
- **SSL/TLS**：內建自簽署憑證 (`rcgen`)，支援自訂憑證
- **外部連結**：一鍵整合 Claude CLI、OpenCode 等外部工具，自動同步 API Key
- **即時票券**：基於 HMAC 的臨時認證票券，用於 WebSocket 即時連線安全傳遞

### 訊息平台整合

透過 `rt-messaging` crate 實作的訊息平台閘道，支援：

釘釘、飛書、QQ、Slack、微信、WhatsApp、Telegram、Discord

支援 Webhook 訊息接收、指令解析、AI 回覆自動回傳。

### 工具系統

47 個內建工具，所有工具統一透過 `Tool` trait 註冊：

| 類別       | 工具                                                                                                                                                                                                       |
| ---------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 檔案操作   | `file_read`, `file_write`, `file_edit`, `file_system` (列表/搜尋/中繼資料)                                                                                                                                 |
| 程式碼執行 | `bash`, `repl`                                                                                                                                                                                             |
| 搜尋       | `grep`, `glob`                                                                                                                                                                                             |
| 瀏覽器     | `browser` (CDP 控制)                                                                                                                                                                                       |
| 電腦控制   | `computer_use` (滑鼠/鍵盤/截圖)                                                                                                                                                                            |
| Web        | `web_search`, `web_fetch`                                                                                                                                                                                  |
| 知識庫     | `knowledge`, `document` (文件解析)                                                                                                                                                                         |
| Git        | `git` (commit/push/branch/diff)                                                                                                                                                                            |
| 開發工具   | `lsp` (語言伺服器協定), `workspace`                                                                                                                                                                        |
| 任務管理   | `plan`, `task_system`, `todo_write`, `cron`                                                                                                                                                                |
| 訊息推播   | `push_notification`, `messaging`                                                                                                                                                                           |
| 資料庫     | `database`                                                                                                                                                                                                 |
| 儲存       | `storage`                                                                                                                                                                                                  |
| 其他       | `agent`, `agent_memory`, `context`, `export`, `integration`, `media`, `media_delivery`, `migration_tool`, `monitor`, `obsidian`, `ocr`, `personality`, `shared_path`, `system_info`, `testing`, `worktree` |

### MCP 協定

基於 `rmcp` crate 的完整 MCP (Model Context Protocol) 實作：

- **傳輸層**：stdio 子處理程序 + Streamable HTTP + WebSocket
- **OAuth 認證**：支援 MCP 伺服器的 OAuth 授權流程
- **工具發現**：自動發現並註冊 MCP 伺服器暴露的工具
- **MCP 管理器**：伺服器生命週期管理、健康檢查、自動重連

### 外掛系統

OpenClaw 相容的三級外掛架構（內建 / 捆綁 / 外部），支援：

- npm 套件安裝，內建市集 UI 支援搜尋與安裝
- 外掛 manifest 定義、權限宣告、沙箱隔離執行
- 自訂工具註冊、Agent 提供者、Hook 攔截
- 技能安裝器：從外掛套件中安裝技能到技能系統

### 安全防護

- **AES-256-GCM 加密**：API Key 與敏感設定的本機加密儲存 (`crypto` crate)
- **提示詞注入防護**：四級防禦管線 (`prompt-guard`)——模式偵測 → 分隔符轉義 → XML 包裝器 → 信任標籤，整合到會話、提示詞建構、Git、RAG 全鏈路
- **SSRF 防護** (`ssrf_guard`)：URL 安全檢查，阻止對內網位址的請求
- **內容過濾** (`content_filter`)：多類型內容安全過濾
- **速率限制** (`rate_limiter`)：工具呼叫與 API 請求的令牌桶限流
- **斷路器** (`circuit_breaker`)：連續失敗自動斷路，保護系統穩定性
- **存取控制** (`tool_access`)：基於策略的工具存取權限控制
- **沙箱隔離**：智慧體與技能的執行環境隔離

### 開發者體驗

- **分散式追蹤** (`telemetry`)：OpenTelemetry 整合，支援 Span/Trace 視覺化
- **遙測** (`telemetry`)：結構化日誌、執行時指標、效能事件採集
- **重播除錯**：智慧體執行軌跡錄製 (`trajectory_recorder`) 與重播
- **DevTools 面板**：前端內建的 Trace/Span 時間線檢視器
- **基準測試框架**：Criterion benchmarks (tool_exec / llm_call / search)，SWE-bench 與 Terminal-bench 評估

### 桌面與行動端體驗

- **響應式佈局**：CSS 斷點自適應桌面 / 平板 / 手機（600px / 900px）
- **11 種語言**：簡體中文、繁體中文、英語、日語、韓語、法語、德語、西班牙語、俄語、印地語、阿拉伯語
- **主題引擎** (`rt-theme`)：深色/淺色主題，跟隨系統或手動切換，Ant Design 6 深度客製化
- **Monaco 編輯器**：內建程式碼編輯器，支援語法高亮、差異預覽、多語言
- **xterm.js 終端**：內建終端模擬器，支援 WebLinks、Unicode 11、搜尋
- **D2 / Mermaid / ECharts**：架構圖、流程圖、互動圖表渲染
- **會話分享**：一鍵生成分享連結，可設定存取權限
- **系統托盤 + 全域快速鍵 + 開機自啟**：無干擾背景執行
- **自動更新**：自動偵測 GitHub Releases 版本更新
- **代理支援**：HTTP 與 SOCKS5 代理設定
- **雲端工作空間**：S3 與 WebDAV 儲存同步，衝突偵測與雙向同步

### 行動端

- Android APK/AAB (arm64-v8a, armeabi-v7a, x86_64)
- iOS IPA (arm64)
- 行動端專屬適配：安全區適配、底部導覽列、Drawer 導覽

---

## 技術架構

### 技術堆疊

| 層級          | 技術                                     |
| ------------- | ---------------------------------------- |
| 桌面框架      | Tauri 2.11                               |
| 前端框架      | React 19 + TypeScript 6                  |
| UI 函式庫     | Ant Design 6 + TailwindCSS 4             |
| 狀態管理      | Zustand 5                                |
| 路由          | React Router 7                           |
| 程式碼編輯器  | Monaco Editor                            |
| 終端          | xterm.js 6                               |
| 工作流編輯器  | ReactFlow 12                             |
| 圖表          | D2 + Mermaid + Recharts + ECharts        |
| 虛擬捲動      | @tanstack/react-virtual + react-virtuoso |
| 拖曳          | @dnd-kit                                 |
| Markdown 渲染 | markstream-react + stream-markdown       |
| 國際化        | i18next + react-i18next                  |
| 建置工具      | Vite 8                                   |
| 測試          | Vitest + Playwright + cargo-nextest      |
| 格式化        | dprint (TS/JSON/Markdown/TOML) + rustfmt |
| Lint          | ESLint + Oxlint + Clippy + cargo-deny    |

### 後端架構：Harness 依賴注入模式

後端採用 Rust workspace 架構，包含 **32 個 crate**，遵循 **Harness 架構模式**：

```
所有 crate 透過 axagent-harness 定義的 trait 介面解耦，
執行時由 axagent-runtime 裝配與注入依賴。

依賴方向：具體實作 → harness ← 呼叫方
```

**harness** 是架構基石——零業務邏輯、零具體實作，僅包含 trait 定義、純資料 DTO、常數與統一錯誤類型。它被所有其他 crate 依賴，自身不依賴任何其他 axagent-* crate。

```
src-tauri/crates/
├── harness/          # 架構基石 — trait 介面、DTO、統一錯誤類型、DI 契約
│                     #   200+ trait 定義涵蓋: Agent/Provider/Tool/RAG/儲存/
│                     #   MCP/外掛/安全/可觀測性/記憶/學習/瀏覽器/訊息等
│
├── entities/         # SeaORM 實體模型
├── dao/              # 資料存取層（CRUD）
├── migration/        # 資料庫遷移
│
├── crypto/           # AES-256-GCM 加解密與金鑰管理
├── credential/       # 憑證安全儲存（API Key 等）
├── storage/          # 檔案儲存抽象（本機 / S3 / WebDAV），支援 ZIP 讀寫
├── cache/            # 通用快取層（記憶體）
├── disk-cache/       # 磁碟檔案層級快取
├── search/           # 檢索引擎（FTS5 + sqlite-vec + candle 嵌入）
├── document-parser/  # 文件文字擷取（PDF/DOCX/XLSX/PPTX）
├── kit/              # 通用工具集 — 路徑/編碼/雜湊/日期等
│
├── runtime-core/     # 執行時公共類型、設定常數
├── runtime/          # 執行時服務編排 — 裝配全部 30+ crate，是 Harness DI 的執行時容器
│                     #   管理: 會話/終端/Webhook/限流/權限/SSRF/事件匯流排/狀態
├── rt-workflow/      # 工作流引擎 — DAG 編排、節點執行器、YAML 序列化
├── rt-messaging/     # 訊息平台閘道 — 釘釘/飛書/QQ/Slack/微信/WhatsApp/Telegram/Discord
├── rt-webhook/       # 通用 Webhook 伺服器與事件分發
├── rt-dashboard/     # 儀表板外掛框架
├── rt-theme/         # 主題引擎 — 深色/淺色切換邏輯
│
├── agent/            # AI 智慧體核心 — 80+ 模組
│                     #   ReAct引擎/層級規劃/深度研究/事實核查/思維樹/反思/
│                     #   自驗證/錯誤恢復/RL最佳化/LoRA微調/評估/工具推薦/A/B測試/
│                     #   協調器/黑板/視覺管線/Web搜尋/學術搜尋/Wiki編譯等
│
├── orchestrator/     # 智慧體編排 — 多智慧體排程、DAG 分解、動態子圖執行
├── providers/        # 模型提供者配接器 — OpenAI/Anthropic/Gemini/Ollama/
│                     #   OpenClaw/Hermes/影像生成(DALL-E/Flux)/Realtime/Responses
├── tools/            # 工具體系 — Tool trait/註冊表/編排/串流/沙箱/47+內建工具
├── gateway/          # API 閘道 — axum HTTP/WS 伺服器、OAuth、速率限制、Prometheus
├── mcp/              # MCP 協定 — stdio + Streamable HTTP，基於 rmcp
├── trajectory/       # 學習系統 — 記憶/技能演化/使用者畫像/夢境整合
├── plugins/          # 外掛系統 — OpenClaw 相容、npm 套件安裝、市集
├── telemetry/        # 可觀測性 — OpenTelemetry、結構化日誌、執行時指標
├── prompt-guard/     # 提示詞注入防護 — L1-L4 多級偵測管線
├── npm/              # npm 註冊表用戶端
└── schema-gen/       # 資料庫 Schema 生成工具
```

### 前端架構

```
src/
├── pages/            # 22 個頁面
│   ├── ChatPage          # 對話主介面
│   ├── WorkflowPage      # 工作流編輯器
│   ├── GatewayPage       # API 閘道管理
│   ├── KnowledgeHubPage  # 知識庫管理
│   ├── MemoryPage        # 記憶管理
│   ├── SkillsPage        # 技能市集
│   ├── SettingsPage      # 設定面板
│   ├── DashboardPage     # 資料儀表板
│   ├── TerminalPage      # 終端
│   ├── FilesPage         # 檔案管理
│   ├── GatewayLinkPage   # 外部連結管理
│   ├── LinkPage          # 整合連結
│   ├── WikiEditorPage    # Wiki 編輯器
│   ├── WikiEditPage      # Wiki 編輯
│   ├── WikiGraphPage     # Wiki 知識圖譜
│   ├── FineTunePage      # LoRA 微調
│   ├── PersonaPage       # 角色管理
│   ├── QuickBarPage      # 快捷欄
│   ├── IngestPage        # 文件攝入
│   ├── WorkflowMarketplace # 工作流市集
│   ├── DynamicUIManagerPage # 動態 UI 管理
│   └── DynamicPageViewer    # 動態頁面檢視器
│
├── components/       # 24 個模組, 200+ 元件
│   ├── chat/         # 對話介面（訊息流/輸入/附件/工具呼叫/產物/思考塊等）
│   ├── workflow/     # 工作流編輯器（節點/連線/面板/範本/AI輔助）
│   ├── gateway/      # API 閘道管理介面
│   ├── settings/     # 設定面板（40+ 子元件）
│   ├── skill/        # 技能編輯器與渲染器
│   ├── benchmark/    # 基準測試面板
│   ├── decomposition/# 技能分解與工具生成
│   ├── devtools/     # Trace/Span 時間線
│   ├── layout/       # 佈局（標題列/側邊欄/命令面板）
│   └── ...
│
├── stores/           # 62 個 Zustand store
│   ├── domain/       # 核心業務狀態
│   ├── feature/      # 功能模組狀態（44 個）
│   └── devtools/     # 開發者工具狀態
│
├── hooks/            # React Hooks
├── lib/              # 工具函式 + Web Workers
├── types/            # TypeScript 類型定義
├── sdk/              # 外部整合 SDK
└── i18n/             # 11 語言翻譯 (zh-CN/zh-TW/en-US/ja/ko/fr/de/es/ru/hi/ar)
```

---

## 資料目錄

```
~/.axagent/                    # 應用程式設定
├── axagent.db                 # SQLite 主資料庫 (SeaORM)
├── master.key                 # AES-256 主金鑰
├── vector_db/                 # sqlite-vec 向量索引
└── ssl/                       # 自簽署 SSL 憑證

~/Documents/axagent/          # 使用者檔案
├── images/                   # 圖片附件
├── files/                    # 檔案附件
└── backups/                  # 自動備份
```

---

## 快速開始

### 環境要求

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://www.rust-lang.org/) 1.75+, edition 2024
- [npm](https://www.npmjs.com/) 10+
- Windows: [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) (MSVC + Windows SDK)
- macOS: Xcode Command Line Tools
- Linux: `build-essential` + `libwebkit2gtk-4.1-dev` + `libssl-dev`

### 建置

```bash
git clone https://github.com/polite0803/AxAgent.git
cd AxAgent
npm install
npm run tauri dev      # 開發模式
npm run tauri build    # 生產建置
```

建置產物位於 `src-tauri/target/release/`。

### 測試

```bash
npm run test           # 前端單元測試 (Vitest watch)
npm run test:run       # 前端單元測試 (單次)
npm run test:e2e       # E2E 測試 (Playwright)

# Rust 後端測試
cd src-tauri && cargo nextest run
cd src-tauri && cargo test

# 類型檢查 & Lint
npm run typecheck
cd src-tauri && cargo clippy -- -D warnings
npm run format

# CI 全量檢查
npm run ci:check
```

---

## 平台支援

| 平台    | 架構                                    |
| ------- | --------------------------------------- |
| Windows | x86_64, ARM64                           |
| macOS   | Apple Silicon (arm64), Intel (x86_64)   |
| Linux   | x86_64, ARM64                           |
| Android | arm64-v8a, armeabi-v7a, x86_64 (模擬器) |
| iOS     | arm64                                   |

---

## 開源協定

本專案基於 [AGPL-3.0-only](LICENSE) 協定開源。

---

## 致謝

AxAgent 建構在眾多優秀開源專案之上，包含但不限於：

- [Tauri](https://tauri.app/) — 跨平台桌面框架
- [React](https://react.dev/) + [Ant Design](https://ant.design/) — 前端 UI
- [SeaORM](https://www.sea-ql.org/SeaORM/) — Rust ORM
- [sqlite-vec](https://github.com/asg017/sqlite-vec) — 向量檢索
- [candle](https://github.com/huggingface/candle) — 本機嵌入推理
- [rmcp](https://github.com/nicholasxjy/rmcp) — Rust MCP SDK
- [ReactFlow](https://reactflow.dev/) — 視覺化工作流編輯器
- [axum](https://github.com/tokio-rs/axum) — HTTP 框架
- [Monaco Editor](https://microsoft.github.io/monaco-editor/) — 程式碼編輯器
- [xterm.js](https://xtermjs.org/) — 終端模擬器
