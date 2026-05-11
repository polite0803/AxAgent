"""根据 en-US 补全 zh-TW 中缺失的 key，从 zh-CN 取值作为翻译参考"""
import json

with open('src/i18n/locales/en-US.json', 'r', encoding='utf-8') as f:
    en = json.load(f)
with open('src/i18n/locales/zh-CN.json', 'r', encoding='utf-8') as f:
    zh = json.load(f)
with open('src/i18n/locales/zh-TW.json', 'r', encoding='utf-8') as f:
    tw = json.load(f)

def leaf_keys(obj, prefix=''):
    keys = set()
    for k, v in obj.items():
        p = f'{prefix}.{k}' if prefix else k
        if isinstance(v, str):
            keys.add(p)
        elif isinstance(v, dict):
            keys.update(leaf_keys(v, p))
    return keys

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

en_keys = leaf_keys(en)
tw_keys = leaf_keys(tw)
missing = sorted(en_keys - tw_keys)
print(f"Missing keys: {len(missing)}")

for p in missing:
    en_val = get_by_path(en, p)
    zh_val = get_by_path(zh, p)
    val = zh_val if zh_val is not None and isinstance(zh_val, str) else en_val
    set_by_path(tw, p, val)

with open('src/i18n/locales/zh-TW.json', 'w', encoding='utf-8') as f:
    json.dump(tw, f, ensure_ascii=False, indent=2)

tw2_keys = leaf_keys(tw)
remaining = sorted(en_keys - tw2_keys)
extra = sorted(tw2_keys - en_keys)
print(f"\nen-US: {len(en_keys)} | zh-TW: {len(tw2_keys)}")
print(f"Still missing: {len(remaining)}")
print(f"Extra keys: {len(extra)}")
if not remaining:
    print("✅ ALL KEYS MATCH!")
