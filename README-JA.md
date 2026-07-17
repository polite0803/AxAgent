# AxAgent

[![Release](https://img.shields.io/github/v/release/polite0803/AxAgent?style=flat-square)](https://github.com/polite0803/AxAgent/releases)
[![Build](https://img.shields.io/github/actions/workflow/status/polite0803/AxAgent/release.yml?style=flat-square)](https://github.com/polite0803/AxAgent/actions)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux%20%7C%20Android%20%7C%20iOS-blue?style=flat-square)
![License](https://img.shields.io/badge/license-AGPL--3.0-green?style=flat-square)

<p align="center">
  <a href="./media/poster-axagent.svg">
    <img src="./media/poster-axagent.svg" alt="AxAgent ポスター" width="80%" />
  </a>
</p>

**AxAgent** は、Tauri 2 ベースのクロスプラットフォーム AI アシスタントデスクトップクライアントです（Windows / macOS / Linux / Android / iOS）。ReAct エージェントエンジン、ビジュアルワークフローオーケストレーション、ローカル RAG ナレッジベース、MCP プロトコル拡張、統合マルチモデルゲートウェイ、ブラウザ自動化、コンピューター制御を統合し、日常の開発・研究・知識管理・自動化のための AI ワークステーションです。

> **言語**: [简体中文](./README.md) | [English](./README-EN.md) | [繁體中文](./README-ZH-TW.md) | [日本語](./README-JA.md) | [한국어](./README-KO.md) | [Français](./README-FR.md) | [Deutsch](./README-DE.md) | [Español](./README-ES.md) | [Русский](./README-RU.md) | [हिन्दी](./README-HI.md) | [العربية](./README-AR.md)

---

## プロジェクトの位置付け

AxAgent は3つの核心的な問題を解決します：

1. **統合マルチモデルアクセスとインテリジェントルーティング** — 単一インターフェースで OpenAI、Anthropic Claude、Google Gemini、Ollama ローカルモデル、および任意の OpenAI 互換 API を使用し、マルチキー割り当て自動ローテーション、タスクタイプ別インテリジェントルーティング、ストリーミング比較に対応
2. **AI の対話から実行へのクローズドループ** — 47+ 内蔵ツール + ビジュアルワークフロー + MCP 拡張 + ブラウザ/コンピューター制御、AI がファイル操作、コード実行、Git 管理、タスクスケジューリングを実現
3. **ローカルファーストのデータ主権** — 会話、ナレッジベース、メモリ、設定はすべてローカル SQLite データベースに保存され、APIキーは AES-256-GCM で暗号化。サードパーティクラウドサービスなしでコア機能が動作

---

## コア機能

### マルチモデルエンジン

- **9つのプロバイダーアダプター**: OpenAI (Chat Completions + Responses + Realtime)、Anthropic Claude、Google Gemini、Ollama (GGUF ローカルモデル管理含む)、OpenClaw、Hermes、およびすべての OpenAI 互換 API
- **マルチキーローテーション**: 同一プロバイダーの複数 API キー、割り当てベースの自動ローテーション、単一キー制限時の自動フェイルオーバー
- **インテリジェントルーティング**: タスクタイプ（コードレビュー / 要約 / 翻訳 / 一般）に応じた自動モデル選択、カスタムルール対応
- **プロバイダーヘルスモニタリング**: 成功率、レイテンシ、可用性のリアルタイム追跡、段階的自動フォールバック
- **AI 画像生成**: DALL-E 3 および Flux (Replicate) マルチサイズプリセット
- **リアルタイム音声**: OpenAI Realtime API ベースの WebSocket 音声会話、割り込みおよびストリーミング文字起こし対応

### エージェントシステム (ReAct エンジン)

- **階層型プランナー** (`hierarchical_planner`): 複雑なタスクを Phase → Task の構造化プランに分解し、DAG トポロジカル実行にコンパイル
- **深層リサーチ** (`deep_research`): マルチソース検索オーケストレーション（検索計画、実行、コンテンツ統合、引用追跡）
- **ファクトチェッカー** (`fact_checker`): AI 駆動の事実検証、ソース分類器と信頼性評価を含む
- **思考の木** (`tree_of_thoughts`): 複数経路の推論探索、分岐評価とバックトラッキング
- **リフレクター** (`reflector`): 実行後の自己評価と改善提案
- **自己検証** (`self_verifier`): 推論結果の自動検証、循環検出付き
- **エラーリカバリー** (`error_recovery_engine`): エラータイプ分類 → リカバリー戦略選択 → 自動リトライまたは計画調整、指数バックオフ対応
- **A/B テスト** (`ab_testing`): 異なる推論戦略の比較評価
- **評価システム** (`evaluator`): 組み込みベンチマークフレームワーク
- **LoRA ファインチューニング** (`fine_tune`): 組み込みトレーニングパイプライン、LoRA アダプター管理
- **RL オプティマイザー** (`rl_optimizer`): 経験フィードバックに基づくポリシー強化学習

**マルチエージェント協調**:

- マスター-スレーブ協調アーキテクチャ、サブエージェント並列実行、依存関係認識スケジューリング
- エージェント間の情報交換のための共有ブラックボード
- 敵対的ディベートモード（Pro/Con ラウンドと論点強度スコアリング）
- マルチプロセスエージェントクラスターの Swarm モード
- プロアクティブモード：エージェントが自発的に提案と操作を開始可能

**コンピューター制御**: AI 駆動のマウスクリック、キーボード入力、画面スクロール。3段階の権限（デフォルト/編集受付/フルアクセス）、サンドボックスパス分離

**ブラウザ自動化**: CDP プロトコルによるブラウザ制御、ナビゲーション、スクリーンショット、クリック、フォーム入力、テキスト抽出に対応

### スキルシステム

- **スキルマーケットプレイス**: コミュニティスキルの閲覧とインストール
- **AI 支援作成**: 自然言語提案からスキル構造を自動生成 (`skill:create`)
- **スキル進化** (`evolution_engine`): 実行フィードバックに基づくスキルの自動分析と改善
- **セマンティックマッチング**: コンテキストに応じたセマンティックスキル推薦
- **スキル分解** (`skill_decomposition`): 複雑なタスクを原子的スキルの組み合わせに自動分解
- **生成ツール**: AI が生成して登録する新しいツール
- **サンドボックス実行**: スキルは隔離されたサンドボックスで安全に実行

### ビジュアルワークフロー

ReactFlow 12 ベースのドラッグ＆ドロップ DAG ワークフローエディター：

- **17種類のノード**: トリガー、エージェント、LLM 呼び出し、条件分岐、並列フォーク、ループ、マージ、遅延、ツール呼び出し、コード実行、サブワークフロー、ベクトル検索、ドキュメント解析、検証、終了、ビジネスルール、エージェントロール
- **Kahn トポロジカルソート実行**: 自動循環依存検出、並列パイプラインスケジューリング
- **組み込みテンプレート**: コードレビュー、バグ修正、ドキュメント、テスト、リファクタリング、探索、パフォーマンス分析、セキュリティ監査、機能開発
- **YAML シリアライゼーション**: ワークフロー定義のインポート/エクスポート
- **バージョン管理**: テンプレートバージョン管理
- **AI 支援設計**: AI 支援のワークフロー設計とノード推薦

### ナレッジ管理

- **マルチナレッジベース RAG**: ドキュメントアップロード → 自動解析（PDF/DOCX/XLSX/PPTX/TXT）→ チャンキング → ベクトルインデックス
- **ハイブリッド検索**: ベクトル類似度（sqlite-vec + candle ローカル埋め込み）+ BM25 全文検索（FTS5）、ハイブリッドランキング
- **Self-RAG**: 検索結果の自動リフレクションと検証
- **リランキング**: Cross-encoder による結果リランキング
- **ナレッジグラフ**: エンティティ抽出 → 関係構築 → ビジュアルグラフ
- **ファイル監視**: `notify` ベースのリアルタイムファイル変更監視、自動増分インデックス
- **LLM Wiki**: AI 支援 Wiki コンパイラとバリデーター

### メモリシステム

- **マルチ名前空間メモリ**: プロジェクト/トピック分離、手動入力と AI 自動抽出に対応
- **永続化統合**: Honcho および Mem0 クローズドループメモリ
- **ユーザープロファイル**: コーディングスタイル、技術スタックの好み、コミュニケーションスタイルの自動学習
- **スタイル転送**: コードスタイル特徴の抽出 → AI 生成コードへの適用
- **ドリーム統合**: メモリ断片と行動パターンのバックグラウンド自動統合、構造化知識の生成
- **プロジェクトメモリ**: プロジェクト単位のコンテキスト永続化

### API ゲートウェイ

`axum` ベースの HTTP + WebSocket ゲートウェイを内蔵：

- **互換エンドポイント**: OpenAI `/v1/chat/completions`、Claude Messages API、Gemini API、および OpenAI Responses と Realtime WebSocket
- **キー管理**: アクセスキーの生成、失効、有効/無効切り替え、有効期限対応
- **使用量追跡**: キー/プロバイダー/日付別のリクエスト数とトークン消費統計、Prometheus メトリクスエクスポート
- **レート制限**: `governor` ベースのトークンバケットアルゴリズム
- **SSL/TLS**: 組み込み自己署名証明書（`rcgen`）、カスタム証明書対応
- **外部リンク**: Claude CLI、OpenCode などの外部ツールとワンクリック統合、API キー自動同期
- **リアルタイムチケット**: HMAC ベースの一時認証チケット、WebSocket 接続の安全な引き渡し

### メッセージングプラットフォーム統合

`rt-messaging` によるマルチプラットフォームゲートウェイ。**DingTalk、Feishu、QQ、Slack、WeChat、WhatsApp、Telegram、Discord** のメッセージ受信、コマンド解析、AI 自動返信に対応。

### ツールシステム

47+ の内蔵ツール、`Tool` trait で統一的に登録：

| カテゴリ           | ツール                                                                                                                                                                                                     |
| ------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| ファイル操作       | `file_read`, `file_write`, `file_edit`, `file_system`                                                                                                                                                      |
| コード実行         | `bash`, `repl`                                                                                                                                                                                             |
| 検索               | `grep`, `glob`                                                                                                                                                                                             |
| ブラウザ           | `browser` (CDP)                                                                                                                                                                                            |
| コンピューター制御 | `computer_use` (マウス/キーボード/スクリーンショット)                                                                                                                                                      |
| Web                | `web_search`, `web_fetch`                                                                                                                                                                                  |
| ナレッジベース     | `knowledge`, `document`                                                                                                                                                                                    |
| Git                | `git` (commit/push/branch/diff)                                                                                                                                                                            |
| 開発ツール         | `lsp`, `workspace`                                                                                                                                                                                         |
| タスク管理         | `plan`, `task_system`, `todo_write`, `cron`                                                                                                                                                                |
| メッセージング     | `push_notification`, `messaging`                                                                                                                                                                           |
| データベース       | `database`                                                                                                                                                                                                 |
| ストレージ         | `storage`                                                                                                                                                                                                  |
| その他             | `agent`, `agent_memory`, `context`, `export`, `integration`, `media`, `media_delivery`, `migration_tool`, `monitor`, `obsidian`, `ocr`, `personality`, `shared_path`, `system_info`, `testing`, `worktree` |

### MCP プロトコル

`rmcp` ベースの完全な MCP (Model Context Protocol) 実装：

- **トランスポート**: stdio サブプロセス + Streamable HTTP + WebSocket
- **OAuth 認証**: MCP サーバーの OAuth 認可フロー対応
- **ツールディスカバリー**: MCP サーバーが公開するツールの自動検出と登録
- **MCP マネージャー**: サーバーライフサイクル管理、ヘルスチェック、自動再接続

### プラグインシステム

OpenClaw 互換の3層プラグインアーキテクチャ（内蔵/バンドル/外部）：

- npm パッケージインストール、マーケットプレイス UI による検索とインストール
- プラグインマニフェスト定義、権限宣言、サンドボックス分離実行
- カスタムツール登録、エージェントプロバイダー、Hook インターセプト
- スキルインストーラー：プラグインパッケージからスキルをスキルシステムにインストール

### セキュリティ

- **AES-256-GCM 暗号化**: API キーと機密設定のローカル暗号化ストレージ（`crypto` crate）
- **プロンプトインジェクション防御**: 4段階防御パイプライン（`prompt-guard`）— パターン検出 → デリミタエスケープ → XML ラッパー → 信頼ラベル、会話/プロンプト構築/Git/RAG の全チェーンに統合
- **SSRF 防御**: URL 安全性チェック、内部ネットワークアドレスへのリクエストをブロック
- **コンテンツフィルタリング**: マルチタイプコンテンツ安全性フィルタリング
- **レート制限**: ツール呼び出しと API リクエストのトークンバケット制限
- **サーキットブレーカー**: 連続失敗時の自動サーキットブレーク
- **アクセス制御**: ポリシーベースのツールアクセス権限制御
- **サンドボックス分離**: エージェントとスキルの実行環境分離

### 開発者ツール

- **分散トレーシング** (`telemetry`): OpenTelemetry 統合、Span/Trace 可視化
- **構造化ログ**: tracing-subscriber + chrono タイムスタンプ
- **リプレイデバッグ**: エージェント実行軌跡の記録（`trajectory_recorder`）と再生
- **DevTools パネル**: Trace Explorer タイムラインビューアー、Benchmark Runner、Tool Recommender
- **ベンチマーク**: Criterion benchmarks（tool_exec / llm_call / search）
- **CI チェック**: `npm run ci:check` 型チェック、lint、フォーマット検証の統合

### デスクトップとモバイル体験

- **レスポンシブレイアウト**: CSS ブレークポイントによるデスクトップ/タブレット/モバイル適応（3段階：`desktop` / `tablet` / `mobile`）
- **11言語**: 簡体字中国語、繁体字中国語、英語、日本語、韓国語、フランス語、ドイツ語、スペイン語、ロシア語、ヒンディー語、アラビア語
- **テーマエンジン** (`rt-theme`): ダーク/ライトテーマ + 複数プリセット（21th 等幅フォントテーマ含む）、Ant Design 6 深層カスタマイズ
- **Monaco エディター**: シンタックスハイライト、差分プレビュー、多言語対応
- **xterm.js ターミナル**: WebLinks、Unicode 11、検索
- **仮想スクロール**: @tanstack/react-virtual + react-virtuoso
- **チャートレンダリング**: D2 + Mermaid + Recharts
- **Global Copy Menu**: カスタムテキスト選択コピーメニュー、ネイティブコンテキストメニュー抑制
- **Command Palette**: Ctrl+K グローバルコマンドパレット
- **システムトレイ + グローバルショートカット + 自動起動**: 非侵襲的なバックグラウンド動作
- **自動更新**: 設定可能間隔の GitHub Releases バージョンチェック
- **プロキシ対応**: HTTP / SOCKS5 プロキシ設定
- **クラウドワークスペース**: S3 および WebDAV ストレージ同期、競合検出と双方向同期

### モバイル

- Android APK/AAB（arm64-v8a, armeabi-v7a, x86_64）
- iOS IPA（arm64）
- モバイル専用対応：セーフエリアインセット、ボトムナビゲーション、ドロワーナビゲーション

---

## 技術アーキテクチャ

### 技術スタック

| レイヤー                     | 技術                                     | バージョン |
| ---------------------------- | ---------------------------------------- | ---------- |
| デスクトップフレームワーク   | Tauri                                    | 2.11       |
| フロントエンドフレームワーク | React                                    | 19         |
| 型システム                   | TypeScript                               | 7          |
| UI ライブラリ                | Ant Design                               | 6          |
| CSS フレームワーク           | TailwindCSS                              | 4          |
| 状態管理                     | Zustand                                  | 5          |
| ルーティング                 | React Router                             | 7          |
| コードエディター             | Monaco Editor                            | 0.55       |
| ターミナル                   | xterm.js                                 | 6          |
| ワークフローエディター       | ReactFlow                                | 12         |
| チャート                     | D2 + Mermaid + Recharts                  |            |
| アニメーション               | Framer Motion                            | 12         |
| 仮想スクロール               | @tanstack/react-virtual + react-virtuoso |            |
| ドラッグ＆ドロップ           | @dnd-kit                                 | 6          |
| Markdown レンダリング        | markstream-react + stream-markdown       |            |
| i18n                         | i18next + react-i18next                  |            |
| ビルドツール                 | Vite                                     | 8          |
| テスト                       | Vitest + Playwright                      |            |
| フォーマット                 | dprint（TS/JSON/Markdown/TOML）+ rustfmt |            |
| Lint                         | ESLint + Oxlint + Clippy                 |            |

### バックエンドアーキテクチャ: Harness 依存性注入

Rust workspace アーキテクチャ、**32 crate**、**Harness DI パターン**に準拠：

> すべての crate は axagent-harness が定義する trait インターフェースを通じて疎結合され、実行時に axagent-runtime が依存関係を組み立てて注入。
> 依存方向：`具象実装 → harness ← 呼び出し元`

**harness** はアーキテクチャの基盤 — ゼロビジネスロジック、ゼロ具象実装、trait 定義、純粋データ DTO、定数、統一エラータイプのみを含む。他のすべての crate から依存され、自身はどの axagent-* crate にも依存しない（200+ trait 定義、Agent/Provider/Tool/RAG/Storage/MCP/Plugins/Security/Observability/Memory/Learning/Browser/Messaging などをカバー）。

```
src-tauri/crates/
├── harness/          # アーキテクチャ基盤 — trait インターフェース、DTO、エラータイプ、DI 契約
├── entities/         # SeaORM エンティティモデル
├── dao/              # データアクセス層（CRUD）
├── migration/        # データベースマイグレーション
├── crypto/           # AES-256-GCM 暗号化/復号と鍵管理
├── credential/       # 認証情報の安全な保存
├── storage/          # ファイルストレージ抽象化（ローカル/S3/WebDAV）、ZIP 読み書き
├── cache/            # インメモリキャッシュ層
├── disk-cache/       # ディスクファイルキャッシュ
├── search/           # 検索エンジン（FTS5 + sqlite-vec + candle ローカル埋め込み）
├── document-parser/  # ドキュメントテキスト抽出（PDF/DOCX/XLSX/PPTX）
├── kit/              # 汎用ユーティリティ（パス/エンコーディング/ハッシュ/日付）
├── runtime-core/     # ランタイム共通型、設定定数
├── runtime/          # ランタイムサービスオーケストレーション — 全 30+ crate を組み立てる DI コンテナ
├── rt-workflow/      # ワークフローエンジン — DAG オーケストレーション、ノード実行器、YAML シリアライゼーション
├── rt-messaging/     # メッセージングプラットフォームゲートウェイ — DingTalk/Feishu/QQ/Slack/WeChat/WhatsApp/Telegram/Discord
├── rt-webhook/       # 汎用 Webhook サーバー
├── rt-dashboard/     # ダッシュボードプラグインフレームワーク
├── rt-theme/         # テーマエンジン
├── agent/            # AI エージェントコア — 80+ モジュール
│                     #   ReActエンジン/階層型計画/深層リサーチ/ファクトチェック/思考の木/
│                     #   リフレクション/自己検証/エラーリカバリー/RL最適化/LoRAファインチューニング/
│                     #   評価/ツール推薦/A/Bテスト/コーディネーター/ブラックボード/ビジョンパイプライン/
│                     #   Web検索/学術検索/Wikiコンパイルなど
├── orchestrator/     # エージェントオーケストレーション — マルチエージェントスケジューリング、DAG 分解、動的サブグラフ実行
├── providers/        # モデルプロバイダーアダプター
├── tools/            # ツールシステム — Tool trait/レジストリ/オーケストレーション/ストリーミング/サンドボックス/47+内蔵ツール
├── gateway/          # API ゲートウェイ — axum HTTP/WS サーバー、OAuth、レート制限、Prometheus
├── mcp/              # MCP プロトコル — stdio + Streamable HTTP、rmcp ベース
├── trajectory/       # 学習システム — メモリ/スキル進化/ユーザープロファイル/ドリーム統合
├── plugins/          # プラグインシステム — OpenClaw 互換、npm パッケージインストール、マーケットプレイス
├── telemetry/        # オブザーバビリティ — OpenTelemetry、構造化ログ、ランタイムメトリクス
├── prompt-guard/     # プロンプトインジェクション防御 — L1-L4 多段検出パイプライン
├── npm/              # npm レジストリクライアント
└── schema-gen/       # データベーススキーマ生成ツール
```

### フロントエンドアーキテクチャ

```
src/
├── pages/            # ページ（サブページ含む 23+）
│   ├── ChatPage           # チャットインターフェース — サイドバー/メッセージストリーム/Agent パネル/マルチタブ
│   ├── DashboardPage      # ダッシュボード — 使用統計/モデル分布/トレンドチャート
│   ├── WorkflowPage       # ワークフローエディター — ReactFlow DAG ビジュアライゼーション
│   ├── KnowledgeHubPage   # ナレッジベース管理 — ドキュメントアップロード/インデックス/検索
│   ├── MemoryPage         # メモリ管理
│   ├── SkillsPage         # スキルマーケットプレイス
│   ├── SettingsPage       # 設定パネル — 40+ 設定項目
│   ├── TerminalPage       # 内蔵ターミナル — xterm.js
│   ├── FilesPage          # ファイル管理
│   ├── GatewayLinkPage    # API ゲートウェイと外部リンク管理
│   ├── QuickBarPage       # クイックバー（独立ウィンドウ）
│   ├── DynamicUIManagerPage / DynamicPageViewer  # 動的 UI エンジン
│   ├── WikiGraphPage / WikiEditPage / IngestPage # LLM Wiki
│   ├── LearningGraphPage  # 学習グラフ
│   ├── FineTunePage       # LoRA ファインチューニング
│   ├── PersonaPage        # ペルソナ管理
│   ├── WorkflowMarketplace # ワークフローマーケットプレイス
│   ├── DevTools/          # TraceExplorer / BenchmarkRunner / ToolRecommender
│   └── Workflow/          # WorkflowListPage
│
├── components/       # 28 モジュール、450+ コンポーネント
│   ├── chat/         # チャット（メッセージストリーム/入力/ChatView/TabBar/RightPanel/添付ファイル/ツール呼び出し表示）
│   ├── layout/       # レイアウト — 17 コンポーネント
│   │                 #   AppInitializer / Sidebar / ContentArea / TitleBar /
│   │                 #   CommandPalette / GlobalCopyMenu / GlobalErrorBoundary /
│   │                 #   GlobalStatusBar / ErrorNotificationToast / AppHeader /
│   │                 #   BackendStatusIndicator / IpcReconnectBanner /
│   │                 #   ModuleErrorBoundary / NotificationBell / UserProfileModal など
│   ├── agent/        # Agent パネル/エントリ/ミニパネル
│   ├── workflow/     # ワークフローエディター（ノード/エッジ/パネル/テンプレート/AIアシスト）
│   ├── settings/     # 設定パネル（40+ サブコンポーネント）
│   ├── skill/        # スキルエディター/レンダラー/フローティングパネル
│   ├── dynamicUI/    # 動的 UI コンポーネントレジストリ（26 内蔵コンポーネント）
│   ├── gateway/      # API ゲートウェイ管理
│   ├── files/        # ファイル管理
│   ├── terminal/     # ターミナルコンポーネント
│   ├── search/       # 検索インターフェース
│   ├── benchmark/    # ベンチマークパネル
│   ├── decomposition/# スキル分解とツール生成
│   ├── devtools/     # Trace/Span タイムライン + RL Training パネル
│   ├── approval/     # 承認ワークフロー UI
│   ├── recommendation/ # ツール/モデル推薦
│   ├── onboarding/   # WelcomeWizard / InteractiveTutorial
│   ├── help/         # ヘルプパネル
│   ├── notification/ # 通知コンポーネント
│   ├── proactive/    # プロアクティブ提案
│   ├── llm-wiki/     # LLM Wiki コンポーネント
│   ├── wiki/         # Wiki コンポーネント
│   ├── fine-tune/    # ファインチューニング UI
│   ├── trace/        # Trace コンポーネント
│   ├── style/        # スタイル/テーマ
│   ├── shared/       # 共有コンポーネント（ErrorBoundary / PageContextProvider）
│   └── common/       # 共通コンポーネント（Icon など）
│
├── stores/           # Zustand 状態管理
│   ├── domain/       # 10 コアビジネスストア（会話/ストリーム/圧縮/設定/マルチモデルなど）
│   ├── feature/      # 48 機能モジュールストア（エージェント/ワークフロー/ナレッジ/スキル/ゲートウェイ/メモリ/ターミナルなど）
│   └── devtools/     # 4 開発者ツールストア
│
├── hooks/            # React Hooks（ショートカット/コマンドパレット/レスポンシブ/スクロールバー/テーマ/アバターなど）
├── lib/              # ユーティリティライブラリ（invoke/pageRegistry/shortcuts/skillLifecycle/
│                     #   chartGenerator/codeExecutor/tokenEstimator/workflowLayout など 45+ モジュール）
├── types/            # TypeScript 型定義
├── theme/            # Shadcn テーマエンジン
├── i18n/             # 11言語翻訳ファイル（zh-CN/zh-TW/en-US/ja/ko/fr/de/es/ru/hi/ar）
├── constants/        # 定数と機能フラグ
└── sdk/              # 外部統合 SDK
```

### 機能フラグ

プロジェクトは `featureFlags.ts` でプログレッシブ機能ロールアウトを管理：

| フラグ              | 状態 | 説明                                            |
| ------------------- | ---- | ----------------------------------------------- |
| `AGENT_IN_THE_LOOP` | ✅   | グローバル Agent Panel + ページコンテキスト注入 |
| `DYNAMIC_UI`        | ✅   | 動的 UI ビルダーエンジン                        |
| `SELF_EVOLUTION_UI` | ❌   | 自己進化フロントエンド制御パネル                |
| `NL_EXTENSION`      | ❌   | 自然言語駆動の動的ビジネス拡張                  |

### Tauri プラグイン

| プラグイン          | 用途                                 |
| ------------------- | ------------------------------------ |
| `autostart`         | 起動時自動起動                       |
| `clipboard-manager` | クリップボード読み書き               |
| `dialog`            | ファイル選択ダイアログ               |
| `fs`                | ファイルシステムアクセス             |
| `global-shortcut`   | グローバルショートカット登録         |
| `notification`      | システム通知                         |
| `opener`            | 外部リンク/ファイルオープン          |
| `process`           | プロセス管理                         |
| `updater`           | 自動更新                             |
| `mcp-bridge`        | MCP プロトコルブリッジ（非 Android） |

---

## データディレクトリ

```
~/.axagent/                    # アプリケーション設定
├── axagent.db                 # SQLite メインデータベース (SeaORM)
├── master.key                 # AES-256 マスターキー
├── vector_db/                 # sqlite-vec ベクトルインデックス
└── ssl/                       # 自己署名 SSL 証明書

~/Documents/axagent/          # ユーザーファイル
├── images/                   # 画像添付
├── files/                    # ファイル添付
└── backups/                  # 自動バックアップ
```

---

## クイックスタート

### 前提条件

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://www.rust-lang.org/) 1.75+（edition 2024）
- [npm](https://www.npmjs.com/) 10+
- Windows: [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)（MSVC + Windows SDK）
- macOS: Xcode Command Line Tools
- Linux: `build-essential` + `libwebkit2gtk-4.1-dev` + `libssl-dev`

### 開発

```bash
git clone https://github.com/polite0803/AxAgent.git
cd AxAgent
npm install
npm run tauri dev      # 開発モード（Vite HMR + Tauri ウィンドウ）
```

### ビルド

```bash
npm run tauri build    # デスクトッププロダクションビルド

npm run tauri:android:build   # Android ビルド
npm run tauri:ios:build       # iOS ビルド
```

デスクトップビルド成果物は `src-tauri/target/release/` にあります。

### テスト

```bash
npm run test           # フロントエンドユニットテスト（Vitest watch）
npm run test:run       # フロントエンドユニットテスト（単一実行）
npm run test:e2e       # E2E テスト（Playwright）

# Rust バックエンドテスト
cd src-tauri && cargo nextest run
cd src-tauri && cargo test

# 型チェック & Lint
npm run typecheck
cd src-tauri && cargo clippy -- -D warnings
npm run format         # dprint フォーマット
npm run lint:eslint    # ESLint チェック
npm run contracts      # API 契約チェック

# 完全 CI チェック
npm run ci:check
```

### スクリプト

| コマンド                 | 用途                             |
| ------------------------ | -------------------------------- |
| `npm run bump`           | インタラクティブバージョンアップ |
| `npm run docs`           | TypeDoc ドキュメント生成         |
| `npm run skill:create`   | 新規スキルスキャフォールド作成   |
| `npm run skill:validate` | スキル定義の検証                 |
| `npm run check:types`    | 型一貫性チェック                 |

---

## プラットフォームサポート

| プラットフォーム | アーキテクチャ                        |
| ---------------- | ------------------------------------- |
| Windows          | x86_64, ARM64                         |
| macOS            | Apple Silicon (arm64), Intel (x86_64) |
| Linux            | x86_64, ARM64                         |
| Android          | arm64-v8a, armeabi-v7a, x86_64        |
| iOS              | arm64                                 |

---

## ライセンス

本プロジェクトは [AGPL-3.0-only](LICENSE) ライセンスの下でオープンソース公開されています。

---

## 謝辞

AxAgent は多くの優れたオープンソースプロジェクトの上に構築されています：

- [Tauri](https://tauri.app/) — クロスプラットフォームデスクトップフレームワーク
- [React](https://react.dev/) + [Ant Design](https://ant.design/) — フロントエンド UI
- [SeaORM](https://www.sea-ql.org/SeaORM/) — Rust ORM
- [sqlite-vec](https://github.com/asg017/sqlite-vec) — ベクトル検索
- [candle](https://github.com/huggingface/candle) — ローカル埋め込み推論
- [rmcp](https://github.com/nicholasxjy/rmcp) — Rust MCP SDK
- [ReactFlow](https://reactflow.dev/) — ビジュアルワークフローエディター
- [axum](https://github.com/tokio-rs/axum) — HTTP フレームワーク
- [Monaco Editor](https://microsoft.github.io/monaco-editor/) — コードエディター
- [xterm.js](https://xtermjs.org/) — ターミナルエミュレーター
- [Zustand](https://zustand.docs.pmnd.rs/) — 状態管理
- [Framer Motion](https://www.framer.com/motion/) — アニメーションライブラリ
- [Recharts](https://recharts.org/) — チャートライブラリ
