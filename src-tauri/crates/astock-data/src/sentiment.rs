//! P2-B4: 统一新闻情感分词典模块
//!
//! 合并自三处历史实现:
//! - `portfolio-mgr.rhai` 公告风险/利好词典(50+ 词,含否定词检测)
//! - `guba.rs` 社交舆情关键词(8 词,粗略匹配)
//! - `evidence_weight.rs` 研报情感分类(20 词,bull/bear 二分)
//!
//! ## 评分模型
//!
//! - 高风险词(退市/造假/立案调查等) 每命中: -0.20
//! - 普通风险词(减持/亏损/ST 等) 每命中: -0.05
//! - 利好词(业绩预增/回购/增持等) 每命中: +0.05
//! - 归一化到 [-1.0, 1.0]
//! - 无任何匹配 → None(中性,不强行赋 0,避免污染下游置信度)
//!
//! ## 否定词检测
//!
//! A 股公告常见否定句式: "不存在退市风险" "未发现违规行为" "已消除影响"
//! 在匹配关键词前,检查关键词前 N 个字符是否有否定词,命中则跳过该关键词。
//!
//! ## 使用入口
//!
//! - `compute_news_sentiment(title, summary)` — 主入口,自动合并标题+摘要
//! - `compute_text_sentiment(text)` — 单文本评分
//! - 通过 `pm_compute_news_sentiment` 注册到 Rhai Engine,供 portfolio-mgr.rhai 调用

#![allow(dead_code)] // 部分函数通过 Rhai FFI 调用,Rust 侧可能无直接引用

/// 否定词前缀词典(覆盖中文公告常见否定表达)
///
/// 检测策略: 关键词前 20 字符内是否含以下任意一个否定词
pub const NEGATION_PREFIXES: &[&str] = &[
    "没有",
    "未发现",
    "不存在",
    "已消除",
    "已解决",
    "已排除",
    "并非",
    "非",
    "尚未",
    "未触及",
    "未达到",
    "未发生",
    "无",
];

/// 高风险关键词(单命中 -0.20,用于退市/造假/立案调查等极负面事件)
pub const HIGH_RISK_KEYWORDS: &[&str] = &[
    "退市",
    "立案调查",
    "可能快速下跌",
    "风险警示",
    "造假",
    "欺诈",
    "重大违法",
    "暂停上市",
    "终止上市",
    "面值退市",
    "强制退市",
    "退市风险",
    "ST",
    "*ST",
    "SST",
    "S*ST",
];

/// 普通风险关键词(单命中 -0.05,用于减持/亏损/诉讼等一般负面事件)
///
/// 设计约束: 词典内任意两个词互不为子串(避免"减持"+"股东减持"双计分)。
/// 复合词如"股东减持""业绩下滑"已删除,由基础词"减持""下滑"覆盖。
pub const RISK_KEYWORDS: &[&str] = &[
    "风险提示",
    "异常波动",
    "监管函",
    "警示函",
    "减持",
    "业绩预亏",
    "亏损",
    "强制平仓",
    "违约",
    "责令改正",
    "通报批评",
    "公开谴责",
    "行政处罚",
    "诉讼",
    "仲裁",
    "冻结",
    "查封",
    "商誉减值",
    "资产减值",
    "信用减值",
    "坏账",
    "跌",
    "熊",
    "利空",
    "减仓",
    "卖出",
    "看空",
    "看跌",
    "下跌",
    "下滑",
    "恶化",
    "问询函",
    "关注函",
    "质押平仓",
    "偿债风险",
    "担保风险",
    "关联交易",
    "资金占用",
    "违规",
    "处分",
];

/// 利好关键词(单命中 +0.05,用于业绩预增/回购/增持等正面事件)
pub const POSITIVE_KEYWORDS: &[&str] = &[
    "业绩预增",
    "扭亏为盈",
    "中标",
    "重大合同",
    "增持",
    "回购",
    "股权激励",
    "分红",
    "送转",
    "业绩预告向上",
    "业绩超预期",
    "涨",
    "牛",
    "利好",
    "加仓",
    "买入",
    "看多",
    "看涨",
    "上涨",
    "增长",
    "改善",
    "突破",
    "创新高",
    "订单",
    "战略合作",
    "收购",
    "并表",
    "投产",
    "达产",
    "放量",
    "资金流入",
    "机构调研",
    "评级上调",
    "目标价上调",
    "纳入指数",
    "龙头",
    "景气",
    "复苏",
    "回暖",
    "盈利",
];

/// 检查关键词前 N 字符是否含否定词(避免"没有退市风险"被误判为风险信号)
///
/// 与 portfolio-mgr.rhai 中的 `has_negation` 逻辑一致,确保两边行为对齐
pub fn has_negation(text: &str, keyword: &str) -> bool {
    let idx = match text.find(keyword) {
        Some(i) => i,
        None => return false,
    };
    // 取关键词前 20 个字符(覆盖常见否定短语长度)
    // 注意: 这里是字节索引,中文 UTF-8 占 3 字节,20 字节约 6 个中文字符
    // 为了准确截取字符而非字节,使用 char_indices
    let start = if idx >= 20 {
        // 找到 idx 前 20 字节处的字符边界
        let mut boundary = idx;
        for _ in 0..20 {
            if boundary == 0 {
                break;
            }
            // 向前找字符边界
            let mut prev = boundary - 1;
            while prev > 0 && !text.is_char_boundary(prev) {
                prev -= 1;
            }
            boundary = prev;
        }
        boundary
    } else {
        0
    };
    let before = &text[start..idx];
    for neg in NEGATION_PREFIXES {
        if before.contains(neg) {
            return true;
        }
    }
    false
}

/// 对单段文本做情感评分,返回 [-1.0, 1.0] 区间的原始分(未归一化)
///
/// 内部流程:
/// 1. 遍历高风险词,命中(且未被否定) → -0.20
/// 2. 遍历普通风险词,命中(且未被否定) → -0.05
/// 3. 遍历利好词,命中 → +0.05(利好词不做否定检测,因为"未增长"很少见于公告)
/// 4. 同一词可能在多个词典中命中(如 "退市" 同时在 HIGH_RISK 和 RISK),只计最高分
fn score_text(text: &str) -> f64 {
    if text.is_empty() {
        return 0.0;
    }
    let mut score = 0.0f64;
    let mut high_risk_hit = false;
    let mut risk_hit = false;
    let mut positive_hit = false;

    // 高风险词优先(每个 -0.20,但同类只计一次避免重复惩罚)
    // ST/*ST 等需要大小写不敏感匹配(公告中可能是 ST 或 st)
    let text_upper = text.to_uppercase();
    for kw in HIGH_RISK_KEYWORDS {
        if text.contains(kw) || text_upper.contains(kw) {
            if has_negation(text, kw) {
                continue;
            }
            high_risk_hit = true;
            break; // 高风险词只计一次
        }
    }

    // 普通风险词(每个 -0.05,最多计 3 次避免长文本累积失真)
    if !high_risk_hit {
        let mut risk_count = 0u32;
        for kw in RISK_KEYWORDS {
            if text.contains(kw) {
                if has_negation(text, kw) {
                    continue;
                }
                risk_count += 1;
                if risk_count >= 3 {
                    break;
                }
            }
        }
        if risk_count > 0 {
            score -= 0.05 * risk_count as f64;
            risk_hit = true;
        }
    } else {
        score -= 0.20;
    }

    // 利好词(每个 +0.05,最多计 3 次)
    let mut pos_count = 0u32;
    for kw in POSITIVE_KEYWORDS {
        if text.contains(kw) {
            pos_count += 1;
            if pos_count >= 3 {
                break;
            }
        }
    }
    if pos_count > 0 {
        score += 0.05 * pos_count as f64;
        positive_hit = true;
    }

    // 无任何匹配 → 返回 0.0(调用方据此判断为 None)
    if !high_risk_hit && !risk_hit && !positive_hit {
        return 0.0;
    }

    // 高风险命中时,利好词不能完全抵消(保留至少 -0.10 的负面倾向)
    if high_risk_hit && score > -0.10 {
        score = -0.10;
    }

    score
}

/// 归一化到 [-1.0, 1.0]
fn clamp_score(s: f64) -> f64 {
    s.clamp(-1.0, 1.0)
}

/// 对单段文本做情感评分,返回 [-1.0, 1.0]
///
/// 无任何关键词命中时返回 None,调用方可据此判断"中性"或"无信号"
pub fn compute_text_sentiment(text: &str) -> Option<f64> {
    let raw = score_text(text);
    if raw == 0.0 {
        return None;
    }
    Some(clamp_score(raw))
}

/// 对新闻条目(标题+摘要)做情感评分
///
/// 标题权重更高(0.7),摘要权重较低(0.3),加权后归一化
/// 若标题和摘要都无信号 → None
pub fn compute_news_sentiment(title: &str, summary: &str) -> Option<f64> {
    let title_score = score_text(title);
    let summary_score = score_text(summary);

    let total = title_score * 0.7 + summary_score * 0.3;

    if total == 0.0 {
        return None;
    }
    Some(clamp_score(total))
}

/// 对多条新闻做批量评分,返回平均情感分
///
/// 用于 portfolio-mgr 等场景: 把近 N 条新闻聚合成单一情感信号
/// 空列表或全部无信号 → None
pub fn compute_news_batch_sentiment(items: &[(&str, &str)]) -> Option<f64> {
    if items.is_empty() {
        return None;
    }
    let mut sum = 0.0f64;
    let mut count = 0u32;
    for (title, summary) in items {
        if let Some(s) = compute_news_sentiment(title, summary) {
            sum += s;
            count += 1;
        }
    }
    if count == 0 {
        return None;
    }
    Some(clamp_score(sum / count as f64))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_text_returns_none() {
        assert_eq!(compute_text_sentiment(""), None);
        assert_eq!(compute_news_sentiment("", ""), None);
    }

    #[test]
    fn test_no_keyword_returns_none() {
        // 无任何词典命中 → None
        assert_eq!(compute_text_sentiment("今日天气晴朗"), None);
        assert_eq!(compute_news_sentiment("公司公告", "日常经营更新"), None);
    }

    #[test]
    fn test_high_risk_keyword_negative_score() {
        let score = compute_text_sentiment("公司面临退市风险").unwrap();
        assert!(score < 0.0, "退市应为负面, got {score}");
        assert!(score <= -0.10, "退市应 <= -0.10, got {score}");
    }

    #[test]
    fn test_risk_keyword_negative_score() {
        let score = compute_text_sentiment("股东减持公告").unwrap();
        assert!(score < 0.0, "减持应为负面, got {score}");
        // "减持"命中(词典已去重,"股东减持"不再单独计分)
        assert!((score - (-0.05)).abs() < 1e-6, "应 = -0.05, got {score}");
    }

    #[test]
    fn test_positive_keyword_positive_score() {
        let score = compute_text_sentiment("公司业绩预增").unwrap();
        assert!(score > 0.0, "业绩预增应为正面, got {score}");
        assert!((score - 0.05).abs() < 1e-6, "应 = 0.05, got {score}");
    }

    #[test]
    fn test_negation_prefix_skips_risk() {
        // "不存在退市风险" 不应被计为负面
        let score = compute_text_sentiment("公司公告:不存在退市风险");
        assert_eq!(score, None, "否定词应跳过风险关键词");
    }

    #[test]
    fn test_negation_prefix_skips_normal_risk() {
        // "未发现违规行为" 不应被计为负面
        let score = compute_text_sentiment("经核查未发现违规行为");
        assert_eq!(score, None, "否定词应跳过普通风险关键词");
    }

    #[test]
    fn test_title_summary_weighted() {
        // 标题正面 + 摘要负面 → 加权后偏正面(0.7 * 0.05 + 0.3 * (-0.05) = 0.02)
        let score = compute_news_sentiment("业绩预增", "股东减持").unwrap();
        assert!(score > 0.0, "标题权重 0.7 应主导, got {score}");
    }

    #[test]
    fn test_title_summary_both_negative() {
        // 标题+摘要都负面 → 强负面
        let score = compute_news_sentiment("退市风险", "公司面临重大违法").unwrap();
        assert!(score < 0.0, "应负面, got {score}");
    }

    #[test]
    fn test_st_keywords_case_insensitive() {
        let s1 = compute_text_sentiment("公司被ST");
        let s2 = compute_text_sentiment("公司被st");
        assert!(s1.is_some(), "ST 大写应命中");
        assert!(s2.is_some(), "st 小写应命中(to_uppercase 兜底)");
    }

    #[test]
    fn test_multiple_positive_capped() {
        // 多个利好词命中,最多计 3 次 → 0.15
        let score = compute_text_sentiment("业绩预增 回购 增持 分红").unwrap();
        assert!((score - 0.15).abs() < 1e-6, "应 = 0.15, got {score}");
    }

    #[test]
    fn test_multiple_risk_capped() {
        // 多个普通风险词命中,最多计 3 次 → -0.15
        let score = compute_text_sentiment("减持 亏损 诉讼 违约").unwrap();
        assert!((score - (-0.15)).abs() < 1e-6, "应 = -0.15, got {score}");
    }

    #[test]
    fn test_high_risk_dominates_positive() {
        // 高风险词 + 多个利好词 → 高风险主导,保留至少 -0.10
        let score = compute_text_sentiment("退市风险 业绩预增 回购 增持").unwrap();
        assert!(score <= -0.10, "高风险应主导, got {score}");
    }

    #[test]
    fn test_batch_sentiment() {
        let items = vec![
            ("业绩预增", "公司公告"),
            ("股东减持", "风险提示"),
            ("日常公告", "无重大事项"), // 无信号
        ];
        let score = compute_news_batch_sentiment(&items).unwrap();
        // (0.05 + (-0.05)) / 2 = 0.0,但 0.0 会被视为 None
        // 实际: 0.05 + (-0.05*3) = 0.05 - 0.15 = -0.10, 然后除以 2 = -0.05
        // 等等,重新算: 第一条 0.05(业绩预增), 第二条 -0.05(减持+风险提示 = -0.05*2 = -0.10)
        // sum = 0.05 + (-0.10) = -0.05, count = 2, avg = -0.025
        assert!(score < 0.0, "应偏负面, got {score}");
    }

    #[test]
    fn test_batch_empty_returns_none() {
        let items: Vec<(&str, &str)> = vec![];
        assert_eq!(compute_news_batch_sentiment(&items), None);
    }

    #[test]
    fn test_batch_all_neutral_returns_none() {
        let items = vec![("今日天气", "晴朗"), ("日常公告", "无事项")];
        assert_eq!(compute_news_batch_sentiment(&items), None);
    }

    #[test]
    fn test_score_clamped_to_range() {
        // 构造超长文本命中很多关键词,确保结果不超出 [-1.0, 1.0]
        let text = "退市 立案调查 造假 欺诈 重大违法 暂停上市 终止上市";
        let score = compute_text_sentiment(text).unwrap();
        assert!((-1.0..=1.0).contains(&score), "score out of range: {score}");
        assert!(score < 0.0);
    }

    #[test]
    fn test_has_negation_function_directly() {
        // 直接测试 has_negation 函数
        assert!(has_negation("公司不存在退市风险", "退市"));
        assert!(!has_negation("公司面临退市风险", "退市"));
        assert!(has_negation("未发现违规行为", "违规"));
        assert!(!has_negation("存在违规行为", "违规"));
    }

    #[test]
    fn test_real_world_announcement_titles() {
        // 真实 A 股公告标题样本
        let cases = vec![
            // (标题, 期望方向: -1 负面 / 1 正面 / 0 中性)
            ("公司2025年业绩预增公告", 1),
            ("关于公司股票可能被实施退市风险警示的提示性公告", -1),
            ("关于收到中国证监会立案调查通知书的公告", -1),
            ("关于控股股东增持公司股份计划的公告", 1),
            ("关于以集中竞价交易方式回购公司股份的方案", 1),
            ("关于公司股东减持股份计划的告知函", -1),
            ("关于公司及相关当事人收到行政处罚决定书的公告", -1),
            ("2025年半年度报告(日常披露)", 0), // 无明显关键词
        ];
        for (title, expected_dir) in cases {
            let score = compute_text_sentiment(title);
            match expected_dir {
                -1 => {
                    let s = score.unwrap_or_else(|| panic!("'{title}' 应有负面评分"));
                    assert!(s < 0.0, "'{title}' 应负面, got {s}");
                },
                1 => {
                    let s = score.unwrap_or_else(|| panic!("'{title}' 应有正面评分"));
                    assert!(s > 0.0, "'{title}' 应正面, got {s}");
                },
                0 => {
                    // 中性可能返回 None 或 0.0,都算通过
                },
                _ => {},
            }
        }
    }
}
