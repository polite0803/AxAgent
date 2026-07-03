"""
为所有 11 个语言文件的 settings.evolution 对象插入 title 字段
修复 SettingsSidebar.tsx 触发的 i18next 警告：
  "Translation key 'settings.evolution (zh-CN)' returned an object instead of string."
"""
import json
import os
from collections import OrderedDict

LOCALES_DIR = r'd:\OneManager\AxAgent\src\i18n\locales'

# 各语言 settings.evolution.title 的翻译（侧边栏标签）
TITLES = {
    'zh-CN': '自我进化',
    'zh-TW': '自我進化',
    'en-US': 'Self-Evolution',
    'ja': '自己進化',
    'ko': '자기 진화',
    'fr': 'Auto-évolution',
    'de': 'Selbstevolution',
    'es': 'Auto-evolución',
    'ru': 'Самоэволюция',
    'ar': 'التطور الذاتي',
    'hi': 'स्व-विकास',
}


def insert_title(obj):
    """在 settings.evolution 对象最前面插入 title 字段，保持其他字段相对顺序。"""
    if not isinstance(obj, dict) or 'evolution' not in obj:
        return False
    evo = obj['evolution']
    if not isinstance(evo, dict) or 'title' in evo:
        return False
    # 重建有序 dict：title 在最前
    new_evo = OrderedDict()
    new_evo['title'] = 'PLACEHOLDER'  # 后面用语言名替换
    for k, v in evo.items():
        new_evo[k] = v
    obj['evolution'] = new_evo
    return True


for fname, title in TITLES.items():
    p = os.path.join(LOCALES_DIR, fname + '.json')
    with open(p, encoding='utf-8') as f:
        d = json.load(f)
    if not insert_title(d.get('settings', {})):
        print(f'[{fname}] 跳过（无 evolution 字段或已有 title）')
        continue
    d['settings']['evolution']['title'] = title
    with open(p, 'w', encoding='utf-8') as f:
        json.dump(d, f, ensure_ascii=False, indent=2)
        f.write('\n')
    print(f'[{fname}] 已添加 title="{title}"')

print('\n完成。')
