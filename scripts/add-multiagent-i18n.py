#!/usr/bin/env python3
"""Add missing multiAgent i18n namespace to all 11 locale files."""
import json
import os

ROOT = os.path.join(os.path.dirname(__file__), "..", "src", "i18n", "locales")

# Master translations (zh-CN = source, en-US = English)
TRANS = {
    "zh-CN": {
        "title": "多智能体协作",
        "subtitle": "将复杂任务分派给专业角色协作完成",
        "rolesTitle": "固定角色",
        "refreshRoles": "刷新角色",
        "noRoles": "暂无可用角色",
        "selectRole": "选择角色",
        "taskDescription": "任务描述",
        "taskPlaceholder": "描述您需要完成的任务...",
        "taskLabel": "任务",
        "contextJsonOptional": "上下文 JSON（可选）",
        "contextJsonPlaceholder": "输入上下文 JSON...",
        "contextJsonInvalid": "JSON 格式无效",
        "provider": "供应商",
        "model": "模型",
        "temperature": "温度",
        "maxTokens": "最大 Token",
        "delegateTitle": "委派任务",
        "delegateBtn": "委派",
        "delegateSuccess": "委派成功",
        "delegateFailed": "委派失败",
        "fillRequired": "请填写任务描述",
        "success": "成功",
        "failed": "失败",
        "tokens": "Token",
        "historyTitle": "委派历史",
        "clearHistory": "清空历史",
        "noHistory": "暂无委派记录",
        "maxConcurrent": "最大并发",
        "timeoutSec": "超时",
        "responseLabel": "响应",
        "temperatureStrict": "严格",
        "temperatureDefault": "默认",
        "temperatureDivergent": "发散",
        "maxTokensDefault": "默认",
    },
    "zh-TW": {
        "title": "多智能體協作",
        "subtitle": "將複雜任務分派給專業角色協作完成",
        "rolesTitle": "固定角色",
        "refreshRoles": "刷新角色",
        "noRoles": "暫無可用角色",
        "selectRole": "選擇角色",
        "taskDescription": "任務描述",
        "taskPlaceholder": "描述您需要完成的任務...",
        "taskLabel": "任務",
        "contextJsonOptional": "上下文 JSON（可選）",
        "contextJsonPlaceholder": "輸入上下文 JSON...",
        "contextJsonInvalid": "JSON 格式無效",
        "provider": "供應商",
        "model": "模型",
        "temperature": "溫度",
        "maxTokens": "最大 Token",
        "delegateTitle": "委派任務",
        "delegateBtn": "委派",
        "delegateSuccess": "委派成功",
        "delegateFailed": "委派失敗",
        "fillRequired": "請填寫任務描述",
        "success": "成功",
        "failed": "失敗",
        "tokens": "Token",
        "historyTitle": "委派歷史",
        "clearHistory": "清空歷史",
        "noHistory": "暫無委派記錄",
        "maxConcurrent": "最大並發",
        "timeoutSec": "超時",
        "responseLabel": "響應",
        "temperatureStrict": "嚴格",
        "temperatureDefault": "預設",
        "temperatureDivergent": "發散",
        "maxTokensDefault": "預設",
    },
    "en-US": {
        "title": "Multi-Agent Collaboration",
        "subtitle": "Delegate complex tasks to specialized roles",
        "rolesTitle": "Fixed Roles",
        "refreshRoles": "Refresh Roles",
        "noRoles": "No roles available",
        "selectRole": "Select Role",
        "taskDescription": "Task Description",
        "taskPlaceholder": "Describe the task you need completed...",
        "taskLabel": "Task",
        "contextJsonOptional": "Context JSON (optional)",
        "contextJsonPlaceholder": "Enter context JSON...",
        "contextJsonInvalid": "Invalid JSON format",
        "provider": "Provider",
        "model": "Model",
        "temperature": "Temperature",
        "maxTokens": "Max Tokens",
        "delegateTitle": "Delegate Task",
        "delegateBtn": "Delegate",
        "delegateSuccess": "Delegation successful",
        "delegateFailed": "Delegation failed",
        "fillRequired": "Please fill in the task description",
        "success": "Success",
        "failed": "Failed",
        "tokens": "Tokens",
        "historyTitle": "Delegation History",
        "clearHistory": "Clear History",
        "noHistory": "No delegation records",
        "maxConcurrent": "Max Concurrent",
        "timeoutSec": "Timeout",
        "responseLabel": "Response",
        "temperatureStrict": "Strict",
        "temperatureDefault": "Default",
        "temperatureDivergent": "Divergent",
        "maxTokensDefault": "Default",
    },
}

# For non-CN/non-EN locales, use English as fallback
FALLBACK_LOCALES = ["ja", "ko", "fr", "de", "es", "ru", "ar", "hi"]

def read_json(path):
    with open(path, "r", encoding="utf-8") as f:
        return json.load(f)

def write_json(path, data):
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        json.dump(data, f, ensure_ascii=False, indent=2)
        f.write("\n")

def add_or_update(data, locale):
    if locale in TRANS:
        data["multiAgent"] = TRANS[locale]
    elif locale in FALLBACK_LOCALES:
        data["multiAgent"] = TRANS["en-US"]
    else:
        print(f"  WARNING: unknown locale {locale}, using zh-CN")
        data["multiAgent"] = TRANS["zh-CN"]

def main():
    # Process zh-CN and zh-TW
    for loc in ["zh-CN", "zh-TW", "en-US"]:
        path = os.path.join(ROOT, f"{loc}.json")
        data = read_json(path)
        add_or_update(data, loc)
        write_json(path, data)
        print(f"  Updated {loc}")

    # Process fallback locales
    for loc in FALLBACK_LOCALES:
        path = os.path.join(ROOT, f"{loc}.json")
        data = read_json(path)
        add_or_update(data, loc)
        write_json(path, data)
        print(f"  Updated {loc}")

if __name__ == "__main__":
    main()
