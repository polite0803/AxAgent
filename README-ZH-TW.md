[简体中文](./README.md) | **繁體中文** | [English](./README-EN.md) | [日本語](./README-JA.md) | [한국어](./README-KO.md) | [Français](./README-FR.md) | [Deutsch](./README-DE.md) | [Español](./README-ES.md) | [Русский](./README-RU.md) | [हिन्दी](./README-HI.md) | [العربية](./README-AR.md)

[![AxAgent](https://github.com/polite0803/AxAgent/blob/main/src/assets/image/logo.png?raw=true)](https://github.com/polite0803/AxAgent)

<p align="center">
    <a href="https://www.producthunt.com/products/axagent?embed=true&amp;utm_source=badge-featured&amp;utm_medium=badge&amp;utm_campaign=badge-axagent" target="_blank" rel="noopener noreferrer"><img alt="AxAgent - Lightweight, high-perf cross-platform AI desktop client | Product Hunt" width="250" height="54" src="https://api.producthunt.com/widgets/embed-image/v1/featured.svg?post_id=1118403&amp;theme=light&amp;t=1775627359538"></a>
</p>

## 執行截圖

| 對話圖表渲染 | 服務商與模型 |
|:---:|:---:|
| ![](.github/images/s1-0412.png) | ![](.github/images/s2-0412.png) |

| 知識庫 | 記憶 |
|:---:|:---:|
| ![](.github/images/s3-0412.png) | ![](.github/images/s4-0412.png) |

| Agent-詢問 | API閘道一鍵接入 |
|:---:|:---:|
| ![](.github/images/s5-0412.png) | ![](.github/images/s6-0412.png) |

| 對話模型選擇 | 對話導航 |
|:---:|:---:|
| ![](.github/images/s7-0412.png) | ![](.github/images/s8-0412.png) |

| Agent-權限審批 | API閘道概覽 |
|:---:|:---:|
| ![](.github/images/s9-0412.png) | ![](.github/images/s10-0412.png) |

## 功能特性

### 對話與模型

- **多供應商支援** — 相容 OpenAI、Anthropic Claude、Google Gemini 等所有 OpenAI 相容 API；支援 Ollama 本地模型、OpenClaw/Hermes 遠程網關接入
- **模型管理** — 支援遠端拉取模型列表、自訂參數（溫度、最大 Token、Top-P 等）
- **多金鑰輪詢** — 每個供應商可設定多個 API Key，自動輪換以分散限流壓力
- **串流輸出** — 即時逐 Token 渲染，thinking 區塊可折疊展開
- **訊息版本** — 每條回覆支援多版本切換，方便對比不同模型或參數的效果
- **對話分支** — 從任意訊息節點派生新分支，支援分支間對比
- **對話管理** — 支援置頂、封存、按時間分組、批次操作
- **對話壓縮** — 自動壓縮冗長對話，保留關鍵資訊以節省上下文空間
- **多模型同答案** — 同一問題同時向多個模型提問，支援答案間對比分析
- **分類系統** — 自訂對話分類，支援按主題組織對話

### AI Agent

- **Agent 模式** — 切換至 Agent 模式，AI 可自主執行多步驟任務：讀寫檔案、執行命令、分析程式碼等
- **三級權限** — 預設模式（寫入需審批）、接受編輯（自動批准修改）、完全存取（無提示），安全可控
- **工作目錄沙箱** — Agent 操作嚴格限制在指定工作目錄內，防止越權存取
- **工具審批面板** — 即時展示工具呼叫請求，支援逐條審核、一鍵始終允許或拒絕
- **成本追蹤** — 每次對話即時統計 Token 用量與費用
- **暫停/恢復** — 支援隨時暫停 Agent 任務，審閱後再恢復執行
- **Bash 命令執行** — 支援在沙箱環境中執行 Shell 命令，自動風險驗證

### 多智慧體系統

- **子 Agent 協調** — 支援建立多個子 Agent，形成主從協調架構
- **並行執行** — 支援多 Agent 並行處理任務，提升複雜任務效率
- **對抗辯論** — 支援多 Agent 對抗辯論模式，透過觀點碰撞產生更好的解決方案
- **工作流引擎** — 強大的工作流編排能力，支援條件分支、循環、並行等複雜邏輯
- **團隊角色** — 為不同 Agent 分配特定角色（程式碼審查、測試、文件等），協同完成開發任務

### 技能系統

- **技能市場** — 內建技能市場，瀏覽和安裝社區貢獻的技能
- **技能創建** — 從提案自動建立技能，支援 Markdown 編輯器
- **技能進化** — AI 自動分析和改進現有技能，提升執行效果
- **技能匹配** — 智慧推薦相關技能，自動應用到合適的對話場景
- **本地技能註冊** — 支援自訂本地工具作為技能使用
- **插件鉤子** — 支援 pre/post 鉤子，在技能執行前後注入自訂邏輯
- **原子技能** — 細粒度技能組件，支援複雜工作流的建構
- **技能分解** — 自動將複雜任務分解為可執行的原子技能
- **生成工具** — AI 自動生成和註冊新工具，擴展 Agent 能力

### 工作流系統

- **工作流編輯器** — 可視化拖放式工作流設計器，支援節點連接和配置
- **工作流模板** — 內建多種預設模板，快速啟動常見任務
- **版本管理** — 工作流模板支援版本控制，可回滾到歷史版本
- **工作引擎** — 強大的工作流執行引擎，支援並行、條件和迴圈執行
- **執行歷史** — 詳細記錄工作流執行歷史，支援狀態追蹤和除錯
- **AI 輔助** — AI 輔助工作流設計，自動生成和優化工作流

### 內容渲染

- **Markdown 渲染** — 完整支援程式碼高亮、LaTeX 數學公式、表格、任務清單
- **Monaco 程式碼編輯器** — 程式碼區塊內嵌 Monaco Editor，支援語法高亮、複製、diff 預覽
- **圖表渲染** — 內建 Mermaid 流程圖與 D2 架構圖渲染
- **Artifact 面板** — 程式碼片段、HTML 草稿、Markdown 筆記、報告可在獨立面板中預覽
- **對話檢視器** — 即時顯示對話結構樹狀圖，快速導航到任意訊息

### 搜尋與知識

- **聯網搜尋** — 整合 Tavily、智譜 WebSearch、Bocha 等，搜尋結果附帶引用來源標注
- **本地知識庫（RAG）** — 支援多知識庫，上傳文件後自動解析分段並建立向量索引，對話時語意檢索相關段落
- **知識圖譜** — 支援知識實體關係圖譜，可視化展示知識點之間的關聯
- **記憶系統** — 多命名空間記憶，可手動新增或由 AI 自動提取關鍵資訊
- **全文搜尋** — FTS5 全文搜尋引擎，支援對話、檔案、記憶的快速檢索
- **上下文管理** — 彈性掛載檔案附件、搜尋結果、知識庫片段、記憶條目、工具輸出

### 工具與擴充

- **MCP 協議** — 完整實作 Model Context Protocol，支援 stdio 和 HTTP/WebSocket 兩種傳輸方式
- **OAuth 認證** — 支援 MCP 伺服器 OAuth 認證流程
- **內建工具** — 提供檔案操作、程式碼執行、搜尋等開箱即用的內建工具
- **工具執行面板** — 可視化展示工具呼叫請求與回傳結果
- **LSP 客戶端** — 內建 LSP 協議支援，程式碼智慧補完和診斷

### API 閘道

- **本地 API 閘道** — 內建 OpenAI 相容、Claude、Gemini 等原生介面的本地 API 伺服器
- **外部連結** — 一鍵接入 Claude CLI、OpenCode 等外部工具，自動同步 API 金鑰
- **API 金鑰管理** — 產生、撤銷、啟停存取金鑰，支援描述備注
- **用量統計** — 依金鑰、供應商、日期維度的請求量與 Token 用量分析
- **診斷工具** — 閘道健康檢查、連線測試、請求調試
- **SSL/TLS 支援** — 內建自簽憑證產生，也支援掛載自訂憑證
- **請求日誌** — 完整記錄所有經過閘道的 API 請求與回應
- **設定範本** — 預置 Claude、Codex、OpenCode、Gemini 等常見 CLI 工具的接入設定範本
- **即時通訊** — 支援 WebSocket 即時事件推送，相容 OpenAI Realtime API

### 資料與安全

- **AES-256 加密** — API Key 等敏感資料使用 AES-256-GCM 加密存儲於本地
- **資料目錄隔離** — 應用程式狀態存儲於 `~/.axagent/`，使用者檔案存儲於 `~/Documents/axagent/`
- **自動備份** — 支援定時自動備份到本地目錄、WebDAV 存儲
- **備份還原** — 一鍵從歷史備份還原完整資料
- **對話匯出** — 支援將對話匯出為 PNG 截圖、Markdown、純文字或 JSON 格式
- **儲存空間管理** — 可視化展示磁碟使用情况，清理不需要的檔案

### 桌面體驗

- **主題切換** — 深色/淺色主題，可跟隨系統或手動指定
- **介面語言** — 完整支援簡體中文、繁體中文、英文、日文、韓文、法文、德文、西班牙文、俄文、印地文與阿拉伯文
- **系統托盤** — 關閉視窗時最小化到系統托盤，不中斷後台服務
- **視窗置頂** — 可將主視窗常駐最頂層
- **全局快捷鍵** — 自訂全局快捷鍵，隨時喚起主視窗
- **開機自啟** — 可選擇隨系統自動啟動
- **代理支援** — 支援 HTTP 和 SOCKS5 代理設定
- **自動更新** — 啟動時自動偵測新版本並提示更新
- **命令面板** — `Cmd/Ctrl+K` 快速訪問所有命令和設定

## 平台支援

| 平台 | 架構 |
|------|------|
| macOS | Apple Silicon (arm64), Intel (x86_64) |
| Windows 10/11 | x86_64, arm64 |
| Linux | x86_64 (AppImage/deb/rpm), arm64 (AppImage/deb/rpm) |

## 快速開始

前往 [Releases](https://github.com/polite0803/AxAgent/releases) 頁面下載適合您平台的安裝包。

## 從原始碼建構

### 前置需求

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://www.rust-lang.org/) 1.75+
- [npm](https://www.npmjs.com/) 10+
- Windows 需要 [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) 和 [Rust MSVC targets](https://doc.rust-lang.org/cargo/reference/config.html#cfgtarget)

### 建構步驟

```bash
# 複製儲存庫
git clone https://github.com/polite0803/AxAgent.git
cd AxAgent

# 安裝依賴
npm install

# 在開發模式執行
npm run tauri dev

# 僅建構前端
npm run build

# 建構桌面應用程式
npm run tauri build
```

建構產物位於 `src-tauri/target/release/` 目錄。

### 測試

```bash
# 執行單元測試
npm test

# 執行端對端測試
npm run test:e2e

# 型別檢查
npm run typecheck
```

## 常見問題

### macOS 提示「已損毀」或「無法驗證開發者」

由於應用程式未經 Apple 簽名，macOS 可能會彈出以下提示之一：

- 「AxAgent」已損毀，無法開啟
- 無法開啟「AxAgent」，因為無法驗證開發者

**解決步驟：**

**1. 允許「任何來源」的應用程式執行**

```bash
sudo spctl --master-disable
```

執行後前往「系統設定 → 隱私權與安全性 → 安全性」，確認已勾選「任何來源」。

**2. 移除應用程式的安全隔離屬性**

```bash
sudo xattr -dr com.apple.quarantine /Applications/AxAgent.app
```

> 如果不確定路徑，可將應用程式圖示拖曳到 `sudo xattr -dr com.apple.quarantine ` 後面。

**3. macOS Ventura 及以上版本的額外步驟**

完成上述步驟後，首次開啟時仍可能被攔截。前往 **「系統設定 → 隱私權與安全性」**，在安全性區域點擊 **「仍要開啟」** 即可，後續無需重複操作。

## 社群支援
- [LinuxDO](https://linux.do)

## 授權條款

本專案採用 [AGPL-3.0](LICENSE) 授權條款。