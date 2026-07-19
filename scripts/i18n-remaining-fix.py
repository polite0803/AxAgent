#!/usr/bin/env python
"""修复剩余未定义 key（排除段级 false positives）。

47 → 剩余的真正缺失 key：
1. actionRouter.* (5)     — P1 错误消息
2. settingsSearch.* (9)  — P1 搜索索引
3. wiki.lint / lintReport  — P1
4. debugPanel.* (11)     — 代码用 root-level，JSON 中不存在
5. workflow.debugPanel   — 从 object 覆盖为 string（子 key 无人引用）
6. workflow.aiPanel.* (2) — P1
7. 单点缺失 (8)          — P1：agentProfile.saveSuccess, skill.error, nudge.snooze, searchUtils.*, shortcuts.*
"""

import json, os

LOCALES_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "src", "i18n", "locales")

# key → {zh, en, tw}
T = {
    # === actionRouter.* ===
    "actionRouter.emitMissingNamespace": {"zh": 'Emit 操作缺少 namespace 参数', "en": 'Emit action missing namespace', "tw": 'Emit 操作缺少 namespace 參數'},
    "actionRouter.emitReservedEvent": {"zh": 'Emit 使用了保留事件名', "en": 'Emit uses reserved event name', "tw": 'Emit 使用了保留事件名稱'},
    "actionRouter.navigatePathTraversal": {"zh": '导航路径包含路径遍历', "en": 'Navigation path contains path traversal', "tw": '導航路徑包含路徑遍歷'},
    "actionRouter.storeGetSelectorInvalid": {"zh": 'Store get selector 无效', "en": 'Store get selector invalid', "tw": 'Store get selector 無效'},
    "actionRouter.storeSetPayloadInvalid": {"zh": 'Store set payload 无效', "en": 'Store set payload invalid', "tw": 'Store set payload 無效'},

    # === settingsSearch.* ===
    "settingsSearch.boot": {"zh": '启动', "en": 'Boot', "tw": '啟動'},
    "settingsSearch.chatSettings": {"zh": '对话', "en": 'Chat', "tw": '對話'},
    "settingsSearch.context": {"zh": '上下文', "en": 'Context', "tw": '上下文'},
    "settingsSearch.general": {"zh": '通用', "en": 'General', "tw": '通用'},
    "settingsSearch.history": {"zh": '历史', "en": 'History', "tw": '歷史'},
    "settingsSearch.language": {"zh": '语言', "en": 'Language', "tw": '語言'},
    "settingsSearch.maxToken": {"zh": '最大 Token', "en": 'Max Token', "tw": '最大 Token'},
    "settingsSearch.startup": {"zh": '启动', "en": 'Startup', "tw": '啟動'},
    "settingsSearch.temperature": {"zh": '温度', "en": 'Temperature', "tw": '溫度'},

    # === wiki ===
    "wiki.lint": {"zh": 'Lint 检查', "en": 'Lint check', "tw": 'Lint 檢查'},
    "wiki.lintReport": {"zh": 'Lint 报告', "en": 'Lint report', "tw": 'Lint 報告'},

    # === root-level debugPanel namespace（DebugPanel.tsx 用 debugPanel.trace 等） ===
    "debugPanel.trace": {"zh": '追踪 ID', "en": 'Trace ID', "tw": '追蹤 ID'},
    "debugPanel.spans": {"zh": '跨度', "en": 'Spans', "tw": '跨度'},
    "debugPanel.tokens": {"zh": '令牌', "en": 'Tokens', "tw": '令牌'},
    "debugPanel.nodeId": {"zh": '节点 ID', "en": 'Node ID', "tw": '節點 ID'},
    "debugPanel.type": {"zh": '类型', "en": 'Type', "tw": '類型'},
    "debugPanel.status": {"zh": '状态', "en": 'Status', "tw": '狀態'},
    "debugPanel.duration": {"zh": '耗时', "en": 'Duration', "tw": '耗時'},
    "debugPanel.subWorkflow": {"zh": '子工作流', "en": 'Sub-workflow', "tw": '子工作流'},
    "debugPanel.executionId": {"zh": '执行 ID', "en": 'Execution ID', "tw": '執行 ID'},
    "debugPanel.workflowId": {"zh": '工作流 ID', "en": 'Workflow ID', "tw": '工作流 ID'},
    "debugPanel.parentExecution": {"zh": '父执行 ID', "en": 'Parent execution ID', "tw": '父執行 ID'},

    # === workflow.debugPanel 从 object → string ===
    "workflow.debugPanel": {"zh": '调试面板', "en": 'Debug panel', "tw": '調試面板'},

    # === workflow.aiPanel.* ===
    "workflow.aiPanel.workflowParsed": {"zh": '工作流解析成功', "en": 'Workflow parsed', "tw": '工作流解析成功'},
    "workflow.aiPanel.workflowParsedLowConfidence": {"zh": '工作流解析完成（置信度较低）', "en": 'Workflow parsed (low confidence)', "tw": '工作流解析完成（信心度較低）'},

    # === 单点缺失 ===
    "agentProfile.saveSuccess": {"zh": '能力集已保存', "en": 'Profile saved', "tw": '能力集已儲存'},
    "skill.error": {"zh": '技能执行错误', "en": 'Skill error', "tw": '技能執行錯誤'},
    "nudge.snooze": {"zh": '稍后提醒', "en": 'Snooze', "tw": '稍後提醒'},
    "searchUtils.highCredibility": {"zh": '高可信度', "en": 'High credibility', "tw": '高可信度'},
    "searchUtils.mediumCredibility": {"zh": '中可信度', "en": 'Medium credibility', "tw": '中可信度'},
    "shortcuts.feishuWechat": {"zh": '飞书/微信', "en": 'Lark/WeChat', "tw": '飛書/微信'},
    "shortcuts.qqWechat": {"zh": 'QQ/微信', "en": 'QQ/WeChat', "tw": 'QQ/微信'},
    "shortcuts.wechatWindows": {"zh": '微信 Windows', "en": 'WeChat Windows', "tw": '微信 Windows'},
}

# 需要将 object 覆盖为 string 的 key
KEYS_TO_OVERWRITE = {"workflow.debugPanel"}


def add_key(data, dot_key, value):
    """将 dot-key 写入嵌套 dict，遇到已存在的 object 可以用 KEYS_TO_OVERWRITE 覆盖。"""
    parts = dot_key.split(".")
    d = data
    for i, p in enumerate(parts):
        if i == len(parts) - 1:
            d[p] = value
        else:
            if p not in d:
                d[p] = {}
            elif not isinstance(d[p], dict):
                d[p] = {}
            d = d[p]


def main():
    print("=" * 60)
    print("修复剩余 true-missing key + P1")
    print("=" * 60)

    for fname in sorted(os.listdir(LOCALES_DIR)):
        if not fname.endswith(".json"):
            continue
        lang = fname.replace(".json", "")
        path = os.path.join(LOCALES_DIR, fname)

        with open(path, encoding="utf-8") as f:
            data = json.load(f)

        count = 0
        for dot_key, trans in sorted(T.items()):
            if lang == "zh-CN":
                val = trans["zh"]
            elif lang == "en-US":
                val = trans["en"]
            elif lang == "zh-TW":
                val = trans["tw"]
            else:
                val = trans["en"]  # 其他语言英文回退

            # 对于 workflow.debugPanel，如果当前是 object 先删除再写
            if dot_key in KEYS_TO_OVERWRITE:
                parts = dot_key.split(".")
                d = data
                for p in parts[:-1]:
                    if isinstance(d, dict) and p in d:
                        d = d[p]
                    else:
                        d = None
                        break
                if d and isinstance(d, dict) and isinstance(d.get(parts[-1]), dict):
                    del d[parts[-1]]

            add_key(data, dot_key, val)
            count += 1

        with open(path, "w", encoding="utf-8") as f:
            json.dump(data, f, ensure_ascii=False, indent=2)

        print(f"  {lang}: +{count} keys")

    print("\n✅ 完成")


if __name__ == "__main__":
    main()
