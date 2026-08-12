// SPDX-License-Identifier: AGPL-3.0-only

//! 市场营销（marketing）领域工作流种子化 — 10 个工作流
//!
//! 生成的工作流：
//! - wf-mkt-ab-test: A/B测试
//! - wf-mkt-analytics: 营销数据分析
//! - wf-mkt-brand-guide: 品牌指南
//! - wf-mkt-competitive-intel: 竞争情报
//! - wf-mkt-email-campaign: 邮件营销活动
//! - wf-mkt-influencer: 红人营销
//! - wf-mkt-pr-plan: 公关传播计划
//! - wf-mkt-seo-audit: SEO审计
//! - wf-mkt-social-plan: 社交媒体策略
//! - wf-mkt-webinar: 线上研讨会

use super::seed_domain_helpers::*;
use sea_orm::DatabaseConnection;

const PROFILE: &str = "opc-cmo-cmo-content-strategist";

/// 种子化市场营销领域的全部工作流
pub(crate) async fn seed_domain_marketing_workflows(
    db: &DatabaseConnection,
) -> Result<usize, String> {
    let mut seeded = 0usize;

    // wf-mkt-ab-test: A/B测试
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-mkt-ab-test",
            "A/B测试",
            "确定假设、变量、样本量 → 配置实验并启动流量分配 → 统计分析结果、得出结论",
            "🧪",
            vec!["opc".to_string(), "marketing".to_string()],
            PROFILE,
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-ab-design",
                    "实验设计",
                    "确定假设、变量、样本量",
                    vec![],
                    Some(PROFILE),
                    "a-ab-design_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-ab-execute",
                    "实验执行",
                    "配置实验并启动流量分配",
                    vec![],
                    Some(PROFILE),
                    "a-ab-execute_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-ab-analyze",
                    "结果分析",
                    "统计分析结果、得出结论",
                    vec![],
                    Some(PROFILE),
                    "a-ab-analyze_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-ab-design", "trigger", "a-ab-design"),
                edge("e-a-ab-design-a-ab-execute", "a-ab-design", "a-ab-execute"),
                edge("e-a-ab-execute-a-ab-analyze", "a-ab-execute", "a-ab-analyze"),
                edge("e-a-ab-analyze-end", "a-ab-analyze", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // wf-mkt-analytics: 营销数据分析
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-mkt-analytics",
            "营销数据分析",
            "采集各渠道营销数据 → 构建营销数据仪表盘 → 提取关键洞察和改进建议",
            "📈",
            vec!["opc".to_string(), "marketing".to_string()],
            PROFILE,
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-mkt-data",
                    "数据采集",
                    "采集各渠道营销数据",
                    vec![],
                    Some(PROFILE),
                    "a-mkt-data_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-mkt-dashboard",
                    "仪表盘构建",
                    "构建营销数据仪表盘",
                    vec![],
                    Some(PROFILE),
                    "a-mkt-dashboard_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-mkt-insight",
                    "洞察提取",
                    "提取关键洞察和改进建议",
                    vec![],
                    Some(PROFILE),
                    "a-mkt-insight_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-mkt-data", "trigger", "a-mkt-data"),
                edge("e-a-mkt-data-a-mkt-dashboard", "a-mkt-data", "a-mkt-dashboard"),
                edge("e-a-mkt-dashboard-a-mkt-insight", "a-mkt-dashboard", "a-mkt-insight"),
                edge("e-a-mkt-insight-end", "a-mkt-insight", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // wf-mkt-brand-guide: 品牌指南
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-mkt-brand-guide",
            "品牌指南",
            "审计现有品牌资产和一致性 → 定义品牌声音、语调、关键词 → 输出品牌指南文档",
            "🎨",
            vec!["opc".to_string(), "marketing".to_string()],
            PROFILE,
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-brand-audit",
                    "品牌审计",
                    "审计现有品牌资产和一致性",
                    vec![],
                    Some(PROFILE),
                    "a-brand-audit_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-brand-voice",
                    "品牌声音定义",
                    "定义品牌声音、语调、关键词",
                    vec![],
                    Some(PROFILE),
                    "a-brand-voice_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-brand-guide",
                    "指南输出",
                    "输出品牌指南文档",
                    vec![],
                    Some(PROFILE),
                    "a-brand-guide_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-brand-audit", "trigger", "a-brand-audit"),
                edge("e-a-brand-audit-a-brand-voice", "a-brand-audit", "a-brand-voice"),
                edge("e-a-brand-voice-a-brand-guide", "a-brand-voice", "a-brand-guide"),
                edge("e-a-brand-guide-end", "a-brand-guide", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // wf-mkt-competitive-intel: 竞争情报
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-mkt-competitive-intel",
            "竞争情报",
            "识别核心竞争对手和跟踪维度 → 收集竞品产品更新、定价变化 → 生成竞争情报周报",
            "🕵️",
            vec!["opc".to_string(), "marketing".to_string()],
            PROFILE,
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-comp-map",
                    "竞品识别",
                    "识别核心竞争对手和跟踪维度",
                    vec![],
                    Some(PROFILE),
                    "a-comp-map_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-comp-monitor",
                    "情报收集",
                    "收集竞品产品更新、定价变化",
                    vec![],
                    Some(PROFILE),
                    "a-comp-monitor_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-comp-report",
                    "报告生成",
                    "生成竞争情报周报",
                    vec![],
                    Some(PROFILE),
                    "a-comp-report_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-comp-map", "trigger", "a-comp-map"),
                edge("e-a-comp-map-a-comp-monitor", "a-comp-map", "a-comp-monitor"),
                edge("e-a-comp-monitor-a-comp-report", "a-comp-monitor", "a-comp-report"),
                edge("e-a-comp-report-end", "a-comp-report", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // wf-mkt-email-campaign: 邮件营销活动
    if seed_domain_template(db, build_domain_template(
        "wf-mkt-email-campaign", "邮件营销活动", "确定目标受众、主题、内容策略 → 撰写邮件文案、设计排版、CTA → 分析打开率、点击率、转化率", "📧",
        vec!["opc".to_string(), "marketing".to_string()],
        PROFILE,
        vec![
            make_trigger(250.0, 0.0),
            make_agent_node("a-email-plan", "策划", "确定目标受众、主题、内容策略", vec![], Some(PROFILE), "a-email-plan_result", 250.0, 150.0),
            make_agent_node("a-email-create", "创作", "撰写邮件文案、设计排版、CTA", vec![], Some(PROFILE), "a-email-create_result", 250.0, 350.0),
            make_agent_node("a-email-analyze", "分析", "分析打开率、点击率、转化率", vec![], Some(PROFILE), "a-email-analyze_result", 250.0, 550.0),
            make_end(250.0, 750.0),
        ],
        vec![
            edge("e-trigger-a-email-plan", "trigger", "a-email-plan"),
            edge("e-a-email-plan-a-email-create", "a-email-plan", "a-email-create"),
            edge("e-a-email-create-a-email-analyze", "a-email-create", "a-email-analyze"),
            edge("e-a-email-analyze-end", "a-email-analyze", "end"),
        ],
    )).await? {
        seeded += 1;
    }

    // wf-mkt-influencer: 红人营销
    if seed_domain_template(db, build_domain_template(
        "wf-mkt-influencer", "红人营销", "搜索行业相关KOL和内容创作者 → 评估粉丝质量、互动率、匹配度 → 制定触达方案并发送合作邀请", "🤳",
        vec!["opc".to_string(), "marketing".to_string()],
        PROFILE,
        vec![
            make_trigger(250.0, 0.0),
            make_agent_node("a-inf-search", "KOL搜索", "搜索行业相关KOL和内容创作者", vec![], Some(PROFILE), "a-inf-search_result", 250.0, 150.0),
            make_agent_node("a-inf-evaluate", "KOL评估", "评估粉丝质量、互动率、匹配度", vec![], Some(PROFILE), "a-inf-evaluate_result", 250.0, 350.0),
            make_agent_node("a-inf-outreach", "触达合作", "制定触达方案并发送合作邀请", vec![], Some(PROFILE), "a-inf-outreach_result", 250.0, 550.0),
            make_end(250.0, 750.0),
        ],
        vec![
            edge("e-trigger-a-inf-search", "trigger", "a-inf-search"),
            edge("e-a-inf-search-a-inf-evaluate", "a-inf-search", "a-inf-evaluate"),
            edge("e-a-inf-evaluate-a-inf-outreach", "a-inf-evaluate", "a-inf-outreach"),
            edge("e-a-inf-outreach-end", "a-inf-outreach", "end"),
        ],
    )).await? {
        seeded += 1;
    }

    // wf-mkt-pr-plan: 公关传播计划
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-mkt-pr-plan",
            "公关传播计划",
            "挖掘有新闻价值的故事角度 → 撰写新闻稿和媒体资料包 → 确定媒体名单并分发稿件",
            "📰",
            vec!["opc".to_string(), "marketing".to_string()],
            PROFILE,
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-pr-story",
                    "故事挖掘",
                    "挖掘有新闻价值的故事角度",
                    vec![],
                    Some(PROFILE),
                    "a-pr-story_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-pr-write",
                    "稿件撰写",
                    "撰写新闻稿和媒体资料包",
                    vec![],
                    Some(PROFILE),
                    "a-pr-write_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-pr-distribute",
                    "分发传播",
                    "确定媒体名单并分发稿件",
                    vec![],
                    Some(PROFILE),
                    "a-pr-distribute_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-pr-story", "trigger", "a-pr-story"),
                edge("e-a-pr-story-a-pr-write", "a-pr-story", "a-pr-write"),
                edge("e-a-pr-write-a-pr-distribute", "a-pr-write", "a-pr-distribute"),
                edge("e-a-pr-distribute-end", "a-pr-distribute", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // wf-mkt-seo-audit: SEO审计
    if seed_domain_template(db, build_domain_template(
        "wf-mkt-seo-audit", "SEO审计", "技术SEO: 爬虫、索引、页面速度 → 关键词策略、内容质量、Meta标签 → 实施优化建议并监控排名变化", "🔍",
        vec!["opc".to_string(), "marketing".to_string()],
        PROFILE,
        vec![
            make_trigger(250.0, 0.0),
            make_agent_node("a-seo-scan", "技术SEO扫描", "技术SEO: 爬虫、索引、页面速度", vec![], Some(PROFILE), "a-seo-scan_result", 250.0, 150.0),
            make_agent_node("a-seo-content", "内容SEO分析", "关键词策略、内容质量、Meta标签", vec![], Some(PROFILE), "a-seo-content_result", 250.0, 350.0),
            make_agent_node("a-seo-optimize", "优化实施", "实施优化建议并监控排名变化", vec![], Some(PROFILE), "a-seo-optimize_result", 250.0, 550.0),
            make_end(250.0, 750.0),
        ],
        vec![
            edge("e-trigger-a-seo-scan", "trigger", "a-seo-scan"),
            edge("e-a-seo-scan-a-seo-content", "a-seo-scan", "a-seo-content"),
            edge("e-a-seo-content-a-seo-optimize", "a-seo-content", "a-seo-optimize"),
            edge("e-a-seo-optimize-end", "a-seo-optimize", "end"),
        ],
    )).await? {
        seeded += 1;
    }

    // wf-mkt-social-plan: 社交媒体策略
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-mkt-social-plan",
            "社交媒体策略",
            "审计现有社交账号和内容表现 → 确定平台、内容类型、发布频率 → 创建月度内容日历和排期",
            "📱",
            vec!["opc".to_string(), "marketing".to_string()],
            PROFILE,
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-soc-audit",
                    "社交审计",
                    "审计现有社交账号和内容表现",
                    vec![],
                    Some(PROFILE),
                    "a-soc-audit_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-soc-strategy",
                    "策略制定",
                    "确定平台、内容类型、发布频率",
                    vec![],
                    Some(PROFILE),
                    "a-soc-strategy_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-soc-calendar",
                    "内容日历",
                    "创建月度内容日历和排期",
                    vec![],
                    Some(PROFILE),
                    "a-soc-calendar_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-soc-audit", "trigger", "a-soc-audit"),
                edge("e-a-soc-audit-a-soc-strategy", "a-soc-audit", "a-soc-strategy"),
                edge("e-a-soc-strategy-a-soc-calendar", "a-soc-strategy", "a-soc-calendar"),
                edge("e-a-soc-calendar-end", "a-soc-calendar", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // wf-mkt-webinar: 线上研讨会
    if seed_domain_template(db, build_domain_template(
        "wf-mkt-webinar", "线上研讨会", "确定主题、嘉宾、时间、渠道 → 准备PPT、推广素材、测试环境 → 发送回放、收集反馈、线索评分", "🎥",
        vec!["opc".to_string(), "marketing".to_string()],
        PROFILE,
        vec![
            make_trigger(250.0, 0.0),
            make_agent_node("a-webinar-plan", "研讨会策划", "确定主题、嘉宾、时间、渠道", vec![], Some(PROFILE), "a-webinar-plan_result", 250.0, 150.0),
            make_agent_node("a-webinar-prep", "会前准备", "准备PPT、推广素材、测试环境", vec![], Some(PROFILE), "a-webinar-prep_result", 250.0, 350.0),
            make_agent_node("a-webinar-follow", "会后跟进", "发送回放、收集反馈、线索评分", vec![], Some(PROFILE), "a-webinar-follow_result", 250.0, 550.0),
            make_end(250.0, 750.0),
        ],
        vec![
            edge("e-trigger-a-webinar-plan", "trigger", "a-webinar-plan"),
            edge("e-a-webinar-plan-a-webinar-prep", "a-webinar-plan", "a-webinar-prep"),
            edge("e-a-webinar-prep-a-webinar-follow", "a-webinar-prep", "a-webinar-follow"),
            edge("e-a-webinar-follow-end", "a-webinar-follow", "end"),
        ],
    )).await? {
        seeded += 1;
    }

    Ok(seeded)
}
