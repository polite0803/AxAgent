#!/usr/bin/env python3
"""合并东方财富行业补漏数据进 lemonhu 知识图谱。

东方财富与同花顺行业名不一致，用工况映射表对齐。
缺口可合并进类似同花顺分类（如 医药生物→生物制药）。
"""

import csv
from hashlib import md5
from pathlib import Path

RAW = Path("knowledge-sources/lemonhu/raw")

# 东方财富→同花顺 行业名映射（人工对齐）
EM2THS = {
    "交通运输": "交通运输",
    "传媒": "传媒娱乐",
    "公用事业": "供水供气",
    "农林牧渔": "农林牧渔",
    "医药生物": "生物制药",
    "商贸零售": "商业百货",
    "国防军工": "飞机制造",
    "基础化工": "化工行业",
    "家用电器": "家电行业",
    "建筑材料": "建筑建材",
    "建筑装饰": "建筑建材",
    "有色金属": "有色金属",
    "机械设备": "机械行业",
    "汽车": "汽车制造",
    "环保": "环保行业",
    "电力设备": "发电设备",
    "电子": "电子器件",
    "石油石化": "石油行业",
    "社会服务": "其它行业",
    "纺织服饰": "纺织行业",
    "综合": "综合行业",
    "计算机": "电子信息",
    "轻工制造": "机械行业",
    "通信": "电子信息",
    "钢铁": "钢铁行业",
    "银行": "金融行业",
    "非银金融": "金融行业",
    "食品饮料": "食品行业",
}


def load_industry_map() -> dict[str, str]:
    """返回 {同花顺行业名: md5_hash}"""
    m = {}
    with open(RAW / "industry.csv", newline="", encoding="utf-8") as f:
        for row in csv.DictReader(f):
            m[row["name"].strip()] = row["industry_id:ID(Industry)"].strip()
    return m


def main():
    industry_map = load_industry_map()
    print(f"已有行业: {len(industry_map)}")
    print()

    # 读已填
    filled: list[tuple[str, str]] = []
    with open(RAW / "missing_industry_filled.csv", newline="", encoding="utf-8") as f:
        for row in csv.DictReader(f):
            filled.append((row["stock_code"].strip(), row["industry"].strip()))
    print(f"已填股票: {len(filled)}")

    # 统计新加的行业（未映射到的）
    mapped = 0
    unmapped_industries: set[str] = set()
    new_edges: list[tuple[str, str, str]] = []  # (stock_code, industry_hash, relation_type)

    for code, em_ind in filled:
        ths_name = EM2THS.get(em_ind)
        if ths_name and ths_name in industry_map:
            hash_id = industry_map[ths_name]
            new_edges.append((code, hash_id, "industry_of"))
            mapped += 1
        else:
            unmapped_industries.add(em_ind)

    # 报未映射的行业
    if unmapped_industries:
        print(f"\n⚠️ 未映射行业 ({len(unmapped_industries)}): {', '.join(sorted(unmapped_industries))}")

    print(f"\n可映射新增边: {mapped}/{len(filled)}")
    if not new_edges:
        print("无新增边，退出")
        return

    # 追加到 stock_industry.csv
    existing_lines = []
    with open(RAW / "stock_industry.csv", newline="", encoding="utf-8") as f:
        for row in csv.DictReader(f):
            existing_lines.append(row)

    existing_set = {(r[":START_ID(Stock)"], r[":END_ID(Industry)"]) for r in existing_lines}
    to_add = [(c, h, t) for c, h, t in new_edges if (c, h) not in existing_set]

    print(f"去重后新增: {len(to_add)} 条边")

    if to_add:
        with open(RAW / "stock_industry.csv", "a", newline="", encoding="utf-8") as f:
            w = csv.writer(f)
            for code, hid, rtype in to_add:
                w.writerow([code, hid, rtype])

    print(f"\nstock_industry.csv 新增 {len(to_add)} 行")
    print("成功。下一步需重新运行 kg_to_linkgraph.py 刷新 link_graph + wiki_pages")


if __name__ == "__main__":
    main()
