# AxAgent

[![Release](https://img.shields.io/github/v/release/polite0803/AxAgent?style=flat-square)](https://github.com/polite0803/AxAgent/releases)
[![Build](https://img.shields.io/github/actions/workflow/status/polite0803/AxAgent/release.yml?style=flat-square)](https://github.com/polite0803/AxAgent/actions)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux%20%7C%20Android%20%7C%20iOS-blue?style=flat-square)
![License](https://img.shields.io/badge/license-AGPL--3.0-green?style=flat-square)

<p align="center">
  <a href="./media/poster-axagent.svg">
    <img src="./media/poster-axagent.svg" alt="AxAgent Poster" width="80%" />
  </a>
</p>

**AxAgent** 是一款基於 Tauri 2 的跨平台 AI 桌面用戶端（Windows / macOS / Linux / Android / iOS），定位為 AI 驅動的日常開發、研究、知識管理與自動化工作檯。它內建 ReAct 智慧體引擎、認知路由（三級分層路由 + 檢索增強路由 RAR）、視覺化工作流程編排、本機 RAG 知識庫、MCP 協定擴充、多模型統一閘道、瀏覽器自動化與電腦控制等能力，讓 AI 從「對話」走向「執行」。

> **語言版本**: [简体中文](./README.md) | [English](./README-EN.md) | [繁體中文](./README-ZH-TW.md) | [日本語](./README-JA.md) | [한국어](./README-KO.md) | [Français](./README-FR.md) | [Deutsch](./README-DE.md) | [Español](./README-ES.md) | [Русский](./README-RU.md) | [हिन्दी](./README-HI.md) | [العربية](./README-AR.md)

---

## 專案定位

AxAgent 解決三個核心問題：

1. **多模型統一接入與智慧排程** — 單一介面同時使用 OpenAI、Anthropic Claude、Google Gemini、DeepSeek、Qwen、GLM、Kimi、文心、Ollama 本機模型及任意 OpenAI 相容 API，支援多 Key 配額自動輪換、依任務類型智慧路由、串流對比
2. **AI 從對話到執行的閉環** — 163+ 內建工具 + 視覺化工作流程 + MCP 擴充 + 瀏覽器/電腦控制，AI 可操作檔案、執行程式碼、管理 Git、排程任務
3. **本機優先的資料主權** — 對話記錄、知識庫、記憶、設定均儲存於本機 SQLite 資料庫，API Key 使用 AES-256-GCM 加密，無需第三方雲端服務即可執行核心功能

---

## 核心能力

### 認知路由系統（Cognitive Router）

AxAgent 以 `cognitive_query` 作為所有對話的統一入口，透過**三級分層路由**將使用者意圖映射到具體能力：

- **L1 領域路由** (`domain_router`): 規則 + LLM 兜底，識別 9 大業務領域（資料分析 / 內容創作 / 溝通 / 維運 / AI 媒體 / 金融 / 自動化 / 通用等）
- **L2 叢集路由** (`cluster_router`): 領域內定位能力叢集（27 個叢集，覆蓋 8 大業務領域）
- **L3 能力路由**: **檢索增強路由（RAR）** — 從能力向量庫召回 Top-K 相似工作流程注入 Prompt，結合工作流程 DAG 圖尋徑，輸出路徑位址（如 `/finance/stock_analysis/tech`）與執行模式
- **執行模式**: `Ask / Plan / Act / Workflow / Direct / Delegate / ParameterExtract / Clarify`，按信心度自動選擇
- **能力系統**: 統一註冊表（`CapabilityRegistry`）+ 向量索引（`CapabilityIndexer`）+ 混合檢索（`CapabilityRetriever`，向量 + BM25 + 標籤硬匹配 + 負樣本排除）
- **系統能力隔離**: 認知編排器與業務工作流程實體隔離，系統能力帶 `SYSTEM_ONLY` 可見性標記，路由層內建自參照熔斷，防止自我指涉悖論
- **三級路由以工作流程 DAG 實現**: 4 個預設路由工作流程範本（主編排 ~20 節點 + L1/L2/L3 子路由），由 `rt-workflow` 引擎執行

### 多模型引擎

- **13 種提供商配接器**: OpenAI（Chat Completions + Responses + Realtime）、Anthropic Claude、Google Gemini、DeepSeek、Qwen、GLM、Kimi、文心一言、Ollama、Llama.cpp（GGUF 本機模型）、OpenClaw、Hermes，以及所有 OpenAI 相容 API
- **多 Key 輪換**: 同一提供商多 API Key，按配額自動輪換，單 Key 限流自動切換
- **智慧路由**: 依任務類型（程式碼審查 / 摘要 / 翻譯 / 通用）自動選擇最佳模型，支援自訂規則
- **提供商健康監控**: 即時追蹤成功率、延遲、可用狀態，支援分級自動降級
- **AI 影像生成**: DALL-E 3 和 Flux 多尺寸預設
- **即時語音**: 基於 OpenAI Realtime API 的 WebSocket 語音對話，支援打斷和串流轉寫

### 智慧體系統（ReAct 引擎）

- **層級規劃器** (`hierarchical_planner`): 複雜任務分解為 Phase → Task 結構化計劃，編譯為 DAG 拓撲執行
- **深度研究** (`deep_research`): 多源搜尋編排，含搜尋計劃、搜尋執行、內容綜合、引用追蹤
- **事實核查** (`fact_checker`): AI 驅動事實驗證，含來源分類器、可信度評估
- **思維樹** (`tree_of_thoughts`): 多路徑推理探索，分支評估與回溯
- **反思器** (`reflector`): 任務執行後自我評估與改進建議
- **自驗證** (`self_verifier`): 推理結果自動校驗，含循環檢測
- **錯誤復原** (`error_recovery_engine`): 錯誤類型分類 → 復原策略選擇 → 自動重試或計劃調整，支援指數退避
- **A/B 測試** (`ab_testing`): 不同推理策略的對比評估
- **評估系統** (`evaluator`): 內建基準測試框架
- **LoRA 微調** (`fine_tune`): 內建訓練管線，支援 LoRA 配接器管理
- **RL 最佳化器** (`rl_optimizer`): 基於經驗回饋的策略強化學習

**多智慧體協作**:

- 主從協調架構，子智慧體並行執行，依賴感知排程
- 共享黑板用於智慧體間資訊交換
- 對抗性辯論模式（Pro/Con 輪次與論點強度評分）
- Swarm 叢集模式，多程序智慧體叢集
- 主動模式：智慧體可主動發起建議和操作

**電腦控制**: AI 驅動滑鼠點擊、鍵盤輸入、螢幕滾動，三級權限（預設/接受編輯/完全存取），沙箱路徑隔離

**瀏覽器自動化**: 透過 CDP 協定控制瀏覽器，支援導航、截圖、點擊、表單填寫、文字提取

### 技能系統

- **技能市集**: 瀏覽和安裝社群技能
- **AI 輔助建立**: 從自然語言提案自動建立技能結構 (`skill:create`)
- **技能演化** (`evolution_engine`): 基於執行回饋自動分析和改進技能
- **語意匹配**: 根據對話上下文語意自動推薦相關技能
- **技能分解** (`skill_decomposition`): 將複雜任務自動分解為原子技能組合
- **生成工具**: AI 生成並註冊新工具
- **沙箱執行**: 技能在隔離沙箱中安全執行

### 視覺化工作流程

基於 ReactFlow 12 的拖放式 DAG 工作流程編輯器：

- **32 種節點類型**: 觸發器、智慧體、LLM 呼叫、條件分支、並行分叉、迴圈、合併、延遲、工具呼叫、程式碼執行、子工作流程、向量檢索、文件解析、驗證、結束、HTTP 請求、Switch、資料庫查詢、通知、審批、檔案操作、資料轉換、Webhook 傳送、紀錄、LLM 分類器、聚合器、郵件、辯論、Swarm、多智慧體、儲存、業務規則
- **Kahn 拓撲排序執行**: 自動檢測迴圈依賴，並行管線排程
- **內建範本**: 程式碼審查、Bug 修復、文件生成、測試、重構、探索、效能分析、安全審查、功能開發
- **YAML 序列化**: 工作流程定義匯入匯出
- **版本管理**: 範本版本控制
- **AI 輔助設計**: AI 輔助工作流程設計、節點推薦與診斷

### 知識管理

- **多知識庫 RAG**: 文件上傳 → 自動解析（PDF/DOCX/XLSX/PPTX/TXT）→ 分塊 → 向量索引
- **混合檢索**: 向量相似度（sqlite-vec + candle 本機嵌入）+ BM25 全文檢索（FTS5），混合排序
- **Self-RAG**: 檢索結果自動反思和驗證
- **重排序**: Cross-encoder 結果重排序
- **知識圖譜**: 實體提取 → 關係建構 → 視覺化圖譜
- **檔案監聽**: 基於 `notify` 的即時檔案變更監聽，自動增量索引
- **LLM Wiki**: AI 輔助的 Wiki 編譯器與驗證器

### 記憶系統

- **多命名空間記憶**: 按專案/主題隔離，支援手動錄入與 AI 自動提取
- **持久化整合**: Honcho 和 Mem0 閉環記憶
- **使用者輪廓**: 自動學習程式碼風格、技術棧偏好、溝通風格
- **風格遷移**: 提取程式碼風格特徵 → 應用到 AI 生成程式碼
- **夢境整合**: 背景自動整合記憶碎片與行為模式，生成結構化知識
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
- **Server 模式**: 可選 `axagent-server` 二進位檔，將桌面應用能力以服務形式對外提供

### 訊息平台整合

透過 `rt-messaging` 實現多平台閘道，支援 **釘釘、飛書、QQ、Slack、微信、WhatsApp、Telegram、Discord** 的訊息接收、命令解析與 AI 自動回覆。

### 工具系統

**163+ 內建工具**，統一透過 `Tool` trait 註冊，覆蓋 15 大類別：

| 類別      | 工具範例                                                                                                                                                             |
| --------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 檔案操作  | `file_read`, `file_write`, `file_edit`, `glob`, `grep`, 目錄/刪除/移動等 11 個                                                                                       |
| Shell/Web | `bash`, `web_fetch`, `web_search`                                                                                                                                    |
| 網路      | `http_request`, `ping`, `dns_lookup`, `json_api`, `rss_reader`, `graphql`, `websocket`                                                                               |
| 瀏覽器    | `browser_navigate`, `browser_click`, `browser_fill`, `browser_screenshot` 等 10 個（CDP）                                                                            |
| 電腦控制  | `computer_use`（滑鼠/鍵盤/截圖）                                                                                                                                     |
| Git       | `git_status`, `git_diff`, `git_commit`, `git_log`, `git_branch`, `git_review`                                                                                        |
| 知識庫    | `list_knowledge_bases`, `search_knowledge`, `add_knowledge_document` 等 6 個                                                                                         |
| 任務管理  | `todo_write`, `task_*`（6 個）, `cron_*`（3 個）, `plan` 相關                                                                                                        |
| 訊息推播  | `push_notification`, `send_message`, 團隊協作工具                                                                                                                    |
| 資料庫    | `database_query`, `database_list_tables`, `database_migration_status`                                                                                                |
| 儲存      | `get_storage_info`, `upload_storage_file`, `download_storage_file` 等 5 個                                                                                           |
| 匯出/格式 | `export_word`, `export_pdf`, `export_xlsx`, `export_pptx`, `render_markdown` 等 9 個                                                                                 |
| OCR       | `ocr_image`, `ocr_detect_langs`                                                                                                                                      |
| Obsidian  | `obsidian_search`, `obsidian_read`, `obsidian_backlinks` 等 9 個                                                                                                     |
| 其他      | `agent`, `delegate_task`, `skills_*`, `lsp`, `repl`, `monitor`, `workspace_*`, `session_search`, `generate_image`, `sequential_thinking`, CI/CD、DevOps、RPC、測試等 |

### MCP 協定

基於 `rmcp` 的完整 MCP (Model Context Protocol) 實作：

- **傳輸層**: stdio 子程序 + Streamable HTTP + SSE
- **OAuth 認證**: 支援 MCP 伺服器的 OAuth 授權流程
- **工具發現**: 自動發現和註冊 MCP 伺服器暴露的工具
- **MCP 管理器**: 伺服器生命週期管理、健康檢查、自動重連

### 外掛系統

OpenClaw 相容的三級外掛架構（內建/捆綁/外部）：

- npm 套件安裝，內建市集 UI 搜尋和安裝
- 外掛 manifest 定義、權限宣告、沙箱隔離執行
- 自訂工具註冊、Agent 提供者、Hook 攔截
- 技能安裝器：從外掛套件安裝技能到技能系統

### 動態 UI 引擎

- **Schema 驅動**: 透過 JSON Schema 宣告式建構介面，無需寫程式碼
- **31 個內建元件**: 容器（7）/ 資料展示（6）/ 表單（9）/ 媒體（4）/ 其他（5）
- **資料綁定**: 宣告式資料源綁定與條件渲染
- **NL2UI**: 自然語言直接生成動態 UI 介面

### ACP 用戶端 SDK

- **ACP（Agent Client Protocol）**: 雙語言 SDK（TypeScript + Python），零第三方依賴
- 工作階段管理、Prompt 發送、工具呼叫記錄、WebSocket 事件串流
- 透過 `/acp/v1/*` 端點與 AxAgent 服務通訊

### 安全防護

- **AES-256-GCM 加密**: API Key 和敏感設定本機加密儲存（`crypto` crate）
- **提示詞注入防護**: 四級防禦管線（`prompt-guard`）—— 模式檢測 → 分隔符號轉義 → XML 包裝器 → 信任標籤，整合到工作階段、提示詞建構、Git、RAG 全鏈路
- **SSRF 防護**: URL 安全檢查，阻止對內網位址的請求
- **內容過濾**: 多類型內容安全過濾
- **速率限制**: 工具呼叫和 API 請求權杖桶限流
- **斷路器**: 連續失敗自動熔斷
- **存取控制**: 基於策略的工具存取權限控制
- **沙箱隔離**: 智慧體和技能執行環境隔離

### 開發者工具

- **分散式追蹤** (`telemetry`): OpenTelemetry 整合，Span/Trace 視覺化
- **結構化紀錄**: tracing-subscriber + chrono 時間戳
- **重播除錯**: 智慧體執行軌跡錄製（`trajectory_recorder`）與重播
- **DevTools 面板**: Trace Explorer 時間線檢視器、Benchmark Runner、Tool Recommender
- **基準測試**: Criterion benchmarks（tool_exec / llm_call / search）
- **CI 檢查**: `npm run ci:check` 整合型別檢查、lint、格式化校驗

### 桌面與行動端體驗

- **響應式佈局**: CSS 斷點自適應桌面/平板/手機（3 級裝置佈局：`desktop` / `tablet` / `mobile`）
- **11 種語言**: 簡體中文、繁體中文、英語、日語、韓語、法語、德語、西班牙語、俄語、印地語、阿拉伯語
- **主題引擎** (`rt-theme`): 深色/淺色主題 + 多個預設，Ant Design 6 深度自訂
- **Monaco 編輯器**: 語法高亮、差異預覽、多語言支援
- **xterm.js 終端機**: WebLinks、Unicode 11、搜尋
- **虛擬滾動**: @tanstack/react-virtual + react-virtuoso
- **圖表渲染**: D2 + Mermaid + Recharts + Sigma（圖譜）
- **Command Palette**: Ctrl+K 全域命令面板
- **系統托盤 + 全域快捷鍵 + 開機自啟**: 無干擾背景執行
- **自動更新**: 可設定間隔的 GitHub Releases 版本檢測
- **代理支援**: HTTP / SOCKS5 代理設定
- **雲端工作空間**: S3 和 WebDAV 儲存同步，衝突檢測與雙向同步

### 行動端

- Android APK/AAB（arm64-v8a, armeabi-v7a, x86_64）
- iOS IPA（arm64）
- 行動端專屬適配：安全區適配、底部導航、Drawer 導航

---

## 技術架構

### 技術棧

| 層級           | 技術                                     | 版本 |
| -------------- | ---------------------------------------- | ---- |
| 桌面框架       | Tauri                                    | 2.11 |
| 前端框架       | React                                    | 19   |
| 型別系統       | TypeScript                               | 7    |
| UI 庫          | Ant Design                               | 6    |
| CSS 框架       | TailwindCSS                              | 4    |
| 狀態管理       | Zustand                                  | 5    |
| 路由           | React Router                             | 7    |
| 程式碼編輯器   | Monaco Editor                            | 0.55 |
| 終端機         | xterm.js                                 | 6    |
| 工作流程編輯器 | ReactFlow                                | 12   |
| 圖表           | D2 + Mermaid + Recharts + Sigma          |      |
| 動畫           | Framer Motion                            | 12   |
| 虛擬滾動       | @tanstack/react-virtual + react-virtuoso |      |
| 拖拽           | @dnd-kit                                 | 6    |
| Markdown 渲染  | markstream-react + stream-markdown       |      |
| 國際化         | i18next + react-i18next                  |      |
| 建置工具       | Vite                                     | 8    |
| 測試           | Vitest + Playwright                      |      |
| 格式化         | dprint（TS/JSON/Markdown/TOML）+ rustfmt |      |
| Lint           | ESLint + Oxlint + Clippy                 |      |

### 後端架構: Harness 依賴注入模式

採用 Rust workspace 架構，包含 **37 個成員**（主 crate + 35 個庫 crate + schema-gen），遵循 **Harness 依賴注入架構**：

> 所有 crate 透過 axagent-harness 定義的 trait 介面解耦，執行時由 axagent-runtime 裝配和注入依賴。
> 依賴方向：`具體實作 → harness ← 呼叫方`

**harness** 是架構基石 — 零業務邏輯、零具體實作，僅含 trait 定義、純資料 DTO、常數和統一錯誤類型。被所有其他 crate 依賴，自身不依賴任何 axagent-* crate（200+ trait 定義，涵蓋 Agent/Provider/Tool/RAG/儲存/MCP/外掛/安全/可觀測性/記憶/學習/瀏覽器/訊息/認知路由等）。

```
src-tauri/crates/
├── harness/          # 架構基石 — trait 介面、DTO、錯誤類型、DI 契約
├── entities/         # SeaORM 實體模型
├── dao/              # 資料存取層（CRUD）
├── migration/        # 資料庫遷移
├── crypto/           # AES-256-GCM 加解密與金鑰管理
├── credential/       # 憑證安全儲存
├── storage/          # 檔案儲存抽象（本機/S3/WebDAV），ZIP 讀寫
├── cache/            # 記憶體快取層
├── disk-cache/       # 磁碟檔案級快取
├── search/           # 檢索引擎（FTS5 + sqlite-vec + candle 本機嵌入）
├── document-parser/  # 文件文字提取（PDF/DOCX/XLSX/PPTX）
├── kit/              # 通用工具集（路徑/編碼/雜湊/日期）
├── runtime-core/     # 執行時共同類型、設定常數
├── runtime/          # 執行時服務編排 — 裝配全部 crate 的 DI 容器
├── rt-workflow/      # 工作流程引擎 — DAG 編排、節點執行器、YAML 序列化
├── rt-messaging/     # 訊息平台閘道 — 釘釘/飛書/QQ/Slack/微信/WhatsApp/Telegram/Discord
├── rt-webhook/       # 通用 Webhook 伺服器
├── rt-dashboard/     # 儀表板外掛框架
├── rt-theme/         # 主題引擎
├── agent/            # AI 智慧體核心 — 80+ 模組
│                     #   ReAct引擎/層級規劃/深度研究/事實核查/思維樹/反思/
│                     #   自驗證/錯誤復原/RL最佳化/LoRA微調/評估/工具推薦/A/B測試/
│                     #   協調器/黑板/視覺管線/Web搜尋/學術搜尋/Wiki編譯等
├── orchestrator/     # 智慧體編排 — 多智慧體排程、DAG 分解、動態子圖執行
├── providers/        # 模型提供商配接器（13 種）
├── tools/            # 工具體系 — Tool trait/註冊表/編排/串流/沙箱/163+內建工具
├── gateway/          # API 閘道 — axum HTTP/WS 伺服器、OAuth、速率限制、Prometheus
├── mcp/              # MCP 協定 — stdio + Streamable HTTP + SSE，基於 rmcp
├── trajectory/       # 學習系統 — 記憶/技能演化/使用者輪廓/夢境整合
├── plugins/          # 外掛系統 — OpenClaw 相容、npm 套件安裝、市集
├── telemetry/        # 可觀測性 — OpenTelemetry、結構化紀錄、執行時指標
├── prompt-guard/     # 提示詞注入防護 — L1-L4 多級檢測管線
├── npm/              # npm 註冊表用戶端
├── crdt/             # 協同編輯資料結構
├── device/           # 裝置管理
├── axagent-mobile/   # 行動端適配層
├── agent-macro/      # 智慧體巨集
├── agent-command-types/ # 智慧體命令類型
└── schema-gen/       # 資料庫 Schema 生成工具
```

### 前端架構

```
src/
├── pages/            # 頁面（24 個）
│   ├── ChatPage           # 對話主介面 — 側邊欄/訊息串流/Agent 面板/多 Tab
│   ├── DashboardPage      # 資料儀表板 — 用量統計/模型分佈/趨勢圖表
│   ├── WorkflowPage       # 工作流程編輯器 — ReactFlow DAG 視覺化
│   ├── KnowledgeHubPage   # 知識庫管理 — 文件上傳/索引/檢索
│   ├── MemoryPage         # 記憶管理
│   ├── SkillsPage         # 技能市集
│   ├── SettingsPage       # 設定面板 — 40+ 設定項
│   ├── TerminalPage       # 內建終端機 — xterm.js
│   ├── FilesPage          # 檔案管理
│   ├── GatewayLinkPage    # API 閘道與外部連結管理
│   ├── QuickBarPage       # 快捷欄（獨立視窗）
│   ├── DynamicUIManagerPage / DynamicPageViewer  # 動態 UI 引擎
│   ├── WikiGraphPage / WikiEditPage / IngestPage # LLM Wiki
│   ├── LearningGraphPage  # 學習圖譜
│   ├── FineTunePage       # LoRA 微調
│   ├── PersonaPage        # 角色管理
│   ├── WorkflowMarketplace # 工作流程市集
│   ├── DevTools/          # TraceExplorer / BenchmarkRunner / ToolRecommender
│   └── Workflow/          # WorkflowListPage
│
├── components/       # 33 個模組，500+ 元件
│   ├── chat/         # 對話（訊息串流/輸入/ChatView/TabBar/RightPanel/附件/工具呼叫渲染）
│   ├── layout/       # 佈局 — AppInitializer / Sidebar / ContentArea / TitleBar /
│   │                 #   CommandPalette / GlobalCopyMenu / GlobalErrorBoundary /
│   │                 #   GlobalStatusBar / ErrorNotificationToast / AppHeader 等
│   ├── agent/        # Agent 面板/入口/迷你面板
│   ├── workflow/     # 工作流程編輯器（節點/連線/面板/範本/AI輔助）
│   ├── settings/     # 設定面板（40+ 子元件）
│   ├── skill/        # 技能編輯器/渲染器/浮動面板
│   ├── dynamicUI/    # 動態 UI 元件（31 個內建元件）
│   ├── gateway/      # API 閘道管理
│   ├── files/        # 檔案管理
│   ├── terminal/     # 終端機元件
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
├── stores/           # Zustand 狀態管理（82 個 store）
│   ├── domain/       # 9 個核心業務 store（對話/串流/壓縮/偏好/多模型等）
│   ├── feature/      # 61 個功能模組 store（智慧體/工作流程/知識庫/技能/閘道/記憶/終端機等）
│   ├── shared/       # 8 個跨元件共享 store（UI/標籤頁/工作區/後端狀態等）
│   └── devtools/     # 4 個開發者工具 store
│
├── hooks/            # React Hooks（快捷鍵/命令面板/響應式/滾動條/主題/Avatar 等）
├── lib/              # 工具函式庫（invoke/pageRegistry/shortcuts/skillLifecycle/
│                     #   chartGenerator/codeExecutor/tokenEstimator/workflowLayout 等 45+ 模組）
├── types/            # TypeScript 型別定義
├── theme/            # Shadcn 主題引擎
├── i18n/             # 11 語言翻譯檔案（zh-CN/zh-TW/en-US/ja/ko/fr/de/es/ru/hi/ar）
├── constants/        # 常數與功能開關
└── sdk/              # ACP 用戶端 SDK（TypeScript + Python）
```

### 功能開關

專案透過 `featureFlags.ts` 管理漸進式功能發佈：

| 開關                | 狀態 | 說明                              |
| ------------------- | ---- | --------------------------------- |
| `AGENT_IN_THE_LOOP` | ✅   | 全域 Agent Panel + 頁面上下文注入 |
| `DYNAMIC_UI`        | ✅   | 動態 UI 建構引擎                  |
| `SELF_EVOLUTION_UI` | ❌   | 自我演化前端控制面                |
| `NL_EXTENSION`      | ❌   | 自然語言驅動動態業務擴充          |

### Tauri 外掛

| 外掛                | 用途              |
| ------------------- | ----------------- |
| `autostart`         | 開機自啟          |
| `clipboard-manager` | 剪貼簿讀寫        |
| `dialog`            | 檔案選擇對話框    |
| `fs`                | 檔案系統存取      |
| `global-shortcut`   | 全域快捷鍵註冊    |
| `notification`      | 系統通知          |
| `opener`            | 外部連結/檔案開啟 |
| `process`           | 程序管理          |
| `updater`           | 自動更新          |

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

### 建置

```bash
npm run tauri build    # 桌面端生產建置

npm run tauri:android:build   # Android 建置
npm run tauri:ios:build       # iOS 建置
```

桌面端建置產物位於 `src-tauri/target/release/`。

### 測試

```bash
npm run test           # 前端單元測試（Vitest watch）
npm run test:run       # 前端單元測試（單次執行）
npm run test:e2e       # E2E 測試（Playwright）

# Rust 後端測試
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

### 常用腳本

| 命令                     | 用途                 |
| ------------------------ | -------------------- |
| `npm run bump`           | 版本號升級（互動式） |
| `npm run docs`           | 生成 TypeDoc 文件    |
| `npm run skill:create`   | 建立新技能脚手架     |
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

## 開源授權

本專案基於 [AGPL-3.0-only](LICENSE) 協議開源。

---

## 致謝

AxAgent 建構在眾多優秀開源專案之上：

- [Tauri](https://tauri.app/) — 跨平台桌面框架
- [React](https://react.dev/) + [Ant Design](https://ant.design/) — 前端 UI
- [SeaORM](https://www.sea-ql.org/SeaORM/) — Rust ORM
- [sqlite-vec](https://github.com/asg017/sqlite-vec) — 向量檢索
- [candle](https://github.com/huggingface/candle) — 本機嵌入推理
- [rmcp](https://github.com/nicholasxjy/rmcp) — Rust MCP SDK
- [ReactFlow](https://reactflow.dev/) — 視覺化工作流程編輯器
- [axum](https://github.com/tokio-rs/axum) — HTTP 框架
- [Monaco Editor](https://microsoft.github.io/monaco-editor/) — 程式碼編輯器
- [xterm.js](https://xtermjs.org/) — 終端機模擬器
- [Zustand](https://zustand.docs.pmnd.rs/) — 狀態管理
- [Framer Motion](https://www.framer.com/motion/) — 動畫庫
- [Recharts](https://recharts.org/) — 圖表庫
