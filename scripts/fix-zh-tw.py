"""将 zh-TW 中所有纯英文文案替换为传统中文（取 zh-CN 对应值）"""
import json, re

with open('src/i18n/locales/zh-TW.json', encoding='utf-8') as f:
    tw = json.load(f)
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

def get_by_path(obj, path):
    for k in path.split('.'):
        if isinstance(obj, dict) and k in obj:
            obj = obj[k]
        else:
            return None
    return obj

def set_by_path(obj, path, value):
    keys = path.split('.')
    for k in keys[:-1]:
        if k not in obj or not isinstance(obj[k], dict):
            obj[k] = {}
        obj = obj[k]
    obj[keys[-1]] = value

# zh-CN 所有的值
zh_items = {p: v for p, v in walk(zh)}

# 查找 zh-TW 中是纯英文（不含任何 CJK）且 zh-CN 有中文值的 key
repl = 0
for p, tw_val in walk(tw):
    s = tw_val.strip()
    # 纯英文字符串（无中/日/韩文字）
    if s and not re.search(r'[\u4e00-\u9fff\u3040-\u309f\u30a0-\u30ff\uac00-\ud7af]', s):
        zh_val = zh_items.get(p)
        if zh_val and re.search(r'[\u4e00-\u9fff]', zh_val):
            set_by_path(tw, p, zh_val)
            print(f'  {p}')
            repl += 1

with open('src/i18n/locales/zh-TW.json', 'w', encoding='utf-8') as f:
    json.dump(tw, f, ensure_ascii=False, indent=2)

print(f'\nReplaced {repl} English strings with Traditional Chinese')
