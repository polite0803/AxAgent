#!/usr/bin/env python3
"""lemonhu/stock-knowledge-graph Neo4j CSV → nodes.csv + edges.csv 标准格式"""

import csv
from pathlib import Path

RAW = Path("knowledge-sources/lemonhu/raw")
OUT = Path("knowledge-sources/lemonhu")


def load_neo4j_csv(path: Path) -> list[dict]:
    """读 Neo4j 格式 CSV（冒号开头的列名）"""
    rows = []
    with open(path, newline="", encoding="utf-8") as f:
        for row in csv.DictReader(f):
            rows.append(row)
    return rows


def main():
    # --- 加载实体 ---
    stocks_raw = load_neo4j_csv(RAW / "stock.csv")
    industries_raw = load_neo4j_csv(RAW / "industry.csv")
    concepts_raw = load_neo4j_csv(RAW / "concept.csv")
    executives_raw = load_neo4j_csv(RAW / "executive.csv")

    # --- 加载关系 ---
    stock_industry_edges = load_neo4j_csv(RAW / "stock_industry.csv")
    stock_concept_edges = load_neo4j_csv(RAW / "stock_concept.csv")
    executive_stock_edges = load_neo4j_csv(RAW / "executive_stock.csv")

    # --- 构建 nodes.csv ---
    # 列: id,title,type,tags
    nodes = []

    # Stock
    for s in stocks_raw:
        code = s["stock_id:ID(Stock)"]
        name = s["name"]
        tags = "ST" if name.startswith(("ST", "*ST", "S*ST", "SST")) else ""
        nodes.append({"id": code, "title": name, "type": "company", "tags": tags})

    # Industry
    for ind in industries_raw:
        iid = ind["industry_id:ID(Industry)"]
        name = ind["name"]
        nodes.append({"id": iid, "title": name, "type": "industry", "tags": ""})

    # Concept
    for c in concepts_raw:
        cid = c["concept_id:ID(Concept)"]
        name = c["name"]
        nodes.append({"id": cid, "title": name, "type": "concept", "tags": ""})

    # Executive
    for p in executives_raw:
        pid = p["person_id:ID(Executive)"]
        name = p["name"]
        gender = p.get("gender", "")
        age = p.get("age:int", "")
        tags = f"性别:{gender};年龄:{age}" if gender or age else ""
        nodes.append({"id": pid, "title": name, "type": "person", "tags": tags})

    # --- 构建 edges.csv ---
    # 列: source,target,type
    edges = []

    for e in stock_industry_edges:
        edges.append({
            "source": e[":START_ID(Stock)"],
            "target": e[":END_ID(Industry)"],
            "type": "in_industry",
        })

    for e in stock_concept_edges:
        edges.append({
            "source": e[":START_ID(Stock)"],
            "target": e[":END_ID(Concept)"],
            "type": "has_concept",
        })

    for e in executive_stock_edges:
        # jobs 字段包含职务名（如"董事长/董事"）
        job = e.get("jobs", "employ_of").strip()
        # 标准化: 如果jobs是多个/分隔的职务，取第一个最显著的
        primary_job = job.split("/")[0].strip() if "/" in job else job
        edges.append({
            "source": e[":START_ID(Executive)"],
            "target": e[":END_ID(Stock)"],
            "type": primary_job,
        })

    # --- 写文件 ---
    OUT.mkdir(exist_ok=True)

    with open(OUT / "nodes.csv", "w", newline="", encoding="utf-8") as f:
        w = csv.DictWriter(f, fieldnames=["id", "title", "type", "tags"])
        w.writeheader()
        w.writerows(nodes)

    with open(OUT / "edges.csv", "w", newline="", encoding="utf-8") as f:
        w = csv.DictWriter(f, fieldnames=["source", "target", "type"])
        w.writeheader()
        w.writerows(edges)

    print(f"nodes: {len(nodes)} (stock={len(stocks_raw)}, industry={len(industries_raw)}, "
          f"concept={len(concepts_raw)}, person={len(executives_raw)})")
    print(f"edges: {len(edges)} (industry={len(stock_industry_edges)}, "
          f"concept={len(stock_concept_edges)}, executive={len(executive_stock_edges)})")


if __name__ == "__main__":
    main()
