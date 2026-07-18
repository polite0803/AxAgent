#!/usr/bin/env python
"""i18n 词条审计脚本：检查未定义、未使用的词条及缺失翻译。"""

import json
import os
import re
from collections import OrderedDict

SRC_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "src")
LOCALES_DIR = os.path.join(SRC_DIR, "i18n", "locales")

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

def collect_all_locales():
    files = sorted(os.listdir(LOCALES_DIR))
    locales = {}
    for fname in files:
        if fname.endswith(".json"):
            lang = fname.replace(".json", "")
            path = os.path.join(LOCALES_DIR, fname)
            locales[lang] = read_locale(path)
            print(f"  {lang}: {len(locales[lang])} keys")
    return locales

# ─── 从源码中提取 t() 调用 ───

T_CALL_RE = re.compile(
    r'(?:^|[^a-zA-Z0-9_$])t\(\s*'
    r"(['\"])((?:(?!\1).)+?)\1\s*"
    r'(?:\s*,\s*[^)]*)?\s*\)',
    re.MULTILINE,
)

def extract_t_calls_from_file(filepath):
    rel = os.path.relpath(filepath, SRC_DIR)
    results = []
    try:
        with open(filepath, encoding="utf-8") as f:
            content = f.read()
    except Exception as e:
        print(f"  ⚠ 跳过 {rel}: {e}")
        return results

    # t('key') 或 t("key")
    for m in T_CALL_RE.finditer(content):
        key = m.group(2)
        # 跳过动态键：包含模板字符串、变量、+ 连接等
        if any(c in key for c in ("${", "+", "`")):
            continue
        # 跳过 i18n 库自身的调用
        if key in ("common:enough",):
            continue
        results.append(key)

    return results

def collect_t_calls():
    calls = {}
    for root, dirs, files in os.walk(SRC_DIR):
        # 跳过 node_modules 和 dist 等
        skip_dirs = {"node_modules", "__pycache__", ".git"}
        dirs[:] = [d for d in dirs if d not in skip_dirs]
        for f in files:
            if f.endswith((".ts", ".tsx", ".js", ".jsx")):
                path = os.path.join(root, f)
                for key in extract_t_calls_from_file(path):
                    calls.setdefault(key, []).append(os.path.relpath(path, SRC_DIR))
    return calls

# ─── 主分析 ───

def main():
    print("=" * 60)
    print("AxAgent i18n 词条审计报告")
    print("=" * 60)

    # 1. 读取 locale 文件
    print("\n📂 语言资源文件统计：")
    locales = collect_all_locales()
    zh_cn = locales.get("zh-CN", {})
    zh_tw = locales.get("zh-TW", {})
    en_us = locales.get("en-US", {})

    print(f"\n📊 zh-CN（源语言）共 {len(zh_cn)} 个词条")
    print(f"📊 en-US（回退语言）共 {len(en_us)} 个词条")

    # 2. 提取代码中的 t() 调用
    print("\n🔍 正在扫描源码中的 t() 调用...")
    t_calls = collect_t_calls()
    print(f"  共找到 {len(t_calls)} 个静态 t() 调用的 key")

    # 3. 未定义的 key：代码用了但 zh-CN 中没有
    undefined_keys = OrderedDict()
    for key in sorted(t_calls):
        if key not in zh_cn:
            undefined_keys[key] = t_calls[key]

    # 4. 未使用的 key：zh-CN 定义了但代码没用到
    used_keys = set(t_calls.keys())
    all_defined = set(zh_cn.keys())
    
    # 对于嵌套 key，检查是否有前缀匹配（例如代码用 "error.CONVERSATION_NOT_FOUND" 但定义有 "error.network"）
    # 即如果定义了 "error" 段，而代码中 t("error.network") 也用了
    
    unused_keys = OrderedDict()
    for key in sorted(all_defined):
        if key not in used_keys:
            # 再检查是否有子 key 被用到（如定义 "error.something" 但 t("error.something.special")）
            # 实际 i18next 不会做前缀匹配，但检查彻底
            unused_keys[key] = zh_cn[key]

    # 5. 缺失翻译：zh-CN 有但其他语言缺失的 key
    print("\n🌐 检查各语言缺失翻译...")
    missing_translations = {}
    for lang, data in locales.items():
        if lang == "zh-CN":
            continue
        missing = OrderedDict()
        for key in zh_cn:
            if key not in data:
                missing[key] = zh_cn[key]
        if missing:
            missing_translations[lang] = missing
            print(f"  {lang}: 缺失 {len(missing)} 个翻译")
        else:
            print(f"  {lang}: ✅ 完整")

    print()

    # ─── 输出报告 ───
    print("\n" + "=" * 60)
    print("📋 报告摘要")
    print("=" * 60)
    print(f"  zh-CN 定义词条:      {len(zh_cn)}")
    print(f"  t() 调用提取的 key:  {len(t_calls)}")
    print(f"  未使用的词条:        {len(unused_keys)}")
    print(f"  未定义的词条:        {len(undefined_keys)}")
    
    print("\n" + "=" * 60)
    print("❌ 未定义的词条（代码中 t() 调用但 zh-CN 中不存在）")
    print("=" * 60)
    if undefined_keys:
        print(f"  共 {len(undefined_keys)} 个：")
        print()
        for key, files in undefined_keys.items():
            print(f"  ❌ {key}")
            for f in files[:3]:  # 最多显示 3 个引用位置
                print(f"      └─ {f}")
            if len(files) > 3:
                print(f"      └─ ... 还有 {len(files)-3} 处引用")
            print()
    else:
        print("  ✅ 没有未定义的词条")

    print("\n" + "=" * 60)
    print("🗑️ 未使用的词条（zh-CN 中定义但代码未引用）")
    print("=" * 60)
    if unused_keys:
        print(f"  共 {len(unused_keys)} 个：")
        # 按前缀分组输出
        groups = OrderedDict()
        for key in unused_keys:
            prefix = key.split(".")[0] if "." in key else "(root)"
            groups.setdefault(prefix, []).append(key)
        
        for prefix, keys in groups.items():
            print(f"\n  [{prefix}] — {len(keys)} 条")
            for k in keys:
                val = unused_keys[k]
                if isinstance(val, str) and len(val) > 60:
                    val = val[:57] + "..."
                print(f"    - {k} → \"{val}\"")
    else:
        print("  ✅ 没有未使用的词条")

    print("\n" + "=" * 60)
    print("🌐 缺失翻译汇总（按语言）")
    print("=" * 60)
    if missing_translations:
        for lang, missing in sorted(missing_translations.items()):
            print(f"\n  [{lang}] 缺失 {len(missing)} 个翻译：")
            count = 0
            for key in missing:
                if count >= 20:
                    print(f"    ... 还有 {len(missing) - count} 条")
                    break
                val = missing[key]
                if isinstance(val, str) and len(val) > 60:
                    val = val[:57] + "..."
                print(f"    - {key} → \"{val}\"")
                count += 1
    else:
        print("  所有语言翻译完整 ✅")

    print("\n" + "=" * 60)
    print("zh-CN 与 en-US 结构一致性检查")
    print("=" * 60)
    zh_only = set(zh_cn.keys()) - set(en_us.keys())
    en_only = set(en_us.keys()) - set(zh_cn.keys())
    if zh_only:
        print(f"\n  zh-CN 有但 en-US 缺失: {len(zh_only)} 条")
        for k in sorted(zh_only)[:15]:
            print(f"    - {k}")
        if len(zh_only) > 15:
            print(f"    ... 还有 {len(zh_only)-15} 条")
    if en_only:
        print(f"\n  en-US 有但 zh-CN 缺失: {len(en_only)} 条")
        for k in sorted(en_only)[:15]:
            val = en_us.get(k, "")
            if isinstance(val, str) and len(val) > 60:
                val = val[:57] + "..."
            print(f"    - {k} → \"{val}\"")
        if len(en_only) > 15:
            print(f"    ... 还有 {len(en_only)-15} 条")
    if not zh_only and not en_only:
        print("  zh-CN 与 en-US 完全一致 ✅")

    print("\n✅ 审计完成。")

if __name__ == "__main__":
    main()
