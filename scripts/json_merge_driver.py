#!/usr/bin/env python3
"""
JSON 深度合并 driver —— 用于 git merge 时自动合并 locale JSON 文件。
策略：深度递归合并，ours 和 theirs 的 key 都保留；同 key 冲突时 ours 优先。

这是可执行脚本，需要在 .gitattributes 里配置：
  src/i18n/locales/*.json merge=json_merge
并在 git config 里注册：
  git config merge.json_merge.name "JSON deep merge for i18n locales"
  git config merge.json_merge.driver "python scripts/json_merge_driver.py %O %A %B"

用法（git 自动调用）：
  python json_merge_driver.py base ours theirs
  python json_merge_driver.py base ours theirs -o output
"""
import json
import sys
from pathlib import Path


def deep_merge(ours, theirs):
    """深度合并 ours 和 theirs。同层冲突时 ours 优先保留。"""
    if isinstance(ours, dict) and isinstance(theirs, dict):
        result = {}
        # theirs 的 key 先加（上游/远程的东西不丢）
        for k, v in theirs.items():
            if k not in result:
                result[k] = v
        # ours 的 key 后加（本地的东西优先）
        for k, v in ours.items():
            if k in theirs:
                # 两边都有 → 递归合并
                result[k] = deep_merge(v, theirs[k])
            else:
                result[k] = v
        return result
    else:
        # 叶子冲突：ours 优先
        return ours


def load_json(path):
    try:
        with open(path, encoding='utf-8') as f:
            return json.load(f)
    except (json.JSONDecodeError, FileNotFoundError):
        # 解析失败 → 当作空 dict（这样至少不会丢对方的内容）
        return {}


def main():
    args = sys.argv[1:]
    if len(args) < 3:
        print('用法: json_merge_driver.py <base> <ours> <theirs> [-o <output>]', file=sys.stderr)
        sys.exit(2)

    base_path = args[0]
    ours_path = args[1]
    theirs_path = args[2]
    out_path = ours_path  # 默认写回 ours

    if '-o' in args:
        idx = args.index('-o')
        if idx + 1 < len(args):
            out_path = args[idx + 1]

    ours = load_json(ours_path)
    theirs = load_json(theirs_path)

    merged = deep_merge(ours, theirs)

    with open(out_path, 'w', encoding='utf-8') as f:
        json.dump(merged, f, ensure_ascii=False, indent=2)
        f.write('\n')

    # 返回 0 表示合并成功（无冲突）
    print(f'[json_merge_driver] merged OK: {out_path}')
    sys.exit(0)


if __name__ == '__main__':
    main()
