#!/usr/bin/env python
"""P0 i18n 修复脚本：补全 11 种语言中缺失的高频词条。

修复内容：
1. approval.*             — 在根级添加 approval 块（代码期望路径）
2. workflow.validation.*  — 扁平化双 validation 嵌套
3. settings.toolAccess    — 单点缺失
4. browserMock.*          — 整个命名空间缺失
5. workflow.props.*       — 聚合/路由/汇总 key 缺失
6. workflow.containers*   — 折叠/展开
7. workflow.importExport.* — YAML 导出
8. workflow.aiPanel.*     — AIPanel key
"""

import json
import os
import copy

LOCALES_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "src", "i18n", "locales")

# ─── 翻译表 ───

# zh-CN 翻译
ZH = {}

# en-US 翻译（来自 defaultValue / 自译）
EN = {}

# zh-TW 翻译
TW = {}

# ─── 1. approval 块 ───

APPROVAL_ZH = {
    "panelTitle": "待审批列表",
    "noPending": "暂无待审批项",
    "message": "审批消息",
    "workflowId": "工作流",
    "status": "状态",
    "expiresAt": "过期时间",
    "actions": "操作",
    "decision": "审批结果",
    "approve": "通过",
    "reject": "拒绝",
    "cancel": "取消",
    "notePlaceholder": "添加备注（可选）",
    "approver": "审批人",
}

APPROVAL_EN = {
    "panelTitle": "Pending Approvals",
    "noPending": "No pending approvals",
    "message": "Approval message",
    "workflowId": "Workflow",
    "status": "Status",
    "expiresAt": "Expires at",
    "actions": "Actions",
    "decision": "Decision",
    "approve": "Approve",
    "reject": "Reject",
    "cancel": "Cancel",
    "notePlaceholder": "Add a note (optional)",
    "approver": "Approver",
}

APPROVAL_TW = {
    "panelTitle": "待審批列表",
    "noPending": "暫無待審批項",
    "message": "審批消息",
    "workflowId": "工作流",
    "status": "狀態",
    "expiresAt": "過期時間",
    "actions": "操作",
    "decision": "審批結果",
    "approve": "通過",
    "reject": "拒絕",
    "cancel": "取消",
    "notePlaceholder": "添加備註（可選）",
    "approver": "審批人",
}

# ─── 2. browserMock 块 ───

BROWSERMOCK_ZH = {
    "greeting": "你好！我可以帮你做什么？",
    "understandGoal": "理解目标",
    "analyzeRequirements": "分析需求",
    "designPlan": "设计计划",
    "implementSteps": "实施步骤",
    "verifyResult": "验证结果",
    "confirmCompletion": "确认完成",
    "receivedMessage": "收到消息",
    "executeLabel": "执行",
    "triggerLabel": "触发",
    "fileRead": "读取文件",
    "fileReadDesc": "读取本地文件内容",
    "fileWrite": "写入文件",
    "fileWriteDesc": "将内容写入本地文件",
    "shellCommand": "Shell 命令",
    "shellCommandDesc": "在终端中执行命令",
    "copyHintCursor": "使用 Cursor 编辑器操作文件",
    "copyHintContinue": "按 Enter 继续...",
    "copyHintEnv": "环境变量已加载",
    "copyHintOpenAI": "使用 OpenAI API 格式调用",
    "nlWorkflowExplanation": "上述内容将自动转换为工作流",
    "doubao": "豆包",
    "tongyi": "通义",
    "siliconFlow": "SiliconFlow",
}

BROWSERMOCK_EN = {
    "greeting": "Hello! What can I help you with?",
    "understandGoal": "Understand goal",
    "analyzeRequirements": "Analyze requirements",
    "designPlan": "Design plan",
    "implementSteps": "Implement steps",
    "verifyResult": "Verify result",
    "confirmCompletion": "Confirm completion",
    "receivedMessage": "Received message",
    "executeLabel": "Execute",
    "triggerLabel": "Trigger",
    "fileRead": "Read file",
    "fileReadDesc": "Read local file content",
    "fileWrite": "Write file",
    "fileWriteDesc": "Write content to local file",
    "shellCommand": "Shell command",
    "shellCommandDesc": "Execute a command in terminal",
    "copyHintCursor": "Use Cursor editor for file operations",
    "copyHintContinue": "Press Enter to continue...",
    "copyHintEnv": "Environment variables loaded",
    "copyHintOpenAI": "Call via OpenAI API format",
    "nlWorkflowExplanation": "The above will be automatically converted to a workflow",
    "doubao": "Doubao",
    "tongyi": "Tongyi",
    "siliconFlow": "SiliconFlow",
}

BROWSERMOCK_TW = {
    "greeting": "你好！我可以幫你做什麼？",
    "understandGoal": "理解目標",
    "analyzeRequirements": "分析需求",
    "designPlan": "設計計劃",
    "implementSteps": "實施步驟",
    "verifyResult": "驗證結果",
    "confirmCompletion": "確認完成",
    "receivedMessage": "收到消息",
    "executeLabel": "執行",
    "triggerLabel": "觸發",
    "fileRead": "讀取文件",
    "fileReadDesc": "讀取本地文件內容",
    "fileWrite": "寫入文件",
    "fileWriteDesc": "將內容寫入本地文件",
    "shellCommand": "Shell 命令",
    "shellCommandDesc": "在終端中執行命令",
    "copyHintCursor": "使用 Cursor 編輯器操作文件",
    "copyHintContinue": "按 Enter 繼續...",
    "copyHintEnv": "環境變量已加載",
    "copyHintOpenAI": "使用 OpenAI API 格式調用",
    "nlWorkflowExplanation": "上述內容將自動轉換為工作流",
    "doubao": "豆包",
    "tongyi": "通義",
    "siliconFlow": "SiliconFlow",
}

# ─── workflow.props.* 缺失 key ───

PROPS_ZH = {
    "aggregAll": "全部（数组）",
    "aggregConcat": "拼接（字符串）",
    "aggregConcatHint": "将多个字符串值拼接在一起",
    "aggregCount": "计数",
    "aggregLsmHint": "汇总文本输入。LLM 调用待实现 — 当前使用拼接。",
    "aggregLlmHint": "汇总文本输入。LLM 调用待实现 — 当前使用拼接。",
    "aggregLlmSummarize": "LLM 汇总（文本）",
    "aggregMerge": "合并（对象）",
    "aggregMergeHint": "浅合并 JSON 对象（后者覆盖）",
    "aggregStrategy": "聚合策略",
    "aggregSum": "求和（数值）",
    "aggregWeighted": "加权（数值）",
    "aggregWeightedHint": "加权求和：请在下方输入逗号分隔的权重",
    "branchTimeoutMs": "超时（毫秒）",
    "defaultCase": "默认分支（兜底）",
    "defaultModel": "使用默认模型",
    "degradeOnTimeout": "超时处理",
    "expressionHint": "每个分支的值是一个 Rhai 表达式，使用 `_value` 引用输入",
    "expressionPlaceholder": "_value > 100",
    "llmRoutingModel": "路由模型（可选）",
    "llmRoutingPrompt": "路由提示词",
    "llmRoutingPromptPlaceholder": "描述如何将输入路由到各分支...",
    "matchModeContains": "包含",
    "matchModeExact": "精确",
    "matchModeExpression": "表达式",
    "matchModeRegex": "正则",
    "summarizeModel": "汇总模型（可选）",
    "summarizePrompt": "汇总提示词",
    "summarizePromptPlaceholder": "描述如何汇总...",
    "useLlmRouting": "LLM 智能路由",
    "waitForAll": "等待全部输入",
    "weights": "权重（逗号分隔）",
    "weightsPlaceholder": "例如 0.5, 1.0, 1.5",
}

PROPS_EN = {
    "aggregAll": "All (array)",
    "aggregConcat": "Concat (string)",
    "aggregConcatHint": "Joins string values together",
    "aggregCount": "Count",
    "aggregLsmHint": "Summarizes text inputs. LLM call pending - currently concats.",
    "aggregLlmHint": "Summarizes text inputs. LLM call pending - currently concats.",
    "aggregLlmSummarize": "LLM Summarize (text)",
    "aggregMerge": "Merge (object)",
    "aggregMergeHint": "Shallow-merges JSON objects (latter overwrites)",
    "aggregStrategy": "Aggregation Strategy",
    "aggregSum": "Sum (numeric)",
    "aggregWeighted": "Weighted (numeric)",
    "aggregWeightedHint": "Weighted sum: enter comma-separated weights below",
    "branchTimeoutMs": "Timeout (ms)",
    "defaultCase": "Default case (fallback)",
    "defaultModel": "Use default model",
    "degradeOnTimeout": "On timeout",
    "expressionHint": "Each case value is a Rhai expression. Use `_value` for input.",
    "expressionPlaceholder": "_value > 100",
    "llmRoutingModel": "Model (optional)",
    "llmRoutingPrompt": "Routing Prompt",
    "llmRoutingPromptPlaceholder": "Describe how to route inputs to cases...",
    "matchModeContains": "Contains",
    "matchModeExact": "Exact",
    "matchModeExpression": "Expression",
    "matchModeRegex": "Regex",
    "summarizeModel": "Model (optional)",
    "summarizePrompt": "Summarize prompt",
    "summarizePromptPlaceholder": "Describe how to summarize...",
    "useLlmRouting": "LLM Smart Routing",
    "waitForAll": "Wait for all inputs",
    "weights": "Weights (comma-separated)",
    "weightsPlaceholder": "e.g. 0.5, 1.0, 1.5",
}

PROPS_TW = {
    "aggregAll": "全部（陣列）",
    "aggregConcat": "拼接（字串）",
    "aggregConcatHint": "將多個字串值拼接在一起",
    "aggregCount": "計數",
    "aggregLsmHint": "匯總文本輸入。LLM 調用待實現 — 當前使用拼接。",
    "aggregLlmHint": "匯總文本輸入。LLM 調用待實現 — 當前使用拼接。",
    "aggregLlmSummarize": "LLM 匯總（文本）",
    "aggregMerge": "合併（物件）",
    "aggregMergeHint": "淺合併 JSON 物件（後者覆蓋）",
    "aggregStrategy": "聚合策略",
    "aggregSum": "求和（數值）",
    "aggregWeighted": "加權（數值）",
    "aggregWeightedHint": "加權求和：請在下方輸入逗號分隔的權重",
    "branchTimeoutMs": "超時（毫秒）",
    "defaultCase": "默認分支（兜底）",
    "defaultModel": "使用默認模型",
    "degradeOnTimeout": "超時處理",
    "expressionHint": "每個分支的值是一個 Rhai 表達式，使用 `_value` 引用輸入",
    "expressionPlaceholder": "_value > 100",
    "llmRoutingModel": "路由模型（可選）",
    "llmRoutingPrompt": "路由提示詞",
    "llmRoutingPromptPlaceholder": "描述如何將輸入路由到各分支...",
    "matchModeContains": "包含",
    "matchModeExact": "精確",
    "matchModeExpression": "表達式",
    "matchModeRegex": "正則",
    "summarizeModel": "匯總模型（可選）",
    "summarizePrompt": "匯總提示詞",
    "summarizePromptPlaceholder": "描述如何匯總...",
    "useLlmRouting": "LLM 智能路由",
    "waitForAll": "等待全部輸入",
    "weights": "權重（逗號分隔）",
    "weightsPlaceholder": "例如 0.5, 1.0, 1.5",
}

# ─── workflow.containers ───

CONTAINERS_ZH = {
    "collapsed": "容器已全部折叠",
    "expanded": "容器已全部展开",
}

CONTAINERS_EN = {
    "collapsed": "All containers collapsed",
    "expanded": "All containers expanded",
}

CONTAINERS_TW = {
    "collapsed": "容器已全部折疊",
    "expanded": "容器已全部展開",
}

# ─── workflow.importExport YAML key ───

IMPORTEXPORT_ZH = {
    "exportYamlBtn": "导出 YAML",
    "exportYamlDesc": "将当前工作流导出为 YAML",
    "uploadYamlFile": "上传 YAML 文件",
    "yamlExportFailed": "YAML 导出失败",
    "yamlExportSuccess": "YAML 导出成功",
    "yamlImportFailed": "YAML 导入失败",
    "yamlImportSuccess": "YAML 导入成功",
}

IMPORTEXPORT_EN = {
    "exportYamlBtn": "Export YAML",
    "exportYamlDesc": "Export current workflow as YAML",
    "uploadYamlFile": "Upload YAML file",
    "yamlExportFailed": "YAML export failed",
    "yamlExportSuccess": "YAML exported",
    "yamlImportFailed": "YAML import failed",
    "yamlImportSuccess": "YAML imported",
}

IMPORTEXPORT_TW = {
    "exportYamlBtn": "匯出 YAML",
    "exportYamlDesc": "將當前工作流匯出為 YAML",
    "uploadYamlFile": "上傳 YAML 文件",
    "yamlExportFailed": "YAML 匯出失敗",
    "yamlExportSuccess": "YAML 匯出成功",
    "yamlImportFailed": "YAML 導入失敗",
    "yamlImportSuccess": "YAML 導入成功",
}

# ─── workflow.aiPanel ───

AIPANEL_ZH = {
    "workflowParsed": "工作流解析成功",
    "workflowParsedLowConfidence": "工作流解析完成（置信度较低，建议手动检查）",
}

AIPANEL_EN = {
    "workflowParsed": "Workflow parsed successfully",
    "workflowParsedLowConfidence": "Workflow parsed (low confidence, manual review recommended)",
}

AIPANEL_TW = {
    "workflowParsed": "工作流解析成功",
    "workflowParsedLowConfidence": "工作流解析完成（信心度較低，建議手動檢查）",
}

# ─── settings ───

SETTINGS_ZH = {"toolAccess": "工具访问权限"}

SETTINGS_EN = {"toolAccess": "Tool Access"}

SETTINGS_TW = {"toolAccess": "工具存取權限"}

# ─── 其他单点缺失 key ───

MISC_ZH = {
    "cancel": "取消",
    "common.inherit": "继承",
    "common.more": "更多",
    "common.noResults": "无结果",
    "nudge.snooze": "稍后提醒",
    "rl.monitor.error": "监控错误",
    "skill.error": "技能执行错误",
    "wiki.lint": "检查",
    "wiki.lintReport": "检查报告",
    "agentProfile.saveSuccess": "能力集已保存",
    "shortcuts.feishuWechat": "飞书/微信",
    "shortcuts.qqWechat": "QQ/微信",
    "shortcuts.wechatWindows": "微信 Windows",
    "searchUtils.highCredibility": "高可信度",
    "searchUtils.mediumCredibility": "中可信度",
    "workflow.debugPanel": "调试面板",
    "workflow.rightPanel.invalidSchemaShape": "无效的 Schema 结构",
    "workflow.mergeNode.branches": "分支",
    "workflow.parallelNode.decorative": "装饰性节点",
}

MISC_EN = {
    "cancel": "Cancel",
    "common.inherit": "Inherit",
    "common.more": "More",
    "common.noResults": "No results",
    "nudge.snooze": "Snooze",
    "rl.monitor.error": "Monitor error",
    "skill.error": "Skill execution error",
    "wiki.lint": "Lint",
    "wiki.lintReport": "Lint report",
    "agentProfile.saveSuccess": "Profile saved",
    "shortcuts.feishuWechat": "Lark/WeChat",
    "shortcuts.qqWechat": "QQ/WeChat",
    "shortcuts.wechatWindows": "WeChat Windows",
    "searchUtils.highCredibility": "High credibility",
    "searchUtils.mediumCredibility": "Medium credibility",
    "workflow.debugPanel": "Debug panel",
    "workflow.rightPanel.invalidSchemaShape": "Invalid schema shape",
    "workflow.mergeNode.branches": "Branches",
    "workflow.parallelNode.decorative": "Decorative node",
}

MISC_TW = {
    "cancel": "取消",
    "common.inherit": "繼承",
    "common.more": "更多",
    "common.noResults": "無結果",
    "nudge.snooze": "稍後提醒",
    "rl.monitor.error": "監控錯誤",
    "skill.error": "技能執行錯誤",
    "wiki.lint": "檢查",
    "wiki.lintReport": "檢查報告",
    "agentProfile.saveSuccess": "能力集已儲存",
    "shortcuts.feishuWechat": "飛書/微信",
    "shortcuts.qqWechat": "QQ/微信",
    "shortcuts.wechatWindows": "微信 Windows",
    "searchUtils.highCredibility": "高可信度",
    "searchUtils.mediumCredibility": "中可信度",
    "workflow.debugPanel": "調試面板",
    "workflow.rightPanel.invalidSchemaShape": "無效的 Schema 結構",
    "workflow.mergeNode.branches": "分支",
    "workflow.parallelNode.decorative": "裝飾性節點",
}

# ─── 语言到翻译的映射 ───

def get_translations(lang):
    """返回 (approval, browserMock, props, containers, importExport, aiPanel, settings, misc)"""
    if lang == "zh-CN":
        return (APPROVAL_ZH, BROWSERMOCK_ZH, PROPS_ZH, CONTAINERS_ZH,
                IMPORTEXPORT_ZH, AIPANEL_ZH, SETTINGS_ZH, MISC_ZH)
    elif lang == "en-US":
        return (APPROVAL_EN, BROWSERMOCK_EN, PROPS_EN, CONTAINERS_EN,
                IMPORTEXPORT_EN, AIPANEL_EN, SETTINGS_EN, MISC_EN)
    elif lang == "zh-TW":
        return (APPROVAL_TW, BROWSERMOCK_TW, PROPS_TW, CONTAINERS_TW,
                IMPORTEXPORT_TW, AIPANEL_TW, SETTINGS_TW, MISC_TW)
    else:
        # 其他语言用英文回退（优于显示 raw key）
        return (APPROVAL_EN, BROWSERMOCK_EN, PROPS_EN, CONTAINERS_EN,
                IMPORTEXPORT_EN, AIPANEL_EN, SETTINGS_EN, MISC_EN)


def flatten_validation(data):
    """扁平化 workflow.validation.validation.* → workflow.validation.*"""
    workflow = data.get("workflow")
    if not workflow or not isinstance(workflow, dict):
        return False
    validation = workflow.get("validation")
    if not validation or not isinstance(validation, dict):
        return False
    nested = validation.get("validation")
    if not nested or not isinstance(nested, dict):
        return False
    # 将 nested 内容合并到 validation 层级
    for k, v in nested.items():
        if k != "validation":  # 不覆盖已有同名校验 key
            validation[k] = v
    # 删除多余的 validation 嵌套
    del validation["validation"]
    return True


def apply_fixes(data, lang):
    """对 data 字典应用所有 P0 修复。"""
    appr, bm, props, cont, ie, ap, sett, misc = get_translations(lang)

    # 1. approval 块（根级）
    if "approval" not in data:
        data["approval"] = dict(appr)
    else:
        for k, v in appr.items():
            data["approval"].setdefault(k, v)

    # 2. browserMock 块
    if "browserMock" not in data:
        data["browserMock"] = dict(bm)
    else:
        for k, v in bm.items():
            data["browserMock"].setdefault(k, v)

    # 3. workflow.props 缺失 key
    workflow = data.setdefault("workflow", {})
    wf_props = workflow.setdefault("props", {})
    for k, v in props.items():
        wf_props.setdefault(k, v)

    # 4. workflow.containers* 单点 key
    wf_cont = data.get("workflow", {})
    for k, v in cont.items():
        wf_cont.setdefault(k, v)

    # 5. workflow.importExport YAML key
    wf_ie = workflow.setdefault("importExport", {})
    for k, v in ie.items():
        wf_ie.setdefault(k, v)

    # 6. workflow.aiPanel
    wf_ap = workflow.setdefault("aiPanel", {})
    for k, v in ap.items():
        wf_ap.setdefault(k, v)

    # 7. settings.toolAccess
    wf_set = data.setdefault("settings", {})
    for k, v in sett.items():
        wf_set.setdefault(k, v)

    # 8. 扁平化 validation 嵌套
    flatten_validation(data)

    # 9. 单点缺失 key
    for k, v in misc.items():
        parts = k.split(".")
        if len(parts) == 1:
            data.setdefault(parts[0], v)
        elif len(parts) == 2:
            parent = data.setdefault(parts[0], {})
            parent.setdefault(parts[1], v)
        elif len(parts) == 3:
            p1 = data.setdefault(parts[0], {})
            p2 = p1.setdefault(parts[1], {})
            p2.setdefault(parts[2], v)

    # 10. workflow.debugPanel
    workflow.setdefault("debugPanel", misc.get("workflow.debugPanel", "Debug panel"))

    return data


def main():
    print("=" * 60)
    print("P0 i18n 修复：批量补全缺失词条")
    print("=" * 60)

    for fname in sorted(os.listdir(LOCALES_DIR)):
        if not fname.endswith(".json"):
            continue
        lang = fname.replace(".json", "")
        path = os.path.join(LOCALES_DIR, fname)

        with open(path, encoding="utf-8") as f:
            data = json.load(f)

        before_count = len([k for k in flatten_keys(data)])
        apply_fixes(data, lang)
        after_count = len([k for k in flatten_keys(data)])

        with open(path, "w", encoding="utf-8") as f:
            json.dump(data, f, ensure_ascii=False, indent=2)

        added = after_count - before_count
        print(f"  {lang}: +{added} keys ({before_count} → {after_count})")

    print("\n✅ P0 修复完成")


def flatten_keys(obj, prefix=""):
    result = {}
    if isinstance(obj, dict):
        for k, v in obj.items():
            p = f"{prefix}.{k}" if prefix else k
            if isinstance(v, dict):
                result.update(flatten_keys(v, p))
            else:
                result[p] = v
    return result


if __name__ == "__main__":
    main()
