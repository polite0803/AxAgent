#!/usr/bin/env python
"""Phase 1: 添加 249 个 truly missing key 到 11 种语言文件。"""

import json, os, re

BASE = os.path.dirname(os.path.abspath(__file__))
LOCALES_DIR = os.path.join(BASE, "..", "src", "i18n", "locales")
LIST_FILE = os.path.join(BASE, "..", "output", "truly-missing-249.txt")


def generate_en(key):
    last = key.split('.')[-1]
    s = re.sub(r'([a-z])([A-Z])', r'\1 \2', last)
    s = re.sub(r'([A-Z]+)([A-Z][a-z])', r'\1 \2', s)
    return s.capitalize()


def generate_zh(key):
    KNOWN = {
        'chat.attachFile': '附加文件',
        'chat.clearContext': '清除上下文',
        'chat.clearConversation': '清除对话',
        'chat.clearConversationConfirmContent': '确定要清除此对话？此操作不可撤销。',
        'chat.clearConversationConfirmTitle': '清除对话',
        'chat.clearHistoryDone': '历史已清除',
        'chat.compressFailed': '压缩失败',
        'chat.compressSuccess': '压缩成功',
        'chat.agentInitFailed': 'Agent 初始化失败',
        'chat.agentInitTransient': 'Agent 暂态',
        'chat.contextCompression': '上下文压缩',
        'chat.contextMessages': '上下文消息',
        'chat.conversationNotFound': '对话未找到',
        'chat.conversationSettings': '对话设置',
        'chat.disableAutoCompression': '禁用自动压缩',
        'chat.dropToAttach': '拖拽文件附加',
        'chat.enableAutoCompression': '启用自动压缩',
        'chat.connector.add': '添加',
        'chat.connector.args': '参数',
        'chat.connector.command': '命令',
        'chat.connector.custom': '自定义',
        'chat.connector.endpoint': '端点',
        'chat.connector.goConfig': '前往配置',
        'chat.connector.noServers': '暂无服务器',
        'chat.connector.placeholderArgs': '输入参数...',
        'chat.connector.placeholderCommand': '输入命令...',
        'chat.connector.placeholderEndpoint': '输入端点...',
        'chat.connector.placeholderName': '输入名称...',
        'chat.connector.title': '连接器',
        'chat.knowledge.title': '知识库',
        'chat.knowledge.empty': '暂无知识库',
        'chat.knowledge.search': '搜索知识库',
        'chat.modelSelector.title': '模型选择',
        'chat.modelSelector.empty': '暂无可用模型',
        'inputArea.resizeHandle': '调整大小',
        'voice.startCall': '开始通话',
        'voice.wakeup': '唤醒',
        'voice.wakeupActive': '唤醒中',
        'common.openDirectory': '打开目录',
        'common.permissionAcceptEdits': '接受编辑',
        'common.permissionDefault': '默认',
        'common.permissionFullAccess': '完全访问',
        'common.workingDirectory': '工作目录',
        'settings.collapseAll': '全部折叠',
        'settings.mcp.builtin': '内置',
        'settings.mcp.custom': '自定义',
        'settings.platform.markerPrefixDesc': '设置消息前缀标记',
        'settings.platform.markerPrefixPlaceholder': '例如 /',
        'settings.selectModels': '选择模型',
        'settings.testSuccess': '测试成功',
        'agent.permissionAcceptEditsWarning': '允许 Agent 直接编辑文件',
        'agent.permissionAcceptEditsWarningTitle': '编辑权限',
        'agent.permissionFullAccessWarning': '允许 Agent 完全访问系统',
        'agent.permissionFullAccessWarningTitle': '完全访问',
        'permission.execute': '执行',
        'permission.readonly': '只读',
        'permission.write': '写入',
    }
    if key in KNOWN:
        return KNOWN[key]
    # Fallback: generate from last word
    last = key.split('.')[-1]
    s = re.sub(r'([a-z])([A-Z])', r'\1 \2', last)
    return s


def generate_tw(key):
    KNOWN = {
        'chat.attachFile': '附加檔案',
        'chat.clearConversation': '清除對話',
        'chat.connector.title': '連接器',
        'inputArea.resizeHandle': '調整大小',
        'voice.startCall': '開始通話',
        'voice.wakeup': '喚醒',
        'voice.wakeupActive': '喚醒中',
    }
    if key in KNOWN:
        return KNOWN[key]
    last = key.split('.')[-1]
    s = re.sub(r'([a-z])([A-Z])', r'\1 \2', last)
    return s


def add_key(data, dot_key, value):
    parts = dot_key.split(".")
    d = data
    for i, p in enumerate(parts):
        if i == len(parts) - 1:
            d[p] = value
        else:
            d = d.setdefault(p, {})


def read_keys():
    keys = []
    with open(LIST_FILE, encoding='utf-8') as f:
        for line in f:
            line = line.strip()
            if line and not line.startswith('#') and not line.startswith('[') and not line.startswith('Truly'):
                key = line.strip()
                if key and '.' in key:
                    keys.append(key)
    return keys


def main():
    keys = read_keys()
    print(f"读取到 {len(keys)} 个 key")

    for fname in sorted(os.listdir(LOCALES_DIR)):
        if not fname.endswith('.json'):
            continue
        lang = fname.replace('.json', '')
        path = os.path.join(LOCALES_DIR, fname)

        with open(path, encoding='utf-8') as f:
            data = json.load(f)

        count = 0
        for key in keys:
            # 检查是否已存在（可能是之前添加过的）
            parts = key.split('.')
            d = data
            exists = True
            for p in parts:
                if isinstance(d, dict) and p in d:
                    d = d[p]
                else:
                    exists = False
                    break
            if exists:
                # 已经存在
                continue

            # 生成翻译
            if lang == 'zh-CN':
                val = generate_zh(key)
            elif lang == 'zh-TW':
                val = generate_tw(key)
            else:
                val = generate_en(key)

            add_key(data, key, val)
            count += 1

        with open(path, 'w', encoding='utf-8') as f:
            json.dump(data, f, ensure_ascii=False, indent=2)

        print(f"  {lang}: +{count} keys")

    print("\nPhase 1 完成")


if __name__ == '__main__':
    main()
