//! 行业配置服务 — 从代码硬编码迁移到 Rust 常量配置
//!
//! 替代 OpcIndustryAdapter 中的静态配置数据（验证规则、KPI 定义、自动化规则、仪表盘等）。
//! 动态业务逻辑（validate、compute_kpis）保留在独立的 Service 中。

use serde::{Deserialize, Serialize};

use super::analytics::{KpiDefinition, KpiValue};
use super::automation::{AutomationAction, AutomationCondition, IndustryAutomationRule};
use super::workflow::{
    DashboardCardDef, KpiCalculationDef, ValidationDef, WorkflowInputField, WorkflowStepDef,
};

/// 行业配置 — 静态数据（可序列化、可存储到配置表）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IndustryConfig {
    /// 行业 ID
    pub industry_id: String,
    /// 行业名称
    pub industry_name: String,
    /// 版本号
    pub version: u32,

    // ── 验证规则 ──
    pub validations: Vec<ValidationDef>,

    // ── KPI 定义 ──
    pub kpi_definitions: Vec<KpiDefinition>,
    pub kpi_calculations: Vec<KpiCalculationDef>,

    // ── 工作流 ──
    pub input_fields: Vec<WorkflowInputField>,
    pub requires_approval: bool,
    /// 工作流步骤（可选，seed 文件中已定义时可为空）
    pub workflow_steps: Vec<WorkflowStepDef>,

    // ── 自动化规则 ──
    pub automation_rules: Vec<IndustryAutomationRule>,

    // ── 仪表盘 ──
    pub dashboard_cards: Vec<DashboardCardDef>,

    // ── 实体类型 ──
    pub entity_types: Vec<String>,
}

impl IndustryConfig {
    pub fn new(industry_id: impl Into<String>, industry_name: impl Into<String>) -> Self {
        Self {
            industry_id: industry_id.into(),
            industry_name: industry_name.into(),
            version: 1,
            validations: Vec::new(),
            kpi_definitions: Vec::new(),
            kpi_calculations: Vec::new(),
            input_fields: Vec::new(),
            requires_approval: false,
            workflow_steps: Vec::new(),
            automation_rules: Vec::new(),
            dashboard_cards: Vec::new(),
            entity_types: Vec::new(),
        }
    }
}

// ── 会计与财务管理 ──

pub fn accounting_config() -> IndustryConfig {
    IndustryConfig {
        industry_id: "accounting".to_string(),
        industry_name: "会计与财务管理".to_string(),
        version: 1,
        validations: vec![
            ValidationDef {
                field: "total".to_string(),
                r#type: "non_negative".to_string(),
                error_message: "发票总金额必须大于等于0".to_string(),
            },
            ValidationDef {
                field: "email".to_string(),
                r#type: "contains_at".to_string(),
                error_message: "客户邮箱格式不正确".to_string(),
            },
        ],
        kpi_definitions: vec![
            KpiDefinition::new(
                "total_revenue",
                "总营收",
                "元",
                super::analytics::MetricType::Currency,
            ),
            KpiDefinition::new(
                "outstanding_invoices",
                "未结清发票",
                "张",
                super::analytics::MetricType::Count,
            ),
            KpiDefinition::new(
                "collection_rate",
                "回款率",
                "%",
                super::analytics::MetricType::Percentage,
            ),
        ],
        kpi_calculations: vec![
            KpiCalculationDef {
                key: "invoice_count".to_string(), name: "发票数量".to_string()
            },
            KpiCalculationDef { key: "total_revenue".to_string(), name: "总营收".to_string() },
            KpiCalculationDef { key: "collection_rate".to_string(), name: "回款率".to_string() },
            KpiCalculationDef {
                key: "avg_processing_time".to_string(),
                name: "平均处理时间".to_string(),
            },
        ],
        input_fields: vec![
            WorkflowInputField {
                key: "company_name".to_string(),
                label: "公司名称".to_string(),
                field_type: "string".to_string(),
                required: true,
                placeholder: Some("如：某某科技有限公司".to_string()),
                default: None,
            },
            WorkflowInputField {
                key: "period".to_string(),
                label: "财务周期".to_string(),
                field_type: "string".to_string(),
                required: false,
                placeholder: Some("如：2026-Q2".to_string()),
                default: None,
            },
            WorkflowInputField {
                key: "focus_area".to_string(),
                label: "关注领域".to_string(),
                field_type: "string".to_string(),
                required: false,
                placeholder: Some("如：成本控制、现金流管理".to_string()),
                default: None,
            },
        ],
        requires_approval: true,
        workflow_steps: Vec::new(),
        automation_rules: vec![
            IndustryAutomationRule::new(
                "accounting_overdue_alert",
                "发票逾期提醒",
                vec![
                    AutomationCondition::OverdueDaysGte { days: 15 },
                    AutomationCondition::EntityTypeIs { entity_type: "invoice".to_string() },
                ],
                vec![AutomationAction::SendNotification {
                    target: "customer".to_string(),
                    message: "您的发票即将逾期".to_string(),
                }],
            ),
            IndustryAutomationRule::new(
                "accounting_payment_reminder",
                "付款到期提醒",
                vec![
                    AutomationCondition::OverdueDaysGte { days: 7 },
                    AutomationCondition::StatusIs { status: "sent".to_string() },
                ],
                vec![AutomationAction::SendNotification {
                    target: "finance_team".to_string(),
                    message: "有发票即将到期".to_string(),
                }],
            ),
        ],
        dashboard_cards: vec![
            DashboardCardDef {
                id: "revenue_card".to_string(),
                title: "本月营收".to_string(),
                kpi_key: "total_revenue".to_string(),
            },
            DashboardCardDef {
                id: "invoice_card".to_string(),
                title: "本月发票数".to_string(),
                kpi_key: "invoice_count".to_string(),
            },
            DashboardCardDef {
                id: "collection_card".to_string(),
                title: "回款率".to_string(),
                kpi_key: "collection_rate".to_string(),
            },
        ],
        entity_types: vec![
            "invoice".to_string(),
            "finance_record".to_string(),
            "customer".to_string(),
        ],
    }
}

// ── 金融投资 ──

pub fn finance_invest_config() -> IndustryConfig {
    IndustryConfig {
        industry_id: "finance_invest".to_string(),
        industry_name: "金融投资".to_string(),
        version: 1,
        validations: vec![
            ValidationDef {
                field: "code".to_string(),
                r#type: "regex".to_string(),
                error_message: "股票代码格式不正确".to_string(),
            },
            ValidationDef {
                field: "amount".to_string(),
                r#type: "positive".to_string(),
                error_message: "投资金额必须为正数".to_string(),
            },
        ],
        kpi_definitions: vec![
            KpiDefinition::new(
                "portfolio_value",
                "组合市值",
                "元",
                super::analytics::MetricType::Currency,
            ),
            KpiDefinition::new("daily_pnl", "日盈亏", "元", super::analytics::MetricType::Currency),
            KpiDefinition::new("win_rate", "胜率", "%", super::analytics::MetricType::Percentage),
            KpiDefinition::new(
                "max_drawdown",
                "最大回撤",
                "%",
                super::analytics::MetricType::Percentage,
            ),
        ],
        kpi_calculations: vec![
            KpiCalculationDef { key: "total_assets".to_string(), name: "总资产".to_string() },
            KpiCalculationDef { key: "daily_return".to_string(), name: "日收益率".to_string() },
            KpiCalculationDef { key: "sharpe_ratio".to_string(), name: "夏普比率".to_string() },
        ],
        input_fields: vec![WorkflowInputField {
            key: "stock_code".to_string(),
            label: "股票代码".to_string(),
            field_type: "string".to_string(),
            required: true,
            placeholder: Some("如：600519".to_string()),
            default: None,
        }],
        requires_approval: false,
        workflow_steps: Vec::new(),
        automation_rules: vec![IndustryAutomationRule::new(
            "finance_risk_alert",
            "风险预警",
            vec![AutomationCondition::Custom { expression: "drawdown > 0.15".to_string() }],
            vec![AutomationAction::SendNotification {
                target: "risk_manager".to_string(),
                message: "组合最大回撤超过15%".to_string(),
            }],
        )],
        dashboard_cards: vec![
            DashboardCardDef {
                id: "portfolio_card".to_string(),
                title: "组合市值".to_string(),
                kpi_key: "portfolio_value".to_string(),
            },
            DashboardCardDef {
                id: "pnl_card".to_string(),
                title: "今日盈亏".to_string(),
                kpi_key: "daily_pnl".to_string(),
            },
            DashboardCardDef {
                id: "risk_card".to_string(),
                title: "最大回撤".to_string(),
                kpi_key: "max_drawdown".to_string(),
            },
        ],
        entity_types: vec!["stock".to_string(), "portfolio".to_string(), "order".to_string()],
    }
}

// ── 软件研发 ──

pub fn software_dev_config() -> IndustryConfig {
    IndustryConfig {
        industry_id: "software_dev".to_string(),
        industry_name: "软件开发".to_string(),
        version: 1,
        validations: vec![
            ValidationDef {
                field: "title".to_string(),
                r#type: "required".to_string(),
                error_message: "需求标题不能为空".to_string(),
            },
            ValidationDef {
                field: "priority".to_string(),
                r#type: "in_list".to_string(),
                error_message: "优先级必须是 P0/P1/P2/P3".to_string(),
            },
        ],
        kpi_definitions: vec![
            KpiDefinition::new(
                "sprint_velocity",
                "迭代速率",
                "点",
                super::analytics::MetricType::Count,
            ),
            KpiDefinition::new(
                "bug_resolution_rate",
                "缺陷修复率",
                "%",
                super::analytics::MetricType::Percentage,
            ),
            KpiDefinition::new(
                "code_coverage",
                "代码覆盖率",
                "%",
                super::analytics::MetricType::Percentage,
            ),
        ],
        kpi_calculations: vec![
            KpiCalculationDef {
                key: "completed_story_points".to_string(),
                name: "完成故事点".to_string(),
            },
            KpiCalculationDef { key: "open_bugs".to_string(), name: "未解决缺陷".to_string() },
            KpiCalculationDef {
                key: "deployment_count".to_string(), name: "部署次数".to_string()
            },
        ],
        input_fields: vec![
            WorkflowInputField {
                key: "project_name".to_string(),
                label: "项目名称".to_string(),
                field_type: "string".to_string(),
                required: true,
                placeholder: Some("如：API 网关重构".to_string()),
                default: None,
            },
            WorkflowInputField {
                key: "priority".to_string(),
                label: "优先级".to_string(),
                field_type: "select".to_string(),
                required: false,
                placeholder: None,
                default: Some("P2".to_string()),
            },
        ],
        requires_approval: false,
        workflow_steps: Vec::new(),
        automation_rules: vec![IndustryAutomationRule::new(
            "dev_deadline_alert",
            "截止日期提醒",
            vec![
                AutomationCondition::OverdueDaysGte { days: 1 },
                AutomationCondition::EntityTypeIs { entity_type: "task".to_string() },
            ],
            vec![AutomationAction::SendNotification {
                target: "assignee".to_string(),
                message: "任务即将到期".to_string(),
            }],
        )],
        dashboard_cards: vec![
            DashboardCardDef {
                id: "velocity_card".to_string(),
                title: "迭代速率".to_string(),
                kpi_key: "sprint_velocity".to_string(),
            },
            DashboardCardDef {
                id: "bugs_card".to_string(),
                title: "未解决缺陷".to_string(),
                kpi_key: "open_bugs".to_string(),
            },
            DashboardCardDef {
                id: "coverage_card".to_string(),
                title: "代码覆盖率".to_string(),
                kpi_key: "code_coverage".to_string(),
            },
        ],
        entity_types: vec![
            "requirement".to_string(),
            "task".to_string(),
            "bug".to_string(),
            "release".to_string(),
        ],
    }
}

// ── 设计 ──

pub fn design_config() -> IndustryConfig {
    IndustryConfig {
        industry_id: "design".to_string(),
        industry_name: "设计".to_string(),
        version: 1,
        validations: vec![ValidationDef {
            field: "name".to_string(),
            r#type: "required".to_string(),
            error_message: "设计名称不能为空".to_string(),
        }],
        kpi_definitions: vec![
            KpiDefinition::new(
                "design_count",
                "设计产出数",
                "个",
                super::analytics::MetricType::Count,
            ),
            KpiDefinition::new(
                "review_pass_rate",
                "评审通过率",
                "%",
                super::analytics::MetricType::Percentage,
            ),
        ],
        kpi_calculations: vec![
            KpiCalculationDef {
                key: "design_count".to_string(), name: "设计产出数".to_string()
            },
            KpiCalculationDef {
                key: "avg_review_time".to_string(),
                name: "平均评审时长".to_string(),
            },
        ],
        input_fields: vec![WorkflowInputField {
            key: "project_type".to_string(),
            label: "项目类型".to_string(),
            field_type: "select".to_string(),
            required: false,
            placeholder: None,
            default: Some("ui_ux".to_string()),
        }],
        requires_approval: true,
        workflow_steps: Vec::new(),
        automation_rules: vec![],
        dashboard_cards: vec![
            DashboardCardDef {
                id: "design_card".to_string(),
                title: "设计产出".to_string(),
                kpi_key: "design_count".to_string(),
            },
            DashboardCardDef {
                id: "review_card".to_string(),
                title: "评审通过率".to_string(),
                kpi_key: "review_pass_rate".to_string(),
            },
        ],
        entity_types: vec!["design".to_string(), "mockup".to_string(), "asset".to_string()],
    }
}

// ── AI 研究 ──

pub fn ai_research_config() -> IndustryConfig {
    IndustryConfig {
        industry_id: "ai_research".to_string(),
        industry_name: "AI 研究与咨询".to_string(),
        version: 1,
        validations: vec![ValidationDef {
            field: "topic".to_string(),
            r#type: "required".to_string(),
            error_message: "研究主题不能为空".to_string(),
        }],
        kpi_definitions: vec![
            KpiDefinition::new(
                "papers_reviewed",
                "审阅论文数",
                "篇",
                super::analytics::MetricType::Count,
            ),
            KpiDefinition::new(
                "models_compared",
                "对比模型数",
                "个",
                super::analytics::MetricType::Count,
            ),
        ],
        kpi_calculations: vec![
            KpiCalculationDef {
                key: "papers_reviewed".to_string(),
                name: "审阅论文数".to_string(),
            },
            KpiCalculationDef {
                key: "avg_review_quality".to_string(),
                name: "平均审阅质量".to_string(),
            },
        ],
        input_fields: vec![WorkflowInputField {
            key: "research_topic".to_string(),
            label: "研究主题".to_string(),
            field_type: "string".to_string(),
            required: true,
            placeholder: Some("如：LLM 对齐技术综述".to_string()),
            default: None,
        }],
        requires_approval: false,
        workflow_steps: Vec::new(),
        automation_rules: vec![],
        dashboard_cards: vec![DashboardCardDef {
            id: "paper_card".to_string(),
            title: "审阅论文".to_string(),
            kpi_key: "papers_reviewed".to_string(),
        }],
        entity_types: vec!["paper".to_string(), "experiment".to_string(), "report".to_string()],
    }
}

// ── 内容与媒体 ──

pub fn content_media_config() -> IndustryConfig {
    IndustryConfig {
        industry_id: "content_media".to_string(),
        industry_name: "内容与媒体".to_string(),
        version: 1,
        validations: vec![ValidationDef {
            field: "title".to_string(),
            r#type: "required".to_string(),
            error_message: "内容标题不能为空".to_string(),
        }],
        kpi_definitions: vec![
            KpiDefinition::new(
                "content_views",
                "内容阅读量",
                "次",
                super::analytics::MetricType::Count,
            ),
            KpiDefinition::new(
                "engagement_rate",
                "互动率",
                "%",
                super::analytics::MetricType::Percentage,
            ),
        ],
        kpi_calculations: vec![
            KpiCalculationDef {
                key: "publish_count".to_string(), name: "发布数量".to_string()
            },
            KpiCalculationDef { key: "total_views".to_string(), name: "总阅读量".to_string() },
        ],
        input_fields: vec![WorkflowInputField {
            key: "content_type".to_string(),
            label: "内容类型".to_string(),
            field_type: "select".to_string(),
            required: false,
            placeholder: None,
            default: Some("article".to_string()),
        }],
        requires_approval: true,
        workflow_steps: Vec::new(),
        automation_rules: vec![],
        dashboard_cards: vec![
            DashboardCardDef {
                id: "views_card".to_string(),
                title: "阅读量".to_string(),
                kpi_key: "content_views".to_string(),
            },
            DashboardCardDef {
                id: "engagement_card".to_string(),
                title: "互动率".to_string(),
                kpi_key: "engagement_rate".to_string(),
            },
        ],
        entity_types: vec!["article".to_string(), "video".to_string(), "post".to_string()],
    }
}

// ── 电子商务 ──

pub fn ecommerce_config() -> IndustryConfig {
    IndustryConfig {
        industry_id: "ecommerce".to_string(),
        industry_name: "电子商务".to_string(),
        version: 1,
        validations: vec![ValidationDef {
            field: "sku".to_string(),
            r#type: "required".to_string(),
            error_message: "商品 SKU 不能为空".to_string(),
        }],
        kpi_definitions: vec![
            KpiDefinition::new("gmv", "GMV", "元", super::analytics::MetricType::Currency),
            KpiDefinition::new(
                "conversion_rate",
                "转化率",
                "%",
                super::analytics::MetricType::Percentage,
            ),
            KpiDefinition::new(
                "return_rate",
                "退货率",
                "%",
                super::analytics::MetricType::Percentage,
            ),
        ],
        kpi_calculations: vec![
            KpiCalculationDef { key: "order_count".to_string(), name: "订单数".to_string() },
            KpiCalculationDef { key: "total_gmv".to_string(), name: "GMV".to_string() },
            KpiCalculationDef {
                key: "new_customers".to_string(), name: "新客户数".to_string()
            },
        ],
        input_fields: vec![],
        requires_approval: false,
        workflow_steps: Vec::new(),
        automation_rules: vec![],
        dashboard_cards: vec![
            DashboardCardDef {
                id: "gmv_card".to_string(),
                title: "GMV".to_string(),
                kpi_key: "gmv".to_string(),
            },
            DashboardCardDef {
                id: "order_card".to_string(),
                title: "订单数".to_string(),
                kpi_key: "order_count".to_string(),
            },
        ],
        entity_types: vec!["product".to_string(), "order".to_string(), "customer".to_string()],
    }
}

// ── 教育培训 ──

pub fn education_config() -> IndustryConfig {
    IndustryConfig {
        industry_id: "education".to_string(),
        industry_name: "教育培训".to_string(),
        version: 1,
        validations: vec![ValidationDef {
            field: "title".to_string(),
            r#type: "required".to_string(),
            error_message: "课程标题不能为空".to_string(),
        }],
        kpi_definitions: vec![
            KpiDefinition::new(
                "student_count",
                "学员数",
                "人",
                super::analytics::MetricType::Count,
            ),
            KpiDefinition::new(
                "completion_rate",
                "完成率",
                "%",
                super::analytics::MetricType::Percentage,
            ),
            KpiDefinition::new("avg_score", "平均分", "分", super::analytics::MetricType::Gauge),
        ],
        kpi_calculations: vec![
            KpiCalculationDef {
                key: "active_students".to_string(), name: "活跃学员".to_string()
            },
            KpiCalculationDef {
                key: "course_completion".to_string(),
                name: "课程完成率".to_string(),
            },
        ],
        input_fields: vec![],
        requires_approval: false,
        workflow_steps: Vec::new(),
        automation_rules: vec![],
        dashboard_cards: vec![
            DashboardCardDef {
                id: "students_card".to_string(),
                title: "学员数".to_string(),
                kpi_key: "student_count".to_string(),
            },
            DashboardCardDef {
                id: "completion_card".to_string(),
                title: "完成率".to_string(),
                kpi_key: "completion_rate".to_string(),
            },
        ],
        entity_types: vec!["course".to_string(), "student".to_string(), "assignment".to_string()],
    }
}

// ── 地理信息 ──

pub fn geospatial_config() -> IndustryConfig {
    IndustryConfig {
        industry_id: "geospatial".to_string(),
        industry_name: "地理信息".to_string(),
        version: 1,
        validations: vec![ValidationDef {
            field: "geometry".to_string(),
            r#type: "valid_geo_json".to_string(),
            error_message: "GeoJSON 格式不正确".to_string(),
        }],
        kpi_definitions: vec![
            KpiDefinition::new(
                "data_points",
                "数据点数",
                "个",
                super::analytics::MetricType::Count,
            ),
            KpiDefinition::new(
                "analysis_count",
                "分析次数",
                "次",
                super::analytics::MetricType::Count,
            ),
        ],
        kpi_calculations: vec![
            KpiCalculationDef { key: "layer_count".to_string(), name: "图层数".to_string() },
            KpiCalculationDef {
                key: "analysis_count".to_string(), name: "分析次数".to_string()
            },
        ],
        input_fields: vec![],
        requires_approval: false,
        workflow_steps: Vec::new(),
        automation_rules: vec![],
        dashboard_cards: vec![DashboardCardDef {
            id: "points_card".to_string(),
            title: "数据点数".to_string(),
            kpi_key: "data_points".to_string(),
        }],
        entity_types: vec!["dataset".to_string(), "layer".to_string(), "analysis".to_string()],
    }
}

// ── 行业咨询 ──

pub fn industry_consulting_config() -> IndustryConfig {
    IndustryConfig {
        industry_id: "industry_consulting".to_string(),
        industry_name: "行业咨询".to_string(),
        version: 1,
        validations: vec![ValidationDef {
            field: "topic".to_string(),
            r#type: "required".to_string(),
            error_message: "咨询主题不能为空".to_string(),
        }],
        kpi_definitions: vec![
            KpiDefinition::new(
                "project_count",
                "项目数",
                "个",
                super::analytics::MetricType::Count,
            ),
            KpiDefinition::new(
                "client_satisfaction",
                "客户满意度",
                "%",
                super::analytics::MetricType::Percentage,
            ),
        ],
        kpi_calculations: vec![
            KpiCalculationDef {
                key: "active_projects".to_string(),
                name: "进行中项目".to_string(),
            },
            KpiCalculationDef {
                key: "client_satisfaction".to_string(),
                name: "客户满意度".to_string(),
            },
        ],
        input_fields: vec![],
        requires_approval: true,
        workflow_steps: Vec::new(),
        automation_rules: vec![],
        dashboard_cards: vec![
            DashboardCardDef {
                id: "projects_card".to_string(),
                title: "项目数".to_string(),
                kpi_key: "project_count".to_string(),
            },
            DashboardCardDef {
                id: "satisfaction_card".to_string(),
                title: "客户满意度".to_string(),
                kpi_key: "client_satisfaction".to_string(),
            },
        ],
        entity_types: vec!["project".to_string(), "client".to_string(), "report".to_string()],
    }
}

// ── 项目管理 ──

pub fn project_management_config() -> IndustryConfig {
    IndustryConfig {
        industry_id: "project_management".to_string(),
        industry_name: "项目管理".to_string(),
        version: 1,
        validations: vec![ValidationDef {
            field: "name".to_string(),
            r#type: "required".to_string(),
            error_message: "项目名称不能为空".to_string(),
        }],
        kpi_definitions: vec![
            KpiDefinition::new(
                "project_count",
                "项目数",
                "个",
                super::analytics::MetricType::Count,
            ),
            KpiDefinition::new(
                "on_time_rate",
                "按时交付率",
                "%",
                super::analytics::MetricType::Percentage,
            ),
            KpiDefinition::new(
                "budget_utilization",
                "预算利用率",
                "%",
                super::analytics::MetricType::Percentage,
            ),
        ],
        kpi_calculations: vec![
            KpiCalculationDef {
                key: "active_projects".to_string(),
                name: "进行中项目".to_string(),
            },
            KpiCalculationDef {
                key: "completed_projects".to_string(),
                name: "已完成项目".to_string(),
            },
            KpiCalculationDef {
                key: "budget_utilization".to_string(),
                name: "预算利用率".to_string(),
            },
        ],
        input_fields: vec![],
        requires_approval: true,
        workflow_steps: Vec::new(),
        automation_rules: vec![IndustryAutomationRule::new(
            "pm_deadline_warning",
            "项目截止警告",
            vec![
                AutomationCondition::OverdueDaysGte { days: 3 },
                AutomationCondition::EntityTypeIs { entity_type: "milestone".to_string() },
            ],
            vec![AutomationAction::SendNotification {
                target: "project_manager".to_string(),
                message: "里程碑即将到期".to_string(),
            }],
        )],
        dashboard_cards: vec![
            DashboardCardDef {
                id: "projects_card".to_string(),
                title: "项目数".to_string(),
                kpi_key: "project_count".to_string(),
            },
            DashboardCardDef {
                id: "ontime_card".to_string(),
                title: "按时交付率".to_string(),
                kpi_key: "on_time_rate".to_string(),
            },
        ],
        entity_types: vec!["project".to_string(), "task".to_string(), "milestone".to_string()],
    }
}

// ── 销售增长 ──

pub fn sales_growth_config() -> IndustryConfig {
    IndustryConfig {
        industry_id: "sales_growth".to_string(),
        industry_name: "销售增长与营销".to_string(),
        version: 1,
        validations: vec![ValidationDef {
            field: "name".to_string(),
            r#type: "required".to_string(),
            error_message: "销售线索名称不能为空".to_string(),
        }],
        kpi_definitions: vec![
            KpiDefinition::new("lead_count", "线索数", "条", super::analytics::MetricType::Count),
            KpiDefinition::new(
                "conversion_rate",
                "转化率",
                "%",
                super::analytics::MetricType::Percentage,
            ),
            KpiDefinition::new(
                "sales_revenue",
                "销售额",
                "元",
                super::analytics::MetricType::Currency,
            ),
        ],
        kpi_calculations: vec![
            KpiCalculationDef { key: "total_leads".to_string(), name: "总线索数".to_string() },
            KpiCalculationDef { key: "conversion_rate".to_string(), name: "转化率".to_string() },
            KpiCalculationDef {
                key: "total_revenue".to_string(), name: "总销售额".to_string()
            },
        ],
        input_fields: vec![],
        requires_approval: false,
        workflow_steps: Vec::new(),
        automation_rules: vec![IndustryAutomationRule::new(
            "sales_follow_up_reminder",
            "跟进提醒",
            vec![
                AutomationCondition::OverdueDaysGte { days: 2 },
                AutomationCondition::StatusIs { status: "new".to_string() },
            ],
            vec![AutomationAction::SendNotification {
                target: "sales_rep".to_string(),
                message: "有新线索待跟进".to_string(),
            }],
        )],
        dashboard_cards: vec![
            DashboardCardDef {
                id: "leads_card".to_string(),
                title: "线索数".to_string(),
                kpi_key: "lead_count".to_string(),
            },
            DashboardCardDef {
                id: "revenue_card".to_string(),
                title: "销售额".to_string(),
                kpi_key: "sales_revenue".to_string(),
            },
        ],
        entity_types: vec!["lead".to_string(), "deal".to_string(), "campaign".to_string()],
    }
}

// ── 安全合规 ──

pub fn security_config() -> IndustryConfig {
    IndustryConfig {
        industry_id: "security".to_string(),
        industry_name: "安全合规".to_string(),
        version: 1,
        validations: vec![ValidationDef {
            field: "type".to_string(),
            r#type: "required".to_string(),
            error_message: "安全事件类型不能为空".to_string(),
        }],
        kpi_definitions: vec![
            KpiDefinition::new(
                "incident_count",
                "安全事件数",
                "起",
                super::analytics::MetricType::Count,
            ),
            KpiDefinition::new(
                "response_time",
                "响应时间",
                "分钟",
                super::analytics::MetricType::Gauge,
            ),
            KpiDefinition::new(
                "compliance_rate",
                "合规率",
                "%",
                super::analytics::MetricType::Percentage,
            ),
        ],
        kpi_calculations: vec![
            KpiCalculationDef {
                key: "incident_count".to_string(), name: "安全事件数".to_string()
            },
            KpiCalculationDef {
                key: "avg_response_time".to_string(),
                name: "平均响应时间".to_string(),
            },
        ],
        input_fields: vec![],
        requires_approval: true,
        workflow_steps: Vec::new(),
        automation_rules: vec![IndustryAutomationRule::new(
            "security_threat_alert",
            "安全威胁告警",
            vec![AutomationCondition::Custom { expression: "severity == 'critical'".to_string() }],
            vec![AutomationAction::SendNotification {
                target: "security_team".to_string(),
                message: "检测到严重安全威胁".to_string(),
            }],
        )],
        dashboard_cards: vec![
            DashboardCardDef {
                id: "incidents_card".to_string(),
                title: "安全事件".to_string(),
                kpi_key: "incident_count".to_string(),
            },
            DashboardCardDef {
                id: "compliance_card".to_string(),
                title: "合规率".to_string(),
                kpi_key: "compliance_rate".to_string(),
            },
        ],
        entity_types: vec!["incident".to_string(), "audit".to_string(), "policy".to_string()],
    }
}

// ── 游戏开发 ──

pub fn game_dev_config() -> IndustryConfig {
    IndustryConfig {
        industry_id: "game_dev".to_string(),
        industry_name: "游戏开发".to_string(),
        version: 1,
        validations: vec![ValidationDef {
            field: "name".to_string(),
            r#type: "required".to_string(),
            error_message: "游戏功能名称不能为空".to_string(),
        }],
        kpi_definitions: vec![
            KpiDefinition::new(
                "active_users",
                "活跃用户数",
                "人",
                super::analytics::MetricType::Count,
            ),
            KpiDefinition::new(
                "retention_rate",
                "留存率",
                "%",
                super::analytics::MetricType::Percentage,
            ),
            KpiDefinition::new("revenue", "收入", "元", super::analytics::MetricType::Currency),
        ],
        kpi_calculations: vec![
            KpiCalculationDef { key: "dau".to_string(), name: "日活跃用户".to_string() },
            KpiCalculationDef { key: "retention_d1".to_string(), name: "次日留存".to_string() },
        ],
        input_fields: vec![],
        requires_approval: false,
        workflow_steps: Vec::new(),
        automation_rules: vec![],
        dashboard_cards: vec![
            DashboardCardDef {
                id: "users_card".to_string(),
                title: "活跃用户".to_string(),
                kpi_key: "active_users".to_string(),
            },
            DashboardCardDef {
                id: "retention_card".to_string(),
                title: "留存率".to_string(),
                kpi_key: "retention_rate".to_string(),
            },
        ],
        entity_types: vec!["feature".to_string(), "asset".to_string(), "build".to_string()],
    }
}

// ── 配置注册表 ──

/// 获取所有行业配置
pub fn get_all_configs() -> Vec<IndustryConfig> {
    vec![
        accounting_config(),
        ai_research_config(),
        content_media_config(),
        design_config(),
        ecommerce_config(),
        education_config(),
        finance_invest_config(),
        game_dev_config(),
        geospatial_config(),
        industry_consulting_config(),
        project_management_config(),
        sales_growth_config(),
        security_config(),
        software_dev_config(),
    ]
}

/// 按行业 ID 获取配置
pub fn get_config(industry_id: &str) -> Option<IndustryConfig> {
    let id = industry_id.replace('-', "_");
    get_all_configs().into_iter().find(|c| c.industry_id == id)
}

/// 列出所有行业 ID 和名称
pub fn list_industries() -> Vec<(String, String)> {
    get_all_configs().into_iter().map(|c| (c.industry_id, c.industry_name)).collect()
}

// ── 共享类型（迁移自 industry/mod.rs） ──

/// 工作流步骤（简化版，用于列表展示）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub id: String,
    pub name: String,
    pub description: String,
    pub order: u32,
}

impl WorkflowStep {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self { id: id.into(), name: name.into(), description: description.into(), order: 0 }
    }

    pub fn with_order(mut self, order: u32) -> Self {
        self.order = order;
        self
    }
}

/// 仪表盘卡片实例（用于运行时展示）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardCard {
    pub id: String,
    pub title: String,
    pub kpi_key: String,
    pub display_value: String,
}

impl DashboardCard {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        kpi_key: impl Into<String>,
        display_value: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            kpi_key: kpi_key.into(),
            display_value: display_value.into(),
        }
    }
}

/// 行业仪表盘摘要
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IndustryDashboard {
    pub industry_id: String,
    pub kpis: Vec<KpiValue>,
    pub cards: Vec<DashboardCard>,
    pub summary: Option<String>,
}
