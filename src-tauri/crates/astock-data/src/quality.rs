//! 数据质量门控模块。
//!
//! 在 phase_2 分析师报告生成完成后，对所有报告执行质量检查，
//! 生成数据质量摘要，注入到后续辩论/风控/决策阶段。

use std::collections::HashMap;

/// 数据质量等级
#[derive(Debug, Clone, PartialEq)]
pub enum QualityGrade {
    /// 全部通过
    A,
    /// 轻微问题
    B,
    /// 部分数据缺失
    C,
    /// 显著缺口
    D,
    /// 严重失败
    F,
}

/// 数据质量检查结果
#[derive(Debug, Clone)]
pub struct QualityCheck {
    pub grade: QualityGrade,
    pub summary: String,
    pub warnings: Vec<String>,
}

/// 检查单个分析师报告的质量
pub fn check_report_quality(
    _expert_id: &str,
    report_text: &str,
    required_items: &[Vec<&str>],
) -> QualityGrade {
    let text = report_text.to_lowercase();

    // 占位报告检测 (LLM 未连接时生成的假报告)
    let is_placeholder = text.contains("\"summary\":\"占位报告")
        || text.contains("agentrunner 未注入")
        || text.contains("placeholder");
    if is_placeholder {
        return QualityGrade::F;
    }

    // 硬检查 1.5: 纯重复检测（同一行连续重复 >= 3 次通常代表 LLM 输出循环/截断）
    let mut last_sentence = "";
    let mut repeat_count = 0;
    let mut max_repeat = 0;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed == last_sentence && !trimmed.is_empty() {
            repeat_count += 1;
            if repeat_count > max_repeat {
                max_repeat = repeat_count;
            }
        } else {
            repeat_count = 0;
        }
        last_sentence = trimmed;
    }
    if max_repeat >= 3 {
        return QualityGrade::F;
    }

    // 硬检查 1: 报告是否为空或过短（<50字符→F, <200字符→D）
    if report_text.trim().is_empty() {
        return QualityGrade::F;
    }
    if report_text.len() < 50 {
        return QualityGrade::F;
    }
    if report_text.len() < 200 {
        return QualityGrade::D;
    }

    // 硬检查 2: 是否包含失败标记
    let failure_markers = [
        "无法获取数据",
        "数据不足，无法",
        "无可用数据",
        "分析失败",
        "无法完成分析",
        "抱歉，我无法",
        "数据获取失败",
    ];
    let has_failure = failure_markers.iter().any(|m| text.contains(m));
    if has_failure {
        return QualityGrade::D;
    }

    // 硬检查 3: 必采清单覆盖率
    let covered = required_items
        .iter()
        .filter(|group| group.iter().any(|keyword| text.contains(&keyword.to_lowercase())))
        .count();
    let total = required_items.len();
    if total == 0 {
        return QualityGrade::B;
    }

    let ratio = covered as f64 / total as f64;

    // 实质分析启发式：除了"提到"关键词，至少要有数字/百分号/明确结论
    // 防止 LLM 用一句"趋势向上"刷满所有必采项
    // 阈值：≥3个数字字符，或包含 % / 看多/看空/建议/买卖持有/增减持
    let has_substance = report_text.chars().filter(|c| c.is_ascii_digit()).count() >= 3
        || report_text.contains('%')
        || report_text.contains("看多")
        || report_text.contains("看空")
        || report_text.contains("建议")
        || report_text.contains("买入")
        || report_text.contains("卖出")
        || report_text.contains("持有")
        || report_text.contains("增持")
        || report_text.contains("减持");

    // 覆盖率分级阈值: ≥0.8→A(有实质)或B(无实质), ≥0.6→B, ≥0.4→C, <0.4→D
    if ratio >= 0.8 {
        if has_substance {
            QualityGrade::A
        } else {
            QualityGrade::B
        }
    } else if ratio >= 0.6 {
        QualityGrade::B
    } else if ratio >= 0.4 {
        QualityGrade::C
    } else {
        QualityGrade::D
    }
}

/// 必采清单 — 每个分析师报告必须包含的关键数据项
pub fn get_required_items(expert_id: &str) -> Vec<Vec<&'static str>> {
    // P2-1 修复(2026-08-09): ID 归一化——DAG 节点 ID 带 "a-" 前缀（a-market-analyst），
    // 原匹配按无前缀角色名（market-analyst），LLM 经 run_quality_gate 传节点 ID 时
    // 全部落 `_` 分支 → total==0 → check_report_quality 无条件返回 B（放水）。
    // 现归一化：去 a- 前缀 + 别名映射（a-hot-money → hot-money-tracker 等）。
    let id = expert_id.strip_prefix("a-").unwrap_or(expert_id);
    let id = match id {
        "hot-money" => "hot-money-tracker",
        "sentiment" => "sentiment-analyst",
        "news" => "news-analyst",
        "fundamentals" => "fundamentals-analyst",
        "policy" => "policy-analyst",
        "lockup" => "lockup-watcher",
        "research" => "research-analyst",
        "sector" => "sector-analyst",
        "catalyst" => "catalyst-analyst",
        other => other,
    };
    match id {
        "market-analyst" => vec![
            vec!["趋势", "走势", "方向"],
            vec!["形态", "图形", "模式"],
            vec!["指标", "MACD", "RSI", "KDJ"],
            vec!["支撑", "支撑位", "底部"],
            vec!["压力", "压力位", "阻力", "阻力位"],
        ],
        "sentiment-analyst" => vec![
            vec!["情绪", "市场情绪", "人气"],
            vec!["乐观", "看多", "积极"],
            vec!["悲观", "看空", "消极"],
            vec!["舆情", "舆论", "社交媒体"],
            vec!["散户", "个人投资者", "零售"],
        ],
        "news-analyst" => vec![
            vec!["公告", "披露", "通告"],
            vec!["新闻", "资讯", "消息"],
            vec!["行业", "产业", "板块"],
            vec!["宏观", "经济", "GDP"],
            vec!["影响", "冲击", "效应"],
        ],
        "fundamentals-analyst" => vec![
            vec!["盈利", "利润", "收益"],
            vec!["营收", "收入", "营业额"],
            vec!["ROE", "净资产收益率"],
            vec!["PE", "市盈率", "估值"],
            vec!["估值", "价值", "定价"],
        ],
        "policy-analyst" => vec![
            vec!["政策", "方针", "规划"],
            vec!["监管", "合规", "审查"],
            vec!["产业", "行业政策"],
            vec!["补贴", "扶持", "优惠"],
            vec!["窗口指导", "约谈", "警示"],
        ],
        "hot-money-tracker" => vec![
            vec!["资金", "成交", "流入"],
            vec!["龙虎榜", "席位", "营业部"],
            vec!["主力", "机构", "大单"],
            vec!["北向", "外资", "沪港通", "深港通"],
            vec!["流入", "净流入", "增仓"],
            vec!["流出", "净流出", "减仓"],
        ],
        "lockup-watcher" => vec![
            vec!["解禁", "限售股", "锁定期"],
            vec!["减持", "套现", "抛售"],
            vec!["质押", "抵押", "担保"],
            vec!["增持", "回购", "护盘"],
            vec!["限售", "锁股", "禁售"],
        ],
        // P1-2 修复(2026-08-09): 原 _ 分支导致这 3 个分析师必采清单为空 → total==0
        // → check_report_quality 直接返回 B（无条件放水）。补全后与 DAG 10 分析师对齐。
        "research-analyst" => vec![
            vec!["研报", "券商", "评级"],
            vec!["目标价", "盈利预测", "EPS"],
            vec!["买入", "增持", "推荐"],
            vec!["评级", "维持", "上调", "下调"],
            vec!["覆盖", "跟踪", "机构"],
        ],
        "sector-analyst" => vec![
            vec!["行业", "景气度", "周期"],
            vec!["板块", "轮动", "涨幅"],
            vec!["估值", "PE", "PB"],
            vec!["龙头", "领涨", "领跌"],
            vec!["政策", "需求", "供给"],
        ],
        "catalyst-analyst" => vec![
            vec!["催化剂", "事件", "驱动"],
            vec!["公告", "业绩预告", "年报"],
            vec!["利好", "利空", "风险"],
            vec!["时间窗口", "临近", "预期"],
            vec!["叙事", "题材", "主题"],
        ],
        _ => vec![],
    }
}

/// 对所有分析师报告执行质量检查，生成数据质量摘要
pub fn run_quality_gate(reports: &HashMap<String, String>) -> QualityCheck {
    let mut grades: Vec<(String, QualityGrade)> = Vec::new();
    let mut warnings = Vec::new();

    // 排除非分析师角色的 ID（辩论员、经理、交易员等）
    let non_analyst_keywords = ["debator", "researcher", "manager", "trader"];

    for (id, report) in reports.iter() {
        if non_analyst_keywords.iter().any(|kw| id.contains(kw)) {
            continue;
        }
        let required = get_required_items(id);
        let grade = check_report_quality(id, report, &required);

        if grade == QualityGrade::F || grade == QualityGrade::D {
            warnings.push(format!(
                "⚠️ {} 报告质量{}，已标记为低置信度",
                id,
                match grade {
                    QualityGrade::D => "D",
                    _ => "F",
                }
            ));
        }
        grades.push((id.clone(), grade));
    }

    let total_count = grades.len();
    let fail_count =
        grades.iter().filter(|(_, g)| *g == QualityGrade::F || *g == QualityGrade::D).count();

    // 失败率分级阈值: 0%→A, ≤20%且≤1个→B, ≤50%→C, ≤80%→D, >80%→F
    let overall = if total_count == 0 {
        // P3-3 修复(2026-08-09): 空报告集=没有任何分析师输出，语义上应最差（F），
        // 原返回 C（中间值）会把"无数据"误判为"基本可用"。
        QualityGrade::F
    } else if fail_count == 0 {
        QualityGrade::A
    } else if fail_count <= (total_count as f64 * 0.2).ceil() as usize && fail_count <= 1 {
        QualityGrade::B
    } else if fail_count <= (total_count as f64 * 0.5).ceil() as usize {
        QualityGrade::C
    } else if fail_count <= (total_count as f64 * 0.8).ceil() as usize {
        QualityGrade::D
    } else {
        QualityGrade::F
    };

    let grade_str = match overall {
        QualityGrade::A => "A",
        QualityGrade::B => "B",
        QualityGrade::C => "C",
        QualityGrade::D => "D",
        QualityGrade::F => "F",
    };

    let summary = format!(
        "数据质量: {}级 | {} | {}",
        grade_str,
        if total_count == 0 {
            "无任何分析师报告".to_string()
        } else {
            format!("{}位分析师中{}位报告存在质量问题", total_count, fail_count)
        },
        if warnings.is_empty() {
            "所有报告通过质量检查".to_string()
        } else {
            warnings.join("; ")
        }
    );

    QualityCheck { grade: overall, summary, warnings }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_empty_report_gets_f() {
        let grade = check_report_quality("market-analyst", "", &[vec!["趋势"]]);
        assert_eq!(grade, QualityGrade::F);
    }

    #[test]
    fn test_short_report_gets_d() {
        let grade = check_report_quality("market-analyst", "短", &[vec!["趋势"]]);
        assert_eq!(grade, QualityGrade::F);
    }

    #[test]
    fn test_failure_marker_gets_d() {
        let report = "无法获取数据，分析失败。".repeat(10);
        let grade = check_report_quality("market-analyst", &report, &[vec!["趋势"]]);
        assert_eq!(grade, QualityGrade::D);
    }

    #[test]
    fn test_full_coverage_gets_a() {
        let report =
            "趋势向上，形态良好，指标多头，支撑强劲，压力位突破。上涨3.5%建议买入。".repeat(10);
        let grade = check_report_quality(
            "market-analyst",
            &report,
            &[vec!["趋势"], vec!["形态"], vec!["指标"], vec!["支撑"], vec!["压力"]],
        );
        assert_eq!(grade, QualityGrade::A);
    }

    #[test]
    fn test_partial_coverage_gets_b_or_c() {
        let report = "趋势向上，形态良好。".repeat(20);
        let grade = check_report_quality(
            "market-analyst",
            &report,
            &[vec!["趋势"], vec!["形态"], vec!["指标"], vec!["支撑"], vec!["压力"]],
        );
        assert!(grade == QualityGrade::C || grade == QualityGrade::B);
    }

    #[test]
    fn test_run_quality_gate_with_mixed_reports() {
        let mut reports = HashMap::new();
        reports.insert(
            "market-analyst".to_string(),
            "趋势向上，形态良好，MACD金叉，支撑位有效，压力位需观察。".repeat(10),
        );
        reports.insert(
            "sentiment-analyst".to_string(),
            "".to_string(), // Empty = F
        );
        reports.insert(
            "news-analyst".to_string(),
            "短".to_string(), // Short = D
        );
        reports.insert(
            "fundamentals-analyst".to_string(),
            "营收增长，ROE稳健，PE合理，估值偏低。".repeat(10),
        );
        reports.insert("policy-analyst".to_string(), "政策利好，产业扶持，监管放松。".repeat(10));
        reports.insert(
            "hot-money-tracker".to_string(),
            "主力流入，龙虎榜机构买入，北向加仓。".repeat(10),
        );
        reports.insert(
            "lockup-watcher".to_string(),
            "近期无解禁，大股东增持，质押比例低。".repeat(10),
        );

        let result = run_quality_gate(&reports);
        // 2 failures out of 7 = grade C
        assert_eq!(result.grade, QualityGrade::C);
        assert_eq!(result.warnings.len(), 2);
    }

    // P1-2 修复(2026-08-09): 新增 3 个分析师的必采清单非空 + 空报告不再被放水 B
    #[test]
    fn test_new_analysts_have_required_items() {
        assert!(!get_required_items("research-analyst").is_empty());
        assert!(!get_required_items("sector-analyst").is_empty());
        assert!(!get_required_items("catalyst-analyst").is_empty());
    }

    #[test]
    fn test_research_analyst_empty_report_gets_f() {
        let required = get_required_items("research-analyst");
        // 原 _ 分支: total==0 → 无条件返回 B（放水）；补清单后空报告应为 F
        let grade = check_report_quality("research-analyst", "", &required);
        assert_eq!(grade, QualityGrade::F);
    }

    #[test]
    fn test_sector_analyst_failure_marker_gets_d() {
        let required = get_required_items("sector-analyst");
        let report = "数据获取失败，无法完成分析。".repeat(10);
        let grade = check_report_quality("sector-analyst", &report, &required);
        assert_eq!(grade, QualityGrade::D);
    }

    // P2-1 修复(2026-08-09): ID 归一化——DAG 节点 ID（a- 前缀）也能匹配到必采清单
    #[test]
    fn test_dag_node_id_maps_to_required_items() {
        // a-market-analyst → market-analyst
        assert_eq!(get_required_items("a-market-analyst"), get_required_items("market-analyst"));
        // a-hot-money → hot-money-tracker（别名映射）
        assert!(!get_required_items("a-hot-money").is_empty());
        assert_eq!(get_required_items("a-hot-money"), get_required_items("hot-money-tracker"));
        // a-research → research-analyst
        assert_eq!(get_required_items("a-research"), get_required_items("research-analyst"));
        // 无前缀角色名不受影响
        assert_eq!(
            get_required_items("fundamentals-analyst"),
            get_required_items("a-fundamentals")
        );
    }

    // P3-3 修复(2026-08-09): 空报告集返回 F（原 C）
    #[test]
    fn test_empty_report_set_gets_f() {
        let result = run_quality_gate(&HashMap::new());
        assert_eq!(result.grade, QualityGrade::F);
        assert!(result.summary.contains("无任何分析师报告"));
    }
}
