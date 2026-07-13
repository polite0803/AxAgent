#!/usr/bin/env python3
"""NeoData 单只查行业 — 每次查 1 只股票，确保结果正确"""

import csv
import json
import re
import subprocess
import sys
import time
from pathlib import Path

RAW = Path("knowledge-sources/lemonhu/raw")
PYTHON = sys.executable
NEODATA = "C:/Users/polit/AppData/Local/Programs/WorkBuddy/resources/app.asar.unpacked/resources/builtin-skills/neodata-financial-search/scripts/query.py"


def load_missing() -> list[tuple[str, str]]:
    codes = {}
    with open(RAW / "stock.csv", newline="", encoding="utf-8") as f:
        for row in csv.DictReader(f):
            codes[row["stock_id:ID(Stock)"].strip()] = row["name"].strip()
    with_industry = set()
    with open(RAW / "stock_industry.csv", newline="", encoding="utf-8") as f:
        for row in csv.DictReader(f):
            with_industry.add(row[":START_ID(Stock)"].strip())
    return [(c, codes[c]) for c in sorted(set(codes) - with_industry)]


def extract_industries(content: str) -> list[str]:
    cleaned = content.replace("所属行业", "")
    m = re.search(r'概念情况如下：(.+?)所属概念', cleaned, re.DOTALL)
    if not m:
        return []
    return list(dict.fromkeys(
        re.findall(r'([\u4e00-\u9fff\w]+)最近交易日涨跌幅', m.group(1))
    ))


def main():
    missing = load_missing()
    print(f"缺行业: {len(missing)} 只")

    # 读已有缓存（断点续传）
    filled: dict[str, str] = {}
    if (RAW / "missing_industry_filled.csv").exists():
        with open(RAW / "missing_industry_filled.csv", newline="", encoding="utf-8") as f:
            for row in csv.DictReader(f):
                filled[row["stock_code"]] = row["industry"]
        print(f"缓存: {len(filled)} 只")

    remaining = [(c, n) for c, n in missing if c not in filled]
    print(f"还需: {len(remaining)} 只")

    for i, (code, name) in enumerate(remaining, 1):
        q = f"{code}{name}的行业分类是什么？"
        try:
            r = subprocess.run(
                [PYTHON, NEODATA, "--query", q],
                capture_output=True, text=True, timeout=30,
                cwd=str(Path(NEODATA).parent),
            )
            if r.returncode != 0:
                print(f"  [{i}/{len(remaining)}] {code} {name}: spawn failed")
                continue

            data = json.loads(r.stdout)
            for item in data.get("data", {}).get("apiData", {}).get("apiRecall", []):
                inds = extract_industries(item.get("content", ""))
                if inds:
                    filled[code] = inds[0]
                    break
        except Exception as e:
            print(f"  [{i}/{len(remaining)}] {code} {name}: {e}")

        if i % 20 == 0:
            _write(filled)
            print(f"  [{i}/{len(remaining)}] 累计 {len(filled)}/{len(missing)}")

        time.sleep(0.3)

    _write(filled)
    print(f"\n完成: {len(filled)}/{len(missing)}")


def _write(d: dict):
    with open(RAW / "missing_industry_filled.csv", "w", newline="", encoding="utf-8") as f:
        w = csv.writer(f)
        w.writerow(["stock_code", "industry"])
        for code, ind in sorted(d.items()):
            w.writerow([code, ind])


if __name__ == "__main__":
    main()
