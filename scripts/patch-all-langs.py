"""补全所有语言文件缺失的 key，删除废弃 fineTune 顶级节"""
import json

langs = ['ar', 'de', 'es', 'fr', 'hi', 'ko', 'ru']

with open('src/i18n/locales/en-US.json', encoding='utf-8') as f:
    en = json.load(f)

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

for lang in langs:
    with open(f'src/i18n/locales/{lang}.json', encoding='utf-8') as f:
        data = json.load(f)
    
    # 删除废弃 fineTune 顶级节
    if 'fineTune' in data:
        del data['fineTune']
    
    # 补全缺失 key
    lk = leaf(data)
    missing = sorted(ek - lk)
    for p in missing:
        en_val = get_by(en, p)
        if en_val is not None and isinstance(en_val, str):
            set_by(data, p, en_val)
    
    # 验证
    lk2 = leaf(data)
    rem = sorted(ek - lk2)
    ext = sorted(lk2 - ek)
    
    with open(f'src/i18n/locales/{lang}.json', 'w', encoding='utf-8') as f:
        json.dump(data, f, ensure_ascii=False, indent=2)
    
    print(f'{lang}: filled {len(missing)} | remove fineTune | remaining={len(rem)} | extra={len(ext)}')
    if rem:
        print(f'  STILL MISSING: {rem[:5]}...')
    if ext:
        print(f'  EXTRA: {ext[:5]}...')

print('\n✅ All languages patched!')
