# AxAgent

[![Release](https://img.shields.io/github/v/release/polite0803/AxAgent?style=flat-square)](https://github.com/polite0803/AxAgent/releases)
[![Build](https://img.shields.io/github/actions/workflow/status/polite0803/AxAgent/release.yml?style=flat-square)](https://github.com/polite0803/AxAgent/actions)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux%20%7C%20Android%20%7C%20iOS-blue?style=flat-square)
![License](https://img.shields.io/badge/license-AGPL--3.0-green?style=flat-square)

**AxAgent** は、**Windows / macOS / Linux / Android / iOS** の5つのプラットフォームに対応したオープンソースのクロスプラットフォームAIアシスタントデスクトップクライアントです。単なるチャットインターフェースにとどまらず、ReActエージェントエンジン、ビジュアルワークフローオーケストレーション、ローカルRAG知識ベース、MCPプロトコル拡張、マルチモデル統合ゲートウェイ、ブラウザ自動化、コンピュータ制御などを統合し、日常的な開発、研究、知識管理、自動化タスクのためのAIワークステーションとして機能します。

> **言語**: [简体中文](./README.md) | [English](./README-EN.md) | [繁體中文](./README-ZH-TW.md) | [日本語](./README-JA.md) | [한국어](./README-KO.md) | [Français](./README-FR.md) | [Deutsch](./README-DE.md) | [Español](./README-ES.md) | [Русский](./README-RU.md) | [हिन्दी](./README-HI.md) | [العربية](./README-AR.md)

---

## プロジェクトの位置付け

AxAgentは3つの中核的問題を解決します：

1. **マルチモデル統合スケジューリング**: 単一のインターフェースでOpenAI、Anthropic Claude、Google Gemini、Ollamaローカルモデル、および任意のOpenAI互換APIを同時に使用し、マルチキーローテーション、インテリジェントモデルルーティング、ストリーミング比較をサポート
2. **AI機能のツール化**: AIを「会話」から「実行」へ拡張——47以上の組み込みツール、ビジュアルワークフロー、MCP拡張、ブラウザ自動化、コンピュータ制御を通じて、AIが直接ファイル操作、コード実行、Git管理、タスクスケジューリングを実行
3. **ローカルファーストのデータ主権**: AI会話、知識ベース、メモリ、設定ファイルはすべてローカルSQLiteデータベースに保存され、APIキーはAES-256-GCMで暗号化。サードパーティのクラウドサービスなしでコア機能を実行可能

---

## コア機能

### マルチモデルエンジン

- **9種類のプロバイダーアダプター**: OpenAI (Chat Completions + Responses + Realtime)、Anthropic Claude、Google Gemini、Ollama (GGUF管理含む)、OpenClaw、Hermes、およびすべてのOpenAI互換API
- **マルチキーローテーション**: 同一プロバイダーに複数のAPIキーを設定し、クォータに基づいて自動ローテーション、単一キーのレート制限中断を回避
- **インテリジェントルーティング**: タスクタイプ（コードレビュー/要約/翻訳/汎用）に応じて最適なモデルを自動選択、カスタムルーティングルールをサポート
- **プロバイダー健全性監視**: 各プロバイダーの成功率、レイテンシ、可用性をリアルタイム追跡、階層型自動デグラデーション（ProviderTier）
- **AI画像生成**: DALL-E 3とFlux (Replicate) の複数サイズプリセット
- **リアルタイム音声**: OpenAI Realtime APIベースのWebSocket音声会話、割り込みとストリーミング文字起こしをサポート

### エージェントシステム

エージェントシステム全体が **ReAct (Reasoning + Acting) エンジン** 上に構築されており、以下の実装済みサブシステムが含まれます：

- **階層型プランナー** (`hierarchical_planner`): 複雑なタスクを依存関係付きの Phase → Task 構造化計画に分解し、DAGトポロジカル実行にコンパイル
- **深層リサーチ** (`deep_research`): マルチソース検索オーケストレーション（検索計画 (`search_planner`)、検索実行 (`search_orchestrator`)、コンテンツ統合 (`content_synthesizer`)、引用追跡 (`citation_tracker`)）
- **ファクトチェッカー** (`fact_checker`): AI駆動の事実検証（ソース分類器 (`source_classifier`)、ソース検証器 (`source_validator`)、信頼性評価器 (`credibility_evaluator`)）
- **思考の木** (`tree_of_thoughts`): マルチパス推論探索、分岐評価とバックトラッキング
- **リフレクター** (`reflector`): タスク実行後の自己評価と改善提案生成
- **自己検証器** (`self_verifier`): 推論結果の自動検証、循環検出 (`cycle_detector`) による無限推論の防止
- **エラー復旧** (`error_recovery_engine`): エラータイプ分類 → 復旧戦略選択 → 自動リトライまたは計画調整、指数バックオフ対応
- **A/Bテスト** (`ab_testing`): 異なる推論戦略の比較評価
- **評価システム** (`evaluator`): 組み込みベンチマークフレームワーク（データセット、メトリクス、レポート生成）
- **LoRAファインチューニング** (`fine_tune`): 組み込みトレーニングパイプライン、LoRAアダプター管理
- **RL最適化器** (`rl_optimizer`): 経験フィードバックベースの方策強化学習（経験リプレイ、方策勾配）
- **ツール推薦器** (`tool_recommender`): コンテキストベースのツール使用パターン分析と推薦

**マルチエージェント協調**:

- マスタースレーブ協調アーキテクチャ (`coordinator`)、子エージェント並列実行、依存関係認識スケジューリング
- エージェント間情報交換用の共有ブラックボード (`shared_blackboard`)
- 対抗的討論モード、Pro/Conラウンドと論点強度スコアリング
- Swarmクラスターモード、マルチプロセスエージェントクラスター（権限同期と自動再接続対応）
- プロアクティブモード (`proactive_mode`): エージェントが自発的に提案と操作を開始可能

**コンピュータ制御**: AI駆動のマウスクリック、キーボード入力、画面スクロール、3段階権限レベル（デフォルト/編集受付/フルアクセス）、サンドボックスパス分離

**ブラウザ自動化**: CDPプロトコル経由でブラウザを制御、ナビゲーション、スクリーンショット、クリック、フォーム入力、テキスト抽出、ページ状態監視をサポート

### スキルシステム

- **スキルマーケットプレイス**: コミュニティスキルの閲覧とインストール
- **AI支援作成**: 自然言語提案からスキル構造を自動作成
- **スキル進化** (`evolution_engine`): 実行フィードバックに基づくスキルの自動分析と改善
- **意味マッチング** (`skill`): 会話コンテキストに基づく関連スキルの意味マッチング、自動推薦
- **スキル分解** (`skill_decomposition`): 複雑なタスクを原子スキルの組み合わせに自動分解
- **生成ツール** (`generated_tool`): AIによる新規ツールの生成と登録
- **サンドボックス実行** (`sandbox`): スキルを分離されたサンドボックス環境で安全に実行

### ビジュアルワークフロー

ReactFlow 12ベースのドラッグ＆ドロップDAGワークフローエディター：

- **17種類のノードタイプ**: トリガー、エージェント、LLM呼び出し、条件分岐、並列フォーク、ループ、マージ、遅延、ツール呼び出し、コード実行、サブワークフロー、ベクトル検索、ドキュメント解析、検証、終了、ビジネスルール、エージェントロール
- **Kahnトポロジカルソート実行**: 循環依存の自動検出、並列パイプラインスケジューリング
- **組み込みテンプレート**: コードレビュー、バグ修正、ドキュメント生成、テスト、リファクタリング、探索、パフォーマンス分析、セキュリティ監査、機能開発
- **YAMLシリアライズ**: ワークフロー定義のYAML形式インポート/エクスポート
- **バージョン管理**: ワークフローテンプレートのバージョン管理
- **AI支援**: AI支援によるワークフロー設計とノード推薦

### 知識管理

- **マルチ知識ベースRAG**: ドキュメントアップロード → 自動解析（PDF/DOCX/XLSX/PPTX/TXT）→ チャンク分割 → ベクトルインデックス
- **ハイブリッド検索**: ベクトル類似度（sqlite-vec + candleローカル埋め込み）+ BM25全文検索（FTS5）、ハイブリッドランキング
- **Self-RAG**: 自己検索拡張生成、検索結果の自動反映と検証
- **リランキング**: Cross-encoder結果リランキングによる精度向上
- **知識グラフ**: エンティティ抽出 (`EntityExtractor`) → 関係構築 → 可視化グラフ
- **ファイル監視**: `notify`ベースのリアルタイムファイル変更監視、自動増分インデックス
- **LLM Wiki**: AI支援Wikiコンパイラとバリデーター、Wikiクロッピングブラウザ拡張対応

### メモリシステム

- **マルチ名前空間メモリ**: プロジェクト/トピックごとに分離、手動入力とAI自動抽出をサポート
- **永続化統合**: HonchoおよびMem0クローズドループメモリ
- **ユーザープロファイル** (`user_profile` / `profile`): コードスタイル（インデント/命名/コメント）、技術スタックの好み、コミュニケーションスタイルを自動学習
- **スタイル転送** (`style`): コードスタイル特徴の抽出 → AI生成コードへの適用
- **ドリーム統合** (`dream`): バックグラウンドでのメモリ断片と行動パターンの自動統合、構造化知識の生成
- **プロジェクトメモリ** (`project_memory`): プロジェクト単位のコンテキスト永続化

### APIゲートウェイ

`axum`ベースの組み込みHTTP + WebSocketゲートウェイサーバー：

- **互換エンドポイント**: OpenAI `/v1/chat/completions`、Claude Messages API、Gemini API、およびOpenAI ResponsesとRealtime WebSocket
- **キー管理**: アクセスキーの生成、失効、有効化/無効化、有効期限設定
- **使用量追跡**: キー、プロバイダー、日付別のリクエスト量とトークン消費統計、Prometheusメトリクスエクスポート
- **レート制限**: `governor`ベースのトークンバケットアルゴリズム、設定可能なレート制限ポリシー
- **SSL/TLS**: 組み込み自己署名証明書 (`rcgen`)、カスタム証明書対応
- **外部リンク**: Claude CLI、OpenCodeなどの外部ツールとのワンクリック統合、APIキー自動同期
- **リアルタイムチケット**: HMACベースの一時認証チケット、WebSocketリアルタイム接続の安全な受け渡し

### メッセージングプラットフォーム統合

`rt-messaging`クレートによるメッセージングプラットフォームゲートウェイ、以下をサポート：

DingTalk、Feishu、QQ、Slack、WeChat、WhatsApp、Telegram、Discord

Webhookメッセージ受信、コマンド解析、AI返信の自動中継をサポート。

### ツールシステム

47個の組み込みツール、すべて `Tool` トレイトを通じて登録：

| カテゴリ         | ツール                                                                                                                                                                                                     |
| ---------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| ファイル操作     | `file_read`, `file_write`, `file_edit`, `file_system` (一覧/検索/メタデータ)                                                                                                                               |
| コード実行       | `bash`, `repl`                                                                                                                                                                                             |
| 検索             | `grep`, `glob`                                                                                                                                                                                             |
| ブラウザ         | `browser` (CDP制御)                                                                                                                                                                                        |
| コンピュータ制御 | `computer_use` (マウス/キーボード/スクリーンショット)                                                                                                                                                      |
| Web              | `web_search`, `web_fetch`                                                                                                                                                                                  |
| 知識ベース       | `knowledge`, `document` (ドキュメント解析)                                                                                                                                                                 |
| Git              | `git` (commit/push/branch/diff)                                                                                                                                                                            |
| 開発ツール       | `lsp` (言語サーバープロトコル), `workspace`                                                                                                                                                                |
| タスク管理       | `plan`, `task_system`, `todo_write`, `cron`                                                                                                                                                                |
| プッシュ通知     | `push_notification`, `messaging`                                                                                                                                                                           |
| データベース     | `database`                                                                                                                                                                                                 |
| ストレージ       | `storage`                                                                                                                                                                                                  |
| その他           | `agent`, `agent_memory`, `context`, `export`, `integration`, `media`, `media_delivery`, `migration_tool`, `monitor`, `obsidian`, `ocr`, `personality`, `shared_path`, `system_info`, `testing`, `worktree` |

### MCPプロトコル

`rmcp`クレートベースの完全なMCP (Model Context Protocol) 実装：

- **転送層**: stdioサブプロセス + Streamable HTTP + WebSocket
- **OAuth認証**: MCPサーバーのOAuth認証フロー対応
- **ツール発見**: MCPサーバーが公開するツールの自動発見と登録
- **MCPマネージャー**: サーバーライフサイクル管理、ヘルスチェック、自動再接続

### プラグインシステム

OpenClaw互換の3層プラグインアーキテクチャ（組み込み/バンドル/外部）、以下をサポート：

- npmパッケージインストール、検索とインストール用の組み込みマーケットプレイスUI
- プラグインマニフェスト定義、権限宣言、サンドボックス分離実行
- カスタムツール登録、Agentプロバイダー、Hookインターセプト
- スキルインストーラー：プラグインパッケージからスキルをスキルシステムにインストール

### セキュリティ

- **AES-256-GCM暗号化**: APIキーと機密設定のローカル暗号化ストレージ (`crypto`クレート)
- **プロンプトインジェクション対策**: 4層防御パイプライン (`prompt-guard`)——パターン検出 → 区切り文字エスケープ → XMLラッパー → 信頼タグ、セッション、プロンプト構築、Git、RAG全体に統合
- **SSRF対策** (`ssrf_guard`): URLセキュリティチェック、内部ネットワークアドレスへのリクエストをブロック
- **コンテンツフィルタリング** (`content_filter`): マルチタイプコンテンツ安全フィルタリング
- **レート制限** (`rate_limiter`): ツール呼び出しとAPIリクエストのトークンバケットレート制限
- **サーキットブレーカー** (`circuit_breaker`): 連続失敗時の自動遮断、システム安定性の保護
- **アクセス制御** (`tool_access`): ポリシーベースのツールアクセス権限制御
- **サンドボックス分離**: エージェントとスキルの実行環境分離

### 開発者体験

- **分散トレーシング** (`telemetry`): OpenTelemetry統合、Span/Trace可視化対応
- **テレメトリ** (`telemetry`): 構造化ログ、ランタイムメトリクス、パフォーマンスイベント収集
- **リプレイデバッグ**: エージェント実行軌跡記録 (`trajectory_recorder`) とリプレイ
- **DevToolsパネル**: フロントエンド内蔵のTrace/Spanタイムラインビューア
- **ベンチマークフレームワーク**: Criterion benchmarks (tool_exec / llm_call / search)、SWE-benchおよびTerminal-bench評価

### デスクトップとモバイル体験

- **レスポンシブレイアウト**: CSSブレークポイントによるデスクトップ/タブレット/モバイル適応（600px/900px）
- **11言語**: 簡体字中国語、繁体字中国語、英語、日本語、韓国語、フランス語、ドイツ語、スペイン語、ロシア語、ヒンディー語、アラビア語
- **テーマエンジン** (`rt-theme`): ダーク/ライトテーマ、システム連動または手動切替、Ant Design 6による深いカスタマイズ
- **Monacoエディター**: 組み込みコードエディター、シンタックスハイライト、差分プレビュー、多言語対応
- **xterm.js端末**: 組み込み端末エミュレーター、WebLinks、Unicode 11、検索対応
- **D2 / Mermaid / ECharts**: アーキテクチャ図、フローチャート、インタラクティブチャートレンダリング
- **セッション共有**: ワンクリック共有リンク生成、アクセス権限制御
- **システムトレイ + グローバルショートカット + 自動起動**: 邪魔にならないバックグラウンド動作
- **自動更新**: GitHub Releasesのバージョン更新を自動検出
- **プロキシ対応**: HTTPおよびSOCKS5プロキシ設定
- **クラウドワークスペース**: S3およびWebDAVストレージ同期、競合検出と双方向同期

### モバイル

- Android APK/AAB (arm64-v8a, armeabi-v7a, x86_64)
- iOS IPA (arm64)
- モバイル専用適応：セーフエリア適応、ボトムナビゲーションバー、Drawerナビゲーション

---

## 技術アーキテクチャ

### 技術スタック

| レイヤー                     | 技術                                     |
| ---------------------------- | ---------------------------------------- |
| デスクトップフレームワーク   | Tauri 2.11                               |
| フロントエンドフレームワーク | React 19 + TypeScript 6                  |
| UIライブラリ                 | Ant Design 6 + TailwindCSS 4             |
| 状態管理                     | Zustand 5                                |
| ルーティング                 | React Router 7                           |
| コードエディター             | Monaco Editor                            |
| 端末                         | xterm.js 6                               |
| ワークフローエディター       | ReactFlow 12                             |
| チャート                     | D2 + Mermaid + Recharts + ECharts        |
| 仮想スクロール               | @tanstack/react-virtual + react-virtuoso |
| ドラッグ＆ドロップ           | @dnd-kit                                 |
| Markdownレンダリング         | markstream-react + stream-markdown       |
| 国際化                       | i18next + react-i18next                  |
| ビルドツール                 | Vite 8                                   |
| テスト                       | Vitest + Playwright + cargo-nextest      |
| フォーマット                 | dprint (TS/JSON/Markdown/TOML) + rustfmt |
| Lint                         | ESLint + Oxlint + Clippy + cargo-deny    |

### バックエンドアーキテクチャ: Harness依存性注入パターン

バックエンドはRustワークスペースアーキテクチャを採用し、**32個のクレート**を含み、**Harnessアーキテクチャパターン**に従います：

```
すべてのクレートはaxagent-harnessが定義するトレイトインターフェースを通じて疎結合され、
実行時にaxagent-runtimeが依存関係を組み立てて注入します。

依存方向: 具象実装 → harness ← 呼び出し元
```

**harness**はアーキテクチャの基盤です——ビジネスロジックも具象実装も含まず、トレイト定義、純粋データDTO、定数、統一エラー型のみを含みます。他のすべてのクレートから依存され、自身は他のaxagent-*クレートに依存しません。

```
src-tauri/crates/
├── harness/          # アーキテクチャ基盤 — トレイトインターフェース、DTO、統一エラー型、DI契約
│                     #   200以上のトレイト定義: Agent/Provider/Tool/RAG/ストレージ/
│                     #   MCP/プラグイン/セキュリティ/可観測性/メモリ/学習/ブラウザ/メッセージングなど
│
├── entities/         # SeaORMエンティティモデル
├── dao/              # データアクセス層（CRUD）
├── migration/        # データベースマイグレーション
│
├── crypto/           # AES-256-GCM暗号化/復号とキー管理
├── credential/       # 資格情報の安全な保存（APIキーなど）
├── storage/          # ファイルストレージ抽象化（ローカル/S3/WebDAV）、ZIP読み書き対応
├── cache/            # 汎用キャッシュ層（メモリ内）
├── disk-cache/       # ディスクファイルレベルキャッシュ
├── search/           # 検索エンジン（FTS5 + sqlite-vec + candle埋め込み）
├── document-parser/  # ドキュメントテキスト抽出（PDF/DOCX/XLSX/PPTX）
├── kit/              # 汎用ユーティリティツールキット — パス/エンコーディング/ハッシュ/日付など
│
├── runtime-core/     # ランタイム共通型、設定定数
├── runtime/          # ランタイムサービスオーケストレーション — 全30以上のクレートを組み立て、Harness DIのランタイムコンテナ
│                     #   管理対象: セッション/端末/Webhook/レート制限/権限/SSRF/イベントバス/状態
├── rt-workflow/      # ワークフローエンジン — DAGオーケストレーション、ノードエグゼキュータ、YAMLシリアライズ
├── rt-messaging/     # メッセージングプラットフォームゲートウェイ — DingTalk/Feishu/QQ/Slack/WeChat/WhatsApp/Telegram/Discord
├── rt-webhook/       # 汎用Webhookサーバーとイベントディスパッチ
├── rt-dashboard/     # ダッシュボードプラグインフレームワーク
├── rt-theme/         # テーマエンジン — ダーク/ライト切替ロジック
│
├── agent/            # AIエージェントコア — 80以上のモジュール
│                     #   ReActエンジン/階層型プランニング/深層リサーチ/ファクトチェック/思考の木/リフレクション/
│                     #   自己検証/エラー復旧/RL最適化/LoRAファインチューニング/評価/ツール推薦/A/Bテスト/
│                     #   コーディネーター/ブラックボード/ビジョンパイプライン/Web検索/学術検索/Wikiコンパイルなど
│
├── orchestrator/     # エージェントオーケストレーション — マルチエージェントスケジューリング、DAG分解、動的サブグラフ実行
├── providers/        # モデルプロバイダーアダプター — OpenAI/Anthropic/Gemini/Ollama/
│                     #   OpenClaw/Hermes/画像生成(DALL-E/Flux)/Realtime/Responses
├── tools/            # ツールシステム — Toolトレイト/レジストリ/オーケストレーション/ストリーミング/サンドボックス/47以上の組み込みツール
├── gateway/          # APIゲートウェイ — axum HTTP/WSサーバー、OAuth、レート制限、Prometheus
├── mcp/              # MCPプロトコル — stdio + Streamable HTTP、rmcpベース
├── trajectory/       # 学習システム — メモリ/スキル進化/ユーザープロファイル/ドリーム統合
├── plugins/          # プラグインシステム — OpenClaw互換、npmパッケージインストール、マーケットプレイス
├── telemetry/        # 可観測性 — OpenTelemetry、構造化ログ、ランタイムメトリクス
├── prompt-guard/     # プロンプトインジェクション対策 — L1-L4多段検出パイプライン
├── npm/              # npmレジストリクライアント
└── schema-gen/       # データベーススキーマ生成ツール
```

### フロントエンドアーキテクチャ

```
src/
├── pages/            # 22ページ
│   ├── ChatPage          # メインチャットインターフェース
│   ├── WorkflowPage      # ワークフローエディター
│   ├── GatewayPage       # APIゲートウェイ管理
│   ├── KnowledgeHubPage  # 知識ベース管理
│   ├── MemoryPage        # メモリ管理
│   ├── SkillsPage        # スキルマーケットプレイス
│   ├── SettingsPage      # 設定パネル
│   ├── DashboardPage     # データダッシュボード
│   ├── TerminalPage      # 端末
│   ├── FilesPage         # ファイル管理
│   ├── GatewayLinkPage   # 外部リンク管理
│   ├── LinkPage          # 統合リンク
│   ├── WikiEditorPage    # Wikiエディター
│   ├── WikiEditPage      # Wiki編集
│   ├── WikiGraphPage     # Wiki知識グラフ
│   ├── FineTunePage      # LoRAファインチューニング
│   ├── PersonaPage       # ペルソナ管理
│   ├── QuickBarPage      # クイックバー
│   ├── IngestPage        # ドキュメント取り込み
│   ├── WorkflowMarketplace # ワークフローマーケットプレイス
│   ├── DynamicUIManagerPage # 動的UI管理
│   └── DynamicPageViewer    # 動的ページビューア
│
├── components/       # 24モジュール、200以上のコンポーネント
│   ├── chat/         # チャットインターフェース（メッセージストリーム/入力/添付/ツール呼び出し/アーティファクト/思考ブロックなど）
│   ├── workflow/     # ワークフローエディター（ノード/エッジ/パネル/テンプレート/AI支援）
│   ├── gateway/      # APIゲートウェイ管理UI
│   ├── settings/     # 設定パネル（40以上のサブコンポーネント）
│   ├── skill/        # スキルエディターとレンダラー
│   ├── benchmark/    # ベンチマークパネル
│   ├── decomposition/# スキル分解とツール生成
│   ├── devtools/     # Trace/Spanタイムライン
│   ├── layout/       # レイアウト（タイトルバー/サイドバー/コマンドパレット）
│   └── ...
│
├── stores/           # 62個のZustandストア
│   ├── domain/       # コアビジネス状態
│   ├── feature/      # 機能モジュール状態（44個）
│   └── devtools/     # 開発者ツール状態
│
├── hooks/            # React Hooks
├── lib/              # ユーティリティ関数 + Web Workers
├── types/            # TypeScript型定義
├── sdk/              # 外部統合SDK
└── i18n/             # 11言語翻訳 (zh-CN/zh-TW/en-US/ja/ko/fr/de/es/ru/hi/ar)
```

---

## データディレクトリ

```
~/.axagent/                    # アプリケーション設定
├── axagent.db                 # SQLiteメインデータベース (SeaORM)
├── master.key                 # AES-256マスターキー
├── vector_db/                 # sqlite-vecベクトルインデックス
└── ssl/                       # 自己署名SSL証明書

~/Documents/axagent/          # ユーザーファイル
├── images/                   # 画像添付
├── files/                    # ファイル添付
└── backups/                  # 自動バックアップ
```

---

## クイックスタート

### 要件

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://www.rust-lang.org/) 1.75+, edition 2024
- [npm](https://www.npmjs.com/) 10+
- Windows: [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) (MSVC + Windows SDK)
- macOS: Xcode Command Line Tools
- Linux: `build-essential` + `libwebkit2gtk-4.1-dev` + `libssl-dev`

### ビルド

```bash
git clone https://github.com/polite0803/AxAgent.git
cd AxAgent
npm install
npm run tauri dev      # 開発モード
npm run tauri build    # 本番ビルド
```

ビルド成果物は `src-tauri/target/release/` に配置されます。

### テスト

```bash
npm run test           # フロントエンドユニットテスト (Vitest watch)
npm run test:run       # フロントエンドユニットテスト (単一実行)
npm run test:e2e       # E2Eテスト (Playwright)

# Rustバックエンドテスト
cd src-tauri && cargo nextest run
cd src-tauri && cargo test

# 型チェック & Lint
npm run typecheck
cd src-tauri && cargo clippy -- -D warnings
npm run format

# CI全チェック
npm run ci:check
```

---

## プラットフォームサポート

| プラットフォーム | アーキテクチャ                                  |
| ---------------- | ----------------------------------------------- |
| Windows          | x86_64, ARM64                                   |
| macOS            | Apple Silicon (arm64), Intel (x86_64)           |
| Linux            | x86_64, ARM64                                   |
| Android          | arm64-v8a, armeabi-v7a, x86_64 (エミュレーター) |
| iOS              | arm64                                           |

---

## ライセンス

本プロジェクトは [AGPL-3.0-only](LICENSE) ライセンスの下でオープンソース公開されています。

---

## 謝辞

AxAgentは以下のような多くの優れたオープンソースプロジェクトの上に構築されています（以下を含むがこれに限定されません）：

- [Tauri](https://tauri.app/) — クロスプラットフォームデスクトップフレームワーク
- [React](https://react.dev/) + [Ant Design](https://ant.design/) — フロントエンドUI
- [SeaORM](https://www.sea-ql.org/SeaORM/) — Rust ORM
- [sqlite-vec](https://github.com/asg017/sqlite-vec) — ベクトル検索
- [candle](https://github.com/huggingface/candle) — ローカル埋め込み推論
- [rmcp](https://github.com/nicholasxjy/rmcp) — Rust MCP SDK
- [ReactFlow](https://reactflow.dev/) — ビジュアルワークフローエディター
- [axum](https://github.com/tokio-rs/axum) — HTTPフレームワーク
- [Monaco Editor](https://microsoft.github.io/monaco-editor/) — コードエディター
- [xterm.js](https://xtermjs.org/) — 端末エミュレーター
