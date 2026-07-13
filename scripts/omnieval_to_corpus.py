#!/usr/bin/env python3
"""OmniEval-KnowledgeCorpus → AxInvest 知识库文档转换脚本。

将 HuggingFace 数据集 RUC-NLPIR/OmniEval-KnowledgeCorpus 的 JSON 记录
（维基中文金融网页 / BAAI-Fin / 金融挑战赛文档 / 合规金融网页）
转换为 AxInvest 知识库可导入的 **Markdown 文档集**（每片段一文档）。

经 KnowledgeHub 导入「金融知识库」后，对话页「启用知识库」开关 /
工作流 KnowledgeRetrievalNode 即可直接检索——
复用项目已有的 RAG 向量链路（axagent_search::rag + indexing），无需另写检索内核。

用法:
    python scripts/omnieval_to_corpus.py <omnieval_json_or_dir> [--out knowledge-sources/omnieval/docs]

输入格式兼容（按字段优先级探测）:
    {"title": "...", "content": "...", "source": "...", "url": "..."}   # 网页/文档典型
    {"text": "...", "title": "..."}                                      # 部分子集
    {"question": "...", "answer": "..."}                                 # QA 子集（拼成单文档）
"""
import argparse
import json
import re
import sys
from pathlib import Path


def extract_doc(rec: dict, idx: int) -> dict | None:
    """从单条 OmniEval 记录提取 (title, content, source, url)。"""
    title = rec.get("title") or rec.get("name") or rec.get("doc_title") or ""
    content = rec.get("content") or rec.get("text") or rec.get("body") or ""
    source = rec.get("source") or rec.get("dataset") or rec.get("subset") or "omnieval"
    url = rec.get("url") or rec.get("link") or rec.get("page_url") or ""

    # QA 型（AutoGen）：拼成单文档
    if not content and (rec.get("question") and rec.get("answer")):
        q, a = rec["question"], rec["answer"]
        title = title or (q[:40] + ("…" if len(q) > 40 else ""))
        content = f"问：{q}\n答：{a}"
        source = source or "omnieval_autogen"

    if not content or not content.strip():
        return None
    return {
        "id": rec.get("id") or f"omnieval-{idx:06d}",
        "title": title.strip() or f"doc-{idx}",
        "content": content.strip(),
        "source": str(source),
        "url": str(url),
    }


def slugify(s: str, max_len: int = 48) -> str:
    s = re.sub(r"[^\w一-鿿]+", "-", s).strip("-")
    return s[:max_len] or "doc"


def iter_records(path: Path):
    """遍历 JSON 文件或目录，逐条 yield 原始记录。"""
    if path.is_dir():
        files = sorted(path.rglob("*.json")) + sorted(path.rglob("*.jsonl"))
    else:
        files = [path]
    for f in files:
        text = f.read_text(encoding="utf-8")
        if str(f).endswith(".jsonl"):
            for line in text.splitlines():
                line = line.strip()
                if line:
                    yield json.loads(line)
        else:
            data = json.loads(text)
            if isinstance(data, dict) and "docs" in data:
                yield from data["docs"]
            elif isinstance(data, list):
                yield from data
            else:
                yield data


def main():
    ap = argparse.ArgumentParser(description="OmniEval → AxInvest 知识库文档")
    ap.add_argument("input", help="OmniEval JSON 文件或目录")
    ap.add_argument("--out", default="knowledge-sources/omnieval/docs",
                    help="输出文档目录（每片段一个 .md）")
    args = ap.parse_args()

    inp = Path(args.input)
    if not inp.exists():
        sys.exit(f"[omnieval_to_corpus] 输入不存在: {inp}")

    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)

    seen: set[str] = set()
    n = 0
    for i, rec in enumerate(iter_records(inp)):
        doc = extract_doc(rec, i)
        if not doc:
            continue
        base = slugify(doc["title"])
        name = base
        k = 1
        while name in seen:
            k += 1
            name = f"{base}-{k}"
        seen.add(name)

        front = [
            "---",
            f"title: {doc['title']}",
            f"source: {doc['source']}",
        ]
        if doc["url"]:
            front.append(f"url: {doc['url']}")
        front.append("---")
        (out / f"{name}.md").write_text(
            "\n".join(front) + "\n\n" + doc["content"] + "\n", encoding="utf-8"
        )
        n += 1
    print(f"[omnieval_to_corpus] 写出 {n} 个知识库文档 → {out}")


if __name__ == "__main__":
    main()
