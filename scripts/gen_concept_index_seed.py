#!/usr/bin/env python3
"""读取 lemonhu nodes.csv → 生成 seed_ashare_ontology Rust 函数代码。

用 MD5 hash 作为 ID（匹配 lemonhu 边数据），中文名作为 display。
"""

import csv
from pathlib import Path

NODES_CSV = Path("knowledge-sources/lemonhu/nodes.csv" if __name__ == "__main__"
                  else "../knowledge-sources/lemonhu/nodes.csv")


def esc(s: str) -> str:
    """转义 Rust 字符串中的特殊字符"""
    return s.replace('\\', '\\\\').replace('"', '\\"')


def main():
    nodes = []
    with open(NODES_CSV, newline="", encoding="utf-8") as f:
        for row in csv.DictReader(f):
            nodes.append(row)

    industries = sorted(
        [n for n in nodes if n["type"] == "industry"],
        key=lambda x: x["title"],
    )
    concepts = sorted(
        [n for n in nodes if n["type"] == "concept"],
        key=lambda x: x["title"],
    )

    lines: list[str] = []
    lines.append("//! 自动生成 — A 股本体现成知识种子")
    lines.append("//! 来源: lemonhu/stock-knowledge-graph (同花顺分类)")
    lines.append(f"//! {len(industries)} 行业, {len(concepts)} 概念")
    lines.append("")
    lines.append("use super::{ConceptIndex, ConceptNode};")
    lines.append("")
    lines.append("pub fn seed_ashare_ontology(idx: &mut ConceptIndex) {")
    lines.append("")

    # Industry
    lines.append("    // === 行业 ({}) ===".format(len(industries)))
    for nd in industries:
        name = esc(nd["title"])
        hid = esc(nd["id"])  # MD5 hash
        lines.append(f'    idx.register(ConceptNode::new("{hid}", "{name}", "industry").with_aliases(&["{name}"]));')

    lines.append("")
    lines.append("    // === 概念 ({}) ===".format(len(concepts)))
    for nd in concepts:
        name = esc(nd["title"])
        hid = esc(nd["id"])
        lines.append(f'    idx.register(ConceptNode::new("{hid}", "{name}", "concept").with_aliases(&["{name}"]));')

    lines.append("}")
    lines.append("")

    print("\n".join(lines))


if __name__ == "__main__":
    main()
