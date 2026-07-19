#!/usr/bin/env python
"""i18n 词条审计脚本 v2：扩展 t() 匹配模式，更接近 i18n Ally 的扫描精度。

支持模式：
1. t("static.key") / t('static.key')              — 静态字符串
2. t(`static.key`)                                  — 模板字面量（无插值）
3. t(`error.${code}`)                               — 模板字面量（含插值）→ 记录 prefix
4. t(`${prefix}.${code}`)                           — 模板字面量（前缀 + 插值）→ 记录 prefix
5. t("prefix." + var)                               — 字符串拼接 → 记录 prefix
6. tOptions / Trans / useTranslation 数组
"""

import json
import os
import re
from collections import OrderedDict, defaultdict

SRC_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "src")
LOCALES_DIR = os.path.join(SRC_DIR, "i18n", "locales")

# ─── 词条展开 ───

def flatten_keys(obj, prefix=""):
    """递归展平嵌套 JSON，返回 {dot_key: value}。"""
    result = OrderedDict()
    if isinstance(obj, dict):
        for k, v in obj.items():
            p = f"{prefix}.{k}" if prefix else k
            if isinstance(v, dict):
                result.update(flatten_keys(v, p))
            else:
                result[p] = v
    return result

def read_locale(filepath):
    with open(filepath, encoding="utf-8") as f:
        return flatten_keys(json.load(f))

# ─── 提取 t() 调用（多模式）───

# 1) t("static.key")  /  t('static.key')  —— 普通字符串字面量
T_STATIC = re.compile(
    r'(?:^|[^a-zA-Z0-9_$])(?:t|Trans|trans|i18n\.t)\(\s*'
    r"(['\"])((?:(?!\1)[^\\]|\\.)*?)\1",
    re.MULTILINE,
)

# 2) t(`key.with.dots`)  —— 模板字面量（无 ${} 插值）
T_TEMPLATE_PURE = re.compile(
    r'(?:^|[^a-zA-Z0-9_$])(?:t|Trans|trans|i18n\.t)\(\s*'
    r'`([^`$]+)`',
    re.MULTILINE,
)

# 3) t(`prefix.${var}`)  —— 模板字面量（前缀 + 插值）
T_TEMPLATE_PREFIX = re.compile(
    r'(?:^|[^a-zA-Z0-9_$])(?:t|Trans|trans|i18n\.t)\(\s*'
    r'`([A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)*)\.\$\{',
    re.MULTILINE,
)

# 3.1) t(`prefix.${expr}.suffix`)  —— 模板字面量（中间插值）
T_TEMPLATE_MID = re.compile(
    r'(?:^|[^a-zA-Z0-9_])(?:t|Trans|trans|i18n\.t)\(\s*'
    r'`([A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)*)\.\$\{',
    re.MULTILINE,
)

# 4) t("prefix." + var)  /  t(`prefix.` + var)  —— 字符串拼接
T_CONCAT_PREFIX = re.compile(
    r'(?:^|[^a-zA-Z0-9_$])(?:t|Trans|trans|i18n\.t)\(\s*'
    r'(?:[`\'"])((?:[A-Za-z_][A-Za-z0-9_]*\.)+)'
    r'(?:[`\'"])?\s*\+',
    re.MULTILINE,
)

# 5) Trans.withNamespaces  /  useTranslation(namespaces)  /  ns: ['a', 'b']
NAMESPACE_BLOCK = re.compile(
    r'(?:useTranslation|i18n\.use)\(\s*\[?\s*'
    r'((?:\s*[\'"][A-Za-z_][A-Za-z0-9_]*[\'"],?)+)',
    re.MULTILINE,
)

# 6) t(condition ? "key.a" : "key.b")  —— 三元表达式
T_TERNARY = re.compile(
    r'(?:^|[^a-zA-Z0-9_$])(?:t|Trans|trans|i18n\.t)\([^()]*?'
    r'\?\s*'
    r'(?:[`\'"])([A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)+)(?:[`\'"])'
    r'\s*:\s*'
    r'(?:[`\'"])([A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)+)(?:[`\'"])',
    re.MULTILINE,
)

# 7) `<Trans i18nKey="static.key">`  —— Trans 组件属性
TRANS_COMPONENT = re.compile(
    r'<Trans\b[^>]*\bi18nKey\s*=\s*'
    r'([\'"])([^\'"]+)\1',
    re.MULTILINE,
)

# 8) 对象字面量中的 i18n key（labelKey / i18nKey / nameKey / descriptionKey / titleKey / tabKey / key）
#    只匹配属性名明确暗示是 i18n key 的场景
KEY_PROP = re.compile(
    r'(?:i18nKey|labelKey|nameKey|descriptionKey|titleKey|tabKey|textKey|tooltipKey|placeholderKey|messageKey|errMsgKey|keyName|translationKey|msgKey)'
    r'\s*:\s*'
    r'([\'"])([^\'"]+?)\1',
    re.MULTILINE,
)


def extract_keys_from_text(content, filepath):
    """从源码中提取所有 i18n 引用 key。"""
    static_keys = []       # 完整 key，如 "error.NETWORK"
    dynamic_prefixes = []  # 动态 prefix，如 "error"
    rel = os.path.relpath(filepath, SRC_DIR)

    # 模式 1: 静态字符串
    for m in T_STATIC.finditer(content):
        key = m.group(2)
        if "${" in key or "{" in key or "}" in key:
            continue
        if not re.search(r'\.', key) and key.isupper() and len(key) > 4:
            continue
        try:
            key = key.encode().decode('unicode_escape')
        except Exception:
            pass
        static_keys.append(key)

    # 模式 2: 纯模板字面量
    for m in T_TEMPLATE_PURE.finditer(content):
        key = m.group(1)
        if not key.strip():
            continue
        static_keys.append(key)

    # 模式 3: 模板字面量 prefix
    for m in T_TEMPLATE_PREFIX.finditer(content):
        prefix = m.group(1)
        dynamic_prefixes.append(prefix)
        static_keys.append(f"{prefix}.__DYNAMIC__")

    # 模式 4: 字符串拼接
    for m in T_CONCAT_PREFIX.finditer(content):
        prefix = m.group(1).rstrip(".")
        dynamic_prefixes.append(prefix)
        static_keys.append(f"{prefix}.__DYNAMIC__")

    # 模式 6: 三元表达式
    for m in T_TERNARY.finditer(content):
        k1, k2 = m.group(1), m.group(2)
        if k1:
            static_keys.append(k1)
        if k2:
            static_keys.append(k2)

    # 模式 7: <Trans i18nKey="...">
    for m in TRANS_COMPONENT.finditer(content):
        key = m.group(2)
        if key and "${" not in key:
            static_keys.append(key)

    # 模式 8: 对象字面量中 i18n key（labelKey, i18nKey 等）
    for m in KEY_PROP.finditer(content):
        key = m.group(2)
        if key and "${" not in key and re.search(r'\.', key):
            static_keys.append(key)

    return static_keys, dynamic_prefixes


def collect_all_references():
    """遍历 src/ 收集所有 i18n 引用。"""
    static_calls = OrderedDict()   # key -> [files]
    dynamic_prefixes = defaultdict(set)  # prefix -> {files}
    
    for root, dirs, files in os.walk(SRC_DIR):
        skip = {"node_modules", "__pycache__", ".git", "dist", "build", "i18n"}
        dirs[:] = [d for d in dirs if d not in skip]
        for f in files:
            if not f.endswith((".ts", ".tsx", ".js", ".jsx")):
                continue
            path = os.path.join(root, f)
            try:
                with open(path, encoding="utf-8") as fp:
                    content = fp.read()
            except Exception:
                continue
            rel = os.path.relpath(path, SRC_DIR)
            keys, prefixes = extract_keys_from_text(content, path)
            for k in keys:
                static_calls.setdefault(k, []).append(rel)
            for p in prefixes:
                dynamic_prefixes[p].add(rel)

    return static_calls, dynamic_prefixes


# ─── 主分析 ───

def main():
    print("=" * 64)
    print("AxAgent i18n 词条审计报告 v2（高精度版）")
    print("=" * 64)

    # 1. 加载所有 locale
    print("\n📂 语言资源文件统计：")
    locales = {}
    for fname in sorted(os.listdir(LOCALES_DIR)):
        if fname.endswith(".json"):
            lang = fname.replace(".json", "")
            locales[lang] = read_locale(os.path.join(LOCALES_DIR, fname))
            print(f"  {lang}: {len(locales[lang])} keys")
    
    zh_cn = locales.get("zh-CN", {})
    en_us = locales.get("en-US", {})

    # 2. 扫描源码
    print("\n🔍 扫描源码中的 i18n 引用...")
    static_calls, dynamic_prefixes = collect_all_references()
    print(f"  静态/可解析 key: {len(static_calls)} 个")
    print(f"  动态 prefix: {len(dynamic_prefixes)} 个")

    # 3. 区分真正静态 key vs __DYNAMIC__ 标记
    real_static = [k for k in static_calls if not k.endswith(".__DYNAMIC__")]
    dynamic_marked = [k for k in static_calls if k.endswith(".__DYNAMIC__")]
    print(f"  其中纯静态 key: {len(real_static)}")
    print(f"  其中动态 prefix 标记: {len(dynamic_marked)}")

    # 4. 计算"已使用"——比 i18n Ally 更宽松的口径
    #    任何 key（包括动态 prefix 覆盖的整段）算使用
    all_used_keys = set()
    
    # 4.1 纯静态 key 本身
    for k in real_static:
        all_used_keys.add(k)
    
    # 4.2 静态 key 的所有祖先路径（i18n Ally 的"已使用段"算法）
    for k in real_static:
        parts = k.split(".")
        for i in range(1, len(parts)):
            all_used_keys.add(".".join(parts[:i]))
    
    # 4.3 动态 prefix 也算使用（覆盖整段）
    for prefix in dynamic_prefixes:
        all_used_keys.add(prefix)
        # prefix 的所有祖先
        parts = prefix.split(".")
        for i in range(1, len(parts)):
            all_used_keys.add(".".join(parts[:i]))

    print(f"  总已使用 key（含祖先段）: {len(all_used_keys)}")

    # 5. 未定义：代码用了但 zh-CN 没有
    # 注意：i18n Ally 的 262 个包含"未定义 key"和"未定义父段"
    # 这里我们以"最严格"：所有静态 key + 所有动态 prefix 都算引用，找它们在 zh-CN 中是否存在
    referenced = set(real_static) | set(dynamic_prefixes.keys())
    undefined_keys = sorted(referenced - set(zh_cn.keys()))
    print(f"\n❌ 未定义 key: {len(undefined_keys)}")

    # 6. 未使用：zh-CN 定义了但所有引用算法都没覆盖
    unused_keys = sorted(set(zh_cn.keys()) - all_used_keys)
    print(f"🗑️ 未使用 key: {len(unused_keys)}")

    # 7. 各语言翻译完整度
    print("\n🌐 各语言翻译完整度：")
    for lang in sorted(locales):
        if lang == "zh-CN":
            continue
        missing = set(zh_cn.keys()) - set(locales[lang].keys())
        extra = set(locales[lang].keys()) - set(zh_cn.keys())
        status = "✅" if not missing and not extra else "⚠️"
        print(f"  {lang}: {len(locales[lang])} keys, 缺 {len(missing)}, 多 {len(extra)} {status}")

    # 8. zh-CN vs en-US 一致性
    zh_only = set(zh_cn.keys()) - set(en_us.keys())
    en_only = set(en_us.keys()) - set(zh_cn.keys())
    print(f"\n🇨🇳 vs 🇺🇸 一致性: zh-CN 独有 {len(zh_only)}, en-US 独有 {len(en_only)}")

    # ─── 输出详细报告 ───
    print("\n" + "=" * 64)
    print(f"❌ 未定义的 key（共 {len(undefined_keys)}）")
    print("=" * 64)
    for k in undefined_keys:
        files = static_calls.get(k) or sorted(dynamic_prefixes.get(k, set()))
        sample = files[:2] if files else ["(动态拼接)"]
        print(f"  ❌ {k}")
        for f in sample:
            print(f"      └─ {f}")
        if isinstance(files, list) and len(files) > 2:
            print(f"      └─ ... 还有 {len(files) - 2} 处")

    print("\n" + "=" * 64)
    print(f"🗑️ 未使用的 key（共 {len(unused_keys)}）")
    print("=" * 64)
    # 按 prefix 分组
    groups = OrderedDict()
    for k in unused_keys:
        prefix = k.split(".")[0] if "." in k else "(root)"
        groups.setdefault(prefix, []).append(k)
    
    for prefix, keys in groups.items():
        print(f"\n  [{prefix}] — {len(keys)} 条")
        for k in keys:
            val = zh_cn[k]
            if isinstance(val, str) and len(val) > 50:
                val = val[:47] + "..."
            print(f"    - {k} → \"{val}\"")

    print("\n✅ 审计完成")
    return {
        "total_defined": len(zh_cn),
        "static_keys": len(real_static),
        "dynamic_prefixes": len(dynamic_prefixes),
        "all_used": len(all_used_keys),
        "undefined": undefined_keys,
        "unused": unused_keys,
    }


if __name__ == "__main__":
    main()
