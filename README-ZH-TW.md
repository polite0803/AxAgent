[**English**](./README-EN.md) | [简体中文](./README.md) | **繁體中文** | [日本語](./README-JA.md) | [한국어](./README-KO.md) | [Français](./README-FR.md) | [Deutsch](./README-DE.md) | [Español](./README-ES.md) | [Русский](./README-RU.md) | [हिन्दी](./README-HI.md) | [العربية](./README-AR.md)

[![AxAgent](https://github.com/polite0803/AxAgent/blob/main/src/assets/image/logo.png?raw=true)](https://github.com/polite0803/AxAgent)

<p align="center">
  <a href="https://www.producthunt.com/products/axagent?embed=true&amp;amp&utm_source=badge-featured&amp;amp;&amp;#10;&amp;amp&amp;amp;;utm_medium=badge&amp;amp;#10&amp;amp;;utm_campaign=badge-axagent" target="_blank" rel="noopener noreferrer"><img alt="AxAgent - Lightweight, high-perf cross-platform AI desktop client | Product Hunt" width="250" height="54" src="https://api.producthunt.com/widgets/embed-image/v1/featured.svg?post_id=1118403&amp;theme=light&amp;t=1775627359538"></a>
</p>

<p align="center">
  <strong>跨平台 AI 桌面/行動應用 | 多智能體協作 | 本地優先</strong>
</p>

<p align="center">
  <a href="https://github.com/polite0803/AxAgent/releases" target="_blank">
    <img src="https://img.shields.io/github/v/release/polite0803/AxAgent?style=flat-square" alt="Release">
  </a>
  <a href="https://github.com/polite0803/AxAgent/actions" target="_blank">
    <img src="https://img.shields.io/github/actions/workflow/status/polite0803/AxAgent/release.yml?style=flat-square" alt="Build">
  </a>
  <img src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux%20%7C%20Android%20%7C%20iOS-blue?style=flat-square" alt="Platform">
  <img src="https://img.shields.io/badge/license-AGPL--3.0-green?style=flat-square" alt="License">
</p>

---

## 什麼是 AxAgent？

**AxAgent v2.0** 是一款功能全面的跨平台 AI 桌面/行動應用，整合了先進的 AI 智慧體能力和豐富的開發者工具。它支援多模型提供商、自主管道執行、視覺化工作流編排、本地知識管理、內建 API 閘道，覆蓋 **Windows / macOS / Linux / Android / iOS** 五大平台。

---

## 截圖預覽

| 對話與模型選擇 | 多智能體儀表盤 |
|:---:|:---:|
| ![](.github/images/s1-0412.png) | ![](.github/images/s5-0412.png) |

| 知識庫 RAG | 記憶與上下文 |
|:---:|:---:|
| ![](.github/images/s3-0412.png) | ![](.github/images/s4-0412.png) |

| 工作流編輯器 | API 網關 |
|:---:|:---:|
| ![](.github/images/s9-0412.png) | ![](.github/images/s10-0412.png) |

---

## 核心功能

### 🤖 AI 模型支援

- **多提供商支援** — 原生整合 OpenAI、Anthropic Claude、Google Gemini、Ollama、OpenClaw、Hermes 及所有 OpenAI 相容 API
- **多 Key 輪換** — 為每個提供商配置多個 API Key，自動輪換分發限流
- **本地模型推理** — 完整支援 Ollama 本地模型，包含 GGUF/GGML 檔案管理
- **Candle 推理引擎** — 內建 Candle 本地推理，支援 rerank/judge 介面，GGUF 按需下載
- **模型管理** — 遠端模型列表獲取，可自訂參數（temperature、max tokens、top-p 等）
- **串流輸出** — 即時逐 Token 渲染，支援可折疊的思考塊（Claude 擴展思考）
- **多模型對比** — 同時向多個模型提問，side-by-side 對比結果
- **函數呼叫** — 跨所有支援提供商的結構化函數呼叫
- **OpenAI Responses API** — 支援 OpenAI Responses 格式傳輸
- **即時 API** — 相容 OpenAI 即時 API 的 WebSocket 事件推送

### 🔐 AI 智慧體系統

智慧體系統基於精密架構構建，具備以下特性：

- **ReAct 推理引擎** — 融合推理與行動，內建自驗證確保任務執行可靠
- **層級規劃器** — 將複雜任務分解為具有階段和依賴關係的結構化計劃
- **任務分解器** — 自動將複雜任務分解為可執行的子任務
- **深度研究** — 多源搜尋編排、引用追蹤與可信度評估
- **事實核查** — AI 驅動的事實驗證與來源分類
- **搜尋編排** — 多搜尋提供商協調，支援搜尋規劃和結果綜合
- **學術搜尋** — 學術文獻檢索和引用分析
- **計算機控制** — AI 控制的滑鼠點擊、鍵盤輸入、螢幕滾動，配合視覺模型分析
- **螢幕感知** — 截圖擷取和視覺模型分析，用於 UI 元素識別
- **三級權限模式** — 預設（需要審批）、接受編輯（自動批准）、完全訪問（無提示）
- **沙箱隔離** — 智慧體操作嚴格限制在指定工作目錄內
- **工具審批面板** — 即時顯示工具呼叫請求，支援逐條審批
- **成本追蹤** — 即時顯示每個對話的 Token 使用量和成本統計
- **暫停/恢復** — 隨時暫停智慧體執行，稍後恢復
- **檢查點系統** — 持久化檢查點用於崩潰恢復和對話重連
- **錯誤恢復引擎** — 自動錯誤分類、根因分析和恢復策略執行
- **循環檢測** — 自動檢測和中斷智慧體推理中的循環行為
- **思維鏈** — 智慧體決策推理的可視化，逐步分解
- **主動模式** — 智慧體可主動提供建議和執行操作
- **目的管理** — 維護和追蹤智慧體的執行目的與上下文

### 👥 多智慧體協作

- **子智慧體協調** — 主從架構，支援多個協作智慧體
- **並行執行** — 多個智慧體並行處理任務，支援依賴感知排程
- **對抗性辯論** — Pro/Con 辯論輪次，支援論點強度評分和反駁追蹤
- **智慧體角色** — 預定義角色（研究員、規劃師、開發者、評審員、綜合員）用於團隊協作
- **智慧體編排器** — 多智慧體團隊的中心化訊息路由和狀態管理
- **通訊圖譜** — 智慧體互動和訊息流的可視化展示
- **Swarm 集群** — 多程序智慧體集群，支援權限同步和自動重連
- **Buddy 夥伴系統** — 可配置的智慧體夥伴，支援物種和屬性定義
- **共享記憶** — 跨智慧體共享的記憶體空間，支援統計和查詢
- **團隊 Cron 註冊** — 團隊級別的定時任務排程

### ⭐ 技能系統

- **技能市場** — 內建市場，瀏覽和安裝社群貢獻的技能
- **技能建立** — 從提案自動建立技能，支援 Markdown 編輯器
- **技能進化** — 基於執行回饋的 AI 驅動的現有技能自動分析和改進
- **技能匹配** — 語義匹配，推薦與對話上下文相關的技能
- **技能分解** — 自動將複雜任務分解為可執行的原子技能（LLM 輔助/多輪/工作流驗證）
- **生成工具** — AI 自動產生並註冊新工具，擴展智慧體能力
- **技能中心** — 集中的技能發現和配置管理介面
- **技能中心用戶端** — 與遠端技能中心整合，支援社群分享
- **技能依賴檢查** — 自動檢測技能依賴和工具可用性
- **技能沙箱容器** — 技能在隔離環境中安全執行

### 🔄 工作流系統

工作流引擎實現了基於 DAG 的任務編排系統：

- **視覺化工作流編輯器** — 拖放式工作流設計器，支援節點連接和配置
- **豐富節點類型** — 15 種節點類型：觸發器、智慧體、LLM、條件、並行、循環、合併、延遲、工具、程式碼、子工作流、向量檢索、文件解析、驗證、結束
- **工作流範本** — 內建預設：程式碼審查、Bug 修復、文件、測試、重構、探索、效能、安全、功能開發
- **DAG 執行** — Kahn 演算法拓撲排序，支援循環檢測
- **並行排程** — 流水線式執行，快速步驟不等慢速步驟
- **重試策略** — 指數退避，每步可配置最大重試次數
- **部分完成** — 失敗的步驟不會阻塞獨立的下游步驟
- **版本管理** — 工作流範本版本控制，支援回滾
- **執行歷史** — 詳細記錄，支援狀態追蹤和除錯
- **AI 輔助** — AI 輔助工作流設計、節點推薦和智慧體提示詞最佳化
- **語義檢查** — 工作流語義驗義驗證，檢測潛在問題
- **n8n 匯入** — 支援從 n8n 目錄匯入工作流
- **除錯面板** — 工作流執行過程的即時除錯和狀態查看

### 📚 知識與記憶

- **知識庫（RAG）** — 多知識庫支援，支援文件上傳、自動解析、分塊和向量索引
- **混合搜尋** — 結合向量相似度搜尋與 BM25 全文排名
- **重排序** — Cross-encoder 重排序，提升檢索精度
- **三級召回管道** — AST 索引 + 向量搜尋 + FTS5 的多級召回機制
- **知識圖譜** — 知識關聯的實體關係可視化（實體、屬性、關係、流、介面）
- **Wiki 系統** — LLM Wiki 編譯器與驗證器，支援知識圖譜可視化與增量同步
- **Wiki 筆記** — 雙向連結筆記系統，支援圖譜檢視和自動連結同步
- **記憶系統** — 多命名空間記憶，支援手動錄入或 AI 自動提取
- **閉環記憶** — 整合 Honcho 和 Mem0 持久化記憶提供商
- **FTS5 全文搜尋** — 跨對話、檔案、記憶的快速檢索
- **對話搜尋** — 跨所有對話對話的高級搜尋
- **上下文管理** — 靈活附加檔案、搜尋結果、知識片段、記憶、工具輸出
- **文件解析** — 多格式文件自動解析和內容提取
- **增量索引** — 檔案變更的增量索引更新

### 🌐 API 閘道

- **本地 API 伺服器** — 內建 OpenAI 相容、Claude 和 Gemini 介面伺服器
- **外部連結** — 一鍵整合 Claude CLI、OpenCode，自動同步 API Key 和模型
- **Key 管理** — 產生、撤銷、啟用/停用存取 Key，支援描述
- **用量分析** — 按 Key、提供商、日期的請求量和 Token 使用量
- **SSL/TLS 支援** — 內建自簽名憑證，支援自訂憑證
- **請求日誌** — 完整記錄所有 API 請求和回應
- **配置範本** — Claude、Codex、OpenCode、Gemini 的預建範本
- **即時 API** — 相容 OpenAI 即時 API 的 WebSocket 事件推送
- **平台整合** — 支援釘釘、飛書、QQ、Slack、微信、WhatsApp、Telegram、Discord
- **閘道診斷** — 連線診斷和程式策略管理
- **限流器** — API 請求速率限制和流量控制
- **持久化佇列** — 請求持久化佇列管理

### 🔧 工具與擴展

- **MCP 協定** — 完整的模型上下文協定實現，支援 stdio 和 HTTP/WebSocket 傳輸
- **OAuth 認證** — MCP 伺服器的 OAuth 流程支援
- **MCP 自動啟動** — MCP 伺服器自動啟動和生命週期管理
- **MCP 工具橋接** — MCP 工具與智慧體工具系統的橋接
- **外掛系統** — OpenClaw 相容的三級外掛架構（內建/捆綁/外部），支援 npm 套件安裝、工具註冊、掛鉤與生命週期管理
- **外掛市場** — 內建市場 UI，支援 npm 搜尋安裝、確認彈窗
- **內建工具** — 全面的檔案操作（讀/寫/編輯）、程式碼執行、搜尋（Grep/Glob）、Bash、Web 搜尋、Web 抓取、計畫管理、Cron 排程、REPL、LSP、上下文管理、計算機控制、訊息推送、待辦事項等
- **工具權限系統** — 工具權限分類、規則管理和使用追蹤
- **Bash 安全** — 命令解析、路徑驗證和沙箱安全控制
- **LSP 用戶端** — 內建語言伺服器協定，支援程式碼補全和診斷
- **AST 索引** — 程式碼檔案的 AST 解析和索引建構
- **終端後端** — 支援本地、Docker 和 SSH 終端連接
- **瀏覽器自動化** — 透過 CDP 整合瀏覽器控制能力（導航、截圖、點擊、填寫、文字提取等）
- **UI 自動化** — 跨平台 UI 元素識別和控制
- **Git 工具** — Git 操作，支援分支檢測和衝突感知
- **工具推薦** — 基於上下文的智慧工具推薦引擎
- **工具編排** — 多工具協調執行和串流輸出
- **工具統計** — 工具使用頻率和效能統計

### 📊 內容渲染

- **Markdown 渲染** — 完整支援程式碼高亮、LaTeX 數學公式、表格、任務列表
- **Monaco 程式碼編輯器** — 內建編輯器，支援語法高亮、複製、差異預覽
- **圖表渲染** — Mermaid 流程圖、D2 架構圖、ECharts 互動式圖表
- **產物面板** — 程式碼片段、HTML 草稿、React 元件、Markdown 筆記，支援即時預覽
- **四種預覽模式** — 程式碼（編輯器）、分屏（並排）、預預覽（僅渲染）、React 元件預覽
- **對話檢查器** — 對話結構的樹形檢視，快速導航
- **引用面板** — 追蹤和顯示來源引用，支援可信度評分
- **資訊圖渲染** — 支援資訊圖可視化展示

### 🛡️ 資料與安全

- **AES-256 加密** — API Key 和敏感資料使用 AES-256-GCM 加密
- **隔離儲存** — 應用狀態儲存在 `~/.axagent/`，使用者檔案儲存在 `~/Documents/axagent/`
- **自動備份** — 計畫備份到本地目錄或 WebDAV 儲存
- **備份恢復** — 一鍵從歷史備份恢復
- **匯出選項** — PNG 截圖、Markdown、純文字、JSON 格式
- **儲存管理** — 可視化磁碟使用顯示和清理工具
- **檔案授權** — 檔案存取授權和撤銷管理
- **操作稽核** — 關鍵操作的稽核日誌記錄

### 🖥️ 桌面體驗

- **主題引擎** — 深色/淺色主題，支援跟隨系統或手動偏好
- **介面語言** — 11 種語言：簡體中文、繁體中文、英語、日語、韓語、法語、德語、西班牙語、俄語、印地語、阿拉伯語
- **系統匣** — 最小化到系統匣，不中斷後台服務
- **置頂視窗** — 視窗置頂於其他視窗之上
- **全域快捷鍵** — 可自訂快捷鍵叫出主視窗
- **QuickBar** — 快速存取浮動條，一鍵喚起
- **開機自啟** — 可選在系統啟動時執行
- **代理支援** — HTTP 和 SOCKS5 代理配置
- **自動更新** — 自動檢查版本，有更新時提示
- **命令面板** — `Cmd/Ctrl+K` 快速存取命令
- **引導精靈** — 首次使用的互動式引導和 Ollama 偵測
- **通知中心** — 統一的應用內通知管理

### 🔬 高級功能

- **深度研究** — 多源搜尋、引用追蹤、可信度評估與內容綜合
- **事實核查** — AI 驅動的事實驗證與來源分類
- **Cron 排程器** — 自動化任務排程，支援每日/每週/每月範本和自訂 cron 表達式
- **Webhook 系統** — 事件訂閱，支援工具完成、智慧體錯誤、對話結束通知
- **使用者畫像** — 自動學習程式碼風格、命名規範、縮排、註解風格、溝通偏好
- **RL 最佳化器** — 強化學習最佳化工具選擇和任務策略
- **LoRA 微調** — 使用 LoRA 進行本地訓練的自訂模型適配
- **主動建議** — 基於對話內容和使用者模式的上下文感知提示
- **上下文預測** — 預測使用者下一步操作並預取相關資源
- **夢境整合** — 後台自動整合記憶與模式，最佳化長期知識
- **錯誤恢復** — 自動錯誤分類、根因分析和恢復建議
- **開發者工具** — Trace、Span、時間線可視化，用於除錯和效能分析
- **基準測試系統** — SWE-bench / Terminal-bench 任務效能評估和指標，帶評分卡
- **風格遷移** — 將學習的程式碼風格偏好套用到生成的程式碼
- **儀表盤外掛** — 可擴展的儀表盤，支援自訂面板和小工具
- **協作共享** — CRDT 即時協作與一鍵對話分享
- **瀏覽器擴充功能** — Wiki Clipper 瀏覽器擴充功能，快速剪藏網頁到 LLM Wiki
- **Python SDK** — 提供 Python SDK 用於與 AxAgent 整合
- **智慧路由** — 請求智慧路由和分類
- **語義快取** — 基於語義的回應快取，減少重複計算
- **上下文壓縮** — 自動壓縮長上下文，最佳化 Token 使用
- **訊息批次處理** — 訊息批次傳送和最佳化
- **連線池** — 資料庫和 API 連線池管理
- **特性開關** — 可配置的功能特性開關系統
- **策略引擎** — 權限和操作策略的集中管理
- **資源治理** — 智慧體資源使用限制和治理
- **LAN 傳輸** — 區域網路檔案傳輸能力

### 🛡️ 提示詞注入防護（Prompt-Guard）

- **四級防護體系** — L1 模式偵測（高風險攔截 + 中風險標記）→ L2 分隔符跳脫 → L3 XML 包裝器 → L4 信任標籤
- **Pipeline 編排器** — 多級偵測管道串聯，支援自訂風險閾值
- **Token Smuggling 偵測** — 針對編碼混淆和 token 走私攻擊的專項偵測
- **Strict 模式** — 嚴格模式測試 + 中風險原因命名 + 自訂模式文件
- **全管道整合** — 已整合到 session / prompt / git / RAG 各環節

### 📱 行動端支援

- **Android 原生** — APK/AAB 建構，支援 arm64-v8a / armeabi-v7a / x86_64
- **iOS 原生** — IPA 建構，支援 arm64
- **自適應佈局** — 桌面/平板/手機三檔自動適配
- **行動端導航** — Drawer 滑出導航 + 底部導航列 + 閃現式浮動按鈕
- **安全區適配** — Android 系統狀態列/導航列 CSS env() 自適應
- **CSP 最佳化** — Android WebView CSP 協定白名單

---

## 技術架構

### 技術堆疊

| 層級 | 技術 |
|------|------|
| **框架** | Tauri 2 + React 19 + TypeScript 6 |
| **UI** | Ant Design 6 + TailwindCSS 4 |
| **狀態管理** | Zustand 5 |
| **路由** | React Router 7 |
| **國際化** | i18next + react-i18next |
| **後端** | Rust + SeaORM 2 + SQLite |
| **向量資料庫** | sqlite-vec |
| **程式碼編輯器** | Monaco Editor |
| **圖表** | Mermaid + D2 + ECharts（CDN） |
| **終端** | xterm.js 6 |
| **工作流** | ReactFlow 11 |
| **圖表渲染** | @antv/infographic |
| **圖示** | Iconify + Lucide |
| **拖曳** | @dnd-kit |
| **建構** | Vite 8 + npm |
| **測試** | Vitest + Playwright + cargo-nextest |
| **格式化** | dprint (TS/JSON) + rustfmt |
| **Lint** | TS: eslint + oxlint / Rust: clippy + cargo-deny |
| **行動端** | Tauri Android + iOS 原生建構 |
| **桌面端** | Windows (MSI) · macOS (DMG) · Linux (AppImage/deb/rpm) |

### 平台支援

| 平台 | 架構 |
|------|------|
| Windows | x86_64, ARM64 |
| macOS | Apple Silicon (arm64), Intel (x86_64) |
| Linux | x86_64, ARM64 |
| Android | arm64-v8a, armeabi-v7a, x86_64（模擬器） |
| iOS | arm64 |

### Rust 後端架構

後端組織為 Rust workspace，包含 **18 個** 專業化的 crates：

```
src-tauri/crates/
├── agent/            # AI 智慧體核心（ReAct 引擎、協調、規劃、深度研究、事實核查等）
├── core/             # 核心工具（資料庫、RAG、加密、MCP、瀏覽器自動化、AST 索引等）
├── providers/        # 模型提供商介面卡（OpenAI、Anthropic、Gemini、Ollama、OpenClaw 等）
├── runtime-core/     # 執行時抽象層（公共類型、trait 定義、配置）
├── runtime/          # 執行時服務（對話管理、MCP、終端、限流、Webhook、權限等）
├── rt-workflow/      # 工作流引擎（DAG 編排、節點執行器、排程器）
├── rt-messaging/     # 訊息閘道（釘釘/飛書/QQ/Slack/微信/WhatsApp/Telegram/Discord 整合）
├── rt-webhook/       # Webhook 伺服器與分發
├── rt-dashboard/     # 儀表盤外掛系統
├── rt-theme/         # 主題引擎
├── gateway/          # API 閘道（HTTP 伺服器、認證、路由、OpenAI 相容介面）
├── tools/            # 工具系統（註冊表、編排、串流輸出、40+ 內建工具）
├── trajectory/       # 學習系統（記憶、技能、RL、使用者畫像、夢境整合）
├── telemetry/        # 遙測與分散式追蹤
├── plugins/          # 外掛系統（OpenClaw 相容，npm 套件安裝）
├── prompt-guard/     # 提示詞注入防護（L1-L4 多級偵測與防禦）
├── migration/        # 資料庫遷移
├── npm/              # npm 套件解析與註冊表
└── code_engine/      # Candle 本地推理引擎（已棄用，功能已整合至 core）
```

### 前端架構

```
src/
├── stores/                    # Zustand 狀態管理
│   ├── domain/               # 核心業務狀態
│   │   ├── conversationStore.ts
│   │   ├── messageStore.ts
│   │   ├── streamStore.ts
│   │   ├── multiModelStore.ts
│   │   ├── preferenceStore.ts
│   │   └── compressStore.ts
│   ├── feature/               # 功能模組狀態（30+ store）
│   │   ├── agentStore.ts
│   │   ├── agentProfileStore.ts
│   │   ├── appConfigStore.ts
│   │   ├── backupStore.ts
│   │   ├── buddyStore.ts
│   │   ├── categoryStore.ts
│   │   ├── decompositionStore.ts
│   │   ├── dreamStore.ts
│   │   ├── executionStore.ts
│   │   ├── expertStore.ts
│   │   ├── fileStore.ts
│   │   ├── gatewayStore.ts
│   │   ├── gatewayLinkStore.ts
│   │   ├── generatedToolStore.ts
│   │   ├── helpStore.ts
│   │   ├── knowledgeStore.ts
│   │   ├── llmWikiStore.ts
│   │   ├── localToolStore.ts
│   │   ├── memoryStore.ts
│   │   ├── mcpStore.ts
│   │   ├── nudgeStore.ts
│   │   ├── onboardingStore.ts
│   │   ├── planStore.ts
│   │   ├── platformStore.ts
│   │   ├── proactiveStore.ts
│   │   ├── promptTemplateStore.ts
│   │   ├── providerStore.ts
│   │   ├── searchStore.ts
│   │   ├── settingsStore.ts
│   │   ├── skillExtensionStore.ts
│   │   ├── skillStore.ts
│   │   ├── styleStore.ts
│   │   ├── terminalStore.ts
│   │   ├── themeStore.ts
│   │   ├── trajectoryStore.ts
│   │   ├── userProfileStore.ts
│   │   ├── wikiStore.ts
│   │   ├── workEngineStore.ts
│   │   └── workflowEditorStore.ts
│   ├── devtools/              # 開發者工具狀態
│   │   ├── tracerStore.ts
│   │   ├── evaluatorStore.ts
│   │   ├── rlStore.ts
│   │   ├── fineTuneStore.ts
│   │   └── recommendationStore.ts
│   └── shared/                # 共享狀態
│       ├── artifactStore.ts
│       ├── chatWorkspaceStore.ts
│       ├── tabStore.ts
│       └── uiStore.ts
│
├── components/                # React 元件（24 個模組）
│   ├── chat/                # 對話介面（90+ 元件）
│   ├── workflow/            # 工作流編輯器（節點/面板/範本/AI 輔助）
│   ├── gateway/             # API 閘道 UI
│   ├── settings/            # 設定面板（40+ 元件）
│   ├── terminal/            # 終端 UI
│   ├── skill/               # 技能編輯器與渲染器
│   ├── benchmark/           # 基準測試面板
│   ├── decomposition/       # 技能分解與工具生成
│   ├── files/               # 檔案管理頁面
│   ├── fine-tune/           # LoRA 微調配置
│   ├── link/                # 外部連結管理
│   ├── llm-wiki/            # LLM Wiki 編輯器
│   ├── proactive/           # 主動建議系統
│   ├── recommendation/      # 工具推薦面板
│   ├── wiki/                # Wiki 管理
│   ├── devtools/            # Trace/Span 時間線
│   ├── style/               # 程式碼風格遷移
│   ├── layout/              # 佈局元件（標題列/側邊欄/命令面板）
│   ├── help/                # 幫助面板
│   ├── onboarding/          # 引導精靈
│   ├── notification/        # 通知中心
│   ├── search/              # 對話搜尋
│   ├── common/              # 通用元件
│   └── shared/              # 共享元件
│
├── pages/                    # 頁面元件（22 個頁面）
│   ├── ChatPage.tsx
│   ├── KnowledgePage.tsx
│   ├── KnowledgeHubPage.tsx
│   ├── MemoryPage.tsx
│   ├── WorkflowPage.tsx
│   ├── WorkflowMarketplace.tsx
│   ├── GatewayPage.tsx
│   ├── GatewayLinkPage.tsx
│   ├── LinkPage.tsx
│   ├── FilesPage.tsx
│   ├── FineTunePage.tsx
│   ├── SkillsPage.tsx
│   ├── WikiPage.tsx
│   ├── WikiEditorPage.tsx
│   ├── WikiGraphPage.tsx
│   ├── LlmWikiPage.tsx
│   ├── LlmWikiEditorPage.tsx
│   ├── IngestPage.tsx
│   ├── QuickBarPage.tsx
│   ├── SettingsPage.tsx
│   └── DevTools/
│       ├── TraceExplorer.tsx
│       ├── BenchmarkRunner.tsx
│       └── ToolRecommender.tsx
│
├── hooks/                    # React hooks（10 個）
├── lib/                      # 工具函數（含 Web Worker）
├── types/                    # TypeScript 類型定義（22 個）
├── sdk/                      # SDK（含 Python SDK）
└── i18n/                     # 11 種語言翻譯
```

## 快速開始

### 下載預建構版本

訪問 [Releases](https://github.com/polite0803/AxAgent/releases) 頁面，下載適合您平台的安裝程式。

### 從原始碼建構

#### 環境要求

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://www.rust-lang.org/) 1.75+
- [npm](https://www.npmjs.com/) 10+
- Windows: [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) + Rust MSVC targets

#### 建構步驟

```bash
# 複製倉庫
git clone https://github.com/polite0803/AxAgent.git
cd AxAgent

# 安裝依賴
npm install

# 開發模式
npm run tauri dev

# 僅建構前端
npm run build

# 建構桌面應用
npm run tauri build
```

建構產物位於 `src-tauri/target/release/`。

### 測試

```bash
# 單元測試
npm run test          # Vitest watch
npm run test:run      # Vitest 單次執行

# E2E 測試
npm run test:e2e      # Playwright
npm run test:e2e:ui   # Playwright UI 模式

# Rust 後端測試
cd src-tauri && cargo nextest run   # cargo-nextest（快 2-3x）
cd src-tauri && cargo test          # 標準測試

# 類型檢查
npm run typecheck     # TypeScript
cd src-tauri && cargo clippy -- -D warnings  # Rust

# 程式碼格式化
npm run format        # dprint
cd src-tauri && cargo fmt

# CI 全量檢查
npm run ci:check
```

---

## 專案結構

```
AxAgent/
├── src/                         # 前端原始碼 (React + TypeScript)
│   ├── components/              # React 元件（24 個模組）
│   │   ├── chat/               # 對話介面（90+ 元件）
│   │   ├── workflow/           # 工作流編輯器元件
│   │   ├── gateway/            # API 閘道元件
│   │   ├── settings/           # 設定面板（40+ 元件）
│   │   ├── terminal/           # 終端元件
│   │   ├── skill/              # 技能編輯器與渲染器
│   │   ├── benchmark/          # 基準測試
│   │   ├── decomposition/      # 技能分解
│   │   ├── files/              # 檔案管理
│   │   ├── fine-tune/          # LoRA 微調
│   │   ├── link/               # 外部連結
│   │   ├── llm-wiki/           # LLM Wiki
│   │   ├── proactive/          # 主動建議
│   │   ├── recommendation/     # 工具推薦
│   │   ├── wiki/               # Wiki 管理
│   │   ├── devtools/           # 開發者工具
│   │   ├── style/              # 程式碼風格
│   │   ├── layout/             # 佈局元件
│   │   ├── help/               # 幫助面板
│   │   ├── onboarding/         # 引導精靈
│   │   ├── notification/       # 通知中心
│   │   ├── search/             # 對話搜尋
│   │   ├── common/             # 通用元件
│   │   └── shared/             # 共享元件
│   ├── pages/                   # 頁面元件（18 個頁面）
│   ├── stores/                  # Zustand 狀態管理（62 個 store）
│   │   ├── domain/            # 核心業務狀態（9 個）
│   │   ├── feature/           # 功能模組狀態（44 個）
│   │   ├── devtools/          # 開發者工具狀態（5 個）
│   │   └── shared/            # 共享狀態（4 個）
│   ├── hooks/                   # React hooks
│   ├── lib/                     # 工具函數（含 Web Worker）
│   ├── types/                   # TypeScript 類型定義
│   ├── sdk/                     # SDK（含 Python SDK）
│   └── i18n/                    # 11 種語言翻譯
│
├── src-tauri/                    # 後端原始碼 (Rust)
│   ├── crates/                  # Rust workspace（18 個 crates）
│   │   ├── agent/             # AI 智慧體核心
│   │   ├── core/              # 資料庫、加密、RAG、MCP
│   │   ├── providers/         # 模型提供商介面卡
│   │   ├── runtime-core/      # 執行時抽象層
│   │   ├── runtime/           # 執行時服務
│   │   ├── rt-workflow/       # 工作流引擎
│   │   ├── rt-messaging/      # 訊息閘道
│   │   ├── rt-webhook/        # Webhook 伺服器
│   │   ├── rt-dashboard/      # 儀表盤外掛
│   │   ├── rt-theme/          # 主題引擎
│   │   ├── gateway/           # API 閘道伺服器
│   │   ├── tools/             # 工具系統
│   │   ├── trajectory/        # 記憶與學習
│   │   ├── telemetry/         # 追蹤與指標
│   │   ├── plugins/           # 外掛系統
│   │   ├── prompt-guard/      # 提示詞注入防護
│   │   ├── migration/         # 資料庫遷移
│   │   └── npm/               # npm 套件解析
│   └── src/                    # Tauri 入口點（70+ 命令模組）
│
├── extension/                  # 瀏覽器擴充功能（Wiki Clipper）
├── e2e/                        # Playwright E2E 測試
├── scripts/                    # 建構與工具腳本
└── website/                    # 專案網站（VitePress）
```

## 資料目錄

```
~/.axagent/                      # 配置目錄
├── axagent.db                   # SQLite 資料庫
├── master.key                   # AES-256 主密鑰
├── vector_db/                   # 向量資料庫 (sqlite-vec)
└── ssl/                         # SSL 憑證

~/Documents/axagent/            # 使用者檔案目錄
├── images/                     # 圖片附件
├── files/                      # 檔案附件
└── backups/                    # 備份檔案
```

---

## 常見問題

### macOS：提示「應用已損壞」或「無法驗證開發者」

由於應用未經過 Apple 簽名：

**1. 允許執行「任何來源」的應用**
```bash
sudo spctl --master-disable
```

然後前往 **系統設定 → 隱私與安全性 → 安全性**，選擇 **任何來源**。

**2. 移除隔離屬性**
```bash
sudo xattr -dr com.apple.quarantine /Applications/AxAgent.app
```

**3. macOS Ventura+ 額外步驟**
前往 **系統設定 → 隱私與安全性**，點擊 **仍要打開**。

---

## 社群

- [LinuxDO](https://linux.do)

## 開源協定

本專案基於 [AGPL-3.0](LICENSE) 協定開源。
