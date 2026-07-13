#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only
"""知识图谱 CSV → AxInvest Wiki / LinkGraph 转换脚本

把"节点 + 关系"形式的开源知识库（lemonhu / shinezai / 自建）转换成
AxInvest 能直接消费的两类产物：

  1. link_graph.json  —— 对齐 `axagent_harness::graph_dtos::GraphData`
     { nodes:[GraphNode], edges:[GraphEdge] }，可直接喂给
     LinkGraph::from_graph_data → detect_communities → GraphInsightAnalyzer
     （即「知识库 → Wiki 冷启动 + 图洞察」链路，见 agent/src/graph_insights.rs）

  2. wiki_pages/<id>.md —— 每个实体一页 Markdown，frontmatter 含
     type/title/author/sources/created_at，正文用 [[wikilinks]] 互链邻居。
     走 ingest_pipeline 的 RawMarkdown 源即可被解析成 Wiki 页面与图边，
     无需 LLM 抽取（LLM 抽取只是增强，种子图用静态 wikilink 即可）。

用法:
    python scripts/kg_to_linkgraph.py knowledge-sources/sample
    python scripts/kg_to_linkgraph.py <含 nodes.csv + edges.csv 的目录>

输入约定:
    nodes.csv : id,title,type,tags?        (type ∈ company|industry|concept|person|...)
    edges.csv : source,target,type         (type 为关系名, 如 in_industry/has_concept)
"""
from __future__ import annotations

import csv
import json
import sys
import time
from pathlib import Path


def load_nodes(nodes_path: Path) -> list[dict]:
    nodes: list[dict] = []
    with open(nodes_path, newline="", encoding="utf-8") as f:
        for row in csv.DictReader(f):
            nodes.append(
                {
                    "id": row["id"].strip(),
                    "title": row["title"].strip(),
                    "node_type": row["type"].strip(),
                    "tags": [t.strip() for t in row.get("tags", "").split(";") if t.strip()],
                    "link_count": 0,
                    "backlink_count": 0,
                    "path": "",
                }
            )
    return nodes


def load_edges(edges_path: Path) -> list[dict]:
    edges: list[dict] = []
    with open(edges_path, newline="", encoding="utf-8") as f:
        for row in csv.DictReader(f):
            edges.append(
                {
                    "source": row["source"].strip(),
                    "target": row["target"].strip(),
                    "edge_type": row["type"].strip(),
                }
            )
    return edges


def build(nodes: list[dict], edges: list[dict]) -> dict:
    """构造对齐 harness GraphData 的 JSON。

    注意：harness 的 GraphNode/GraphEdge 用 #[serde(rename = "type")]
    所以 wire 键是 `type`（而非 node_type/edge_type），此处必须对齐。
    """
    out_deg: dict[str, int] = {}
    in_deg: dict[str, int] = {}
    for e in edges:
        out_deg[e["source"]] = out_deg.get(e["source"], 0) + 1
        in_deg[e["target"]] = in_deg.get(e["target"], 0) + 1
    out_nodes = []
    for n in nodes:
        out_nodes.append(
            {
                "id": n["id"],
                "title": n["title"],
                "type": n["node_type"],
                "tags": n["tags"],
                "link_count": out_deg.get(n["id"], 0),
                "backlink_count": in_deg.get(n["id"], 0),
                "path": f"wiki_pages/{n['id']}.md",
            }
        )
    out_edges = [
        {"source": e["source"], "target": e["target"], "type": e["edge_type"]}
        for e in edges
    ]
    return {"nodes": out_nodes, "edges": out_edges}


def write_wiki_pages(out_dir: Path, nodes: list[dict], edges: list[dict]) -> None:
    titles = {n["id"]: n["title"] for n in nodes}
    neighbors: dict[str, list[tuple[str, str]]] = {}
    for e in edges:
        neighbors.setdefault(e["source"], []).append((e["target"], e["edge_type"]))
        neighbors.setdefault(e["target"], []).append((e["source"], e["edge_type"]))

    wp_dir = out_dir / "wiki_pages"
    wp_dir.mkdir(exist_ok=True)
    for n in nodes:
        links = neighbors.get(n["id"], [])
        wikilinks = "\n".join(
            f"- [[{titles.get(t, t)}]] ({rel})" for t, rel in links
        )
        content = (
            "---\n"
            f"type: {n['node_type']}\n"
            f"title: {n['title']}\n"
            "author: kg-import\n"
            "sources: [knowledge-graph]\n"
            f"created_at: {int(time.time())}\n"
            "---\n\n"
            f"# {n['title']}\n\n"
            f"> 自动从知识图谱导入的 {n['node_type']} 节点。\n\n"
            "## 关联\n"
            f"{wikilinks}\n"
        )
        (wp_dir / f"{n['id']}.md").write_text(content, encoding="utf-8")


def main() -> int:
    if len(sys.argv) < 2:
        print("usage: kg_to_linkgraph.py <dir-containing-nodes.csv-and-edges.csv>")
        return 1
    d = Path(sys.argv[1])
    if not d.exists() or not d.is_dir():
        print(f"not found: {d}")
        return 1

    nodes = load_nodes(d / "nodes.csv")
    edges = load_edges(d / "edges.csv")
    data = build(nodes, edges)

    (d / "link_graph.json").write_text(
        json.dumps(data, ensure_ascii=False, indent=2), encoding="utf-8"
    )
    write_wiki_pages(d, nodes, edges)

    print(
        f"OK: {len(nodes)} nodes, {len(edges)} edges -> "
        f"{d / 'link_graph.json'}, {d / 'wiki_pages'}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
