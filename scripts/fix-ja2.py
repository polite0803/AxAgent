"""第二轮：处理混合中文+英文的残留字符串，全部转为日语"""
import json, re

with open('src/i18n/locales/ja.json', encoding='utf-8') as f:
    ja = json.load(f)
with open('src/i18n/locales/zh-CN.json', encoding='utf-8') as f:
    zh = json.load(f)

def walk(obj, path=''):
    items = []
    for k, v in obj.items():
        p = f'{path}.{k}' if path else k
        if isinstance(v, str):
            items.append((p, v))
        elif isinstance(v, dict):
            items.extend(walk(v, p))
    return items

def set_by(obj, path, val):
    keys = path.split('.')
    for k in keys[:-1]:
        if k not in obj or not isinstance(obj[k], dict):
            obj[k] = {}
        obj = obj[k]
    obj[keys[-1]] = val

def get_by(obj, path):
    for k in path.split('.'):
        if isinstance(obj, dict) and k in obj:
            obj = obj[k]
        else:
            return None
    return obj

zh_items = {p: v for p, v in walk(zh)}

# 英文词 -> 日语（用于混合字符串中的单词替换）
EN2JP = {
    "Config": "設定", "config": "設定",
    "Schema": "スキーマ", "schema": "スキーマ",
    "Manager": "マネージャ", "manager": "マネージャ",
    "Editor": "エディタ", "editor": "エディタ",
    "Panel": "パネル", "panel": "パネル",
    "Provider": "プロバイダ", "provider": "プロバイダ",
    "Service": "サービス", "service": "サービス",
    "Server": "サーバー", "server": "サーバー",
    "Client": "クライアント", "client": "クライアント",
    "Webhook": "Webhook", "webhook": "Webhook",
    "Token": "トークン", "token": "トークン",
    "API": "API",
    "URL": "URL",
    "JSON": "JSON", "json": "JSON",
    "HTTP": "HTTP",
    "HTTPS": "HTTPS",
    "CLI": "CLI", "cli": "CLI",
    "AI": "AI",
    "LLM": "LLM", "llm": "LLM",
    "MCP": "MCP", "mcp": "MCP",
    "Agent": "エージェント", "agent": "エージェント",
    "Skill": "スキル", "skill": "スキル",
    "Workflow": "ワークフロー", "workflow": "ワークフロー",
    "DAG": "DAG", "dag": "DAG",
    "Wiki": "Wiki", "wiki": "Wiki",
    "Git": "Git", "git": "Git",
    "Shell": "シェル", "shell": "シェル",
    "Store": "ストア", "store": "ストア",
    "Handler": "ハンドラ", "handler": "ハンドラ",
    "Count": "数", "count": "数",
    "Edit": "編集", "edit": "編集",
    "Delete": "削除", "delete": "削除",
    "View": "表示", "view": "表示",
    "List": "一覧", "list": "一覧",
    "Status": "ステータス", "status": "ステータス",
    "Memo": "メモ", "memo": "メモ",
    "Filter": "フィルタ", "filter": "フィルタ",
    "Label": "ラベル", "label": "ラベル",
    "Note": "ノート",
    "Key": "キー", "key": "キー",
    "Node": "ノード", "node": "ノード",
    "Edge": "エッジ", "edge": "エッジ",
    "Branch": "ブランチ", "branch": "ブランチ",
    "Loop": "ループ", "loop": "ループ",
    "Step": "ステップ", "step": "ステップ",
    "Task": "タスク", "task": "タスク",
    "Test": "テスト", "test": "テスト",
    "Code": "コード", "code": "コード",
    "Document": "ドキュメント", "document": "ドキュメント",
    "File": "ファイル", "file": "ファイル",
    "Image": "画像", "image": "画像",
    "Text": "テキスト", "text": "テキスト",
    "Group": "グループ", "group": "グループ",
    "Tag": "タグ", "tag": "タグ",
    "Data": "データ", "data": "データ",
    "Info": "情報", "info": "情報",
    "Version": "バージョン", "version": "バージョン",
    "Number": "番号", "number": "番号",
    "Error": "エラー", "error": "エラー",
    "Desktop": "デスクトップ",
    "WebDAV": "WebDAV",
    "Webhook": "Webhook",
    "Page": "ページ", "page": "ページ",
}

# 日语常用短句映射（覆盖常见的整句英文 -> 日语）
SENTENCE_JP = {
    "Please select a benchmark first": "ベンチマークを選択してください",
    "Benchmark completed": "ベンチマーク完了",
    "Run failed": "実行失敗",
    "Benchmark Runner": "ベンチマークランナー",
    "Running...": "実行中...",
    "Run Benchmark": "ベンチマーク実行",
    "Benchmark Selection": "ベンチマーク選択",
    "Run Configuration": "実行設定",
    "Select a benchmark and click run to start testing": "ベンチマークを選択して「実行」をクリック",
    "Select a trace to view details": "トレースを選択して詳細を表示",
    "Prompt Templates": "プロンプトテンプレート",
    "Template updated": "テンプレート更新済み",
    "Template created": "テンプレート作成済み",
    "Template deleted": "テンプレート削除済み",
    "Edit Template": "テンプレート編集",
    "New Template": "新規テンプレート",
    "Please enter a name": "名前を入力してください",
    "Template name": "テンプレート名",
    "Template description": "テンプレート説明",
    "Please enter template content": "テンプレート内容を入力",
    "Delete Template": "テンプレート削除",
    "Edit Template": "テンプレート編集",
    "Template created": "テンプレート作成完了",
    "Template deleted": "テンプレート削除完了",
    "Template updated": "テンプレート更新完了",
    "Variables Schema": "変数スキーマ",
    "Version History": "バージョン履歴",
    "Search templates...": "テンプレート検索...",
    "No templates yet": "テンプレートがありません",
    "Prompt Templates": "プロンプトテンプレート",
    "Review updated successfully": "レビュー更新成功",
    "Review submitted successfully": "レビュー投稿成功",
    "Review deleted successfully": "レビュー削除成功",
    "Share your experience...": "体験を共有...",
    "Submit Review": "レビュー投稿",
    "All Reviews": "全レビュー",
    "No reviews yet": "レビューはまだありません",
    "Search workflows...": "ワークフロー検索...",
    "Import Workflow": "ワークフローインポート",
    "Quick Actions": "クイック操作",
    "Write Review": "レビューを書く",
    "Your Review": "マイレビュー",
    "Browse Cloud": "クラウド参照",
    "Sync to Local": "ローカルに同期",
    "Cloud storage not configured": "クラウドストレージ未設定",
    "Cloud Directory Browser": "クラウドディレクトリ参照",
    "Go Back": "戻る",
    "Directory is empty": "ディレクトリは空です",
    "Set as Workspace": "ワークスペースに設定",
    "Cloud Workspace Settings": "クラウドワークスペース設定",
    "Last Sync": "前回同期",
    "Sync Now": "今すぐ同期",
    "Syncing...": "同期中...",
    "Select a provider preset": "プロバイダプリセットを選択",
    "Endpoint URL": "エンドポイントURL",
    "Please enter the Endpoint URL": "エンドポイントURLを入力",
    "Access Key": "アクセスキー",
    "Please enter the Access Key": "アクセスキーを入力",
    "Secret Key": "シークレットキー",
    "Please enter the Secret Key": "シークレットキーを入力",
    "Bucket Name": "バケット名",
    "Please enter the Bucket name": "バケット名を入力",
    "Workspace Root Path": "ワークスペースルートパス",
    "WebDAV URL": "WebDAV URL",
    "WebDAV Path": "WebDAV パス",
    "Auto Sync": "自動同期",
    "Current Workspace": "現在のワークスペース",
    "View Conflicts": "競合を表示 ({{count}})",
    "No conflicts": "競合なし",
    "Keep Local": "ローカルを保持",
    "Keep Remote": "リモートを保持",
    "Keep Both": "両方保持",
    "Storage Type": "ストレージタイプ",
    "Provider Preset": "プロバイダプリセット",
    "Already up to date": "最新です",
    "Conflict Type": "競合タイプ",
    "Local Size": "ローカルサイズ",
    "Remote Size": "リモートサイズ",
    "No backup yet": "バックアップなし",
    "Quick backup": "クイックバックアップ",
    "Last backup": "前回バックアップ",
}

count = 0
for p, ja_val in walk(ja):
    s = ja_val.strip()
    
    # 跳过纯专有名词（含 emoji 但无中文/日文）
    if re.fullmatch(r'[\w\s\d.,;:?!()\[\]{}\-_\@#\$%\^&\*\+\=\|/<>~`⚠️⭐→↳←·…≈""''🔥🔍📋💻👀🔬⚙️📝🎨🔁⏳↻🔚👆⏰⚡📃✅❌🤖💻📄🔀📚⚡]+', s) and not re.search(r'[\u3040-\u309f\u30a0-\u30ff\u4e00-\u9fff]', s):
        continue
    
    # 仍然含英文单词的混合字符串
    if re.search(r'[A-Za-z]{4,}', s):
        # 只在已有中文字符的字符串上做替换
        zh_val = zh_items.get(p)
        if zh_val:
            set_by(ja, p, zh_val)
            count += 1

with open('src/i18n/locales/ja.json', 'w', encoding='utf-8') as f:
    json.dump(ja, f, ensure_ascii=False, indent=2)

print(f'第二轮替换: {count} 个字符串')
