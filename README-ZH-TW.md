# AxAgent

[![Release](https://img.shields.io/github/v/release/polite0803/AxAgent?style=flat-square)](https://github.com/polite0803/AxAgent/releases)
[![Build](https://img.shields.io/github/actions/workflow/status/polite0803/AxAgent/release.yml?style=flat-square)](https://github.com/polite0803/AxAgent/actions)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux%20%7C%20Android%20%7C%20iOS-blue?style=flat-square)
![License](https://img.shields.io/badge/license-AGPL--3.0-green?style=flat-square)

<p align="center">
  <a href="./media/poster-axagent.svg">
    <img src="./media/poster-axagent.svg" alt="AxAgent 宣傳海報" width="80%" />
  </a>
</p>

**AxAgent** 是一款基於 Tauri 2 的跨平台 AI 助手桌面用戶端（Windows / macOS / Linux / Android / iOS）。它整合了 ReAct 智慧體引擎、視覺化工作流編排、本地 RAG 知識庫、MCP 協定擴充、多模型統一閘道、瀏覽器自動化與電腦控制等能力，定位為 AI 驅動的日常開發、研究、知識管理和自動化工作台。

> **語言版本**: [简体中文](./README.md) | [English](./README-EN.md) | [繁體中文](./README-ZH-TW.md) | [日本語](./README-JA.md) | [한국어](./README-KO.md) | [Français](./README-FR.md) | [Deutsch](./README-DE.md) | [Español](./README-ES.md) | [Русский](./README-RU.md) | [हिन्दी](./README-HI.md) | [العربية](./README-AR.md)

---

## 專案定位

AxAgent 解決三個核心問題：

1. **多模型統一接入與智慧調度** — 單一介面同時使用 OpenAI、Anthropic Claude、Google Gemini、Ollama 本地模型及任意 OpenAI 相容 API，支援多 Key 配額自動輪換、按任務類型智慧路由、流式對比
2. **AI 從對話到執行的閉環** — 47+ 內建工具 + 視覺化工作流 + MCP 擴充 + 瀏覽器/電腦控制，AI 可操作檔案、執行程式碼、管理 Git、排程任務
3. **本地優先的資料主權** — 對話記錄、知識庫、記憶、設定均儲存於本地 SQLite 資料庫，API Key 使用 AES-256-GCM 加密，無需第三方雲端服務即可執行核心功能

---

## 核心能力

### 多模型引擎

- **9 種提供商配接器**: OpenAI (Chat Completions + Responses + Realtime)、Anthropic Claude、Google Gemini、Ollama (含 GGUF 本地模型管理)、OpenClaw、Hermes，以及所有 OpenAI 相容 API
- **多 Key 輪換**: 同一提供商多 API Key，按配額自動輪換，單 Key 限流自動切換
- **智慧路由**: 按任務類型（程式碼審查 / 摘要 / 翻譯 / 通用）自動選擇最優模型，支援自訂規則
- **提供商健康監控**: 即時追蹤成功率、延遲、可用狀態，支援分級自動降級
- **AI 圖像生成**: DALL-E 3 和 Flux (Replicate) 多尺寸預設
- **即時語音**: 基於 OpenAI Realtime API 的 WebSocket 語音對話，支援打斷和流式轉寫

### 智慧體系統 (ReAct 引擎)

- **層級規劃器** (`hierarchical_planner`): 複雜任務分解為 Phase → Task 結構化計畫，編譯為 DAG 拓撲執行
- **深度研究** (`deep_research`): 多源搜尋編排，含搜尋計畫、搜尋執行、內容綜合、引用追蹤
- **事實核查** (`fact_checker`): AI 驅動事實驗證，含來源分類器、可信度評估
- **思維樹** (`tree_of_thoughts`): 多路徑推理探索，分支評估與回溯
- **反思器** (`reflector`): 任務執行後自我評估與改進建議
- **自驗證** (`self_verifier`): 推理結果自動校驗，含循環檢測
- **錯誤恢復** (`error_recovery_engine`): 錯誤類型分類 → 恢復策略選擇 → 自動重試或計畫調整，支援指數退避
- **A/B 測試** (`ab_testing`): 不同推理策略的對比評估
- **評估系統** (`evaluator`): 內建基準測試框架
- **LoRA 微調** (`fine_tune`): 內建訓練流水線，支援 LoRA 配接器管理
- **RL 優化器** (`rl_optimizer`): 基於經驗回饋的策略強化學習

**多智慧體協作**:

- 主從協調架構，子智慧體並行執行，依賴感知排程
- 共享黑板用於智慧體間資訊交換
- 對抗性辯論模式（Pro/Con 輪次與論點強度評分）
- Swarm 叢集模式，多行程智慧體叢集
- 主動模式：智慧體可主動發起建議和操作

**電腦控制**: AI 驅動滑鼠點擊、鍵盤輸入、螢幕捲動，三級權限（預設/接受編輯/完全存取），沙箱路徑隔離

**瀏覽器自動化**: 透過 CDP 協定控制瀏覽器，支援導航、截圖、點擊、表單填寫、文字擷取

### 技能系統

- **技能市場**: 瀏覽和安裝社群技能
- **AI 輔助建立**: 從自然語言提案自動建立技能結構 (`skill:create`)
- **技能進化** (`evolution_engine`): 基於執行回饋自動分析和改進技能
- **語義匹配**: 根據對話上下文語義自動推薦相關技能
- **技能分解** (`skill_decomposition`): 將複雜任務自動分解為原子技能組合
- **生成工具**: AI 生成並註冊新工具
- **沙箱執行**: 技能在隔離沙箱中安全執行

### 視覺化工作流

基於 ReactFlow 12 的拖放式 DAG 工作流編輯器：

- **17 種節點類型**: 觸發器、智慧體、LLM 呼叫、條件分支、平行分叉、迴圈、合併、延遲、工具呼叫、程式碼執行、子工作流、向量檢索、文件解析、驗證、結束、業務規則、Agent 角色
- **Kahn 拓撲排序執行**: 自動檢測循環依賴，平行管線排程
- **內建範本**: 程式碼審查、Bug 修復、文件生成、測試、重構、探索、效能分析、安全審查、功能開發
- **YAML 序列化**: 工作流定義匯入匯出
- **版本管理**: 範本版本控制
- **AI 輔助設計**: AI 輔助工作流設計和節點推薦

### 知識管理

- **多知識庫 RAG**: 文件上傳 → 自動解析（PDF/DOCX/XLSX/PPTX/TXT）→ 分塊 → 向量索引
- **混合檢索**: 向量相似度（sqlite-vec + candle 本地嵌入）+ BM25 全文檢索（FTS5），混合排序
- **Self-RAG**: 檢索結果自動反思和驗證
- **重排序**: Cross-encoder 結果重排序
- **知識圖譜**: 實體擷取 → 關係構建 → 視覺化圖譜
- **檔案監聽**: 基於 `notify` 的即時檔案變更監聽，自動增量索引
- **LLM Wiki**: AI 輔助的 Wiki 編譯器與驗證器

### 記憶系統

- **多命名空間記憶**: 按專案/主題隔離，支援手動錄入與 AI 自動擷取
- **持久化整合**: Honcho 和 Mem0 閉環記憶
- **使用者畫像**: 自動學習程式碼風格、技術棧偏好、溝通風格
- **風格遷移**: 擷取程式碼風格特徵 → 應用到 AI 生成程式碼
- **夢境整合**: 後台自動整合記憶碎片與行為模式，生成結構化知識
- **專案記憶**: 按專案維度的上下文持久化

### API 閘道

內建基於 `axum` 的 HTTP + WebSocket 閘道：

- **相容端點**: OpenAI `/v1/chat/completions`、Claude Messages API、Gemini API，以及 OpenAI Responses 和 Realtime WebSocket
- **Key 管理**: 生成、撤銷、啟用/停用存取金鑰，支援過期時間
- **用量追蹤**: 按 Key/提供商/日期的請求量和 token 消耗統計，Prometheus 指標匯出
- **速率限制**: 基於 `governor` 的權杖桶演算法
- **SSL/TLS**: 內建自簽章憑證（`rcgen`），支援自訂憑證
- **外部連結**: 一鍵整合 Claude CLI、OpenCode 等外部工具，自動同步 API Key
- **即時門票**: 基於 HMAC 的臨時認證票據，用於 WebSocket 連線安全傳遞

### 訊息平台整合

透過 `rt-messaging` 實現多平台閘道，支援 **釘釘、飛書、QQ、Slack、微信、WhatsApp、Telegram、Discord** 的訊息接收、命令解析與 AI 自動回覆。

### 工具系統

47+ 內建工具，統一透過 `Tool` trait 註冊：

| 類別       | 工具                                                                                                                                                                                                       |
| ---------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 檔案操作   | `file_read`, `file_write`, `file_edit`, `file_system`                                                                                                                                                      |
| 程式碼執行 | `bash`, `repl`                                                                                                                                                                                             |
| 搜尋       | `grep`, `glob`                                                                                                                                                                                             |
| 瀏覽器     | `browser` (CDP)                                                                                                                                                                                            |
| 電腦控制   | `computer_use` (滑鼠/鍵盤/截圖)                                                                                                                                                                            |
| Web        | `web_search`, `web_fetch`                                                                                                                                                                                  |
| 知識庫     | `knowledge`, `document`                                                                                                                                                                                    |
| Git        | `git` (commit/push/branch/diff)                                                                                                                                                                            |
| 開發工具   | `lsp`, `workspace`                                                                                                                                                                                         |
| 任務管理   | `plan`, `task_system`, `todo_write`, `cron`                                                                                                                                                                |
| 訊息推送   | `push_notification`, `messaging`                                                                                                                                                                           |
| 資料庫     | `database`                                                                                                                                                                                                 |
| 儲存       | `storage`                                                                                                                                                                                                  |
| 其他       | `agent`, `agent_memory`, `context`, `export`, `integration`, `media`, `media_delivery`, `migration_tool`, `monitor`, `obsidian`, `ocr`, `personality`, `shared_path`, `system_info`, `testing`, `worktree` |

### MCP 協定

基於 `rmcp` 的完整 MCP (Model Context Protocol) 實現：

- **傳輸層**: stdio 子行程 + Streamable HTTP + WebSocket
- **OAuth 認證**: 支援 MCP 伺服器的 OAuth 授權流程
- **工具發現**: 自動發現和註冊 MCP 伺服器暴露的工具
- **MCP 管理器**: 伺服器生命週期管理、健康檢查、自動重連

### 外掛系統

OpenClaw 相容的三級外掛架構（內建/捆綁/外部）：

- npm 套件安裝，內建市場 UI 搜尋和安裝
- 外掛 manifest 定義、權限宣告、沙箱隔離執行
- 自訂工具註冊、Agent 提供者、Hook 攔截
- 技能安裝器：從外掛套件安裝技能到技能系統

### 安全防護

- **AES-256-GCM 加密**: API Key 和敏感設定本地加密儲存（`crypto` crate）
- **提示詞注入防護**: 四級防禦管線（`prompt-guard`）—— 模式檢測 → 分隔符轉義 → XML 包裝器 → 信任標籤，整合到對話、提示詞構建、Git、RAG 全鏈路
- **SSRF 防護**: URL 安全檢查，阻止對內網位址的請求
- **內容過濾**: 多類型內容安全過濾
- **速率限制**: 工具呼叫和 API 請求權杖桶限流
- **熔斷器**: 連續失敗自動熔斷
- **存取控制**: 基於策略的工具存取權限控制
- **沙箱隔離**: 智慧體和技能執行環境隔離

### 開發者工具

- **分散式追蹤** (`telemetry`): OpenTelemetry 整合，Span/Trace 視覺化
- **結構化日誌**: tracing-subscriber + chrono 時間戳
- **回放除錯**: 智慧體執行軌跡錄製（`trajectory_recorder`）與回放
- **DevTools 面板**: Trace Explorer 時間線檢視器、Benchmark Runner、Tool Recommender
- **基準測試**: Criterion benchmarks（tool_exec / llm_call / search）
- **CI 檢查**: `npm run ci:check` 整合型別檢查、lint、格式化校驗

### 桌面與行動端體驗

- **響應式佈局**: CSS 斷點自適應桌面/平板/手機（3 級裝置佈局：`desktop` / `tablet` / `mobile`）
- **11 種語言**: 簡體中文、繁體中文、英語、日語、韓語、法語、德語、西班牙語、俄語、印地語、阿拉伯語
- **主題引擎** (`rt-theme`): 深色/淺色主題 + 多個預設（含 21th 等寬字型主題），Ant Design 6 深度定製
- **Monaco 編輯器**: 語法高亮、差異預覽、多語言支援
- **xterm.js 終端**: WebLinks、Unicode 11、搜尋
- **虛擬捲動**: @tanstack/react-virtual + react-virtuoso
- **圖表渲染**: D2 + Mermaid + Recharts
- **Global Copy Menu**: 自訂文字選取複製選單，阻止系統原生右鍵選單
- **Command Palette**: Ctrl+K 全域命令面板
- **系統托盤 + 全域快捷鍵 + 開機自啟**: 無干擾後台執行
- **自動更新**: 可設定間隔的 GitHub Releases 版本檢測
- **代理支援**: HTTP / SOCKS5 代理設定
- **雲端工作區**: S3 和 WebDAV 儲存同步，衝突檢測與雙向同步

### 行動端

- Android APK/AAB（arm64-v8a, armeabi-v7a, x86_64）
- iOS IPA（arm64）
- 行動端專屬配適：安全區配適、底部導航、Drawer 導航

---

## 技術架構

### 技術棧

| 層級          | 技術                                     | 版本 |
| ------------- | ---------------------------------------- | ---- |
| 桌面框架      | Tauri                                    | 2.11 |
| 前端框架      | React                                    | 19   |
| 型別系統      | TypeScript                               | 7    |
| UI 庫         | Ant Design                               | 6    |
| CSS 框架      | TailwindCSS                              | 4    |
| 狀態管理      | Zustand                                  | 5    |
| 路由          | React Router                             | 7    |
| 程式碼編輯器  | Monaco Editor                            | 0.55 |
| 終端          | xterm.js                                 | 6    |
| 工作流編輯器  | ReactFlow                                | 12   |
| 圖表          | D2 + Mermaid + Recharts                  |      |
| 動畫          | Framer Motion                            | 12   |
| 虛擬捲動      | @tanstack/react-virtual + react-virtuoso |      |
| 拖曳          | @dnd-kit                                 | 6    |
| Markdown 渲染 | markstream-react + stream-markdown       |      |
| 國際化        | i18next + react-i18next                  |      |
| 構建工具      | Vite                                     | 8    |
| 測試          | Vitest + Playwright                      |      |
| 格式化        | dprint（TS/JSON/Markdown/TOML）+ rustfmt |      |
| Lint          | ESLint + Oxlint + Clippy                 |      |

### 後端架構: Harness 依賴注入模式

採用 Rust workspace 架構，包含 **32 個 crate**，遵循 **Harness 依賴注入架構**：

> 所有 crate 透過 axagent-harness 定義的 trait 介面解耦，執行時由 axagent-runtime 裝配和注入依賴。
> 依賴方向：`具體實現 → harness ← 呼叫方`

**harness** 是架構基石 — 零業務邏輯、零具體實現，僅含 trait 定義、純資料 DTO、常數和統一錯誤型別。被所有其他 crate 依賴，自身不依賴任何 axagent-* crate（200+ trait 定義，涵蓋 Agent/Provider/Tool/RAG/儲存/MCP/外掛/安全/可觀測性/記憶/學習/瀏覽器/訊息等）。

```
src-tauri/crates/
├── harness/          # 架構基石 — trait 介面、DTO、錯誤型別、DI 契約
├── entities/         # SeaORM 實體模型
├── dao/              # 資料存取層（CRUD）
├── migration/        # 資料庫遷移
├── crypto/           # AES-256-GCM 加解密與金鑰管理
├── credential/       # 憑據安全儲存
├── storage/          # 檔案儲存抽象（本地/S3/WebDAV），ZIP 讀寫
├── cache/            # 記憶體快取層
├── disk-cache/       # 磁碟檔案級快取
├── search/           # 檢索引擎（FTS5 + sqlite-vec + candle 本地嵌入）
├── document-parser/  # 文件文字擷取（PDF/DOCX/XLSX/PPTX）
├── kit/              # 通用工具集（路徑/編碼/雜湊/日期）
├── runtime-core/     # 執行時公共型別、設定常數
├── runtime/          # 執行時服務編排 — 裝配全部 30+ crate 的 DI 容器
├── rt-workflow/      # 工作流引擎 — DAG 編排、節點執行器、YAML 序列化
├── rt-messaging/     # 訊息平台閘道 — 釘釘/飛書/QQ/Slack/微信/WhatsApp/Telegram/Discord
├── rt-webhook/       # 通用 Webhook 伺服器
├── rt-dashboard/     # 儀表板外掛框架
├── rt-theme/         # 主題引擎
├── agent/            # AI 智慧體核心 — 80+ 模組
│                     #   ReAct引擎/層級規劃/深度研究/事實核查/思維樹/反思/
│                     #   自驗證/錯誤恢復/RL優化/LoRA微調/評估/工具推薦/A/B測試/
│                     #   協調器/黑板/視覺管線/Web搜尋/學術搜尋/Wiki編譯等
├── orchestrator/     # 智慧體編排 — 多智慧體排程、DAG 分解、動態子圖執行
├── providers/        # 模型提供商配接器
├── tools/            # 工具體系 — Tool trait/註冊表/編排/流式/沙箱/47+內建工具
├── gateway/          # API 閘道 — axum HTTP/WS 伺服器、OAuth、速率限制、Prometheus
├── mcp/              # MCP 協定 — stdio + Streamable HTTP，基於 rmcp
├── trajectory/       # 學習系統 — 記憶/技能進化/使用者畫像/夢境整合
├── plugins/          # 外掛系統 — OpenClaw 相容、npm 套件安裝、市場
├── telemetry/        # 可觀測性 — OpenTelemetry、結構化日誌、執行時指標
├── prompt-guard/     # 提示詞注入防護 — L1-L4 多級檢測管線
├── npm/              # npm 登錄檔客戶端
└── schema-gen/       # 資料庫 Schema 生成工具
```

### 前端架構

```
src/
├── pages/            # 頁面（含子頁面 23+）
│   ├── ChatPage           # 對話主介面 — 側邊欄/訊息流/Agent 面板/多 Tab
│   ├── DashboardPage      # 資料儀表板 — 用量統計/模型分布/趨勢圖表
│   ├── WorkflowPage       # 工作流編輯器 — ReactFlow DAG 視覺化
│   ├── KnowledgeHubPage   # 知識庫管理 — 文件上傳/索引/檢索
│   ├── MemoryPage         # 記憶管理
│   ├── SkillsPage         # 技能市場
│   ├── SettingsPage       # 設定面板 — 40+ 設定項
│   ├── TerminalPage       # 內建終端 — xterm.js
│   ├── FilesPage          # 檔案管理
│   ├── GatewayLinkPage    # API 閘道與外部連結管理
│   ├── QuickBarPage       # 快捷欄（獨立視窗）
│   ├── DynamicUIManagerPage / DynamicPageViewer  # 動態 UI 引擎
│   ├── WikiGraphPage / WikiEditPage / IngestPage # LLM Wiki
│   ├── LearningGraphPage  # 學習圖譜
│   ├── FineTunePage       # LoRA 微調
│   ├── PersonaPage        # 角色管理
│   ├── WorkflowMarketplace # 工作流市場
│   ├── DevTools/          # TraceExplorer / BenchmarkRunner / ToolRecommender
│   └── Workflow/          # WorkflowListPage
│
├── components/       # 28 個模組，450+ 元件
│   ├── chat/         # 對話（訊息流/輸入/ChatView/TabBar/RightPanel/附件/工具呼叫渲染）
│   ├── layout/       # 佈局 — 17 個元件
│   │                 #   AppInitializer / Sidebar / ContentArea / TitleBar /
│   │                 #   CommandPalette / GlobalCopyMenu / GlobalErrorBoundary /
│   │                 #   GlobalStatusBar / ErrorNotificationToast / AppHeader /
│   │                 #   BackendStatusIndicator / IpcReconnectBanner /
│   │                 #   ModuleErrorBoundary / NotificationBell / UserProfileModal 等
│   ├── agent/        # Agent 面板/入口/迷你面板
│   ├── workflow/     # 工作流編輯器（節點/連線/面板/範本/AI輔助）
│   ├── settings/     # 設定面板（40+ 子元件）
│   ├── skill/        # 技能編輯器/渲染器/浮動面板
│   ├── dynamicUI/    # 動態 UI 元件註冊表（26 個內建元件）
│   ├── gateway/      # API 閘道管理
│   ├── files/        # 檔案管理
│   ├── terminal/     # 終端元件
│   ├── search/       # 搜尋介面
│   ├── benchmark/    # 基準測試面板
│   ├── decomposition/# 技能分解與工具生成
│   ├── devtools/     # Trace/Span 時間線 + RL Training 面板
│   ├── approval/     # 審批流程介面
│   ├── recommendation/ # 工具/模型推薦
│   ├── onboarding/   # WelcomeWizard / InteractiveTutorial
│   ├── help/         # 幫助面板
│   ├── notification/ # 通知元件
│   ├── proactive/    # 主動建議
│   ├── llm-wiki/     # LLM Wiki 元件
│   ├── wiki/         # Wiki 元件
│   ├── fine-tune/    # 微調介面
│   ├── trace/        # Trace 元件
│   ├── style/        # 樣式/主題
│   ├── shared/       # 共享元件（ErrorBoundary / PageContextProvider）
│   └── common/       # 通用元件（Icon 等）
│
├── stores/           # Zustand 狀態管理
│   ├── domain/       # 10 個核心業務 store（對話/流/壓縮/偏好/多模型等）
│   ├── feature/      # 48 個功能模組 store（智慧體/工作流/知識庫/技能/閘道/記憶/終端等）
│   └── devtools/     # 4 個開發者工具 store
│
├── hooks/            # React Hooks（快捷鍵/命令面板/響應式/捲軸/主題/Avatar 等）
├── lib/              # 工具函式庫（invoke/pageRegistry/shortcuts/skillLifecycle/
│                     #   chartGenerator/codeExecutor/tokenEstimator/workflowLayout 等 45+ 模組）
├── types/            # TypeScript 型別定義
├── theme/            # Shadcn 主題引擎
├── i18n/             # 11 語言翻譯檔案（zh-CN/zh-TW/en-US/ja/ko/fr/de/es/ru/hi/ar）
├── constants/        # 常數與功能開關
└── sdk/              # 外部整合 SDK
```

### 功能開關

專案透過 `featureFlags.ts` 管理漸進式功能發布：

| 開關                | 狀態 | 說明                              |
| ------------------- | ---- | --------------------------------- |
| `AGENT_IN_THE_LOOP` | ✅   | 全域 Agent Panel + 頁面上下文注入 |
| `DYNAMIC_UI`        | ✅   | 動態 UI 構建引擎                  |
| `SELF_EVOLUTION_UI` | ❌   | 自我進化前端控制面                |
| `NL_EXTENSION`      | ❌   | 自然語言驅動動態業務擴充          |

### Tauri 外掛

| 外掛                | 用途                       |
| ------------------- | -------------------------- |
| `autostart`         | 開機自啟                   |
| `clipboard-manager` | 剪貼簿讀寫                 |
| `dialog`            | 檔案選擇對話框             |
| `fs`                | 檔案系統存取               |
| `global-shortcut`   | 全域快捷鍵註冊             |
| `notification`      | 系統通知                   |
| `opener`            | 外部連結/檔案開啟          |
| `process`           | 行程管理                   |
| `updater`           | 自動更新                   |
| `mcp-bridge`        | MCP 協定橋接（非 Android） |

---

## 資料目錄

```
~/.axagent/                    # 應用設定
├── axagent.db                 # SQLite 主資料庫 (SeaORM)
├── master.key                 # AES-256 主金鑰
├── vector_db/                 # sqlite-vec 向量索引
└── ssl/                       # 自簽章 SSL 憑證

~/Documents/axagent/          # 使用者檔案
├── images/                   # 圖片附件
├── files/                    # 檔案附件
└── backups/                  # 自動備份
```

---

## 快速開始

### 環境要求

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://www.rust-lang.org/) 1.75+（edition 2024）
- [npm](https://www.npmjs.com/) 10+
- Windows: [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)（MSVC + Windows SDK）
- macOS: Xcode Command Line Tools
- Linux: `build-essential` + `libwebkit2gtk-4.1-dev` + `libssl-dev`

### 開發

```bash
git clone https://github.com/polite0803/AxAgent.git
cd AxAgent
npm install
npm run tauri dev      # 開發模式（前端 Vite HMR + Tauri 視窗）
```

### 構建

```bash
npm run tauri build    # 桌面端生產構建

npm run tauri:android:build   # Android 構建
npm run tauri:ios:build       # iOS 構建
```

桌面端構建產物位於 `src-tauri/target/release/`。

### 測試

```bash
npm run test           # 前端單元測試（Vitest watch）
npm run test:run       # 前端單元測試（單次執行）
npm run test:e2e       # E2E 測試（Playwright）

# Rust 後端測試
cd src-tauri && cargo nextest run
cd src-tauri && cargo test

# 型別檢查 & Lint
npm run typecheck
cd src-tauri && cargo clippy -- -D warnings
npm run format         # dprint 格式化
npm run lint:eslint    # ESLint 檢查
npm run contracts      # API 契約檢查

# CI 全量檢查
npm run ci:check
```

### 常用指令碼

| 命令                     | 用途                 |
| ------------------------ | -------------------- |
| `npm run bump`           | 版本號升級（互動式） |
| `npm run docs`           | 生成 TypeDoc 文件    |
| `npm run skill:create`   | 建立新技能腳手架     |
| `npm run skill:validate` | 驗證技能定義         |
| `npm run check:types`    | 型別一致性檢查       |

---

## 平台支援

| 平台    | 架構                                  |
| ------- | ------------------------------------- |
| Windows | x86_64, ARM64                         |
| macOS   | Apple Silicon (arm64), Intel (x86_64) |
| Linux   | x86_64, ARM64                         |
| Android | arm64-v8a, armeabi-v7a, x86_64        |
| iOS     | arm64                                 |

---

## 開源協議

本專案基於 [AGPL-3.0-only](LICENSE) 協議開源。

---

## 致謝

AxAgent 構建在眾多優秀開源專案之上：

- [Tauri](https://tauri.app/) — 跨平台桌面框架
- [React](https://react.dev/) + [Ant Design](https://ant.design/) — 前端 UI
- [SeaORM](https://www.sea-ql.org/SeaORM/) — Rust ORM
- [sqlite-vec](https://github.com/asg017/sqlite-vec) — 向量檢索
- [candle](https://github.com/huggingface/candle) — 本地嵌入推理
- [rmcp](https://github.com/nicholasxjy/rmcp) — Rust MCP SDK
- [ReactFlow](https://reactflow.dev/) — 視覺化工作流編輯器
- [axum](https://github.com/tokio-rs/axum) — HTTP 框架
- [Monaco Editor](https://microsoft.github.io/monaco-editor/) — 程式碼編輯器
- [xterm.js](https://xtermjs.org/) — 終端模擬器
- [Zustand](https://zustand.docs.pmnd.rs/) — 狀態管理
- [Framer Motion](https://www.framer.com/motion/) — 動畫庫
- [Recharts](https://recharts.org/) — 圖表庫
