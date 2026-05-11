"""将 zh-TW 中残留的简体中文转为繁体中文"""
import json, re
from zhconv import convert

with open('src/i18n/locales/zh-TW.json', encoding='utf-8') as f:
    tw = json.load(f)

count = 0
def walk(obj):
    global count
    if isinstance(obj, dict):
        for k, v in obj.items():
            if isinstance(v, str):
                # 检查是否包含简体字 -> 转繁体
                converted = convert(v, 'zh-tw')
                if converted != v:
                    obj[k] = converted
                    count += 1
            elif isinstance(v, dict):
                walk(v)

walk(tw)

with open('src/i18n/locales/zh-TW.json', 'w', encoding='utf-8') as f:
    json.dump(tw, f, ensure_ascii=False, indent=2)

print(f'Converted {count} strings from Simplified to Traditional Chinese')
