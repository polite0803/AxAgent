[**English**](./README-EN.md) | [简体中文](./README.md) | [繁體中文](./README-ZH-TW.md) | **日本語** | [한국어](./README-KO.md) | [Français](./README-FR.md) | [Deutsch](./README-DE.md) | [Español](./README-ES.md) | [Русский](./README-RU.md) | [हिन्दी](./README-HI.md) | [العربية](./README-AR.md)

[![AxAgent](https://github.com/polite0803/AxAgent/blob/main/src/assets/image/logo.png?raw=true)](https://github.com/polite0803/AxAgent)

<p align="center">
  <a href="https://www.producthunt.com/products/axagent?embed=true&amp&amp&utm_source=badge-featured&amp&amp;&amp;#10;&amp;amp&amp&amp;;utm_medium=badge&amp&amp;#10&amp;amp;;utm_campaign=badge-axagent" target="_blank" rel="noopener noreferrer"><img alt="AxAgent - Lightweight, high-perf cross-platform AI desktop client | Product Hunt" width="250" height="54" src="https://api.producthunt.com/widgets/embed-image/v1/featured.svg?post_id=1118403&amp;theme=light&amp;t=1775627359538"></a>
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
- **マルチキーローテーション** — 各プロバイダーに対して複数の API キーを設定可能、自動ローテーションでレート制限を分散
- **ローカルモデルサポート** — Ollama ローカルモデルの完全なサポート、GGUF/GGML ファイル管理を含む
- **モデル管理** — リモートモデルリストの取得、カスタマイズ可能なパラメータ（temperature、max tokens、top-p など）
- **ストリーミング出力** — リアルタイムのトークン単位レンダリング、折りたたみ可能な思考ブロック（Claude 拡張思考）をサポート
- **マルチモデル比較** — 複数のモデルに同時に同じ質問を送信し、サイドバイサイドで結果を比較
- **関数呼び出し** — サポートされているすべてのプロバイダーにわたる構造化関数呼び出し
- **OpenAI Responses API** — OpenAI Responses 形式の転送をサポート
- **リアルタイム API** — OpenAI リアルタイム API 互換の WebSocket イベントプッシュ

### 🔐 AI エージェントシステム

エージェントシステムは、高度なアーキテクチャに基づいて構築され、以下の機能を備えています：

- **ReAct推論エンジン** — 推論と行動を統合し、自己検証を組み込んでタスク実行の信頼性を確保
- **階層的プランナー** — 複雑なタスクを段階と依存関係を持つ構造化されたプランに分解
- **タスク分解器** — 複雑なタスクを実行可能なサブタスクに自動分解
- **ディープリサーチ** — マルチソース検索オーケストレーション、引用追跡と信頼性評価
- **ファクトチェック** — AI 駆動の事実検証とソース分類
- **検索オーケストレーション** — マルチ検索プロバイダー調整、検索計画と結果統合をサポート
- **学術検索** — 学術文献検索と引用分析
- **コンピュータ制御** — AI 制御のマウスクリック、キーボード入力、画面スクロール、ビジョンモデル分析との連携
- **画面知覚** — スクリーンキャプチャとビジョンモデル分析、UI 要素の識別に使用
- **3段階の権限モード** — デフォルト（承認が必要）、編集を受け入れる（自動承認）、完全アクセス（プロンプトなし）
- **サンドボックス分離** — エージェント操作は指定された作業ディレクトリに厳密に制限
- **ツール承認パネル** — ツール呼び出しリクエストのリアルタイム表示、項目ごとのレビューをサポート
- **コスト追跡** — 各セッションのトークン使用量とコスト統計のリアルタイム表示
- **一時停止/再開** — エージェントの実行をいつでも一時停止し、後から再開
- **チェックポイントシステム** — クラッシュ回復とセッション再開のための永続化チェックポイント
- **エラー回復エンジン** — 自動エラー分類、根本原因分析と回復戦略の実行
- **ループ検出** — エージェント推論中の循環動作の自動検出と中断
- **思考チェーン** — エージェントの意思決定推論の視覚化、ステップバイステップ分解
- **プロアクティブモード** — エージェントが自発的に提案と操作を実行
- **目的管理** — エージェントの実行目的とコンテキストの維持と追跡

### 👥 マルチエージェントコラボレーション

- **サブエージェント調整** — マスター-スレーブアーキテクチャ、複数の協調エージェントをサポート
- **並列実行** — 複数のエージェントがタスクを並行処理、依存関係認識スケジューリングをサポート
- **敵対的ディベート** — Pro/Con ディベートラウンド、議論強度スコアリングと反論追跡をサポート
- **エージェントロール** — チームコラボレーションのための定義済みロール（研究者、プランナー、開発者、レビュアー、シンセサイザー）
- **エージェントオーケストレーター** — マルチエージェントチームの集中型メッセージルーティングと状態管理
- **コミュニケーショングラフ** — エージェントの相互作用とメッセージフローの視覚的表現
- **Swarm クラスタ** — マルチプロセスエージェントクラスタ、権限同期と自動再接続をサポート
- **Buddy パートナーシステム** — 設定可能なエージェントパートナー、種と属性の定義をサポート
- **共有メモリ** — エージェント間で共有されるメモリ空間、統計とクエリをサポート
- **チーム Cron 登録** — チームレベルの定期タスクスケジューリング

### ⭐ スキルシステム

- **スキルマーケットプレイス** — 組み込みマーケットプレイスでコミュニティ貢献のスキルを閲覧とインストール
- **スキル作成** — プロポーザルから自動的にスキルを作成、Markdown エディタをサポート
- **スキル進化** — 実行フィードバックに基づく AI 駆動の既存スキルの自動分析と改善
- **スキルマッチング** — 意味的マッチングで会話コンテキストに関連するスキルを推奨
- **スキル分解** — 複雑なタスクの自動分解と実行可能なアトミックスキルへの変換（LLM支援/マルチターン/ワークフロー検証）
- **生成ツール** — AI による新しいツールの自動生成と登録、エージェント能力を拡張
- **スキルハブ** — 集中型のスキル発見と設定管理インターフェース
- **スキルハブクライアント** — リモートスキルハブとの統合、コミュニティ共有をサポート
- **スキル依存チェック** — スキル依存関係とツール可用性の自動検出
- **スキルサンドボックスコンテナ** — スキルを隔離環境で安全に実行

### 🔄 ワークフローシステム

ワークフローエンジンは DAG ベースのタスクオーケストレーションシステムを実装しています：

- **ビジュアルワークフローエディタ** — ドラッグ＆ドロップ式のワークフローデザイナー、ノード接続と設定をサポート
- **豊富なノードタイプ** — 15 のノードタイプ：トリガー、エージェント、LLM、条件、並列、ループ、マージ、遅延、ツール、コード、サブワークフロー、ベクター検索、ドキュメントパーサー、検証、終了
- **ワークフローテンプレート** — 組み込みプリセット：コードレビュー、バグ修正、ドキュメント、テスト、リファクタリング、探索、パフォーマンス、セキュリティ、機能開発
- **DAG 実行** — トポロジカルソートのための Kahn アルゴリズム、循環検出をサポート
- **並列ディスパッチ** — パイプラインスタイルの実行、高速ステップは低速ステップを待ちません
- **再試行ポリシー** — 指数バックオフ、各ステップで設定可能な最大再試行回数
- **部分完了** — 失敗したステップは独立した下流ステップをブロックしません
- **バージョン管理** — ワークフローテンプレートのバージョン管理、ロールバックをサポート
- **実行履歴** — 詳細な記録、ステータス追跡とデバッグをサポート
- **AI 支援** — AI 支援ワークフロー設計、ノード推奨とエージェントプロンプト最適化
- **セマンティックチェック** — ワークフローのセマンティック検証、潜在的な問題を検出
- **n8n インポート** — n8n ディレクトリからのワークフローインポートをサポート
- **デバッグパネル** — ワークフロー実行プロセスのリアルタイムデバッグと状態確認

### 📚 ナレッジとメモリ

- **ナレッジベース（RAG）** — マルチナレッジベースサポート、ドキュメントアップロード、自動解析、チャンク化、ベクターインデックスをサポート
- **ハイブリッド検索** — ベクター類似性検索と BM25 全文ランキングの組み合わせ
- **リランキング** — クロスエンコーダーリランキング、取得精度の向上
- **3段階リコールパイプライン** — AST インデックス + ベクター検索 + FTS5 のマルチレベルリコール機構
- **ナレッジグラフ** — ナレッジ関連性のエンティティ関係可視化（エンティティ、属性、関係、フロー、インターフェース）
- **Wiki システム** — LLM Wiki コンパイラとバリデーター、ナレッジグラフ可視化と増分同期をサポート
- **Wiki ノート** — 双方向リンクノートシステム、グラフビューと自動リンク同期をサポート
- **メモリシステム** — マルチ名前空間メモリ、手動入力または AI 自動抽出をサポート
- **クローズドループメモリ** — Honcho と Mem0 永続化メモリプロバイダーとの統合
- **FTS5 全文検索** — 会話、ファイル、メモリ全体の高速検索
- **セッション検索** — すべての会話セッション全体の高度な検索
- **コンテキスト管理** — ファイル、検索結果、ナレッジスニペット、メモリ、ツール出力の柔軟な添付
- **ドキュメントパーサー** — マルチフォーマットドキュメントの自動解析とコンテンツ抽出
- **増分インデックス** — ファイル変更の増分インデックス更新

### 🌐 API ゲートウェイ

- **ローカル API サーバー** — 組み込みの OpenAI 互換、Claude、Gemini インターフェースサーバー
- **外部リンク** — ワンクリックで Claude CLI、OpenCode との統合、API キーとモデルの自動同期
- **キー管理** — 生成、取り消し、有効化/無効化、説明付きアクセスキーの管理
- **使用量分析** — キー、プロバイダー、日付ごとのリクエスト量とトークン使用量
- **SSL/TLS サポート** — 組み込み自己署名証明書、カスタム証明書をサポート
- **リクエストログ** — すべての API リクエストとレスポンスの完全な記録
- **設定テンプレート** — Claude、Codex、OpenCode、Gemini のプリセットテンプレート
- **リアルタイム API** — OpenAI リアルタイム API 互換の WebSocket イベントプッシュ
- **プラットフォーム統合** — 钉钉、飛書、QQ、Slack、WeChat、WhatsApp、Telegram、Discord のサポート
- **ゲートウェイ診断** — 接続診断とプログラムポリシー管理
- **レートリミッター** — API リクエストのレート制限とトラフィック制御
- **永続化キュー** — リクエストの永続化キュー管理

### 🔧 ツールと拡張

- **MCP プロトコル** — 完全なモデルコンテキストプロトコル実装、stdio と HTTP/WebSocket トランスポートをサポート
- **OAuth 認証** — MCP サーバーの OAuth フローサポート
- **MCP 自動起動** — MCP サーバーの自動起動とライフサイクル管理
- **MCP ツールブリッジ** — MCP ツールとエージェントツールシステムのブリッジング
- **プラグインシステム** — 内蔵/バンドル/外部の3層プラグインアーキテクチャ、ツール登録、フックとライフサイクル管理をサポート
- **組み込みツール** — 包括的なファイル操作（読み/書き/編集）、コード実行、検索（Grep/Glob）、Bash、Web 検索、Web スクレイピング、プラン管理、Cron スケジューリング、REPL、LSP、コンテキスト管理、コンピュータ制御、メッセージプッシュ、ToDo など
- **ツール権限システム** — ツール権限の分類、ルール管理と使用追跡
- **Bash セキュリティ** — コマンド解析、パス検証とサンドボックスセキュリティ制御
- **LSP クライアント** — 組み込み言語サーバープロトコル、コード補完と診断をサポート
- **AST インデックス** — コードファイルの AST 解析とインデックス構築
- **ターミナルバックエンド** — ローカル、Docker、SSH ターミナル接続をサポート
- **ブラウザ自動化** — CDP によるブラウザ制御機能の統合（ナビゲーション、スクリーンショット、クリック、入力、テキスト抽出など）
- **UI 自動化** — クロスプラットフォーム UI 要素識別と制御
- **Git ツール** — ブランチ検出と競合認識をサポートする Git 操作
- **ツール推奨** — コンテキストベースのインテリジェントツール推奨エンジン
- **ツールオーケストレーション** — マルチツールの協調実行とストリーミング出力
- **ツール統計** — ツール使用頻度とパフォーマンス統計

### 📊 コンテンツレンダリング

- **Markdown レンダリング** — コードハイライト、LaTeX 数式、テーブル、タスクリストの完全なサポート
- **Monaco コードエディタ** — 組み込みエディタ、構文ハイライト、コピー、差分プレビューをサポート
- **ダイアグラムレンダリング** — Mermaid フローチャート、D2 アーキテクチャダイアグラム、ECharts インタラクティブチャート
- **アーティファクトパネル** — コードスニペット、HTML ドラフト、React コンポーネント、Markdown ノート、リアルタイムプレビューをサポート
- **4つのプレビューモード** — コード（エディタ）、スプリット（並列）、プレビュー（レンダリングのみ）、React コンポーネントプレビュー
- **セッションインスペクター** — セッション構造のツリービュー、クイックナビゲーション
- **引用パネル** — ソース引用の追跡と表示、信頼性スコアリングをサポート
- **インフォグラフィックレンダリング** — インフォグラフィックの可視化表示をサポート

### 🛡️ データとセキュリティ

- **AES-256 暗号化** — API キーと機密データは AES-256-GCM で暗号化
- **分離ストレージ** — アプリケーションデータは `~/.axagent/`、ユーザーファイルは `~/Documents/axagent/` に保存
- **自動バックアップ** — ローカルディレクトリまたは WebDAV ストレージへのスケジュールバックアップ
- **バックアップ復元** — ワンクリックで履歴バックアップから復元
- **エクスポートオプション** — PNG スクリーンショット、Markdown、プレーンテキスト、JSON 形式
- **ストレージ管理** — 視覚的なディスク使用量表示とクリーンアップツール
- **ファイル認可** — ファイルアクセスの認可と取り消し管理
- **操作監査** — 重要操作の監査ログ記録

### 🖥️ デスクトップ体験

- **テーマエンジン** — ダーク/ライトテーマ、システムフォローまたは手動設定をサポート
- **インターフェース言語** — 11 の言語：簡体字中文、繁体字中文、英語、日本語、韓国語、フランス語、ドイツ語、スペイン語、ロシア語、ヒンディー語、アラビア語
- **システムトレイ** — バックグラウンドサービスを中断せずにトレイに最小化
- **常に手前** — 他のウィンドウより前にウィンドウを固定
- **グローバルショートカット** — カスタマイズ可能なショートカットでメインウィンドウを呼び出し
- **QuickBar** — クイックアクセスフローティングバー、ワンクリックで起動
- **自動起動** — システム起動時のオプションの起動
- **プロキシサポート** — HTTP と SOCKS5 プロキシ設定
- **自動更新** — 自動バージョン確認と更新プロンプト
- **コマンドパレット** — `Cmd/Ctrl+K` クイックコマンドアクセス
- **オンボーディングウィザード** — 初回起動時のインタラクティブガイドと Ollama 検出
- **通知センター** — 統合されたアプリ内通知管理

### 🔬 上級機能

- **ディープリサーチ** — マルチソース検索、引用追跡、信頼性評価とコンテンツ統合
- **ファクトチェック** — AI 駆動の事実検証とソース分類
- **Cron スケジューラー** — 毎日/毎週/毎月テンプレートとカスタム cron 式による自動化タスクスケジューリング
- **Webhook システム** — ツール完了、エージェントエラー、セッション終了通知のイベントサブスクリプション
- **ユーザープロファイリング** — コードスタイル、命名規則、インデント、コメントスタイル、コミュニケーション設定の自動学習
- **RL オプティマイザー** — ツール選択とタスク戦略の最適化のための強化学習
- **LoRA ファインチューニング** — LoRA によるローカルトレーニングを使用したカスタムモデル適応
- **プロアクティブ提案** — 会話内容とユーザーパターンに基づくコンテキスト対応のヒント
- **コンテキスト予測** — ユーザーの次の操作を予測し、関連リソースを事前取得
- **ドリーム統合** — バックグラウンドでメモリとパターンを自動統合、長期知識を最適化
- **エラー回復** — 自動エラー分類、根本原因分析、回復提案
- **開発者ツール** — デバッグとパフォーマンス分析のための Trace、Span、タイムライン可視化
- **ベンチマークシステム** — SWE-bench / Terminal-bench タスクパフォーマンス評価と指標、スコアカード付き
- **スタイル転送** — 学習したコードスタイル設定を生成されたコードに適用
- **ダッシュボードプラグイン** — カスタムパネルとウィジェットをサポートする拡張可能なダッシュボード
- **コラボレーション共有** — CRDT リアルタイムコラボレーションとワンクリックセッション共有
- **ブラウザ拡張** — Wiki Clipper ブラウザ拡張、Web ページを LLM Wiki に素早くクリップ
- **Python SDK** — AxAgent との統合のための Python SDK を提供
- **スマートルーター** — リクエストのインテリジェントルーティングと分類
- **セマンティックキャッシュ** — セマンティクスベースのレスポンスキャッシュ、重複計算を削減
- **コンテキスト圧縮** — 長いコンテキストの自動圧縮、トークン使用量を最適化
- **メッセージバッチ処理** — メッセージの一括送信と最適化
- **接続プール** — データベースと API 接続プール管理
- **フィーチャーフラグ** — 設定可能な機能フィーチャーフラグシステム
- **ポリシーエンジン** — 権限と操作ポリシーの集中管理
- **リソースガバナー** — エージェントのリソース使用制限とガバナンス
- **LAN 転送** — ローカルエリアネットワークファイル転送機能

---

## 技術アーキテクチャ

### 技術スタック

| レイヤー | 技術 |
|---------|------|
| **フレームワーク** | Tauri 2 + React 19 + TypeScript 6 |
| **UI** | Ant Design 6 + TailwindCSS 4 |
| **状態管理** | Zustand 5 |
| **ルーティング** | React Router 7 |
| **国際化** | i18next + react-i18next |
| **バックエンド** | Rust + SeaORM 2 + SQLite |
| **ベクトル DB** | sqlite-vec |
| **コードエディタ** | Monaco Editor |
| **ダイアグラム** | Mermaid + D2 + ECharts（CDN） |
| **ターミナル** | xterm.js 6 |
| **ワークフロー** | ReactFlow 11 |
| **ビルド** | Vite 8 + npm |

### Rust バックエンドアーキテクチャ

バックエンドは、Rust workspace として整理された 10 の専門化した crates で構成されています：

```
src-tauri/crates/
├── agent/         # AI エージェントコア
│   ├── react_engine.rs          # ReAct 推論エンジン
│   ├── coordinator.rs           # エージェント調整
│   ├── hierarchical_planner.rs  # タスク分解
│   ├── task_decomposer.rs       # サブタスク分解
│   ├── self_verifier.rs         # 出力検証
│   ├── verification_agent.rs    # 検証エージェント
│   ├── error_recovery_engine.rs # エラー回復エンジン
│   ├── error_classifier.rs      # エラー分類
│   ├── recovery_strategies.rs   # 回復戦略
│   ├── loop_detector.rs         # ループ検出
│   ├── vision_pipeline.rs       # 画面知覚
│   ├── deep_research.rs         # ディープリサーチ
│   ├── fact_checker.rs          # ファクトチェック
│   ├── research_agent.rs        # リサーチエージェント
│   ├── search_planner.rs        # 検索計画
│   ├── search_orchestrator.rs   # 検索オーケストレーション
│   ├── academic_search.rs       # 学術検索
│   ├── source_validator.rs      # ソース検証
│   ├── source_classifier.rs     # ソース分類
│   ├── credibility_evaluator.rs # 信頼性評価
│   ├── citation_tracker.rs      # 引用追跡
│   ├── content_synthesizer.rs   # コンテンツ統合
│   ├── outline_builder.rs       # アウトライン構築
│   ├── reference_builder.rs     # 参考文献構築
│   ├── proactive_mode.rs        # プロアクティブモード
│   ├── purpose_manager.rs       # 目的管理
│   ├── graph_insights.rs        # グラフインサイト
│   ├── insight_generator.rs     # インサイト生成
│   ├── schema_manager.rs        # スキーマ管理
│   ├── ingest_pipeline.rs       # データインジェストパイプライン
│   ├── session_manager.rs       # セッション管理
│   ├── health_checker.rs        # ヘルスチェック
│   ├── metrics.rs               # メトリクス収集
│   ├── evaluator/               # ベンチマーク評価
│   ├── fine_tune/               # LoRA ファインチューニング
│   ├── rl_optimizer/            # RL 戦略最適化
│   └── tool_recommender/        # ツール推奨エンジン
│
├── core/          # コアユーティリティ
│   ├── db.rs                   # SeaORM データベース
│   ├── vector_store.rs         # sqlite-vec 統合
│   ├── rag.rs                  # RAG 抽象化レイヤー
│   ├── hybrid_search.rs        # ベクター + FTS5 検索
│   ├── recall_pipeline.rs      # 3段階リコールパイプライン
│   ├── crypto.rs               # AES-256 暗号化
│   ├── mcp_client.rs           # MCP プロトコルクライアント
│   ├── browser_automation.rs   # ブラウザ自動化
│   ├── computer_control.rs     # コンピュータ制御
│   ├── screen_vision.rs        # 画面ビジョン
│   ├── screen_capture.rs       # 画面キャプチャ
│   ├── ui_automation.rs        # UI 自動化
│   ├── ast_index.rs            # AST インデックス
│   ├── incremental_indexer.rs  # 増分インデックス
│   ├── document_parser.rs      # ドキュメントパーサー
│   ├── markdown_parser.rs      # Markdown パーサー
│   ├── text_chunker.rs         # テキストチャンキング
│   ├── token_counter.rs        # トークンカウント
│   ├── token_budget.rs         # トークンバジェット
│   ├── file_index.rs           # ファイルインデックス
│   ├── file_authorizer.rs      # ファイル認可
│   ├── file_store.rs           # ファイルストア
│   ├── cache.rs                # キャッシュ管理
│   ├── disk_cache.rs           # ディスクキャッシュ
│   ├── cache_persister.rs      # キャッシュ永続化
│   ├── cache_snapshot.rs       # キャッシュスナップショット
│   ├── vector_cache.rs         # ベクターキャッシュ
│   ├── marketplace_service.rs  # マーケットプレイスサービス
│   ├── marketplace.rs          # マーケットプレイス抽象化
│   ├── operation_audit.rs      # 操作監査
│   ├── unified_config.rs       # 統一設定
│   ├── platform_config.rs      # プラットフォーム設定
│   ├── command_validator.rs    # コマンド検証
│   ├── shell_parser.rs         # Shell パーサー
│   ├── output_processor.rs     # 出力処理
│   ├── storage_inventory.rs    # ストレージインベントリ
│   ├── storage_migration.rs    # ストレージマイグレーション
│   ├── storage_paths.rs        # ストレージパス
│   ├── s3_backup.rs            # S3 バックアップ
│   ├── webdav.rs               # WebDAV 同期
│   ├── git_tools.rs            # Git ツール
│   ├── sandbox_runner.rs       # サンドボックスランナー
│   ├── search.rs               # 検索抽象化
│   ├── reranker.rs             # リランキング
│   ├── model_knowledge.rs      # モデルナレッジ
│   ├── prompt_template.rs      # プロンプトテンプレート
│   ├── preset_templates.rs     # プリセットテンプレート
│   ├── workflow_types.rs       # ワークフロー型
│   ├── workflow_version.rs     # ワークフローバージョン
│   ├── path_vars.rs            # パス変数
│   ├── entity/                 # SeaORM エンティティ（40+ テーブル）
│   └── repo/                   # データリポジトリ（30+ リポジトリ）
│
├── gateway/       # API ゲートウェイ
│   ├── server.rs               # HTTP サーバー
│   ├── handlers.rs             # API ハンドラー
│   ├── routes.rs               # ルート定義
│   ├── auth.rs                 # 認証
│   ├── middleware.rs           # ミドルウェア
│   ├── metrics.rs              # メトリクス収集
│   ├── native.rs               # ネイティブ統合
│   ├── marketplace_handlers.rs # マーケットプレイスインターフェース
│   └── realtime.rs             # WebSocket サポート
│
├── plugins/       # プラグインシステム
│   ├── hooks.rs                # フックランナー
│   ├── agent_provider.rs       # エージェントプロバイダー
│   ├── test_isolation.rs       # テスト分離
│   └── lib.rs                  # プラグインレジストリとライフサイクル
│
├── providers/     # モデルアダプター
│   ├── adapter.rs              # アダプターインターフェース
│   ├── registry.rs             # プロバイダーレジストリ
│   ├── openai.rs               # OpenAI API
│   ├── openai_responses.rs     # OpenAI Responses API
│   ├── anthropic.rs            # Claude API
│   ├── gemini.rs               # Gemini API
│   ├── ollama.rs               # Ollama ローカル
│   ├── openclaw.rs             # OpenClaw
│   ├── hermes.rs               # Hermes
│   ├── image_gen.rs            # 画像生成
│   ├── realtime_client.rs      # リアルタイム API クライアント
│   └── transport/              # トランスポート層（Chat Completions / Responses / Anthropic）
│
├── runtime/       # ランタイムサービス
│   ├── session.rs              # セッション管理
│   ├── workflow_engine.rs      # DAG オーケストレーション
│   ├── work_engine/            # ワークエンジン（ノード実行器 + スケジューラー + キャッシュ層）
│   ├── mcp.rs                  # MCP サーバー
│   ├── mcp_client.rs           # MCP クライアント
│   ├── mcp_server.rs           # MCP サーバー実装
│   ├── mcp_stdio.rs            # MCP stdio トランスポート
│   ├── mcp_autostart.rs        # MCP 自動起動
│   ├── mcp_lifecycle_hardened.rs # MCP ライフサイクル管理
│   ├── mcp_tool_bridge.rs      # MCP ツールブリッジ
│   ├── cron/                   # タスクスケジューリング
│   ├── terminal/               # ターミナルバックエンド（ローカル/Docker/SSH）
│   ├── benchmarks/             # SWE-bench / Terminal-bench
│   ├── collaboration/          # CRDT コラボレーションとセッション共有
│   ├── tool_generator/         # AI ツール生成
│   ├── message_gateway/        # プラットフォーム統合（钉钉/飛書/QQ/Slack/WeChat/WhatsApp/Telegram/Discord）
│   ├── buddy/                  # Buddy パートナーシステム（種/属性/マネージャー）
│   ├── swarm/                  # Swarm クラスタ（プロセスバックエンド/権限同期/再接続）
│   ├── tasks/                  # バックグラウンドタスク（ドリーム/リモートエージェント/インプロセスチームメイト）
│   ├── adversarial_debate.rs   # 敵対的ディベート
│   ├── agent_orchestrator.rs   # マルチエージェントオーケストレーション
│   ├── agent_roles.rs          # エージェントロール
│   ├── webhook_dispatcher.rs   # Webhook ディスパッチ
│   ├── webhook_server.rs       # Webhook サーバー
│   ├── session_search.rs       # セッション検索
│   ├── dashboard_plugin.rs     # ダッシュボードプラグイン
│   ├── dashboard_registry.rs   # ダッシュボードレジストリ
│   ├── permissions.rs          # 権限管理
│   ├── permission_enforcer.rs  # 権限執行
│   ├── policy_engine.rs        # ポリシーエンジン
│   ├── trust_resolver.rs       # トラストリゾルバー
│   ├── resource_governor.rs    # リソースガバナー
│   ├── green_contract.rs       # グリーンコントラクト
│   ├── feature_flags.rs        # フィーチャーフラグ
│   ├── module_switch.rs        # モジュールスイッチ
│   ├── mode_selector.rs        # モードセレクター
│   ├── config.rs               # ランタイム設定
│   ├── config_validate.rs      # 設定検証
│   ├── prompt.rs               # プロンプト管理
│   ├── prompt_cache.rs         # プロンプトキャッシュ
│   ├── compact.rs              # コンテキスト圧縮
│   ├── summary_compression.rs  # 要約圧縮
│   ├── compact_thresholds.rs   # 圧縮閾値
│   ├── compact_warning.rs      # 圧縮警告
│   ├── reactive_compact.rs     # リアクティブ圧縮
│   ├── session_memory_compact.rs # セッションメモリ圧縮
│   ├── message_importance.rs   # メッセージ重要度評価
│   ├── message_batching.rs     # メッセージバッチ処理
│   ├── rate_limiter.rs         # レートリミッター
│   ├── connection_pool.rs      # 接続プール
│   ├── persistent_queue.rs     # 永続化キュー
│   ├── persistent_queue_manager.rs # キューマネージャー
│   ├── health_check.rs         # ヘルスチェック
│   ├── cache_guard.rs          # キャッシュガード
│   ├── checkpoint.rs           # チェックポイント
│   ├── branch_lock.rs          # ブランチロック
│   ├── stale_base.rs           # 古いベースライン検出
│   ├── watch_patterns.rs       # ウォッチパターン
│   ├── lan_transfer.rs         # LAN 転送
│   ├── tls_config.rs           # TLS 設定
│   ├── sse.rs                  # SSE イベントストリーム
│   ├── api_server.rs           # API サーバー
│   ├── gateway_auth.rs         # ゲートウェイ認証
│   ├── gateway_metrics.rs      # ゲートウェイメトリクス
│   ├── bash.rs                 # Bash 実行
│   ├── bash_validation.rs      # Bash 検証
│   ├── shell_hooks.rs          # Shell フック
│   ├── shell_completer.rs      # Shell 補完
│   ├── terminal_analyzer.rs    # ターミナル分析
│   ├── git_context.rs          # Git コンテキスト
│   ├── git_tools.rs            # Git ツール
│   ├── file_ops.rs             # ファイル操作
│   ├── hooks.rs                # フック管理
│   ├── hook_chain.rs           # フックチェーン
│   ├── hook_config.rs          # フック設定
│   ├── plugin_hooks.rs         # プラグインフック
│   ├── plugin_lifecycle.rs     # プラグインライフサイクル
│   ├── profile.rs              # プロファイル
│   ├── profile_manager.rs      # プロファイルマネージャー
│   ├── oauth.rs                # OAuth 認証
│   ├── usage.rs                # 使用量統計
│   ├── bootstrap.rs            # ブートストラップ
│   ├── worker_boot.rs          # Worker 起動
│   ├── fork_bridge.rs          # Fork ブリッジ
│   ├── task_packet.rs          # タスクパケット
│   ├── task_router.rs          # タスクルーター
│   ├── task_registry.rs        # タスクレジストリ
│   ├── transform_pipeline.rs   # 変換パイプライン
│   ├── transport_handlers.rs   # トランスポートハンドラー
│   ├── general_engine.rs       # 汎用エンジン
│   ├── engine_bridge.rs        # エンジンブリッジ
│   ├── conversation.rs         # 会話管理
│   ├── session_control.rs      # セッション制御
│   ├── shared_memory.rs        # 共有メモリ
│   ├── validation_executor.rs  # 検証実行器
│   ├── recovery_recipes.rs     # 回復レシピ
│   ├── error_recovery.rs       # エラー回復
│   ├── theme_engine.rs         # テーマエンジン
│   ├── token_budget_predictor.rs # トークンバジェット予測
│   ├── team_cron_registry.rs   # チーム Cron 登録
│   ├── module_dream.rs         # ドリームモジュール
│   ├── json.rs                 # JSON ユーティリティ
│   └── lane_events.rs          # Lane イベント
│
├── telemetry/     # テレメトリとトレーシング
│   ├── tracer.rs              # 分散トレーシング
│   ├── metrics.rs             # メトリクス収集
│   ├── span.rs                # Span 管理
│   ├── event.rs               # イベント定義
│   ├── collector.rs           # データ収集
│   ├── exporter.rs            # データエクスポート
│   └── storage.rs             # ストレージバックエンド
│
├── tools/         # ツールシステム
│   ├── registry.rs             # ツールレジストリ
│   ├── builtin_tools.rs        # 組み込みツール定義
│   ├── builtin_handlers.rs     # 組み込みツールハンドラー
│   ├── orchestration.rs        # ツールオーケストレーション
│   ├── streaming.rs            # ストリーミング出力
│   ├── stats.rs                # 使用統計
│   ├── recorder.rs             # 実行記録
│   ├── agent_def_loader.rs     # エージェント定義ローダー
│   ├── agent_def_types.rs      # エージェント定義型
│   ├── bash/                   # Bash ツール（パーサー/サンドボックス/セキュリティ/パス検証）
│   ├── hooks/                  # フック（レジストリ/実行器）
│   ├── mcp/                    # MCP ツール（レジストリ/OAuth/ラッパー）
│   ├── permissions/            # 権限（分類器/ルール/トラッカー）
│   └── tools/                  # 具体的なツール実装
│       ├── agent.rs            # エージェントツール
│       ├── bash.rs             # Bash 実行
│       ├── context.rs          # コンテキスト管理
│       ├── cron.rs             # Cron スケジューリング
│       ├── glob.rs             # ファイルグロブ
│       ├── grep.rs             # コンテンツ検索
│       ├── lsp.rs              # LSP ツール
│       ├── monitor.rs          # モニターツール
│       ├── plan.rs             # プランツール
│       ├── repl.rs             # REPL ツール
│       ├── skill.rs            # スキルツール
│       ├── web_fetch.rs        # Web スクレイピング
│       ├── web_search.rs       # Web 検索
│       ├── file_read.rs        # ファイル読み取り
│       ├── file_write.rs       # ファイル書き込み
│       ├── file_edit.rs        # ファイル編集
│       ├── computer_use.rs     # コンピュータ制御
│       ├── messaging.rs        # メッセージ送信
│       ├── push_notification.rs # プッシュ通知
│       ├── task_system.rs      # タスクシステム
│       ├── todo_write.rs       # ToDo 書き込み
│       └── batch_missing.rs    # バッチ欠落検出
│
├── trajectory/    # 学習システム
│   ├── memory.rs              # メモリ管理
│   ├── memory_provider.rs     # メモリプロバイダーインターフェース
│   ├── auto_memory.rs         # 自動メモリ抽出
│   ├── skill.rs               # スキルシステム
│   ├── skill_manager.rs       # スキルマネージャー
│   ├── skill_evolution.rs     # スキル進化
│   ├── skill_matcher.rs       # スキルマッチング
│   ├── skill_proposal.rs      # スキルプロポーザル
│   ├── skills_hub_adapter.rs  # スキルハブアダプター
│   ├── skills_hub_client.rs   # スキルハブクライアント
│   ├── skill_decomposition/   # スキル分解（LLM支援/マルチターン/ワークフロー検証/ツール解析）
│   ├── rl.rs                  # RL 報酬信号
│   ├── rl_trainer.rs          # RL トレーナー
│   ├── training_env.rs        # トレーニング環境
│   ├── behavior_learner.rs    # 振る舞い学習
│   ├── behavior_tracker.rs    # 振る舞い追跡
│   ├── pattern.rs             # パターン認識
│   ├── pattern_analyzer.rs    # パターン分析
│   ├── user_profile.rs        # ユーザープロファイリング
│   ├── preference_learner.rs  # 嗜好学習
│   ├── adaptation.rs          # 適応調整
│   ├── dream_consolidation.rs # ドリーム統合
│   ├── parallel_execution.rs  # 並列実行サービス
│   ├── style_extractor.rs     # スタイル抽出
│   ├── style_applier.rs       # スタイル適用
│   ├── style_vectorizer.rs    # スタイルベクトル化
│   ├── style_migrator.rs      # スタイル転送
│   ├── suggestion_engine.rs   # 提案エンジン
│   ├── proactive_assistant.rs # プロアクティブアシスタント
│   ├── context_predictor.rs   # コンテキスト予測
│   ├── task_prefetcher.rs     # タスク事前取得
│   ├── reminder_manager.rs    # リマインダー管理
│   ├── nudge.rs               # ナッジシステム
│   ├── insight.rs             # インサイト生成
│   ├── compactor.rs           # データ圧縮
│   ├── trajectory.rs          # トラジェクトリ管理
│   ├── trajectory_compressor.rs # トラジェクトリ圧縮
│   ├── sub_agent.rs           # サブエージェント
│   ├── batch.rs               # バッチ処理
│   ├── context.rs             # コンテキスト管理
│   ├── fts5.rs                # FTS5 検索
│   ├── hooks.rs               # フック
│   ├── storage.rs             # ストレージ
│   ├── scheduled_task.rs      # 定期タスク
│   └── memory_providers/      # メモリプロバイダー（Honcho/Mem0/クローズドループ/サービス）
│
└── migration/     # データベースマイグレーション
    └── m20240101_000001~000010  # 10 のマイグレーションファイル
```

### フロントエンドアーキテクチャ

```
src/
├── stores/                    # Zustand 状態管理
│   ├── domain/               # コアビジネス状態
│   │   ├── conversationStore.ts
│   │   ├── messageStore.ts
│   │   ├── streamStore.ts
│   │   ├── multiModelStore.ts
│   │   ├── preferenceStore.ts
│   │   └── compressStore.ts
│   ├── feature/               # 機能モジュール状態（30+ store）
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
│   ├── devtools/              # 開発者ツール状態
│   │   ├── tracerStore.ts
│   │   ├── evaluatorStore.ts
│   │   ├── rlStore.ts
│   │   ├── fineTuneStore.ts
│   │   └── recommendationStore.ts
│   └── shared/                # 共有状態
│       ├── artifactStore.ts
│       ├── chatWorkspaceStore.ts
│       ├── tabStore.ts
│       └── uiStore.ts
│
├── components/                # React コンポーネント（24 モジュール）
│   ├── chat/                # チャットインターフェース（90+ コンポーネント）
│   ├── workflow/            # ワークフローエディタ（ノード/パネル/テンプレート/AI 支援）
│   ├── gateway/             # API ゲートウェイ UI
│   ├── settings/            # 設定パネル（40+ コンポーネント）
│   ├── terminal/            # ターミナル UI
│   ├── skill/               # スキルエディタとレンダラー
│   ├── benchmark/           # ベンチマークパネル
│   ├── decomposition/       # スキル分解とツール生成
│   ├── files/               # ファイル管理ページ
│   ├── fine-tune/           # LoRA ファインチューニング設定
│   ├── link/                # 外部リンク管理
│   ├── llm-wiki/            # LLM Wiki エディタ
│   ├── proactive/           # プロアクティブ提案システム
│   ├── recommendation/      # ツール推奨パネル
│   ├── wiki/                # Wiki 管理
│   ├── devtools/            # Trace/Span タイムライン
│   ├── style/               # コードスタイル転送
│   ├── layout/              # レイアウトコンポーネント（タイトルバー/サイドバー/コマンドパレット）
│   ├── help/                # ヘルプパネル
│   ├── onboarding/          # オンボーディングウィザード
│   ├── notification/        # 通知センター
│   ├── search/              # セッション検索
│   ├── common/              # 共通コンポーネント
│   └── shared/              # 共有コンポーネント
│
├── pages/                    # ページコンポーネント（22 ページ）
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
├── lib/                      # ユーティリティ関数（Web Worker 含む）
├── types/                    # TypeScript 型定義（22 個）
├── sdk/                      # SDK（Python SDK 含む）
└── i18n/                     # 11 言語翻訳
```

### プラットフォームサポート

| プラットフォーム | アーキテクチャ |
|----------------|---------------|
| macOS | Apple Silicon (arm64), Intel (x86_64) |
| Windows | x86_64, ARM64 |
| Linux | x86_64, ARM64 (AppImage/deb/rpm) |

## クイックスタート

### ビルド済みダウンロード

[Releases](https://github.com/polite0803/AxAgent/releases) ページにアクセスし、お使いのプラットフォーム用のインストーラをダウンロードしてください。

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

# コードフォーマット
npm run format

# CI チェック
npm run ci:check
```

---

## プロジェクト構造

```
AxAgent/
├── src/                         # フロントエンドソース (React + TypeScript)
│   ├── components/              # React コンポーネント（24 モジュール）
│   │   ├── chat/               # チャットインターフェース（90+ コンポーネント）
│   │   ├── workflow/           # ワークフローエディタコンポーネント
│   │   ├── gateway/            # API ゲートウェイコンポーネント
│   │   ├── settings/           # 設定パネル（40+ コンポーネント）
│   │   ├── terminal/           # ターミナルコンポーネント
│   │   ├── skill/              # スキルエディタとレンダラー
│   │   ├── benchmark/          # ベンチマーク
│   │   ├── decomposition/      # スキル分解
│   │   ├── files/              # ファイル管理
│   │   ├── fine-tune/          # LoRA ファインチューニング
│   │   ├── link/               # 外部リンク
│   │   ├── llm-wiki/           # LLM Wiki
│   │   ├── proactive/          # プロアクティブ提案
│   │   ├── recommendation/     # ツール推奨
│   │   ├── wiki/               # Wiki 管理
│   │   ├── devtools/           # 開発者ツール
│   │   ├── style/              # コードスタイル
│   │   ├── layout/             # レイアウトコンポーネント
│   │   ├── help/               # ヘルプパネル
│   │   ├── onboarding/         # オンボーディングウィザード
│   │   ├── notification/       # 通知センター
│   │   ├── search/             # セッション検索
│   │   ├── common/             # 共通コンポーネント
│   │   └── shared/             # 共有コンポーネント
│   ├── pages/                   # ページコンポーネント（22 ページ）
│   ├── stores/                  # Zustand 状態管理
│   │   ├── domain/            # コアビジネス状態（6 store）
│   │   ├── feature/           # 機能モジュール状態（30+ store）
│   │   ├── devtools/          # 開発者ツール状態（5 store）
│   │   └── shared/            # 共有状態（4 store）
│   ├── hooks/                   # React hooks（10 個）
│   ├── lib/                     # ユーティリティ関数（Web Worker 含む）
│   ├── types/                   # TypeScript 型定義（22 個）
│   ├── sdk/                     # SDK（Python SDK 含む）
│   └── i18n/                    # 11 言語翻訳
│
├── src-tauri/                    # バックエンドソース (Rust)
│   ├── crates/                  # Rust workspace（10 crates）
│   │   ├── agent/             # AI エージェントコア
│   │   ├── core/              # データベース、暗号化、RAG
│   │   ├── gateway/           # API ゲートウェイサーバー
│   │   ├── plugins/           # プラグインシステム
│   │   ├── providers/         # モデルプロバイダーアダプター
│   │   ├── runtime/           # ランタイムサービス
│   │   ├── tools/             # ツールシステム
│   │   ├── trajectory/        # メモリと学習
│   │   ├── telemetry/         # トレーシングとメトリクス
│   │   └── migration/         # データベースマイグレーション
│   └── src/                    # Tauri エントリーポイント（70+ コマンドモジュール）
│
├── extension/                  # ブラウザ拡張（Wiki Clipper）
├── e2e/                        # Playwright E2E テスト
├── scripts/                    # ビルドとツールスクリプト
└── website/                    # プロジェクトウェブサイト（VitePress）
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

次に **システム設定 → プライバシーとセキュリティ → セキュリティ** に移動し、**すべてのソース** を選択します。

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
