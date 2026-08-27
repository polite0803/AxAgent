#!/usr/bin/env python3
"""
i18n 翻译完整性校验脚本。
用法：python scripts/check_i18n.py [--json] [--strict]

做三件事：
  1. 所有 locale JSON 文件能否被正确解析（语法检查）
  2. zh-CN.json 中所有值是否为空（空值即缺翻译）
  3. 扫描代码中所有 t("xxx.yyy.zzz") 调用，检查是否都能在 zh-CN.json 中找到对应 key

--json   只检查 JSON 语法（合并后快速验证用）
--strict 严格模式：任何缺失都非零退出码（CI 强制）

退出码：0 = 通过，1 = 有问题
"""
import json
import os
import re
import sys
from pathlib import Path

ROOT = Path(__file__).parent.parent
LOCALES_DIR = ROOT / 'src' / 'i18n' / 'locales'
SRC_DIR = ROOT / 'src'

# 扫描代码里 t("...") 调用的正则（覆盖多种写法）
T_CALL_RE = re.compile(
    r'''t\(\s*
        (?:
            ["']([a-zA-Z_][\w.]*)["']       # t("foo.bar")
          | `([a-zA-Z_][\w.]*)`              # t(`foo.bar`)
        )
        [,\s)]
    ''',
    re.VERBOSE,
)

# 明显不是业务 i18n key 的顶部前缀（组件名、HTML 标签、技术名词）
NOISE_TOP = {
    'antd', 'common', 'api', 'a', 'import', 'function', 'handler',
    'html2canvas', 'iframe', 'navigate', 'pre', 'prompt', 'schema',
    'script', 'template', 'textarea', 'timelineJump', 'trigger',
    'vendor_', 'view', 'wikiId', 'left', 'right', 'json',
    'analysisId', 'import_', 'toolchain', 'layout', 'stockCode', 'tab',
    'evolutionConsentModal',
}

# 单段 key 里明显是技术名词/组件名的（t("POST")、t("button") 不是合法业务 key）
SINGLE_NOISE = {
    'Input', 'Button', 'Select', 'Modal', 'Table', 'Card', 'Tabs', 'Drawer',
    'Popover', 'Tooltip', 'Menu', 'Space', 'Form', 'InputNumber', 'DatePicker',
    'POST', 'GET', 'PUT', 'DELETE', 'PATCH', 'JSON', 'HTML', 'API', 'URL',
    'canvas', 'button', 'aside', 'bad', 'Package', 'Unknown', '_', '__route',
    'left', 'right', 'tab', 'view', 'pre', 'navigate', 'prompt', 'trigger',
    'function', 'handler', 'schema', 'import', 'template', 'textarea',
    'script', 'iframe', 'layout', 'json', 'vendor_', 'import_', 'toolchain',
    'analysisId', 'stockCode', 'wikiId', 'timelineJump', 'html2canvas',
}


def is_real_i18n_key(raw_key):
    """判断扫描到的 key 是否是真正业务 i18n key。"""
    key = raw_key.strip()
    # 1) 必须至少两段（"foo.bar"），单段的 99% 是技术名词
    if '.' not in key:
        return False
    # 2) 正则必须能识别：字母/下划线开头，只能含字母数字下划线和点
    if not re.match(r'^[a-zA-Z][a-zA-Z0-9_]*(?:\.[a-zA-Z][a-zA-Z0-9_]*)*$', key):
        return False
    # 3) 顶部前缀不在噪声集合里
    if key.split('.')[0] in NOISE_TOP:
        return False
    # 4) 点号结尾的截断 key（正则误匹配）
    if key.endswith('.'):
        return False
    return True


def find_json_syntax_errors():
    """检查所有 locale JSON 文件能否解析。"""
    errors = []
    for f in sorted(LOCALES_DIR.glob('*.json')):
        try:
            json.load(open(f, encoding='utf-8'))
        except json.JSONDecodeError as e:
            errors.append(f'{f.name}: {e}')
    return errors


def find_empty_values(zh):
    """找 zh-CN 里值为空字符串的叶子 key。"""
    empty = []
    def walk(obj, prefix=''):
        if isinstance(obj, dict):
            for k, v in obj.items():
                walk(v, f'{prefix}.{k}' if prefix else k)
        elif isinstance(obj, str) and obj == '':
            empty.append(prefix)
    walk(zh)
    return empty


def collect_tree_keys(tree):
    """把嵌套 JSON 拍平成 set('a.b.c') 的 key 集合。"""
    keys = set()
    def walk(obj, prefix=''):
        if isinstance(obj, dict):
            for k, v in obj.items():
                key = f'{prefix}.{k}' if prefix else k
                keys.add(key)
                walk(v, key)
        # 叶子不加入 keys（叶子 key 已在上一层加入）
    walk(tree)
    return keys


def scan_code_t_keys():
    """扫描 src/ 下所有 ts/tsx/js 文件中 t() 调用的 key。"""
    keys = set()
    for ext in ('*.ts', '*.tsx', '*.js', '*.jsx'):
        for f in SRC_DIR.rglob(ext):
            if f.name.endswith('.d.ts'):
                continue
            try:
                text = f.read_text(encoding='utf-8', errors='ignore')
            except Exception:
                continue
            for m in T_CALL_RE.finditer(text):
                key = m.group(1) or m.group(2)
                if is_real_i18n_key(key):
                    keys.add(key)
    return keys


def main():
    args = set(sys.argv[1:])
    json_only = '--json' in args
    strict = '--strict' in args

    print('=' * 60)
    print('i18n 翻译完整性校验')
    print('=' * 60)

    # 1) JSON 语法
    errors = find_json_syntax_errors()
    if errors:
        print(f'\n❌ JSON 语法错误 ({len(errors)}):')
        for e in errors:
            print(f'  {e}')
        if strict:
            sys.exit(1)
    else:
        print(f'\n✅ 全部 {len(list(LOCALES_DIR.glob("*.json")))} 个 locale JSON 文件语法正常')

    if json_only:
        return

    # 2) zh-CN 空值
    zh = json.load(open(LOCALES_DIR / 'zh-CN.json', encoding='utf-8'))
    empty = find_empty_values(zh)
    if empty:
        print(f'\n⚠️  zh-CN 中有 {len(empty)} 个空值 key:')
        for k in empty[:20]:
            print(f'  {k} = ""')
        if len(empty) > 20:
            print(f'  ... 还有 {len(empty) - 20} 个')
        if strict:
            sys.exit(1)
    else:
        print('✅ zh-CN 无空值')

    # 3) 代码引用 vs locale 覆盖
    zh_keys = collect_tree_keys(zh)
    code_keys = scan_code_t_keys()
    missing = sorted(code_keys - zh_keys)

    # 反向：locale 有但代码没用到 = 可能是上游合并进来还没被消费的
    orphan = sorted(zh_keys - code_keys)

    print(f'\n📊 代码中 t() 调用唯一 key: {len(code_keys)}')
    print(f'📊 zh-CN 唯一 key 路径:   {len(zh_keys)}')

    if missing:
        print(f'\n❌ 代码引用但 zh-CN 缺失 ({len(missing)}):')
        for k in missing[:20]:
            print(f'  {k}')
        if len(missing) > 20:
            print(f'  ... 还有 {len(missing) - 20} 个')
        if strict:
            sys.exit(1)
    else:
        print('✅ 代码引用的所有 key 在 zh-CN 中都有定义')

    if orphan:
        # orphan 不一定是问题（上游合并进来还没被消费、或用 key() 等非 t() 方式引用）
        # 只报告，不作为退出条件
        print(f'\nℹ️  zh-CN 中有 {len(orphan)} 个 key 未被代码引用（可能是上游新增）')

    print('\n' + '=' * 60)
    if missing or errors or empty:
        if strict:
            print('❌ 校验未通过 (strict 模式)')
            sys.exit(1)
        print('⚠️  校验有问题，详见上方输出')
    else:
        print('🎉 全部通过')


if __name__ == '__main__':
    main()
