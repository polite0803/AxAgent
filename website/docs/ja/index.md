---
layout: home
title: AxAgent — オープンソース AI デスクトップクライアント & ゲートウェイ
titleTemplate: false

head:
  - - meta
    - name: description
      content: AxAgent は無料のオープンソース AI デスクトップクライアントで、AI ゲートウェイを内蔵しています。OpenAI、Claude、Gemini、DeepSeek などのマルチモデル対話、MCP サーバー、ナレッジベース、プライバシー重視。

hero:
  name: AxAgent
  text: あなたの AI デスクトップアシスタント
  tagline: マルチモデルチャット、MCP ツール、API ゲートウェイ、ナレッジベース — すべて一つのオープンソースクライアントで
  image:
    src: /logo.png
    alt: AxAgent
  actions:
    - theme: brand
      text: はじめる
      link: /ja/guide/getting-started
    - theme: alt
      text: ダウンロード
      link: /ja/download
    - theme: alt
      text: GitHub
      link: https://github.com/polite0803/AxAgent

features:
  - icon: robot
    title: マルチモデルチャット
    details: OpenAI、Claude、Gemini、DeepSeek、Qwen などすべての互換 API に接続。マルチキーローテーション、ストリーミング出力、思考ブロック対応。
  - icon: thunderbolt
    title: AI Agent (Beta)
    details: Agent モードで自律的なタスク実行。ファイルの読み書き、コマンド実行、コード分析——3段階の権限制御と作業ディレクトリサンドボックスで安全に制御。
  - icon: api
    title: MCP ツール呼び出し
    details: Model Context Protocol を完全実装。stdio、SSE、StreamableHTTP をサポート。ワンクリックで外部ツールに接続、@axagent/fetch などのビルトイン MCP ツール付き。
  - icon: cloud-server
    title: ビルトイン API ゲートウェイ
    details: ローカル OpenAI 互換 API サーバー。Claude Code、Codex、Gemini CLI などのバックエンドとして使用可能。キー管理、レート制限、SSL/TLS 対応。
  - icon: book
    title: ナレッジベース & RAG
    details: sqlite-vec によるローカルベクトル埋め込み。プライベートドキュメントに基づく AI 回答、データはローカルから外に出ません。
  - icon: search
    title: ウェブ検索
    details: Tavily、Zhipu WebSearch、Bocha と統合。引用ソース付きの検索結果が会話コンテキストに自動注入されます。
  - icon: edit
    title: リッチコンテンツレンダリング
    details: Markdown、LaTeX、Mermaid 図、D2 アーキテクチャ図、Monaco コードエディター（diff プレビュー付き）、Artifact パネル。
  - icon: desktop
    title: デスクトップ体験
    details: グローバルショートカット、システムトレイ、自動起動、最前面表示、ダーク/ライトテーマ、プロキシサポート。
  - icon: lock
    title: プライバシー & セキュリティ
    details: すべてのデータをローカルに保存。AES-256 で API キーを暗号化。ローカルまたは WebDAV への自動バックアップ。PNG/Markdown/JSON 形式でエクスポート。
---
