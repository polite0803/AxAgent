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

**AxAgent** は Tauri 2 をベースにしたクロスプラットフォームの AI デスクトップクライアント（Windows / macOS / Linux / Android / iOS）であり、AI 駆動の日常開発・研究・ナレッジ管理・自動化ワークベンチとして位置づけられています。ReAct エージェントエンジン、認知ルーティング（3 段階の階層ルーティング + 検索拡張ルーティング RAR）、ビジュアルワークフローオーケストレーション、ローカル RAG ナレッジベース、MCP プロトコル拡張、マルチモデル統一ゲートウェイ、ブラウザ自動化、コンピュータ制御などの機能を内蔵し、AI を「対話」から「実行」へと導きます。

> **言語バージョン**: [简体中文](./README.md) | [English](./README-EN.md) | [繁體中文](./README-ZH-TW.md) | [日本語](./README-JA.md) | [한국어](./README-KO.md) | [Français](./README-FR.md) | [Deutsch](./README-DE.md) | [Español](./README-ES.md) | [Русский](./README-RU.md) | [हिन्दी](./README-HI.md) | [العربية](./README-AR.md)

---

## プロジェクトの位置づけ

AxAgent は 3 つの核心的な課題を解決します：

1. **マルチモデル統一アクセスとスマートスケジューリング** — 単一のインターフェースで OpenAI、Anthropic Claude、Google Gemini、DeepSeek、Qwen、GLM、Kimi、文心、Ollama ローカルモデル、および任意の OpenAI 互換 API を同時に利用可能。複数 Key のクォータ自動ローテーション、タスク種別に応じたスマートルーティング、ストリーミング比較をサポート
2. **AI の対話から実行へのクローズドループ** — 163+ の内蔵ツール + ビジュアルワークフロー + MCP 拡張 + ブラウザ/コンピュータ制御により、AI はファイル操作、コード実行、Git 管理、タスクスケジューリングを実行可能
3. **ローカルファーストのデータ主権** — 会話履歴、ナレッジベース、メモリ、設定はすべてローカルの SQLite データベースに保存され、API Key は AES-256-GCM で暗号化。サードパーティのクラウドサービスなしでコア機能を実行可能

---

## コア機能

### 認知ルーティングシステム（Cognitive Router）

AxAgent は `cognitive_query` をすべての会話の統一エントリポイントとし、**3 段階の階層ルーティング**によってユーザーの意図を具体的な能力にマッピングします：

- **L1 ドメインルーティング** (`domain_router`): ルール + LLM フォールバックにより、9 大業務ドメイン（データ分析 / コンテンツ制作 / コミュニケーション / 運用保守 / AI メディア / 金融 / 自動化 / 汎用など）を識別
- **L2 クラスタールーティング** (`cluster_router`): ドメイン内で能力クラスタを特定（27 個のクラスタ、8 大業務ドメインをカバー）
- **L3 能力ルーティング**: **検索拡張ルーティング（RAR）** — 能力ベクトルライブラリから Top-K の類似ワークフローをリコールして Prompt に注入し、ワークフロー DAG グラフの経路探索と組み合わせて、パスアドレス（例：`/finance/stock_analysis/tech`）と実行モードを出力
- **実行モード**: `Ask / Plan / Act / Workflow / Direct / Delegate / ParameterExtract / Clarify` を信頼度に応じて自動選択
- **能力システム**: 統一レジストリ（`CapabilityRegistry`）+ ベクトルインデックス（`CapabilityIndexer`）+ ハイブリッド検索（`CapabilityRetriever`、ベクトル + BM25 + タグ完全一致 + 負サンプル除外）
- **システム能力の分離**: 認知オーケストレーターと業務ワークフローを物理的に分離し、システム能力には `SYSTEM_ONLY` 可視性マークを付与。ルーティング層に自己参照サーキットブレーカーを内蔵し、自己言及のパラドックスを防止
- **3 段階ルーティングはワークフロー DAG で実装**: 4 つのプリセットルーティングワークフローテンプレート（メインオーケストレーション約 20 ノード + L1/L2/L3 サブルーティング）を `rt-workflow` エンジンで実行

### マルチモデルエンジン

- **13 種のプロバイダーアダプター**: OpenAI（Chat Completions + Responses + Realtime）、Anthropic Claude、Google Gemini、DeepSeek、Qwen、GLM、Kimi、文心一言、Ollama、Llama.cpp（GGUF ローカルモデル）、OpenClaw、Hermes、およびすべての OpenAI 互換 API
- **複数 Key のローテーション**: 同一プロバイダーの複数 API Key をクォータに応じて自動ローテーションし、単一 Key のレート制限時は自動的に切り替え
- **スマートルーティング**: タスク種別（コードレビュー / 要約 / 翻訳 / 汎用）に応じて最適なモデルを自動選択し、カスタムルールをサポート
- **プロバイダーヘルスモニタリング**: 成功率、レイテンシ、可用状態をリアルタイムで追跡し、段階的な自動デグレードをサポート
- **AI 画像生成**: DALL-E 3 と Flux の複数サイズプリセット
- **リアルタイム音声**: OpenAI Realtime API ベースの WebSocket 音声対話。割り込みとストリーミング文字起こしをサポート

### エージェントシステム（ReAct エンジン）

- **階層プランナー** (`hierarchical_planner`): 複雑なタスクを Phase → Task の構造化プランに分解し、DAG トポロジーにコンパイルして実行
- **ディープリサーチ** (`deep_research`): 複数ソースの検索オーケストレーション。検索プラン、検索実行、コンテンツ統合、引用追跡を含む
- **ファクトチェック** (`fact_checker`): AI 駆動の事実検証。ソース分類器、信頼性評価を含む
- **思考の木** (`tree_of_thoughts`): 複数パスの推論探索。分岐評価とバックトラック
- **リフレクター** (`reflector`): タスク実行後の自己評価と改善提案
- **自己検証** (`self_verifier`): 推論結果の自動検証。循環検出を含む
- **エラーリカバリ** (`error_recovery_engine`): エラータイプの分類 → リカバリ戦略の選択 → 自動リトライまたはプラン調整。指数バックオフをサポート
- **A/B テスト** (`ab_testing`): 異なる推論戦略の比較評価
- **評価システム** (`evaluator`): 内蔵ベンチマークテストフレームワーク
- **LoRA ファインチューニング** (`fine_tune`): 内蔵トレーニングパイプライン。LoRA アダプター管理をサポート
- **RL オプティマイザー** (`rl_optimizer`): 経験フィードバックに基づく方策強化学習

**マルチエージェント協調**:

- マスター・スレーブ調整アーキテクチャ。子エージェントを並列実行し、依存関係を考慮したスケジューリング
- エージェント間の情報交換に共有ブラックボードを使用
- 対抗的ディベートモード（Pro/Con ラウンドと論点強度スコアリング）
- Swarm クラスターモード。マルチプロセスエージェントクラスター
- プロアクティブモード：エージェントが自発的に提案や操作を開始可能

**コンピュータ制御**: AI によるマウスクリック、キーボード入力、画面スクロール。3 段階の権限（デフォルト / 編集を許可 / フルアクセス）とサンドボックスパス分離

**ブラウザ自動化**: CDP プロトコルでブラウザを制御。ナビゲーション、スクリーンショット、クリック、フォーム入力、テキスト抽出をサポート

### スキルシステム

- **スキルマーケット**: コミュニティスキルの閲覧とインストール
- **AI 支援による作成**: 自然言語の提案からスキル構造を自動生成 (`skill:create`)
- **スキル進化** (`evolution_engine`): 実行フィードバックに基づいてスキルを自動分析・改善
- **セマンティックマッチング**: 会話コンテキストの意味に基づいて関連スキルを自動推薦
- **スキル分解** (`skill_decomposition`): 複雑なタスクを原子的なスキルの組み合わせに自動分解
- **ツール生成**: AI が新しいツールを生成して登録
- **サンドボックス実行**: スキルを隔離されたサンドボックス内で安全に実行

### ビジュアルワークフロー

ReactFlow 12 ベースのドラッグ＆ドロップ式 DAG ワークフローエディター：

- **32 種のノードタイプ**: トリガー、エージェント、LLM 呼び出し、条件分岐、並列フォーク、ループ、マージ、遅延、ツール呼び出し、コード実行、サブワークフロー、ベクトル検索、ドキュメント解析、検証、終了、HTTP リクエスト、Switch、データベースクエリ、通知、承認、ファイル操作、データ変換、Webhook 送信、ログ、LLM 分類器、アグリゲーター、メール、ディベート、Swarm、マルチエージェント、ストレージ、ビジネスルール
- **Kahn トポロジカルソート実行**: 循環依存を自動検出し、並列パイプラインをスケジューリング
- **内蔵テンプレート**: コードレビュー、バグ修正、ドキュメント生成、テスト、リファクタリング、探索、パフォーマンス分析、セキュリティレビュー、機能開発
- **YAML シリアライズ**: ワークフロー定義のインポート/エクスポート
- **バージョン管理**: テンプレートのバージョン管理
- **AI 支援設計**: AI によるワークフロー設計支援、ノード推薦と診断

### ナレッジ管理

- **マルチナレッジベース RAG**: ドキュメントアップロード → 自動解析（PDF/DOCX/XLSX/PPTX/TXT）→ チャンク分割 → ベクトルインデックス
- **ハイブリッド検索**: ベクトル類似度（sqlite-vec + candle ローカル埋め込み）+ BM25 全文検索（FTS5）によるハイブリッドランキング
- **Self-RAG**: 検索結果の自動リフレクションと検証
- **リランキング**: Cross-encoder による結果の再ランキング
- **ナレッジグラフ**: エンティティ抽出 → 関係構築 → 可視化グラフ
- **ファイル監視**: `notify` ベースのリアルタイムファイル変更監視による自動インクリメンタルインデックス
- **LLM Wiki**: AI 支援の Wiki コンパイラとバリデーター

### メモリシステム

- **マルチネームスペースメモリ**: プロジェクト/トピックごとに分離し、手動入力と AI 自動抽出をサポート
- **永続化統合**: Honcho と Mem0 によるクローズドループメモリ
- **ユーザープロファイル**: コードスタイル、技術スタックの好み、コミュニケーションスタイルを自動学習
- **スタイル転移**: コードスタイルの特徴を抽出 → AI 生成コードに適用
- **ドリーム統合**: バックグラウンドで記憶の断片と行動パターンを自動統合し、構造化された知識を生成
- **プロジェクトメモリ**: プロジェクト単位でのコンテキスト永続化

### API ゲートウェイ

`axum` ベースの HTTP + WebSocket ゲートウェイを内蔵：

- **互換エンドポイント**: OpenAI `/v1/chat/completions`、Claude Messages API、Gemini API、および OpenAI Responses と Realtime WebSocket
- **Key 管理**: アクセスキーの生成、失効、有効/無効化。有効期限をサポート
- **使用量トラッキング**: Key/プロバイダー/日付ごとのリクエスト数とトークン消費量を集計し、Prometheus メトリクスをエクスポート
- **レート制限**: `governor` ベースのトークンバケットアルゴリズム
- **SSL/TLS**: 内蔵の自己署名証明書（`rcgen`）。カスタム証明書をサポート
- **外部連携**: Claude CLI、OpenCode などの外部ツールをワンクリックで統合し、API Key を自動同期
- **リアルタイムチケット**: HMAC ベースの一時認証チケット。WebSocket 接続の安全な受け渡しに使用
- **Server モード**: オプションの `axagent-server` バイナリにより、デスクトップアプリの機能をサービスとして外部に提供

### メッセージプラットフォーム統合

`rt-messaging` によるマルチプラットフォームゲートウェイを実装し、**DingTalk、Feishu、QQ、Slack、WeChat、WhatsApp、Telegram、Discord** のメッセージ受信、コマンド解析、AI 自動応答をサポート。

### ツールシステム

**163+ の内蔵ツール**。すべて `Tool` trait を介して登録され、15 大カテゴリをカバー：

| カテゴリ          | ツール例                                                                                                                                                                 |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| ファイル操作      | `file_read`, `file_write`, `file_edit`, `glob`, `grep`, ディレクトリ/削除/移動など 11 個                                                                                 |
| Shell/Web         | `bash`, `web_fetch`, `web_search`                                                                                                                                        |
| ネットワーク      | `http_request`, `ping`, `dns_lookup`, `json_api`, `rss_reader`, `graphql`, `websocket`                                                                                   |
| ブラウザ          | `browser_navigate`, `browser_click`, `browser_fill`, `browser_screenshot` など 10 個（CDP）                                                                              |
| コンピュータ制御  | `computer_use`（マウス/キーボード/スクリーンショット）                                                                                                                   |
| Git               | `git_status`, `git_diff`, `git_commit`, `git_log`, `git_branch`, `git_review`                                                                                            |
| ナレッジベース    | `list_knowledge_bases`, `search_knowledge`, `add_knowledge_document` など 6 個                                                                                           |
| タスク管理        | `todo_write`, `task_*`（6 個）, `cron_*`（3 個）, `plan` 関連                                                                                                            |
| メッセージ通知    | `push_notification`, `send_message`, チームコラボレーションツール                                                                                                        |
| データベース      | `database_query`, `database_list_tables`, `database_migration_status`                                                                                                    |
| ストレージ        | `get_storage_info`, `upload_storage_file`, `download_storage_file` など 5 個                                                                                             |
| エクスポート/形式 | `export_word`, `export_pdf`, `export_xlsx`, `export_pptx`, `render_markdown` など 9 個                                                                                   |
| OCR               | `ocr_image`, `ocr_detect_langs`                                                                                                                                          |
| Obsidian          | `obsidian_search`, `obsidian_read`, `obsidian_backlinks` など 9 個                                                                                                       |
| その他            | `agent`, `delegate_task`, `skills_*`, `lsp`, `repl`, `monitor`, `workspace_*`, `session_search`, `generate_image`, `sequential_thinking`, CI/CD、DevOps、RPC、テストなど |

### MCP プロトコル

`rmcp` ベースの完全な MCP（Model Context Protocol）実装：

- **トランスポート層**: stdio 子プロセス + Streamable HTTP + SSE
- **OAuth 認証**: MCP サーバーの OAuth 認可フローをサポート
- **ツールディスカバリー**: MCP サーバーが公開するツールを自動検出して登録
- **MCP マネージャー**: サーバーのライフサイクル管理、ヘルスチェック、自動再接続

### プラグインシステム

OpenClaw 互換の 3 段階プラグインアーキテクチャ（内蔵 / バンドル / 外部）：

- npm パッケージでインストール。内蔵マーケット UI で検索とインストール
- プラグイン manifest の定義、権限宣言、サンドボックス分離実行
- カスタムツール登録、Agent プロバイダー、Hook インターセプト
- スキルインストーラー：プラグインパッケージからスキルをスキルシステムにインストール

### ダイナミック UI エンジン

- **Schema 駆動**: JSON Schema で宣言的に UI を構築。コードを書く必要なし
- **31 個の内蔵コンポーネント**: コンテナ（7）/ データ表示（6）/ フォーム（9）/ メディア（4）/ その他（5）
- **データバインディング**: 宣言的なデータソースバインドと条件付きレンダリング
- **NL2UI**: 自然言語からダイナミック UI を直接生成

### ACP クライアント SDK

- **ACP（Agent Client Protocol）**: 2 言語対応 SDK（TypeScript + Python）。サードパーティ依存ゼロ
- セッション管理、Prompt 送信、ツール呼び出し記録、WebSocket イベントストリーム
- `/acp/v1/*` エンドポイント経由で AxAgent サービスと通信

### セキュリティ

- **AES-256-GCM 暗号化**: API Key と機密設定をローカルで暗号化して保存（`crypto` crate）
- **プロンプトインジェクション対策**: 4 段階の防御パイプライン（`prompt-guard`）— パターン検出 → 区切り文字エスケープ → XML ラッパー → 信頼タグ。セッション、プロンプト構築、Git、RAG の全経路に統合
- **SSRF 対策**: URL のセキュリティチェックにより、内部ネットワークアドレスへのリクエストをブロック
- **コンテンツフィルタリング**: 複数タイプのコンテンツ安全フィルタリング
- **レート制限**: ツール呼び出しと API リクエストのトークンバケット制限
- **サーキットブレーカー**: 連続失敗時に自動的に遮断
- **アクセス制御**: ポリシーベースのツールアクセス権限制御
- **サンドボックス分離**: エージェントとスキルの実行環境を分離

### 開発者ツール

- **分散トレーシング** (`telemetry`): OpenTelemetry 統合。Span/Trace の可視化
- **構造化ログ**: tracing-subscriber + chrono タイムスタンプ
- **リプレイデバッグ**: エージェント実行トレースの記録（`trajectory_recorder`）とリプレイ
- **DevTools パネル**: Trace Explorer タイムラインビューア、Benchmark Runner、Tool Recommender
- **ベンチマーク**: Criterion benchmarks（tool_exec / llm_call / search）
- **CI チェック**: `npm run ci:check` で型チェック、lint、フォーマット検証を統合

### デスクトップ・モバイル体験

- **レスポンシブレイアウト**: CSS ブレークポイントでデスクトップ/タブレット/スマートフォンに適応（3 段階のデバイスレイアウト：`desktop` / `tablet` / `mobile`）
- **11 言語**: 簡体字中国語、繁体字中国語、英語、日本語、韓国語、フランス語、ドイツ語、スペイン語、ロシア語、ヒンディー語、アラビア語
- **テーマエンジン** (`rt-theme`): ダーク/ライトテーマ + 複数のプリセット。Ant Design 6 を深くカスタマイズ
- **Monaco エディター**: シンタックスハイライト、差分プレビュー、多言語サポート
- **xterm.js ターミナル**: WebLinks、Unicode 11、検索
- **仮想スクロール**: @tanstack/react-virtual + react-virtuoso
- **チャート描画**: D2 + Mermaid + Recharts + Sigma（グラフ）
- **コマンドパレット**: Ctrl+K のグローバルコマンドパネル
- **システムトレイ + グローバルショートカット + 自動起動**: 邪魔にならないバックグラウンド実行
- **自動アップデート**: 設定可能な間隔での GitHub Releases バージョン検出
- **プロキシサポート**: HTTP / SOCKS5 プロキシ設定
- **クラウドワークスペース**: S3 と WebDAV ストレージ同期。競合検出と双方向同期

### モバイル

- Android APK/AAB（arm64-v8a, armeabi-v7a, x86_64）
- iOS IPA（arm64）
- モバイル専用の最適化：セーフエリア対応、ボトムナビゲーション、Drawer ナビゲーション

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
| チャート                     | D2 + Mermaid + Recharts + Sigma          |            |
| アニメーション               | Framer Motion                            | 12         |
| 仮想スクロール               | @tanstack/react-virtual + react-virtuoso |            |
| ドラッグ＆ドロップ           | @dnd-kit                                 | 6          |
| Markdown レンダリング        | markstream-react + stream-markdown       |            |
| 国際化                       | i18next + react-i18next                  |            |
| ビルドツール                 | Vite                                     | 8          |
| テスト                       | Vitest + Playwright                      |            |
| フォーマッター               | dprint（TS/JSON/Markdown/TOML）+ rustfmt |            |
| Lint                         | ESLint + Oxlint + Clippy                 |            |

### バックエンドアーキテクチャ: Harness 依存性注入パターン

Rust workspace アーキテクチャを採用し、**37 のメンバー**（メイン crate + 35 のライブラリ crate + schema-gen）で構成。**Harness 依存性注入アーキテクチャ**に従います：

> すべての crate は axagent-harness で定義された trait インターフェースによって疎結合され、実行時に axagent-runtime が依存関係を組み立てて注入します。
> 依存方向：`具体実装 → harness ← 呼び出し側`

**harness** はアーキテクチャの基盤です — ビジネスロジックゼロ、具体実装ゼロで、trait 定義、純粋なデータ DTO、定数、統一エラータイプのみを含みます。他のすべての crate から依存され、自身はどの axagent-* crate にも依存しません（200+ の trait 定義。Agent/Provider/Tool/RAG/ストレージ/MCP/プラグイン/セキュリティ/可観測性/メモリ/学習/ブラウザ/メッセージ/認知ルーティングなどをカバー）。

```
src-tauri/crates/
├── harness/          # 架构基石 — trait 接口、DTO、错误类型、DI 契约
├── entities/         # SeaORM 实体模型
├── dao/              # 数据访问层（CRUD）
├── migration/        # 数据库迁移
├── crypto/           # AES-256-GCM 加解密与密钥管理
├── credential/       # 凭据安全存储
├── storage/          # 文件存储抽象（本地/S3/WebDAV），ZIP 读写
├── cache/            # 内存缓存层
├── disk-cache/       # 磁盘文件级缓存
├── search/           # 检索引擎（FTS5 + sqlite-vec + candle 本地嵌入）
├── document-parser/  # 文档文本提取（PDF/DOCX/XLSX/PPTX）
├── kit/              # 通用工具集（路径/编码/哈希/日期）
├── runtime-core/     # 运行时公共类型、配置常量
├── runtime/          # 运行时服务编排 — 装配全部 crate 的 DI 容器
├── rt-workflow/      # 工作流引擎 — DAG 编排、节点执行器、YAML 序列化
├── rt-messaging/     # 消息平台网关 — 钉钉/飞书/QQ/Slack/微信/WhatsApp/Telegram/Discord
├── rt-webhook/       # 通用 Webhook 服务器
├── rt-dashboard/     # 仪表盘插件框架
├── rt-theme/         # 主题引擎
├── agent/            # AI 智能体核心 — 80+ 模块
│                     #   ReAct引擎/层级规划/深度研究/事实核查/思维树/反思/
│                     #   自验证/错误恢复/RL优化/LoRA微调/评估/工具推荐/A/B测试/
│                     #   协调器/黑板/视觉管线/Web搜索/学术搜索/Wiki编译等
├── orchestrator/     # 智能体编排 — 多智能体调度、DAG 分解、动态子图执行
├── providers/        # 模型提供商适配器（13 种）
├── tools/            # 工具体系 — Tool trait/注册表/编排/流式/沙箱/163+内置工具
├── gateway/          # API 网关 — axum HTTP/WS 服务器、OAuth、速率限制、Prometheus
├── mcp/              # MCP 协议 — stdio + Streamable HTTP + SSE，基于 rmcp
├── trajectory/       # 学习系统 — 记忆/技能进化/用户画像/梦境整合
├── plugins/          # 插件系统 — OpenClaw 兼容、npm 包安装、市场
├── telemetry/        # 可观测性 — OpenTelemetry、结构化日志、运行时指标
├── prompt-guard/     # 提示词注入防护 — L1-L4 多级检测管线
├── npm/              # npm 注册表客户端
├── crdt/             # 协同编辑数据结构
├── device/           # 设备管理
├── axagent-mobile/   # 移动端适配层
├── agent-macro/      # 智能体宏
├── agent-command-types/ # 智能体命令类型
└── schema-gen/       # 数据库 Schema 生成工具
```

### フロントエンドアーキテクチャ

```
src/
├── pages/            # 页面（24 个）
│   ├── ChatPage           # 对话主界面 — 侧边栏/消息流/Agent 面板/多 Tab
│   ├── DashboardPage      # 数据仪表盘 — 用量统计/模型分布/趋势图表
│   ├── WorkflowPage       # 工作流编辑器 — ReactFlow DAG 可视化
│   ├── KnowledgeHubPage   # 知识库管理 — 文档上传/索引/检索
│   ├── MemoryPage         # 记忆管理
│   ├── SkillsPage         # 技能市场
│   ├── SettingsPage       # 设置面板 — 40+ 配置项
│   ├── TerminalPage       # 内置终端 — xterm.js
│   ├── FilesPage          # 文件管理
│   ├── GatewayLinkPage    # API 网关与外部链接管理
│   ├── QuickBarPage       # 快捷栏（独立窗口）
│   ├── DynamicUIManagerPage / DynamicPageViewer  # 动态 UI 引擎
│   ├── WikiGraphPage / WikiEditPage / IngestPage # LLM Wiki
│   ├── LearningGraphPage  # 学习图谱
│   ├── FineTunePage       # LoRA 微调
│   ├── PersonaPage        # 角色管理
│   ├── WorkflowMarketplace # 工作流市场
│   ├── DevTools/          # TraceExplorer / BenchmarkRunner / ToolRecommender
│   └── Workflow/          # WorkflowListPage
│
├── components/       # 33 个模块，500+ 组件
│   ├── chat/         # 对话（消息流/输入/ChatView/TabBar/RightPanel/附件/工具调用渲染）
│   ├── layout/       # 布局 — AppInitializer / Sidebar / ContentArea / TitleBar /
│   │                 #   CommandPalette / GlobalCopyMenu / GlobalErrorBoundary /
│   │                 #   GlobalStatusBar / ErrorNotificationToast / AppHeader 等
│   ├── agent/        # Agent 面板/入口/迷你面板
│   ├── workflow/     # 工作流编辑器（节点/连线/面板/模板/AI辅助）
│   ├── settings/     # 设置面板（40+ 子组件）
│   ├── skill/        # 技能编辑器/渲染器/浮动面板
│   ├── dynamicUI/    # 动态 UI 组件（31 个内置组件）
│   ├── gateway/      # API 网关管理
│   ├── files/        # 文件管理
│   ├── terminal/     # 终端组件
│   ├── search/       # 搜索界面
│   ├── benchmark/    # 基准测试面板
│   ├── decomposition/# 技能分解与工具生成
│   ├── devtools/     # Trace/Span 时间线 + RL Training 面板
│   ├── approval/     # 审批流程界面
│   ├── recommendation/ # 工具/模型推荐
│   ├── onboarding/   # WelcomeWizard / InteractiveTutorial
│   ├── help/         # 帮助面板
│   ├── notification/ # 通知组件
│   ├── proactive/    # 主动建议
│   ├── llm-wiki/     # LLM Wiki 组件
│   ├── wiki/         # Wiki 组件
│   ├── fine-tune/    # 微调界面
│   ├── trace/        # Trace 组件
│   ├── style/        # 样式/主题
│   ├── shared/       # 共享组件（ErrorBoundary / PageContextProvider）
│   └── common/       # 通用组件（Icon 等）
│
├── stores/           # Zustand 状态管理（82 个 store）
│   ├── domain/       # 9 个核心业务 store（对话/流/压缩/偏好/多模型等）
│   ├── feature/      # 61 个功能模块 store（智能体/工作流/知识库/技能/网关/记忆/终端等）
│   ├── shared/       # 8 个跨组件共享 store（UI/标签页/工作区/后端状态等）
│   └── devtools/     # 4 个开发者工具 store
│
├── hooks/            # React Hooks（快捷键/命令面板/响应式/滚动条/主题/Avatar 等）
├── lib/              # 工具函数库（invoke/pageRegistry/shortcuts/skillLifecycle/
│                     #   chartGenerator/codeExecutor/tokenEstimator/workflowLayout 等 45+ 模块）
├── types/            # TypeScript 类型定义
├── theme/            # Shadcn 主题引擎
├── i18n/             # 11 语言翻译文件（zh-CN/zh-TW/en-US/ja/ko/fr/de/es/ru/hi/ar）
├── constants/        # 常量与功能开关
└── sdk/              # ACP 客户端 SDK（TypeScript + Python）
```

### 機能フラグ

プロジェクトは `featureFlags.ts` で段階的な機能リリースを管理します：

| フラグ              | 状態 | 説明                                             |
| ------------------- | ---- | ------------------------------------------------ |
| `AGENT_IN_THE_LOOP` | ✅   | グローバル Agent パネル + ページコンテキスト注入 |
| `DYNAMIC_UI`        | ✅   | ダイナミック UI 構築エンジン                     |
| `SELF_EVOLUTION_UI` | ❌   | 自己進化のフロントエンドコントロールパネル       |
| `NL_EXTENSION`      | ❌   | 自然言語駆動のダイナミックビジネス拡張           |

### Tauri プラグイン

| プラグイン          | 用途                         |
| ------------------- | ---------------------------- |
| `autostart`         | 自動起動                     |
| `clipboard-manager` | クリップボードの読み書き     |
| `dialog`            | ファイル選択ダイアログ       |
| `fs`                | ファイルシステムアクセス     |
| `global-shortcut`   | グローバルショートカット登録 |
| `notification`      | システム通知                 |
| `opener`            | 外部リンク/ファイルを開く    |
| `process`           | プロセス管理                 |
| `updater`           | 自動アップデート             |

---

## データディレクトリ

```
~/.axagent/                    # 应用配置
├── axagent.db                 # SQLite 主数据库 (SeaORM)
├── master.key                 # AES-256 主密钥
├── vector_db/                 # sqlite-vec 向量索引
└── ssl/                       # 自签名 SSL 证书

~/Documents/axagent/          # 用户文件
├── images/                   # 图片附件
├── files/                    # 文件附件
└── backups/                  # 自动备份
```

---

## クイックスタート

### 環境要件

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
npm run tauri dev      # 开发模式（前端 Vite HMR + Tauri 窗口）
```

### ビルド

```bash
npm run tauri build    # 桌面端生产构建

npm run tauri:android:build   # Android 构建
npm run tauri:ios:build       # iOS 构建
```

デスクトップ版のビルド成果物は `src-tauri/target/release/` に出力されます。

### テスト

```bash
npm run test           # 前端单元测试（Vitest watch）
npm run test:run       # 前端单元测试（单次运行）
npm run test:e2e       # E2E 测试（Playwright）

# Rust 后端测试
cd src-tauri && cargo test

# 类型检查 & Lint
npm run typecheck
cd src-tauri && cargo clippy -- -D warnings
npm run format         # dprint 格式化
npm run lint:eslint    # ESLint 检查
npm run contracts      # API 契约检查

# CI 全量检查
npm run ci:check
```

### よく使うスクリプト

| コマンド                 | 用途                                     |
| ------------------------ | ---------------------------------------- |
| `npm run bump`           | バージョン番号のアップグレード（対話式） |
| `npm run docs`           | TypeDoc ドキュメントの生成               |
| `npm run skill:create`   | 新しいスキルのスキャフォールド作成       |
| `npm run skill:validate` | スキル定義の検証                         |
| `npm run check:types`    | 型の整合性チェック                       |

---

## 対応プラットフォーム

| プラットフォーム | アーキテクチャ                        |
| ---------------- | ------------------------------------- |
| Windows          | x86_64, ARM64                         |
| macOS            | Apple Silicon (arm64), Intel (x86_64) |
| Linux            | x86_64, ARM64                         |
| Android          | arm64-v8a, armeabi-v7a, x86_64        |
| iOS              | arm64                                 |

---

## オープンソースライセンス

本プロジェクトは [AGPL-3.0-only](LICENSE) ライセンスの下でオープンソースとして公開されています。

---

## 謝辞

AxAgent は数多くの優れたオープンソースプロジェクトの上に構築されています：

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
