"""补全 ja.json 中缺失的 key，从 en-US 取值作为临时占位"""
import json

with open('src/i18n/locales/en-US.json', encoding='utf-8') as f:
    en = json.load(f)
with open('src/i18n/locales/ja.json', encoding='utf-8') as f:
    ja = json.load(f)

def leaf(obj, p=''):
    kk = set()
    for k, v in obj.items():
        pp = f'{p}.{k}' if p else k
        if isinstance(v, str): kk.add(pp)
        elif isinstance(v, dict): kk.update(leaf(v, pp))
    return kk

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
    if not isinstance(obj.get(keys[-1]), dict):
        obj[keys[-1]] = val

ek = leaf(en)
jk = leaf(ja)
missing = sorted(ek - jk)

# Also remove top-level fineTune if it exists
if 'fineTune' in ja:
    del ja['fineTune']
    print('Removed top-level fineTune from ja')

count = 0
for p in missing:
    en_val = get_by(en, p)
    if en_val is not None and isinstance(en_val, str):
        set_by(ja, p, en_val)
        count += 1

with open('src/i18n/locales/ja.json', 'w', encoding='utf-8') as f:
    json.dump(ja, f, ensure_ascii=False, indent=2)

# verify
jk2 = leaf(ja)
rem = sorted(ek - jk2)
ext = sorted(jk2 - ek)
print(f'Filled: {count} keys')
print(f'en-US: {len(ek)} | ja: {len(jk2)}')
print(f'Still missing: {len(rem)}')
print(f'Extra: {len(ext)}')
if ext:
    for e in ext:
        print(f'  Extra: {e}')
if not rem and not ext:
    print('✅ ALL KEYS MATCH!')
