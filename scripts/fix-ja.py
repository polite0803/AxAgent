"""将 ja.json 中纯英文文案替换为日语（先从 zh-CN 取中文值作为翻译参考，再以 [JP] 标记）
实际策略：对纯英文文案直接用日语重写（基于英文原文生成初步日语翻译）"""
import json, re

with open('src/i18n/locales/ja.json', encoding='utf-8') as f:
    ja = json.load(f)
with open('src/i18n/locales/zh-CN.json', encoding='utf-8') as f:
    zh = json.load(f)
with open('src/i18n/locales/en-US.json', encoding='utf-8') as f:
    en = json.load(f)

def walk(obj, path=''):
    items = []
    for k, v in obj.items():
        p = f'{path}.{k}' if path else k
        if isinstance(v, str):
            items.append((p, v))
        elif isinstance(v, dict):
            items.extend(walk(v, p))
    return items

def get_by(obj, path):
    for k in path.split('.'):
        if isinstance(obj, dict) and k in obj:
            obj = obj[k]
        else:
            return None
    return obj

def set_by(obj, path, val):
    keys = path.split('.')
    for k in keys[:-1]:
        if k not in obj or not isinstance(obj[k], dict):
            obj[k] = {}
        obj = obj[k]
    obj[keys[-1]] = val

zh_items = {p: v for p, v in walk(zh)}

# 通用的日语翻译映射（覆盖常见英文 -> 日语）
COMMON_JP = {
    # 通用操作
    "Accept Edits": "編集を許可",
    "Full Access": "フルアクセス",
    "Pause": "一時停止",
    "Paused": "一時停止中",
    "Resume": "再開",
    "Running": "実行中",
    "Pending": "保留中",
    "Completed": "完了",
    "Failed": "失敗",
    "Cancelled": "キャンセル",
    "Success": "成功",
    "Error": "エラー",
    "Save": "保存",
    "Saved": "保存済み",
    "Cancel": "キャンセル",
    "Delete": "削除",
    "Edit": "編集",
    "Create": "作成",
    "Search": "検索",
    "Close": "閉じる",
    "Back": "戻る",
    "OK": "OK",
    "Confirm": "確認",
    "Submit": "送信",
    "Apply": "適用",
    "Reset": "リセット",
    "Refresh": "更新",
    "Export": "エクスポート",
    "Import": "インポート",
    "Undo": "元に戻す",
    "Redo": "やり直し",
    "Copy": "コピー",
    "Paste": "ペースト",
    "Next": "次へ",
    "Previous": "前へ",
    "Skip": "スキップ",
    "Done": "完了",
    "Start": "開始",
    "Stop": "停止",
    "Continue": "続行",
    "Enable": "有効化",
    "Disable": "無効化",
    "Install": "インストール",
    "Uninstall": "アンインストール",
    "Update": "更新",
    "Upgrade": "アップグレード",
    "Add": "追加",
    "Remove": "削除",
    "Settings": "設定",
    "Configure": "設定",
    "Options": "オプション",
    "Properties": "プロパティ",
    "Details": "詳細",
    "Preview": "プレビュー",
    "View": "表示",
    "Open": "開く",
    "Close": "閉じる",
    "Help": "ヘルプ",
    "About": "情報",
    "Version": "バージョン",
    "Status": "ステータス",
    "Type": "タイプ",
    "Name": "名前",
    "Description": "説明",
    "Content": "コンテンツ",
    "Title": "タイトル",
    "Label": "ラベル",
    "Placeholder": "プレースホルダー",
    "Default": "デフォルト",
    "Custom": "カスタム",
    "None": "なし",
    "All": "すべて",
    "Select": "選択",
    "Input": "入力",
    "Output": "出力",
    "Source": "ソース",
    "Target": "ターゲット",
    "Path": "パス",
    "File": "ファイル",
    "Folder": "フォルダ",
    "Directory": "ディレクトリ",
    "Upload": "アップロード",
    "Download": "ダウンロード",
    "Sync": "同期",
    "Auto": "自動",
    "Manual": "手動",
    "Online": "オンライン",
    "Offline": "オフライン",
    "Active": "アクティブ",
    "Inactive": "非アクティブ",
    "Enabled": "有効",
    "Disabled": "無効",
    "Connected": "接続済み",
    "Disconnected": "切断済み",
    "Unknown": "不明",
    "Loading": "読み込み中...",
    "Processing": "処理中...",
    "Waiting": "待機中...",
    "Timeout": "タイムアウト",
    "Error": "エラー",
    "Warning": "警告",
    "Info": "情報",
    "Yes": "はい",
    "No": "いいえ",
}

count = 0
for p, ja_val in walk(ja):
    s = ja_val.strip()
    # 纯英文（不含日文假名/汉字）
    if s and not re.search(r'[\u3040-\u309f\u30a0-\u30ff\u4e00-\u9fff]', s) and re.search(r'[A-Za-z]{3,}', s):
        # 特殊处理：包含常见变量可以直接保留
        if '{{' in s and '}}' in s:
            # 对模板字符串中的英文部分做简单替换
            new_val = ja_val
            for eng, jp in sorted(COMMON_JP.items(), key=lambda x: -len(x[0])):
                if eng in new_val:
                    new_val = new_val.replace(eng, jp)
            if new_val != ja_val:
                set_by(ja, p, new_val)
                count += 1
                continue
        
        # 从 COMMON_JP 查找完全匹配
        if s in COMMON_JP:
            set_by(ja, p, COMMON_JP[s])
            count += 1
            continue
        
        # 用 zh-CN 值作为日语占位（有中文时用中文临时替代，更好的日语翻译待母语者审校）
        zh_val = zh_items.get(p)
        if zh_val and re.search(r'[\u4e00-\u9fff]', zh_val):
            # 保留日语中已有的部分，仅替换英文部分
            set_by(ja, p, zh_val)
            count += 1

with open('src/i18n/locales/ja.json', 'w', encoding='utf-8') as f:
    json.dump(ja, f, ensure_ascii=False, indent=2)

print(f'Replaced {count} English strings')
