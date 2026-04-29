[**English**](./README-EN.md) | [简体中文](./README.md) | [繁體中文](./README-ZH-TW.md) | **日本語** | [한국어](./README-KO.md) | [Français](./README-FR.md) | [Deutsch](./README-DE.md) | [Español](./README-ES.md) | [Русский](./README-RU.md) | [हिन्दी](./README-HI.md) | [العربية](./README-AR.md)

[![AxAgent](https://github.com/polite0803/AxAgent/blob/main/src/assets/image/logo.png?raw=true)](https://github.com/polite0803/AxAgent)

<p align="center">
  <a href="https://www.producthunt.com/products/axagent?embed=true&amp&amp&utm_source=badge-featured&amp&amp;&amp;#10;&amp;amp&amp&amp;;utm_medium=badge&amp&amp;#10&amp&amp;;utm_campaign=badge-axagent" target="_blank" rel="noopener noreferrer"><img alt="AxAgent - Lightweight, high-perf cross-platform AI desktop client | Product Hunt" width="250" height="54" src="https://api.producthunt.com/widgets/embed-image/v1/featured.svg?post_id=1118403&amp;theme=light&amp;t=1775627359538"></a>
</p>

<p align="center">
  <strong>クロスプラットフォーム AI デスクトップクライアント | マルチエージェントコラボレーション | ローカルファースト</strong>
</p>

<p align="center">
  <a href="https://github.com/polite0803/AxAgent/releases" target="_blank">
    <img src="https://img.shields.io/github/v/release/polite0803/AxAgent?style=flat-square" alt="Release">
  </a>
  <a href="https://github.com/polite0803/AxAgent/actions" target="_blank">
    <img src="https://img.shields.io/github/actions/workflow/status/polite0803/AxAgent/release.yml?style=flat-square" alt="Build">
  </a>
  <img src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-blue?style=flat-square" alt="Platform">
  <img src="https://img.shields.io/badge/license-AGPL--3.0-green?style=flat-square" alt="License">
</p>

---

## AxAgent とは？

AxAgent は、先進的な AI エージェント機能と豊富な開発者ツールを組み合わせた、多機能なクロスプラットフォーム AI デスクトップアプリケーションです。マルチプロバイダーモデルサポート、自律型エージェント実行、ビジュアルワークフローオーケストレーション、ローカルナレッジ管理、内蔵 API ゲートウェイを備えています。

---

## スクリーンショット

| チャットとモデル選択 | マルチエージェントダッシュボード |
|:---:|:---:|
| ![](.github/images/s1-0412.png) | ![](.github/images/s5-0412.png) |

| ナレッジベース RAG | メモリとコンテキスト |
|:---:|:---:|
| ![](.github/images/s3-0412.png) | ![](.github/images/s4-0412.png) |

| ワークフローエディタ | API ゲートウェイ |
|:---:|:---:|
| ![](.github/images/s9-0412.png) | ![](.github/images/s10-0412.png) |

---

## 主な機能

### 🤖 AI モデルサポート

- **マルチプロバイダーサポート** — OpenAI、Anthropic Claude、Google Gemini、Ollama、OpenClaw、Hermes およびすべての OpenAI 互換 API とのネイティブ統合
- **マルチキーローテーション** — 各プロバイダーに対して複数の API キーを設定可能、自动ローテーションでレート制限を分散
- **ローカルモデルサポート** — Ollama ローカルモデルの完全なサポート、GGUF/GGML ファイル管理を含む
- **モデル管理** — リモートモデルリストの取得、カスタマイズ可能なパラメータ（temperature、max tokens、top-p など）
- **ストリーミング出力** — リアルタイムのトークン単位レンダリング、折りたたみ可能な思考ブロック（Claude 拡張思考）をサポート
- **マルチモデル比較** — 複数のモデルに同時に同じ質問を送信し、サイドバイサイドで結果を比較
- **関数呼び出し** — サポートされているすべてのプロバイダーにわたる構造化関数呼び出し

### 🔐 AI エージェントシステム

エージェントシステムは、高度なアーキテクチャに基づいて構築され、以下の機能を備えています：

- **ReAct推論エンジン** — 推論と行動を統合し、自己検証を組み込んでタスク実行の信頼性を確保
- **階層的プランナー** — 複雑なタスクを段階と依存関係を持つ構造化されたプランに分解
- **ツールレジストリ** — 動的なツール登録、意味的バージョン管理与突衝検出をサポート
- **コンピュータ制御** — AI 制御のマウスクリック、キーボード入力、画面スクロール、ビジョンモデル分析との連携
- **画面知覚** — スクリーンキャプチャとビジョンモデル分析，用于 UI 要素の識別
- **3段階の権限モード** — デフォルト（承認が必要）、編集を受け入れる（自動承認）、完全アクセス（プロンプトなし）
- **サンドボックス分離** — エージェント操作は指定された作業ディレクトリに厳密に制限
- **ツール承認パネル** — ツール呼び出しリクエストのリアルタイム表示、項目ごとのレビューをサポート
- **コスト追跡** — 各セッションのトークン使用量とコスト統計のリアルタイム表示
- **一時停止/再開** — エージェントの実行をいつでも一時停止し、後から再開
- **チェックポイントシステム** — クラッシュ回復とセッション再開のための永続化チェックポイント
- **エラー回復エンジン** — 自動エラー分類と回復戦略の実行

### 👥 マルチエージェントコラボレーション

- **サブエージェント調整** — マスター-スレーブアーキテクチャ、複数の協調エージェントをサポート
- **並列実行** — 複数のエージェントがタスクを並行処理、依存関係認識スケジューリングをサポート
- **敵対的デbate** — Pro/Con デbetラウンド、議論強度スコアリングと反論追跡をサポート
- **エージェントロール** — チームコラボレーションのための定義済みロール（研究者、プランナー、開発者、レビュアー、シンセサイザー）
- **エージェントオーケストレーター** — マルチエージェントチームの集中型メッセージルーティングと状態管理
- **コミュニケーショングラフ** — エージェントの相互作用とメッセージフローの視覚的表現

### ⭐ スキルシステム

- **スキルマーケットプレイス** — 組み込みマーケットプレイスでコミュニティ貢献のスキルを閲覧とインストール
- **スキル作成** — プロポーザルから自動的にスキルを作成、Markdown エディタをサポート
- **スキル進化** — 実行フィードバックに基づく AI 駆動の既存スキルの自動分析と改善
- **スキルマッチング** — 意味的マッチングで会話コンテキストに関連するスキルを推奨
- **アトミックスキル** — 複雑なワークフローに構成可能な細粒度スキルコンポーネント
- **スキル分解** — 複雑なタスクの自動分解と実行可能なアトミックスキルへの変換
- **生成ツール** — AI による新しいツールの自動生成と登録、エージェント能力を拡張
- **スキルハブ** — 集中型のスキル発見と設定管理インターフェース
- **スキルハブクライアント** — リモートスキルハブとの統合、コミュニティ共有をサポート

### 🔄 ワークフローシステム

ワークフローエンジンは DAG ベースのタスクオーケストレーションシステムを実装しています：

- **ビジュアルワークフローエディタ** — ドラッグ＆ドロップ式のワークフローデザイナー、ノート接続と設定をサポート
- **豊富なノートタイプ** — 14 のノートタイプ：トリガー、エージェント、LLM、条件、並列、ループ、マージ、遅延、ツール、コード、アトミックスキル、ベクター検索、ドキュメントパーサー、検証
- **ワークフローテンプレート** — 組み込みプリセット：コードレビュー、バグ修正、ドキュメント、テスト、リファクタリング、探索、パフォーマンス、セキュリティ、機能開発
- **DAG 実行** — トポロジカルソートのための Kahn アルゴリズム、循環検出をサポート
- **並列ディスパッチ** — パイプラインスタイルの実行、高速ステップは低速ステップを待ちません
- **再試行ポリシー** — 指数バックオフ、各ステップで設定可能な最大再試行回数
- **部分完了** — 失敗したステップは独立した下流ステップをブロックしません
- **バージョン管理** — ワークフローテンプレートのバージョン管理、ロールバックをサポート
- **実行履歴** — 詳細な記録、ステータス追跡とデバッグをサポート
- **AI 支援** — AI 支援ワークフロー設計と最適化

### 📚 ナレッジとメモリ

- **ナレッジベース（RAG）** — マルチナレッジベースサポート、ドキュメントアップロード、自動解析、、チャンク化、ベクターインデックスをサポート
- **ハイブリッド検索** — ベクター類似性検索と BM25 全文ランキングの組み合わせ
- **リランキング** — クロスエンコーダーリランキング、取得精度の向上
- **ナレッジグラフ** — ナレッジ関連性のエンティティ関係可視化
- **メモリシステム** — マルチ名前空間メモリ、手動入力または AI 自動抽出をサポート
- **クロースドループメモリ** — Honcho と Mem0 永続化メモリプロバイダーとの統合
- **FTS5 全文検索** — 会話、ファイル、メモリ全体の高速検索
- **セッション検索** — すべての会話セッション全体の高度な検索
- **コンテキスト管理** — ファイル、検索結果、ナレッジスニペット、メモリ、ツール出力の柔軟な添付

### 🌐 API ゲートウェイ

- **ローカル API サーバー** — 組み込みの OpenAI 互換、Claude、 Gemini インターフェースサーバー
- **外部リンク** — ワンクリックで Claude CLI、OpenCode との統合、API キーの自動同期
- **キー管理** — 生成、取り消し、有効化/無効化、説明付きアクセスキーの管理
- **使用量分析** — キー、プロバイダー、日付ごとのリクエスト量とトークン使用量
- **SSL/TLS サポート** — 組み込み自己署名証明書、カスタム証明書をサポート
- **リクエストログ** — すべての API リクエストとレスポンスの完全な記録
- **設定テンプレート** — Claude、Codex、OpenCode、Gemini のプリセットテンプレート
- **リアルタイム API** — OpenAI リアルタイム API 互換の WebSocket イベントプッシュ
- **プラットフォーム統合** — 钉钉、飛書、QQ、Slack、WeChat、WhatsApp のサポート

### 🔧 ツールと拡張

- **MCP プロトコル** — 完全なモデルコンテキストプロトコル実装、stdio と HTTP/WebSocket トランスポートをサポート
- **OAuth 認証** — MCP サーバーの OAuth フローサポート
- **組み込みツール** — ファイル操作、コード実行、検索などの総合的なツールセット
- **LSP クライアント** — 組み込み言語サーバープロトコル、コード補完と診断をサポート
- **ターミナルバックエンド** — ローカル、Docker、SSH ターミナル接続をサポート
- **ブラウザ自動化** — CDP によるブラウザ制御機能の統合
- **UI 自動化** — クロスプラットフォーム UI 要素識別と制御
- **Git ツール** — ブランチ検出と競合認識をサポートする Git 操作

### 📊 コンテンツレンダリング

- **Markdown レンダリング** — コードハイライト、LaTeX 数式、テーブル、タスクリストの完全なサポート
- **Monaco コードエディタ** — 組み込みエディタ、構文ハイライト、コピー、差分プレビューをサポート
- **ダイアグラムレンダリング** — Mermaid フローチャート、D2 アーキテクチャダイアグラム、ECharts インタラクティブチャート
- **アーティファクトパネル** — コードスニペット、HTML ドラフト、React コンポーネント、Markdown ノート、リアルタイムプレビューをサポート
- **3つのプレビューモード** — コード（エディタ）、スプリット（並列）、プレビュー（レンダリングのみ）
- **セッションインスペクター** — セッション構造のツリービュー、快速ナビゲーション
- **引用パネル** — ソース引用の追跡と表示、信頼性スコアリングをサポート

### 🛡️ データとセキュリティ

- **AES-256 暗号化** — API キーと機密データは AES-256-GCM で暗号化
- **分離ストレージ** — アプリケッションデータは `~/.axagent/`、ユーザーファイルは `~/Documents/axagent/` に保存
- **自動バックアップ** — ローカルディレクトリまたは WebDAV ストレージへのスケジュールバックアップ
- **バックアップ復元** — ワンクリックで履歴バックアップから復元
- **エクスポートオプション** — PNG スクリーンショット、Markdown、プレーンテキスト、JSON 形式
- **ストレージ管理** — 視覚的なディスク使用量表示とクリーンアップツール

### 🖥️ デスクトップ体験

- **テーマエンジン** — ダーク/ライトテーマ、システムフォローまたは手動設定をサポート
- **インターフェース言語** — 12 の言語：簡体字中文、繁体字中文、英語、日本語、韓国語、フランス語、ドイツ語、スペイン語、ロシア語、ヒンディー語、アラビア語
- **システムトレイ** — バックグラウンドサービスを中断せずにトレイに最小化
- **常に手前** — 他のウィンドウより前にウィンドウを固定
- **グローバルショートカット** — カスタム可能やかな，均.oきまeld
- **自動起動** — システム起動時のオプションの起動
- **プロキシサポート** — HTTP と SOCKS5 プロキシ設定
- **自動更新** — 自動バージョン確認と更新プロンプト
- **コマンドパレット** — `Cmd/Ctrl+K` クイックコマンドアクセス

### 🔬 上級機能

- **Cron スケジューラー** — 毎日/毎週/毎月テンプレートとカスタム cron 式による自動化タスクスケジューリング
- **Webhook システム** — ツール完了、エージェントエラー、セッション終了通知のイベントサブスクリプション
- **ユーザー プロファイリング** — コードスタイル、命名規則、インデント、コメントスタイル、コミュニケーション設定の自動学習
- **RL オプティマイザー** — ツール選択とタスク戦略の最適化のための強化学習
- **LoRA ファ微調整** — LoRA によるローカルトレーニングを使用したカスタムモデル適応
- **主动的提案** — 会話内容とユーザーパターンに基づくコンテキスト対応のヒント
- **思考チェーン** — エージェントの意思決定推論の視覚化、ステップバイステップ分解
- **エラー回復** — 自動エラー分類、根本原因分析、回復提案
- **開発者ツール** — デバッグとパフォーマンス分析のための Trace、Span、タイムライン可視化
- **ベンチマークシステム** — スコアカード付きのタスクパフォーマンス評価と指標
- **スタイル転送** — 学習したコードスタイル設定生成されたコードに適用
- **ダッシュボードプラグイン** — カスタムパネルとウィジェットをサポートする拡張可能なダッシュボード

---

## 技術アーキテクチャ

### 技術スタック

| レイヤー | 技術 |
|---------|------|
| **フレームワーク** | Tauri 2 + React 19 + TypeScript |
| **UI** | Ant Design 6 + TailwindCSS 4 |
| **状態管理** | Zustand 5 |
| **国際化** | i18next + react-i18next |
| **バックエンド** | Rust + SeaORM + SQLite |
| **ベクトル DB** | sqlite-vec |
| **コードエディタ** | Monaco Editor |
| **ダイアグラム** | Mermaid + D2 + ECharts |
| **ターミナル** | xterm.js |
| **ビルド** | Vite + npm |

### Rust バックエンドアーキテクチャ

バックエンドは、Rust workspace として整理された専門化した crates で構成されています：

```
src-tauri/crates/
├── agent/         # AI エージェントコア
│   ├── react_engine.rs       # ReAct 推論エンジン
│   ├── tool_registry.rs      # 動的ツール登録
│   ├── coordinator.rs        # エージェント調整
│   ├── hierarchical_planner.rs # タスク分解
│   ├── self_verifier.rs      # 出力検証
│   ├── error_recovery_engine.rs # エラー処理
│   ├── vision_pipeline.rs    # 画面知覚
│   └── fine_tune/            # LoRA ファ微調整
│
├── core/          # コアユーティリティ
│   ├── db.rs               # SeaORM データベース
│   ├── vector_store.rs     # sqlite-vec 統合
│   ├── rag.rs             # RAG 抽象化レイヤー
│   ├── hybrid_search.rs    # ベクター + FTS5 検索
│   ├── crypto.rs           # AES-256 暗号化
│   └── mcp_client.rs       # MCP プロトコルクライアント
│
├── gateway/       # API ゲートウェイ
│   ├── server.rs          # HTTP サーバー
│   ├── handlers.rs         # API ハンドラー
│   ├── auth.rs            # 認証
│   └── realtime.rs        # WebSocket サポート
│
├── providers/     # モデルアダプター
│   ├── openai.rs         # OpenAI API
│   ├── anthropic.rs      # Claude API
│   ├── gemini.rs         # Gemini API
│   └── ollama.rs         # Ollama ローカル
│
├── runtime/       # ランタイムサービス
│   ├── session.rs        # セッション管理
│   ├── workflow_engine.rs # DAG オーケストレーション
│   ├── mcp.rs            # MCP サーバー
│   ├── cron/             # タスクスケジューリング
│   ├── terminal/         # ターミナルバックエンド
│   ├── shell_hooks.rs    # Shell 統合
│   └── message_gateway/  # プラットフォーム統合
│
└── trajectory/   # 学習システム
    ├── memory.rs         # メモリ管理
    ├── skill.rs          # スキルシステム
    ├── rl.rs             # RL 報酬信号
    ├── behavior_learner.rs # パターン学習
    └── user_profile.rs   # ユーザープロファイリング
```

### フロントエンドアーキテクチャ

```
src/
├── stores/                    # Zustand 状態管理
│   ├── domain/               # コアビジネス状態
│   │   ├── conversationStore.ts
│   │   ├── messageStore.ts
│   │   └── streamStore.ts
│   ├── feature/               # 功能モジュール状態
│   │   ├── agentStore.ts
│   │   ├── gatewayStore.ts
│   │   ├── workflowEditorStore.ts
│   │   └── knowledgeStore.ts
│   └── shared/                # 共有状態
│
├── components/
│   ├── chat/                # チャットインターフェース（60+ コンポーネント）
│   ├── workflow/            # ワークフローエディタ
│   ├── gateway/             # API ゲートウェイ UI
│   ├── settings/            # 設定パネル
│   └── terminal/            # ターミナル UI
│
└── pages/                    # ページコンポーネント
```

### プラットフォームサポート

| プラットフォーム | アーキテクチャ |
|----------------|---------------|
| macOS | Apple Silicon (arm64), Intel (x86_64) |
| Windows | x86_64, ARM64 |
| Linux | x86_64, ARM64 (AppImage/deb/rpm) |

## クイックスタート

### ビルド済みダウンロード

[Releases](https://github.com/polite0803/AxAgent/releases) ページにアクセス、お使いのプラットフォーム用のインストーラをダウンロードしてください。

### ソースからビルド

#### 必要環境

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://www.rust-lang.org/) 1.75+
- [npm](https://www.npmjs.com/) 10+
- Windows: [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) + Rust MSVC targets

#### ビルド手順

```bash
# リポジトリをクローン
git clone https://github.com/polite0803/AxAgent.git
cd AxAgent

# 依存関係をインストール
npm install

# 開発モード
npm run tauri dev

# フロントエンドのみビルド
npm run build

# デスクトップアプリケーションをビルド
npm run tauri build
```

ビルド成果物は `src-tauri/target/release/` にあります。

### テスト

```bash
# ユニットテスト
npm run test

# E2E テスト
npm run test:e2e

# 型チェック
npm run typecheck
```

---

## プロジェクト構造

```
AxAgent/
├── src/                         # フロントエンドソース (React + TypeScript)
│   ├── components/              # React コンポーネント
│   │   ├── chat/               # チャットインターフェース（60+ コンポーネント）
│   │   ├── workflow/           # ワークフローエディタコンポーネント
│   │   ├── gateway/            # API ゲートウェイコンポーネント
│   │   ├── settings/           # 設定パネル
│   │   └── terminal/          # ターミナルコンポーネント
│   ├── pages/                   # ページコンポーネント
│   ├── stores/                  # Zustand 状態管理
│   │   ├── domain/            # コアビジネス状態
│   │   └── feature/           # 功能モジュール状態
│   ├── hooks/                   # React hooks
│   ├── lib/                     # ユーティリティ関数
│   ├── types/                   # TypeScript 型定義
│   └── i18n/                    # 12 か国語の翻訳
│
├── src-tauri/                    # バックエンドソース (Rust)
│   ├── crates/                  # Rust workspace（9 crates）
│   │   ├── agent/             # AI エージェントコア
│   │   ├── core/              # データベース、暗号化、RAG
│   │   ├── gateway/           # API ゲートウェイサーバー
│   │   ├── providers/         # モデルプロバイダーアダプター
│   │   ├── runtime/           # ランタイムサービス
│   │   ├── trajectory/       # メモリと学習
│   │   └── telemetry/        # トレーシングと指標
│   └── src/                    # Tauri エントリーポイント
│
├── e2e/                        # Playwright E2E テスト
├── scripts/                    # ビルドスクリプト
└── docs/                       # ドキュメント
```

## データディレクトリ

```
~/.axagent/                      # 設定ディレクトリ
├── axagent.db                   # SQLite データベース
├── master.key                   # AES-256 マスターキー
├── vector_db/                   # ベクトルデータベース (sqlite-vec)
└── ssl/                         # SSL 証明書

~/Documents/axagent/            # ユーザーファイルディレクトリ
├── images/                     # 画像添付ファイル
├── files/                      # ファイル添付ファイル
└── backups/                    # バックアップファイル
```

---

## よくある質問

### macOS：「アプリが破損しています」または「開発者を検証できません」

アプリが Apple によって署名されていないため：

**1. 「すべてのソース」からのアプリを許可**
```bash
sudo spctl --master-disable
```

次に **システム設定 → プライバシーとセキュリティ → セキュリティ** に移動し、**すべてのソース** を選択して ****。

**2. 検疫属性を削除**
```bash
sudo xattr -dr com.apple.quarantine /Applications/AxAgent.app
```

**3. macOS Ventura+ の追加手順**
**システム設定 → プライバシーとセキュリティ** に移動し、**それでも開く** をクリックします。

---

## コミュニティ

- [LinuxDO](https://linux.do)

## ライセンス

このプロジェクトは [AGPL-3.0](LICENSE) ライセンスの下でライセンスされています。
