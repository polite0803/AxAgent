// SPDX-License-Identifier: AGPL-3.0-only

//! OPC 行业专属操作后端命令
//!
//! 为 9 个垂直行业的操作入口提供后端配置和 prompt 生成。

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::AppState;
use axagent_orchestrator::{ReinforcementLearningConfig, RewardWeightConfig};

/// 行业操作类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    /// 对话类型：启动一个带有预设 prompt 的新对话
    Conversation,
    /// 工作流类型：创建一个行业专属工作流
    Workflow,
}

/// 行业操作配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndustryActionConfig {
    /// 操作唯一标识
    pub key: String,
    /// 操作标签
    pub label: String,
    /// 操作描述
    pub description: String,
    /// 操作类型
    pub action_type: ActionType,
    /// 预设 system prompt（对话类型使用）
    pub system_prompt: String,
    /// 预设 user prompt 模板（对话类型使用，支持 {{input}} 变量）
    pub user_prompt_template: String,
    /// 关联的工作流 ID（工作流类型使用）
    pub workflow_id: Option<String>,
    /// 操作图标标识
    pub icon: String,
    /// 分类标签
    pub tags: Vec<String>,
}

/// 行业工作流配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndustryWorkflowConfig {
    /// 工作流唯一标识
    pub id: String,
    /// 工作流名称
    pub name: String,
    /// 工作流描述
    pub description: String,
    /// 工作流版本
    pub version: String,
    /// 工作流模板 ID（对应 workflow_template 表）
    pub template_id: String,
}

/// 行业完整配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndustryConfig {
    /// 行业 ID
    pub id: String,
    /// 行业名称
    pub name: String,
    /// 行业图标
    pub icon: String,
    /// 行业描述
    pub description: String,
    /// 行业专属操作列表
    pub actions: Vec<IndustryActionConfig>,
    /// 行业专属工作流列表
    pub workflows: Vec<IndustryWorkflowConfig>,
}

// ── 9 个行业配置定义 ──────────────────────────────────────────────

fn ai_research_config() -> IndustryConfig {
    IndustryConfig {
        id: "ai-research".to_string(),
        name: "人工智能研究".to_string(),
        icon: "🤖".to_string(),
        description: "AI 技术调研、模型评测、应用场景分析".to_string(),
        actions: vec![
            IndustryActionConfig {
                key: "ai-paper".to_string(),
                label: "AI 论文调研".to_string(),
                description: "扫描最新 AI 论文并提取关键技术突破".to_string(),
                action_type: ActionType::Conversation,
                system_prompt: "你是一位资深的 AI 研究员，擅长追踪前沿技术论文。请帮助用户调研 AI 领域的最新进展，重点关注大语言模型、多模态、Agent 系统等方向。".to_string(),
                user_prompt_template: "请帮我调研最近一个月 AI 领域的重要论文和技术突破，重点关注大模型、多模态、Agent 等方向。请列出 Top 5 论文并总结核心贡献。".to_string(),
                workflow_id: None,
                icon: "FileSearchOutlined".to_string(),
                tags: vec!["ai".to_string(), "research".to_string(), "papers".to_string()],
            },
            IndustryActionConfig {
                key: "ai-benchmark".to_string(),
                label: "模型性能对比".to_string(),
                description: "对比主流 LLM 模型在中文场景下的表现".to_string(),
                action_type: ActionType::Conversation,
                system_prompt: "你是一位 AI 模型评测专家，熟悉主流 LLM 的能力边界和 benchmark 结果。".to_string(),
                user_prompt_template: "请对比分析 GPT-4o、Claude 3.5 Sonnet、Gemini 1.5 Pro、Qwen 等主流大模型在中文场景下的性能表现，包括推理能力、代码生成、长文本处理、多语言支持等维度。".to_string(),
                workflow_id: None,
                icon: "LineChartOutlined".to_string(),
                tags: vec!["ai".to_string(), "benchmark".to_string(), "llm".to_string()],
            },
            IndustryActionConfig {
                key: "ai-application".to_string(),
                label: "AI 应用场景分析".to_string(),
                description: "分析 AI 技术在特定行业的应用机会".to_string(),
                action_type: ActionType::Conversation,
                system_prompt: "你是一位 AI 应用咨询顾问，擅长将 AI 技术与行业场景结合。".to_string(),
                user_prompt_template: "请分析 AI 技术在 {{input}} 行业的应用场景，包括现有方案、成熟度、投资回报率和推荐路径。".to_string(),
                workflow_id: None,
                icon: "ExperimentOutlined".to_string(),
                tags: vec!["ai".to_string(), "application".to_string()],
            },
            IndustryActionConfig {
                key: "ai-report".to_string(),
                label: "生成 AI 研究报告".to_string(),
                description: "自动生成行业 AI 技术全景报告".to_string(),
                action_type: ActionType::Conversation,
                system_prompt: "你是一位专业的 AI 行业分析师，擅长撰写深度研究报告。".to_string(),
                user_prompt_template: "请生成一份 {{input}} 领域的 AI 技术研究报告，涵盖技术现状、主要玩家、应用案例、趋势预测和投资建议。".to_string(),
                workflow_id: None,
                icon: "FileTextOutlined".to_string(),
                tags: vec!["ai".to_string(), "report".to_string()],
            },
        ],
        workflows: vec![
            IndustryWorkflowConfig {
                id: "wf-ai-research-1".to_string(),
                name: "AI 技术调研".to_string(),
                description: "技术扫描 → 论文分析 → 趋势预测".to_string(),
                version: "1.0".to_string(),
                template_id: "workflow-ai-research".to_string(),
            },
            IndustryWorkflowConfig {
                id: "wf-ai-research-2".to_string(),
                name: "LLM 选型评估".to_string(),
                description: "需求分析 → 模型对比 → 选型建议".to_string(),
                version: "1.0".to_string(),
                template_id: "workflow-ai-llm-selection".to_string(),
            },
            IndustryWorkflowConfig {
                id: "wf-ai-research-3".to_string(),
                name: "AI 产品分析".to_string(),
                description: "竞品扫描 → 功能拆解 → 差异化定位".to_string(),
                version: "1.0".to_string(),
                template_id: "workflow-ai-product-analysis".to_string(),
            },
        ],
    }
}

fn software_dev_config() -> IndustryConfig {
    IndustryConfig {
        id: "software-dev".to_string(),
        name: "软件开发".to_string(),
        icon: "💻".to_string(),
        description: "代码审查、架构设计、API 文档、Bug 分析".to_string(),
        actions: vec![
            IndustryActionConfig {
                key: "sd-code-review".to_string(),
                label: "代码审查".to_string(),
                description: "分析代码质量，提供改进建议".to_string(),
                action_type: ActionType::Conversation,
                system_prompt: "你是一位资深代码审查专家，精通多种编程语言和设计模式。".to_string(),
                user_prompt_template: "请帮我审查以下代码，关注代码质量、潜在 bug、性能问题和最佳实践：\n\n```{{input}}\n```".to_string(),
                workflow_id: None,
                icon: "CodeOutlined".to_string(),
                tags: vec!["dev".to_string(), "code-review".to_string()],
            },
            IndustryActionConfig {
                key: "sd-architecture".to_string(),
                label: "架构设计咨询".to_string(),
                description: "系统架构评估和优化建议".to_string(),
                action_type: ActionType::Conversation,
                system_prompt: "你是一位资深软件架构师，熟悉分布式系统、微服务架构和云原生设计。".to_string(),
                user_prompt_template: "请对 {{input}} 的系统架构进行评估，分析当前设计的优缺点，并提供改进建议和最佳实践。".to_string(),
                workflow_id: None,
                icon: "ApartmentOutlined".to_string(),
                tags: vec!["dev".to_string(), "architecture".to_string()],
            },
            IndustryActionConfig {
                key: "sd-api-doc".to_string(),
                label: "API 文档生成".to_string(),
                description: "根据代码或接口定义自动生成文档".to_string(),
                action_type: ActionType::Conversation,
                system_prompt: "你是一位技术文档撰写专家，擅长编写清晰准确的 API 文档。".to_string(),
                user_prompt_template: "请为以下代码生成完整的 API 文档，包括接口说明、参数描述、返回值、使用示例等：\n\n{{input}}".to_string(),
                workflow_id: None,
                icon: "BookOutlined".to_string(),
                tags: vec!["dev".to_string(), "documentation".to_string()],
            },
            IndustryActionConfig {
                key: "sd-bug".to_string(),
                label: "Bug 分析修复".to_string(),
                description: "分析错误日志，定位问题根因".to_string(),
                action_type: ActionType::Conversation,
                system_prompt: "你是一位资深调试专家，擅长分析错误日志和定位 Bug 根因。".to_string(),
                user_prompt_template: "请帮我分析以下错误信息，定位可能的根因并提供修复方案：\n\n{{input}}".to_string(),
                workflow_id: None,
                icon: "BugOutlined".to_string(),
                tags: vec!["dev".to_string(), "debug".to_string()],
            },
        ],
        workflows: vec![
            IndustryWorkflowConfig {
                id: "wf-sd-1".to_string(),
                name: "代码重构".to_string(),
                description: "代码分析 → 重构建议 → 生成方案".to_string(),
                version: "1.0".to_string(),
                template_id: "workflow-sd-refactor".to_string(),
            },
            IndustryWorkflowConfig {
                id: "wf-sd-2".to_string(),
                name: "技术选型".to_string(),
                description: "需求分析 → 技术调研 → 选型对比".to_string(),
                version: "1.0".to_string(),
                template_id: "workflow-sd-tech-selection".to_string(),
            },
            IndustryWorkflowConfig {
                id: "wf-sd-3".to_string(),
                name: "性能优化".to_string(),
                description: "性能分析 → 瓶颈定位 → 优化方案".to_string(),
                version: "1.0".to_string(),
                template_id: "workflow-sd-performance".to_string(),
            },
        ],
    }
}

fn finance_invest_config() -> IndustryConfig {
    IndustryConfig {
        id: "finance-invest".to_string(),
        name: "金融投资".to_string(),
        icon: "📈".to_string(),
        description: "个股分析、财报解读、估值计算、风险评估".to_string(),
        actions: vec![
            IndustryActionConfig {
                key: "fi-stock-analysis".to_string(),
                label: "个股分析".to_string(),
                description: "全面分析上市公司基本面和技术面".to_string(),
                action_type: ActionType::Conversation,
                system_prompt: "你是一位资深股票分析师，精通基本面分析和技术面分析。".to_string(),
                user_prompt_template: "请对股票 {{input}} 进行全面分析，包括：基本面（财务数据、行业地位、竞争优势）、技术面（走势、成交量、关键指标）、消息面（近期公告、行业政策）。".to_string(),
                workflow_id: None,
                icon: "StockOutlined".to_string(),
                tags: vec!["finance".to_string(), "stock".to_string()],
            },
            IndustryActionConfig {
                key: "fi-financial-report".to_string(),
                label: "财报解读".to_string(),
                description: "深度解读上市公司财务报告".to_string(),
                action_type: ActionType::Conversation,
                system_prompt: "你是一位财务分析师，精通会计准则和财务报表分析。".to_string(),
                user_prompt_template: "请帮我解读 {{input}} 公司的最新财报，重点分析营收增长、利润率变化、现金流状况、资产负债表健康度，并与历史数据和同行对比。".to_string(),
                workflow_id: None,
                icon: "SolutionOutlined".to_string(),
                tags: vec!["finance".to_string(), "financial-report".to_string()],
            },
            IndustryActionConfig {
                key: "fi-valuation".to_string(),
                label: "估值计算".to_string(),
                description: "使用 DCF、PE 等方法进行公司估值".to_string(),
                action_type: ActionType::Conversation,
                system_prompt: "你是一位估值专家，精通 DCF、PE、PB、EV/EBITDA 等估值方法。".to_string(),
                user_prompt_template: "请使用多种方法对 {{input}} 公司进行估值，包括 DCF（假设增长率、折现率）、可比公司法（PE、PB 倍数）、历史估值法，并给出估值区间和投资建议。".to_string(),
                workflow_id: None,
                icon: "CalculatorOutlined".to_string(),
                tags: vec!["finance".to_string(), "valuation".to_string()],
            },
            IndustryActionConfig {
                key: "fi-risk".to_string(),
                label: "风险评估".to_string(),
                description: "评估投资风险和不确定性因素".to_string(),
                action_type: ActionType::Conversation,
                system_prompt: "你是一位风险管理专家，擅长识别和量化投资风险。".to_string(),
                user_prompt_template: "请评估 {{input}} 投资的主要风险因素，包括行业风险、公司风险、财务风险、政策风险、市场风险，并给出风险等级和对冲建议。".to_string(),
                workflow_id: None,
                icon: "SafetyOutlined".to_string(),
                tags: vec!["finance".to_string(), "risk".to_string()],
            },
        ],
        workflows: vec![
            IndustryWorkflowConfig {
                id: "wf-fi-1".to_string(),
                name: "A股深度研究".to_string(),
                description: "基本面分析 → 技术面分析 → 投资建议".to_string(),
                version: "1.0".to_string(),
                template_id: "workflow-fi-stock-research".to_string(),
            },
            IndustryWorkflowConfig {
                id: "wf-fi-2".to_string(),
                name: "行业对比分析".to_string(),
                description: "行业扫描 → 公司对比 → 选股建议".to_string(),
                version: "1.0".to_string(),
                template_id: "workflow-fi-industry-compare".to_string(),
            },
            IndustryWorkflowConfig {
                id: "wf-fi-3".to_string(),
                name: "投资组合优化".to_string(),
                description: "持仓分析 → 风险评估 → 调仓建议".to_string(),
                version: "1.0".to_string(),
                template_id: "workflow-fi-portfolio".to_string(),
            },
        ],
    }
}

fn sales_growth_config() -> IndustryConfig {
    IndustryConfig {
        id: "sales-growth".to_string(),
        name: "销售增长".to_string(),
        icon: "🚀".to_string(),
        description: "客户画像、转化漏斗、销售文案、竞品策略".to_string(),
        actions: vec![
            IndustryActionConfig {
                key: "sg-persona".to_string(),
                label: "客户画像".to_string(),
                description: "构建精准的目标客户画像".to_string(),
                action_type: ActionType::Conversation,
                system_prompt: "你是一位资深市场研究员，擅长用户画像和目标市场分析。".to_string(),
                user_prompt_template: "请帮我构建 {{input}} 产品/服务的目标客户画像，包括人口统计特征、行为特征、痛点需求、购买动机，并给出触达建议。".to_string(),
                workflow_id: None,
                icon: "TeamOutlined".to_string(),
                tags: vec!["sales".to_string(), "persona".to_string()],
            },
            IndustryActionConfig {
                key: "sg-funnel".to_string(),
                label: "转化漏斗分析".to_string(),
                description: "分析并优化用户转化路径".to_string(),
                action_type: ActionType::Conversation,
                system_prompt: "你是一位增长黑客，擅长转化率优化和用户行为分析。".to_string(),
                user_prompt_template: "请分析 {{input}} 的转化漏斗，从曝光 → 点击 → 注册 → 付费的每个环节，找出转化瓶颈并提供优化建议。".to_string(),
                workflow_id: None,
                icon: "FunnelPlotOutlined".to_string(),
                tags: vec!["sales".to_string(), "conversion".to_string()],
            },
            IndustryActionConfig {
                key: "sg-copy".to_string(),
                label: "销售文案生成".to_string(),
                description: "撰写高转化率的销售文案".to_string(),
                action_type: ActionType::Conversation,
                system_prompt: "你是一位资深文案策划，擅长撰写高转化率的营销文案。".to_string(),
                user_prompt_template: "请为 {{input}} 撰写销售文案，包括：标题（3 个版本）、卖点阐述、行动呼吁、FAQ。要求：结构清晰、痛点明确、利益点突出。".to_string(),
                workflow_id: None,
                icon: "EditOutlined".to_string(),
                tags: vec!["sales".to_string(), "copywriting".to_string()],
            },
            IndustryActionConfig {
                key: "sg-competitor".to_string(),
                label: "竞品销售策略".to_string(),
                description: "分析竞品销售策略并制定应对方案".to_string(),
                action_type: ActionType::Conversation,
                system_prompt: "你是一位竞品分析专家，擅长拆解竞争对手策略。".to_string(),
                user_prompt_template: "请分析 {{input}} 的主要竞争对手的销售策略，包括定价、渠道、促销、客户定位，并给出差异化竞争建议。".to_string(),
                workflow_id: None,
                icon: "TrophyOutlined".to_string(),
                tags: vec!["sales".to_string(), "competitor".to_string()],
            },
        ],
        workflows: vec![
            IndustryWorkflowConfig {
                id: "wf-sg-1".to_string(),
                name: "获客策略优化".to_string(),
                description: "渠道分析 → 策略制定 → 执行计划".to_string(),
                version: "1.0".to_string(),
                template_id: "workflow-sg-acquisition".to_string(),
            },
            IndustryWorkflowConfig {
                id: "wf-sg-2".to_string(),
                name: "转化提升方案".to_string(),
                description: "漏斗诊断 → 方案设计 → A/B 测试".to_string(),
                version: "1.0".to_string(),
                template_id: "workflow-sg-conversion".to_string(),
            },
            IndustryWorkflowConfig {
                id: "wf-sg-3".to_string(),
                name: "客户留存计划".to_string(),
                description: "流失分析 → 留存策略 → 复购设计".to_string(),
                version: "1.0".to_string(),
                template_id: "workflow-sg-retention".to_string(),
            },
        ],
    }
}

fn content_media_config() -> IndustryConfig {
    IndustryConfig {
        id: "content-media".to_string(),
        name: "内容媒体".to_string(),
        icon: "✏️".to_string(),
        description: "文章写作、SEO 优化、视频脚本、内容日历".to_string(),
        actions: vec![
            IndustryActionConfig {
                key: "cm-writing".to_string(),
                label: "文章写作".to_string(),
                description: "撰写高质量的长文或短文".to_string(),
                action_type: ActionType::Conversation,
                system_prompt: "你是一位资深内容创作者，擅长撰写各类文章。".to_string(),
                user_prompt_template: "请以专业的口吻撰写一篇关于 {{input}} 的文章，要求结构清晰、论据充分、语言流畅。字数不少于 1500 字。".to_string(),
                workflow_id: None,
                icon: "EditOutlined".to_string(),
                tags: vec!["content".to_string(), "writing".to_string()],
            },
            IndustryActionConfig {
                key: "cm-seo".to_string(),
                label: "SEO 优化".to_string(),
                description: "优化内容的搜索引擎可见性".to_string(),
                action_type: ActionType::Conversation,
                system_prompt: "你是一位 SEO 专家，熟悉搜索引擎算法和内容优化技巧。".to_string(),
                user_prompt_template: "请对以下内容进行 SEO 优化，包括关键词布局、标题结构、meta 描述、内部链接建议：\n\n{{input}}".to_string(),
                workflow_id: None,
                icon: "SearchOutlined".to_string(),
                tags: vec!["content".to_string(), "seo".to_string()],
            },
            IndustryActionConfig {
                key: "cm-video".to_string(),
                label: "视频脚本".to_string(),
                description: "撰写短视频或长视频脚本".to_string(),
                action_type: ActionType::Conversation,
                system_prompt: "你是一位资深视频编剧，擅长创作吸引眼球的视频脚本。".to_string(),
                user_prompt_template: "请为一个关于 {{input}} 的短视频撰写脚本（时长 1-3 分钟），包含：开场钩子、核心内容、CTA 行动号召。提供分镜描述。".to_string(),
                workflow_id: None,
                icon: "VidCameraOutlined".to_string(),
                tags: vec!["content".to_string(), "video".to_string()],
            },
            IndustryActionConfig {
                key: "cm-calendar".to_string(),
                label: "内容日历".to_string(),
                description: "规划内容发布节奏和主题".to_string(),
                action_type: ActionType::Conversation,
                system_prompt: "你是一位内容策划专家，擅长制定内容营销策略。".to_string(),
                user_prompt_template: "请为 {{input}} 品牌制定未来一个月的内容发布日历，包括主题规划、发布渠道、内容形式、关键时间节点。".to_string(),
                workflow_id: None,
                icon: "CalendarOutlined".to_string(),
                tags: vec!["content".to_string(), "planning".to_string()],
            },
        ],
        workflows: vec![
            IndustryWorkflowConfig {
                id: "wf-cm-1".to_string(),
                name: "爆款内容生成".to_string(),
                description: "选题策划 → 内容创作 → 优化打磨".to_string(),
                version: "1.0".to_string(),
                template_id: "workflow-cm-viral-content".to_string(),
            },
            IndustryWorkflowConfig {
                id: "wf-cm-2".to_string(),
                name: "多平台适配".to_string(),
                description: "内容创作 → 平台适配 → 分发策略".to_string(),
                version: "1.0".to_string(),
                template_id: "workflow-cm-multi-platform".to_string(),
            },
            IndustryWorkflowConfig {
                id: "wf-cm-3".to_string(),
                name: "IP 打造方案".to_string(),
                description: "人设定位 → 内容规划 → 粉丝运营".to_string(),
                version: "1.0".to_string(),
                template_id: "workflow-cm-ip-building".to_string(),
            },
        ],
    }
}

fn industry_consulting_config() -> IndustryConfig {
    IndustryConfig {
        id: "industry-consulting".to_string(),
        name: "行业咨询".to_string(),
        icon: "🎯".to_string(),
        description: "行业报告、市场预测、进入策略、竞品分析".to_string(),
        actions: vec![
            IndustryActionConfig {
                key: "ic-report".to_string(),
                label: "行业报告".to_string(),
                description: "撰写深度行业研究报告".to_string(),
                action_type: ActionType::Conversation,
                system_prompt: "你是一位资深行业分析师，擅长撰写深度研究报告。".to_string(),
                user_prompt_template: "请撰写一份关于 {{input}} 行业的深度研究报告，涵盖：行业概览、市场规模、竞争格局、发展趋势、主要玩家、投资机会。".to_string(),
                workflow_id: None,
                icon: "FileTextOutlined".to_string(),
                tags: vec!["consulting".to_string(), "report".to_string()],
            },
            IndustryActionConfig {
                key: "ic-forecast".to_string(),
                label: "市场预测".to_string(),
                description: "预测行业未来发展趋势".to_string(),
                action_type: ActionType::Conversation,
                system_prompt: "你是一位趋势预测专家，擅长分析行业发展规律。".to_string(),
                user_prompt_template: "请预测 {{input}} 行业未来 3-5 年的发展趋势，包括市场规模增长、技术变革、政策影响、竞争格局变化，并给出概率评估。".to_string(),
                workflow_id: None,
                icon: "LineChartOutlined".to_string(),
                tags: vec!["consulting".to_string(), "forecast".to_string()],
            },
            IndustryActionConfig {
                key: "ic-entry".to_string(),
                label: "进入策略".to_string(),
                description: "制定行业进入和扩张策略".to_string(),
                action_type: ActionType::Conversation,
                system_prompt: "你是一位战略咨询顾问，擅长市场进入策略制定。".to_string(),
                user_prompt_template: "请为 {{input}} 行业制定市场进入策略，包括：目标市场选择、进入模式（新建/并购/合作）、竞争定位、核心资源需求、风险评估。".to_string(),
                workflow_id: None,
                icon: "FlagOutlined".to_string(),
                tags: vec!["consulting".to_string(), "strategy".to_string()],
            },
            IndustryActionConfig {
                key: "ic-competitor".to_string(),
                label: "竞品分析".to_string(),
                description: "深度分析竞争对手优劣势".to_string(),
                action_type: ActionType::Conversation,
                system_prompt: "你是一位竞品分析专家，擅长拆解竞争对手。".to_string(),
                user_prompt_template: "请对 {{input}} 行业的主要竞争对手进行深度分析，包括：产品对比、定价策略、市场份额、SWOT 分析、竞争策略建议。".to_string(),
                workflow_id: None,
                icon: "FundOutlined".to_string(),
                tags: vec!["consulting".to_string(), "competitor".to_string()],
            },
        ],
        workflows: vec![
            IndustryWorkflowConfig {
                id: "wf-ic-1".to_string(),
                name: "行业扫描".to_string(),
                description: "市场调研 → 竞争分析 → 机会识别".to_string(),
                version: "1.0".to_string(),
                template_id: "workflow-ic-industry-scan".to_string(),
            },
            IndustryWorkflowConfig {
                id: "wf-ic-2".to_string(),
                name: "进入评估".to_string(),
                description: "吸引力评估 → 可行性分析 → 决策建议".to_string(),
                version: "1.0".to_string(),
                template_id: "workflow-ic-entry-eval".to_string(),
            },
            IndustryWorkflowConfig {
                id: "wf-ic-3".to_string(),
                name: "战略规划".to_string(),
                description: "愿景制定 → 路径规划 → 执行框架".to_string(),
                version: "1.0".to_string(),
                template_id: "workflow-ic-strategy".to_string(),
            },
        ],
    }
}

fn accounting_config() -> IndustryConfig {
    IndustryConfig {
        id: "accounting".to_string(),
        name: "会计".to_string(),
        icon: "💰".to_string(),
        description: "税务筹划、报表解读、成本分析、预算规划".to_string(),
        actions: vec![
            IndustryActionConfig {
                key: "ac-tax".to_string(),
                label: "税务筹划".to_string(),
                description: "合法合规优化税务结构".to_string(),
                action_type: ActionType::Conversation,
                system_prompt: "你是一位税务筹划专家，精通各国税法和税务优化方案。".to_string(),
                user_prompt_template: "请为 {{input}} 设计税务筹划方案，包括：合法节税策略、税收优惠利用、架构优化建议、风险提示。".to_string(),
                workflow_id: None,
                icon: "DollarCircleOutlined".to_string(),
                tags: vec!["accounting".to_string(), "tax".to_string()],
            },
            IndustryActionConfig {
                key: "ac-financials".to_string(),
                label: "报表解读".to_string(),
                description: "深度解读财务报表关键指标".to_string(),
                action_type: ActionType::Conversation,
                system_prompt: "你是一位财务分析专家，精通财务报表分析。".to_string(),
                user_prompt_template: "请解读 {{input}} 的财务报表，重点分析：收入趋势、利润变化、现金流质量、资产结构、关键财务比率，并与行业基准对比。".to_string(),
                workflow_id: None,
                icon: "DashboardOutlined".to_string(),
                tags: vec!["accounting".to_string(), "financials".to_string()],
            },
            IndustryActionConfig {
                key: "ac-cost".to_string(),
                label: "成本分析".to_string(),
                description: "拆解成本结构，识别优化空间".to_string(),
                action_type: ActionType::Conversation,
                system_prompt: "你是一位成本管理专家，擅长成本结构分析和优化。".to_string(),
                user_prompt_template: "请分析 {{input}} 的成本结构，识别主要成本项、成本驱动因素、优化机会，并提供降本建议。".to_string(),
                workflow_id: None,
                icon: "PieChartOutlined".to_string(),
                tags: vec!["accounting".to_string(), "cost".to_string()],
            },
            IndustryActionConfig {
                key: "ac-budget".to_string(),
                label: "预算规划".to_string(),
                description: "制定年度预算和资金计划".to_string(),
                action_type: ActionType::Conversation,
                system_prompt: "你是一位预算管理专家，擅长制定和执行预算计划。".to_string(),
                user_prompt_template: "请为 {{input}} 制定年度预算规划，包括：收入预测、成本预算、资本支出计划、现金流预测、关键财务目标。".to_string(),
                workflow_id: None,
                icon: "BarChartOutlined".to_string(),
                tags: vec!["accounting".to_string(), "budget".to_string()],
            },
        ],
        workflows: vec![
            IndustryWorkflowConfig {
                id: "wf-ac-1".to_string(),
                name: "税务优化".to_string(),
                description: "现状分析 → 筹划设计 → 方案评估".to_string(),
                version: "1.0".to_string(),
                template_id: "workflow-ac-tax-optimization".to_string(),
            },
            IndustryWorkflowConfig {
                id: "wf-ac-2".to_string(),
                name: "财务健康诊断".to_string(),
                description: "指标计算 → 健康评估 → 改进建议".to_string(),
                version: "1.0".to_string(),
                template_id: "workflow-ac-health-check".to_string(),
            },
            IndustryWorkflowConfig {
                id: "wf-ac-3".to_string(),
                name: "成本控制".to_string(),
                description: "成本分析 → 控制点识别 → 优化方案".to_string(),
                version: "1.0".to_string(),
                template_id: "workflow-ac-cost-control".to_string(),
            },
        ],
    }
}

fn ecommerce_config() -> IndustryConfig {
    IndustryConfig {
        id: "ecommerce".to_string(),
        name: "电商".to_string(),
        icon: "🛒".to_string(),
        description: "选品分析、定价策略、营销方案、店铺诊断".to_string(),
        actions: vec![
            IndustryActionConfig {
                key: "ec-selection".to_string(),
                label: "选品分析".to_string(),
                description: "基于市场数据选择热销商品".to_string(),
                action_type: ActionType::Conversation,
                system_prompt: "你是一位电商选品专家，擅长市场趋势分析和选品策略。".to_string(),
                user_prompt_template: "请分析 {{input}} 品类的电商选品机会，包括：市场容量、竞争格局、利润空间、趋势预测，推荐 Top 10 选品方向。".to_string(),
                workflow_id: None,
                icon: "SearchOutlined".to_string(),
                tags: vec!["ecommerce".to_string(), "selection".to_string()],
            },
            IndustryActionConfig {
                key: "ec-pricing".to_string(),
                label: "定价策略".to_string(),
                description: "制定有竞争力的定价方案".to_string(),
                action_type: ActionType::Conversation,
                system_prompt: "你是一位定价策略专家，精通电商定价模型。".to_string(),
                user_prompt_template: "请为 {{input}} 制定定价策略，包括：成本结构分析、竞品价格对比、价格带选择、促销定价建议。".to_string(),
                workflow_id: None,
                icon: "TagOutlined".to_string(),
                tags: vec!["ecommerce".to_string(), "pricing".to_string()],
            },
            IndustryActionConfig {
                key: "ec-marketing".to_string(),
                label: "营销方案".to_string(),
                description: "制定全渠道营销推广方案".to_string(),
                action_type: ActionType::Conversation,
                system_prompt: "你是一位电商营销专家，精通全渠道推广策略。".to_string(),
                user_prompt_template: "请为 {{input}} 制定营销推广方案，包括：核心卖点提炼、渠道组合（站内/站外/社交）、内容创意、投放策略、预算分配。".to_string(),
                workflow_id: None,
                icon: "NotificationOutlined".to_string(),
                tags: vec!["ecommerce".to_string(), "marketing".to_string()],
            },
            IndustryActionConfig {
                key: "ec-diagnosis".to_string(),
                label: "店铺诊断".to_string(),
                description: "全方位诊断店铺经营状况".to_string(),
                action_type: ActionType::Conversation,
                system_prompt: "你是一位电商运营诊断专家，擅长发现店铺问题和优化机会。".to_string(),
                user_prompt_template: "请对 {{input}} 店铺进行全方位诊断，包括：流量分析、转化分析、客单价分析、复购分析、竞品对比，找出 Top 5 问题点并给出优化建议。".to_string(),
                workflow_id: None,
                icon: "BugOutlined".to_string(),
                tags: vec!["ecommerce".to_string(), "diagnosis".to_string()],
            },
        ],
        workflows: vec![
            IndustryWorkflowConfig {
                id: "wf-ec-1".to_string(),
                name: "爆款打造".to_string(),
                description: "选品分析 → 卖点提炼 → 推广执行".to_string(),
                version: "1.0".to_string(),
                template_id: "workflow-ec-hit-product".to_string(),
            },
            IndustryWorkflowConfig {
                id: "wf-ec-2".to_string(),
                name: "竞品监控".to_string(),
                description: "竞品跟踪 → 策略分析 → 应对方案".to_string(),
                version: "1.0".to_string(),
                template_id: "workflow-ec-competitor-watch".to_string(),
            },
            IndustryWorkflowConfig {
                id: "wf-ec-3".to_string(),
                name: "大促策划".to_string(),
                description: "活动策划 → 资源准备 → 执行复盘".to_string(),
                version: "1.0".to_string(),
                template_id: "workflow-ec-promo-planning".to_string(),
            },
        ],
    }
}

fn education_config() -> IndustryConfig {
    IndustryConfig {
        id: "education".to_string(),
        name: "教育".to_string(),
        icon: "📚".to_string(),
        description: "课程设计、知识图谱、学习路径、教材生成".to_string(),
        actions: vec![
            IndustryActionConfig {
                key: "ed-course".to_string(),
                label: "课程设计".to_string(),
                description: "设计系统化的课程体系".to_string(),
                action_type: ActionType::Conversation,
                system_prompt: "你是一位课程设计专家，擅长系统化教学设计。".to_string(),
                user_prompt_template: "请为 {{input}} 设计一门完整课程，包括：目标学员画像、学习目标、课程大纲（10-15 章节）、教学方法、考核方式。".to_string(),
                workflow_id: None,
                icon: "ReadOutlined".to_string(),
                tags: vec!["education".to_string(), "course".to_string()],
            },
            IndustryActionConfig {
                key: "ed-knowledge".to_string(),
                label: "知识图谱".to_string(),
                description: "构建学科知识体系图谱".to_string(),
                action_type: ActionType::Conversation,
                system_prompt: "你是一位知识管理专家，擅长构建学科知识图谱。".to_string(),
                user_prompt_template: "请为 {{input}} 领域构建知识图谱，展示核心概念、它们之间的关系、学习依赖顺序，并以层级结构呈现。".to_string(),
                workflow_id: None,
                icon: "ApartmentOutlined".to_string(),
                tags: vec!["education".to_string(), "knowledge".to_string()],
            },
            IndustryActionConfig {
                key: "ed-path".to_string(),
                label: "学习路径".to_string(),
                description: "定制化学习路径规划".to_string(),
                action_type: ActionType::Conversation,
                system_prompt: "你是一位学习规划专家，擅长为不同背景的学员制定学习路径。".to_string(),
                user_prompt_template: "请为一个 {{input}} 背景的学员，规划从入门到精通的完整学习路径，包括阶段划分、推荐资源、实践项目、里程碑考核。".to_string(),
                workflow_id: None,
                icon: "RouteOutlined".to_string(),
                tags: vec!["education".to_string(), "learning-path".to_string()],
            },
            IndustryActionConfig {
                key: "ed-textbook".to_string(),
                label: "教材生成".to_string(),
                description: "自动生成结构化教材内容".to_string(),
                action_type: ActionType::Conversation,
                system_prompt: "你是一位教材编写专家，擅长系统化知识呈现。".to_string(),
                user_prompt_template: "请为 {{input}} 生成教材的目录结构和核心章节内容，要求：结构清晰、循序渐进、配有案例和练习。".to_string(),
                workflow_id: None,
                icon: "BookOutlined".to_string(),
                tags: vec!["education".to_string(), "textbook".to_string()],
            },
        ],
        workflows: vec![
            IndustryWorkflowConfig {
                id: "wf-ed-1".to_string(),
                name: "课程体系设计".to_string(),
                description: "需求分析 → 体系规划 → 内容开发".to_string(),
                version: "1.0".to_string(),
                template_id: "workflow-ed-course-system".to_string(),
            },
            IndustryWorkflowConfig {
                id: "wf-ed-2".to_string(),
                name: "学习路径规划".to_string(),
                description: "能力评估 → 路径设计 → 资源配置".to_string(),
                version: "1.0".to_string(),
                template_id: "workflow-ed-learning-path".to_string(),
            },
            IndustryWorkflowConfig {
                id: "wf-ed-3".to_string(),
                name: "教学内容开发".to_string(),
                description: "大纲设计 → 内容撰写 → 审核优化".to_string(),
                version: "1.0".to_string(),
                template_id: "workflow-ed-content-dev".to_string(),
            },
        ],
    }
}

// ── 配置注册表 ──────────────────────────────────────────────────

/// 获取所有 9 个行业配置
pub fn get_all_industry_configs() -> Vec<IndustryConfig> {
    vec![
        ai_research_config(),
        software_dev_config(),
        finance_invest_config(),
        sales_growth_config(),
        content_media_config(),
        industry_consulting_config(),
        accounting_config(),
        ecommerce_config(),
        education_config(),
    ]
}

/// 根据行业 ID 获取配置
pub fn get_industry_config(industry_id: &str) -> Option<IndustryConfig> {
    get_all_industry_configs().into_iter().find(|c| c.id == industry_id)
}

/// 根据行业 ID 和操作 key 获取操作配置
pub fn get_action_config(industry_id: &str, action_key: &str) -> Option<IndustryActionConfig> {
    get_industry_config(industry_id)
        .and_then(|config| config.actions.into_iter().find(|a| a.key == action_key))
}

/// 执行行业操作（写操作逻辑封装）
///
/// 为 Agent 提供真正的执行能力，返回一个包含完整执行上下文的 "执行包"。
/// Agent 可以将此包作为当前任务的 System Prompt 和初始 User Prompt。
pub fn execute_industry_action(
    industry_id: &str,
    action_key: &str,
    user_input: Option<&str>,
) -> Result<serde_json::Value, String> {
    let action = get_action_config(industry_id, action_key)
        .ok_or_else(|| format!("操作不存在: {industry_id}/{action_key}"))?;

    let user_prompt = match user_input {
        Some(input) if !input.trim().is_empty() => {
            action.user_prompt_template.replace("{{input}}", input)
        },
        _ => action.user_prompt_template.clone(),
    };

    Ok(serde_json::json!({
        "status": "executed",
        "industryId": industry_id,
        "actionKey": action.key,
        "actionLabel": action.label,
        "systemPrompt": action.system_prompt,
        "userPrompt": user_prompt,
        "executionId": format!("exec-{}-{}-{}", industry_id, action_key, uuid::Uuid::new_v4().simple()),
    }))
}

/// 创建行业工作流实例（写操作逻辑封装）
///
/// 允许 Agent 根据行业模板创建一个新的工作流实例。
/// 在实际项目中，这将涉及数据库写入和工作流引擎初始化。
pub fn create_industry_workflow(
    industry_id: &str,
    workflow_id: &str,
    custom_name: Option<&str>,
) -> Result<serde_json::Value, String> {
    let workflow = get_workflow_config(industry_id, workflow_id)
        .ok_or_else(|| format!("工作流不存在: {industry_id}/{workflow_id}"))?;

    let instance_id = format!("wf-instance-{}", uuid::Uuid::new_v4().simple());
    let name = custom_name.unwrap_or(&workflow.name).to_string();

    Ok(serde_json::json!({
        "status": "created",
        "instanceId": instance_id,
        "industryId": industry_id,
        "workflowTemplateId": workflow.id,
        "name": name,
        "description": workflow.description,
        "version": workflow.version,
        "createdAt": chrono::Utc::now().to_rfc3339(),
    }))
}

/// 根据行业 ID 和工作流 ID 获取工作流配置
pub fn get_workflow_config(industry_id: &str, workflow_id: &str) -> Option<IndustryWorkflowConfig> {
    get_industry_config(industry_id)
        .and_then(|config| config.workflows.into_iter().find(|w| w.id == workflow_id))
}

// ── Tauri 命令 ──────────────────────────────────────────────────

/// 获取所有行业列表（简要信息）
#[tauri::command]
pub async fn opc_list_industries() -> Result<serde_json::Value, String> {
    let configs = get_all_industry_configs();
    let list: Vec<serde_json::Value> = configs
        .into_iter()
        .map(|c| {
            serde_json::json!({
                "id": c.id,
                "name": c.name,
                "icon": c.icon,
                "description": c.description,
                "actionCount": c.actions.len(),
                "workflowCount": c.workflows.len(),
            })
        })
        .collect();
    serde_json::to_value(list).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 获取行业完整配置（含操作和工作流）
#[tauri::command]
pub async fn opc_get_industry_config(industry_id: String) -> Result<serde_json::Value, String> {
    let config =
        get_industry_config(&industry_id).ok_or_else(|| format!("行业不存在: {industry_id}"))?;
    serde_json::to_value(config).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 获取行业操作的执行配置（用于前端调用）
#[tauri::command]
pub async fn opc_get_action_config(
    industry_id: String,
    action_key: String,
) -> Result<serde_json::Value, String> {
    let action = get_action_config(&industry_id, &action_key)
        .ok_or_else(|| format!("操作不存在: {industry_id}/{action_key}"))?;
    serde_json::to_value(action).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 获取行业工作流配置
#[tauri::command]
pub async fn opc_get_workflow_config(
    industry_id: String,
    workflow_id: String,
) -> Result<serde_json::Value, String> {
    let workflow = get_workflow_config(&industry_id, &workflow_id)
        .ok_or_else(|| format!("工作流不存在: {industry_id}/{workflow_id}"))?;
    serde_json::to_value(workflow).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 构建带行业上下文的对话 prompt
/// 返回 system_prompt 和初始 user_prompt，供前端使用
#[tauri::command]
pub async fn opc_build_industry_prompt(
    industry_id: String,
    action_key: String,
    user_input: Option<String>,
) -> Result<serde_json::Value, String> {
    let action = get_action_config(&industry_id, &action_key)
        .ok_or_else(|| format!("操作不存在: {industry_id}/{action_key}"))?;

    let user_prompt = match user_input {
        Some(input) if !input.trim().is_empty() => {
            action.user_prompt_template.replace("{{input}}", &input)
        },
        _ => action.user_prompt_template.clone(),
    };

    Ok(serde_json::json!({
        "systemPrompt": action.system_prompt,
        "userPrompt": user_prompt,
        "actionKey": action.key,
        "actionLabel": action.label,
        "industryId": industry_id,
    }))
}

/// 获取行业所有操作列表
#[tauri::command]
pub async fn opc_list_industry_actions(industry_id: String) -> Result<serde_json::Value, String> {
    let config =
        get_industry_config(&industry_id).ok_or_else(|| format!("行业不存在: {industry_id}"))?;
    let actions: Vec<serde_json::Value> = config
        .actions
        .into_iter()
        .map(|a| {
            serde_json::json!({
                "key": a.key,
                "label": a.label,
                "description": a.description,
                "actionType": a.action_type,
                "icon": a.icon,
                "tags": a.tags,
            })
        })
        .collect();
    serde_json::to_value(actions).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 获取行业所有工作流列表
#[tauri::command]
pub async fn opc_list_industry_workflows(industry_id: String) -> Result<serde_json::Value, String> {
    let config =
        get_industry_config(&industry_id).ok_or_else(|| format!("行业不存在: {industry_id}"))?;
    let workflows: Vec<serde_json::Value> = config
        .workflows
        .into_iter()
        .map(|w| {
            serde_json::json!({
                "id": w.id,
                "name": w.name,
                "description": w.description,
                "version": w.version,
                "templateId": w.template_id,
            })
        })
        .collect();
    serde_json::to_value(workflows).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_all_industry_configs() {
        let configs = get_all_industry_configs();
        assert_eq!(configs.len(), 9, "应有 9 个行业配置");
    }

    #[test]
    fn test_get_industry_config() {
        let config = get_industry_config("ai-research").expect("AI 研究行业应存在");
        assert_eq!(config.name, "人工智能研究");
        assert_eq!(config.actions.len(), 4);
        assert_eq!(config.workflows.len(), 3);
    }

    #[test]
    fn test_get_action_config() {
        let action = get_action_config("ai-research", "ai-paper").expect("操作应存在");
        assert_eq!(action.label, "AI 论文调研");
        assert!(action.system_prompt.contains("AI 研究员"));
    }

    #[test]
    fn test_get_workflow_config() {
        let wf = get_workflow_config("ai-research", "wf-ai-research-1").expect("工作流应存在");
        assert_eq!(wf.name, "AI 技术调研");
    }

    #[test]
    fn test_build_industry_prompt_with_input() {
        let action = get_action_config("sales-growth", "sg-persona").unwrap();
        let prompt = action.user_prompt_template.replace("{{input}}", "SaaS");
        assert!(prompt.contains("SaaS"));
        assert!(prompt.contains("客户画像"));
    }

    #[test]
    fn test_unknown_industry_returns_none() {
        assert!(get_industry_config("nonexistent").is_none());
    }

    #[test]
    fn test_unknown_action_returns_none() {
        assert!(get_action_config("ai-research", "nonexistent").is_none());
    }
}

// ── 学习配置 ──────────────────────────────────────────────────

/// 行业学习配置
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IndustryLearningConfigView {
    pub version: u32,
    pub industry_id: String,
    pub industry_name: String,
    pub reflection_enabled: bool,
    pub evolution_enabled: bool,
    pub self_improvement_enabled: bool,
    pub reinforcement_learning_enabled: bool,
    /// 完整的强化学习配置
    pub reinforcement_learning: axagent_orchestrator::ReinforcementLearningConfig,
    pub config_path: String,
}

/// 获取行业学习配置
pub fn get_industry_learning_config(industry_id: &str) -> Option<IndustryLearningConfigView> {
    let industry_names: std::collections::HashMap<&str, &str> = [
        ("ai-research", "人工智能研究"),
        ("software-dev", "软件开发"),
        ("finance-invest", "金融投资"),
        ("sales-growth", "销售增长"),
        ("content-media", "内容媒体"),
        ("industry-consulting", "行业咨询"),
        ("accounting", "会计"),
        ("ecommerce", "电商运营"),
        ("education", "教育"),
    ]
    .into();

    let file_name = match industry_id {
        "ai-research" => "ai_research.yaml",
        "software-dev" => "software_dev.yaml",
        "finance-invest" => "finance_invest.yaml",
        "sales-growth" => "sales_growth.yaml",
        "content-media" => "content_media.yaml",
        "industry-consulting" => "industry_consulting.yaml",
        "accounting" => "accounting.yaml",
        "ecommerce" => "ecommerce.yaml",
        "education" => "education.yaml",
        _ => return None,
    };

    let config_path = format!("configs/industry_learning/{}", file_name);

    let content = match std::fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(_) => {
            tracing::warn!("学习配置文件不存在: {}", config_path);
            return None;
        },
    };

    let parsed: serde_json::Value = match serde_yaml::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("解析学习配置失败: {}, 错误: {}", config_path, e);
            return None;
        },
    };

    let industry_name = industry_names.get(industry_id).unwrap_or(&industry_id).to_string();
    let version = parsed.get("version").and_then(|v| v.as_u64()).unwrap_or(1) as u32;
    let reflection_enabled = parsed
        .get("reflection")
        .and_then(|r| r.get("enabled"))
        .and_then(|e| e.as_bool())
        .unwrap_or(true);
    let evolution_enabled = parsed
        .get("evolution")
        .and_then(|e| e.get("workflow_evolver"))
        .and_then(|w| w.get("enabled"))
        .and_then(|e| e.as_bool())
        .unwrap_or(false);
    let self_improvement_enabled = parsed
        .get("self_improvement")
        .and_then(|s| s.get("enabled"))
        .and_then(|e| e.as_bool())
        .unwrap_or(false);
    let reinforcement_learning_enabled = parsed
        .get("reinforcement_learning")
        .and_then(|r| r.get("enabled"))
        .and_then(|e| e.as_bool())
        .unwrap_or(false);

    // 解析完整的强化学习配置
    let reinforcement_learning: axagent_orchestrator::ReinforcementLearningConfig = parsed
        .get("reinforcement_learning")
        .and_then(|r| {
            serde_json::from_value::<axagent_orchestrator::ReinforcementLearningConfig>(r.clone())
                .ok()
        })
        .unwrap_or_default();

    Some(IndustryLearningConfigView {
        version,
        industry_id: industry_id.to_string(),
        industry_name,
        reflection_enabled,
        evolution_enabled,
        self_improvement_enabled,
        reinforcement_learning_enabled,
        reinforcement_learning,
        config_path,
    })
}

/// 获取所有行业学习配置列表
pub fn get_all_industry_learning_configs() -> Vec<IndustryLearningConfigView> {
    let industry_ids = [
        "ai-research",
        "software-dev",
        "finance-invest",
        "sales-growth",
        "content-media",
        "industry-consulting",
        "accounting",
        "ecommerce",
        "education",
    ];

    industry_ids.iter().filter_map(|id| get_industry_learning_config(id)).collect()
}

// ── Tauri 命令（学习配置） ──────────────────────────────────

/// 获取行业学习配置
#[tauri::command]
pub async fn opc_get_learning_config(industry_id: String) -> Result<serde_json::Value, String> {
    let config = get_industry_learning_config(&industry_id)
        .ok_or_else(|| format!("行业学习配置不存在: {industry_id}"))?;
    serde_json::to_value(config).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 获取所有行业学习配置
#[tauri::command]
pub async fn opc_list_learning_configs() -> Result<serde_json::Value, String> {
    let configs = get_all_industry_learning_configs();
    serde_json::to_value(configs).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 触发工作流反思
#[tauri::command]
pub async fn opc_reflect_on_workflow(
    state: State<'_, AppState>,
    industry_id: String,
    workflow_id: String,
    workflow_result: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let config = get_industry_learning_config(&industry_id)
        .ok_or_else(|| format!("行业学习配置不存在: {industry_id}"))?;

    if !config.reflection_enabled {
        return Err(format!("行业 {} 的反思功能未启用", industry_id));
    }

    // 获取行业适配器
    let registry = state.learning.industry_adapter_registry.lock().await;
    let adapter =
        registry.get(&industry_id).ok_or_else(|| format!("行业适配器不存在: {industry_id}"))?;

    let template = adapter.reflection_template().clone();
    drop(registry);

    // 构建反思请求
    let request = axagent_orchestrator::ReflectionRequest {
        industry_id: industry_id.clone(),
        workflow_id: workflow_id.clone(),
        workflow_result: workflow_result.clone(),
        ..Default::default()
    };

    // 调用学习引擎
    let engine = &state.learning.industry_learning_engine;
    let result = engine.reflect_on_workflow(&template, &request).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    serde_json::to_value(result).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 触发工作流进化
#[tauri::command]
pub async fn opc_evolve_workflow(
    state: State<'_, AppState>,
    industry_id: String,
    workflow_id: String,
    reason: String,
) -> Result<serde_json::Value, String> {
    let config = get_industry_learning_config(&industry_id)
        .ok_or_else(|| format!("行业学习配置不存在: {industry_id}"))?;

    if !config.evolution_enabled {
        return Err(format!("行业 {} 的进化功能未启用", industry_id));
    }

    // 获取行业适配器
    let registry = state.learning.industry_adapter_registry.lock().await;
    let adapter =
        registry.get(&industry_id).ok_or_else(|| format!("行业适配器不存在: {industry_id}"))?;

    let constraints = adapter.evolution_constraints().clone();
    drop(registry);

    // 构建进化请求
    let request = axagent_orchestrator::EvolutionRequest {
        industry_id: industry_id.clone(),
        workflow_id: workflow_id.clone(),
        reason: reason.clone(),
    };

    // 调用学习引擎
    let engine = &state.learning.industry_learning_engine;
    let result = engine.evolve_workflow(&constraints, &request).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    serde_json::to_value(result).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 执行自我改进
#[tauri::command]
pub async fn opc_run_self_improvement(
    state: State<'_, AppState>,
    industry_id: String,
    target: String,
) -> Result<serde_json::Value, String> {
    let config = get_industry_learning_config(&industry_id)
        .ok_or_else(|| format!("行业学习配置不存在: {industry_id}"))?;

    if !config.self_improvement_enabled {
        return Err(format!("行业 {} 的自我改进功能未启用", industry_id));
    }

    // 构建自我改进请求
    let request = axagent_orchestrator::SelfImprovementRequest {
        industry_id: industry_id.clone(),
        target: target.clone(),
    };

    // 调用学习引擎
    let engine = &state.learning.industry_learning_engine;
    let result = engine.run_self_improvement(&request).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    serde_json::to_value(result).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 触发行业学习闭环（自动模式）
///
/// 根据行业配置自动触发反思、进化和自我改进。
/// 通常在工作流执行完成后调用，实现 "执行 → 反思 → 进化 → 改进" 的闭环。
#[tauri::command]
pub async fn opc_trigger_industry_learning(
    state: State<'_, AppState>,
    industry_id: String,
    workflow_id: String,
    workflow_result: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let config = get_industry_learning_config(&industry_id)
        .ok_or_else(|| format!("行业学习配置不存在: {industry_id}"))?;

    // 获取行业适配器
    let registry = state.learning.industry_adapter_registry.lock().await;
    let adapter =
        registry.get(&industry_id).ok_or_else(|| format!("行业适配器不存在: {industry_id}"))?;

    let template = adapter.reflection_template().clone();
    let constraints = adapter.evolution_constraints().clone();
    let rl_config_from_adapter = adapter.learning_config().reinforcement_learning.clone();
    drop(registry);

    let engine = &state.learning.industry_learning_engine;
    let mut last_quality_score: f64 = 0.0;

    let mut reflection_result = serde_json::json!({
        "status": "skipped",
    });
    let mut evolution_result: Option<serde_json::Value> = None;
    let mut self_improvement_result: Option<serde_json::Value> = None;
    let mut rl_result: Option<serde_json::Value> = None;

    // 1. 触发反思（如果启用）
    if config.reflection_enabled {
        let request = axagent_orchestrator::ReflectionRequest {
            industry_id: industry_id.clone(),
            workflow_id: workflow_id.clone(),
            workflow_result: workflow_result.clone(),
            ..Default::default()
        };

        match engine.reflect_on_workflow(&template, &request).await {
            Ok(result) => {
                let quality_score = result.quality_score;
                last_quality_score = quality_score / 100.0; // 转换为 0.0-1.0 范围
                reflection_result = serde_json::json!({
                    "status": "success",
                    "quality_score": quality_score / 100.0,
                    "message": result.summary,
                });

                // 如果质量分数低于 70，自动触发进化
                if quality_score < 70.0 && config.evolution_enabled {
                    let evolution_request = axagent_orchestrator::EvolutionRequest {
                        industry_id: industry_id.clone(),
                        workflow_id: workflow_id.clone(),
                        reason: format!("质量分数较低 ({:.2})，触发进化优化", quality_score),
                    };

                    match engine.evolve_workflow(&constraints, &evolution_request).await {
                        Ok(evo_result) => {
                            evolution_result = Some(serde_json::json!({
                                "status": "success",
                                "reason": format!("质量分数较低 ({:.2})，触发进化优化", quality_score),
                                "message": evo_result.message,
                            }));
                        },
                        Err(e) => {
                            evolution_result = Some(serde_json::json!({
                                "status": "failed",
                                "reason": format!("质量分数较低 ({:.2})，触发进化优化", quality_score),
                                "message": format!("进化失败: {}", e),
                            }));
                        },
                    }
                }
            },
            Err(e) => {
                reflection_result = serde_json::json!({
                    "status": "failed",
                    "message": format!("反思失败: {}", e),
                });
            },
        }
    }

    // 2. 触发自我改进（如果启用且反思未失败）
    if config.self_improvement_enabled && reflection_result["status"] != "failed" {
        let improvement_request = axagent_orchestrator::SelfImprovementRequest {
            industry_id: industry_id.clone(),
            target: format!("workflow_{}_optimization", workflow_id),
        };

        match engine.run_self_improvement(&improvement_request).await {
            Ok(result) => {
                self_improvement_result = Some(serde_json::json!({
                    "status": "success",
                    "target": improvement_request.target,
                    "message": result.message,
                }));
            },
            Err(e) => {
                self_improvement_result = Some(serde_json::json!({
                    "status": "failed",
                    "target": improvement_request.target,
                    "message": format!("自我改进失败: {}", e),
                }));
            },
        }
    }

    // 3. 触发强化学习（如果启用）
    let rl_config = rl_config_from_adapter;
    if rl_config.enabled {
        // 读取 YAML 配置中的完整 RL 参数
        let full_rl_config = load_rl_config(&industry_id).unwrap_or(rl_config);

        match engine
            .run_reinforcement_learning(
                &industry_id,
                &workflow_id,
                last_quality_score,
                &workflow_result,
                &full_rl_config,
            )
            .await
        {
            Ok(rl_data) => {
                let has_experience =
                    rl_data.get("experienceRecorded").is_some_and(|v| !v.is_null());
                let pool_size = rl_data.get("poolSize").and_then(|v| v.as_u64()).unwrap_or(0);
                let policy_optimized =
                    rl_data.get("policyOptimized").and_then(|v| v.as_bool()).unwrap_or(false);
                rl_result = Some(serde_json::json!({
                    "status": "success",
                    "experience_recorded": has_experience,
                    "pool_size": pool_size,
                    "policy_optimized": policy_optimized,
                    "message": format!("RL 状态: {}", rl_data["status"]),
                }));
            },
            Err(e) => {
                rl_result = Some(serde_json::json!({
                    "status": "failed",
                    "message": format!("强化学习失败: {}", e),
                }));
            },
        }
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    Ok(serde_json::json!({
        "reflection": reflection_result,
        "evolution": evolution_result,
        "self_improvement": self_improvement_result,
        "reinforcement_learning": rl_result,
        "triggered_at": now,
    }))
}

// ── RL 辅助函数 ──────────────────────────────────────────

/// 从 YAML 文件加载完整的 RL 配置
pub(crate) fn load_rl_config(industry_id: &str) -> Option<ReinforcementLearningConfig> {
    let file_name = match industry_id {
        "ai-research" => "ai_research.yaml",
        "software-dev" => "software_dev.yaml",
        "finance-invest" => "finance_invest.yaml",
        "sales-growth" => "sales_growth.yaml",
        "content-media" => "content_media.yaml",
        "industry-consulting" => "industry_consulting.yaml",
        "accounting" => "accounting.yaml",
        "ecommerce" => "ecommerce.yaml",
        "education" => "education.yaml",
        _ => return None,
    };

    let config_path = format!("configs/industry_learning/{}", file_name);
    let content = std::fs::read_to_string(&config_path).ok()?;
    let parsed: serde_json::Value = serde_yaml::from_str(&content).ok()?;

    let rl_section = parsed.get("reinforcement_learning")?;

    let enabled = rl_section.get("enabled").and_then(|e| e.as_bool()).unwrap_or(false);
    let reward_model = rl_section.get("reward_model").and_then(|r| r.as_str()).map(String::from);
    let auto_train_threshold =
        rl_section.get("auto_train_threshold").and_then(|t| t.as_u64()).unwrap_or(50) as usize;
    let learning_rate = rl_section.get("learning_rate").and_then(|l| l.as_f64()).unwrap_or(0.01);
    let gamma = rl_section.get("gamma").and_then(|g| g.as_f64()).unwrap_or(0.95);
    let epsilon = rl_section.get("epsilon").and_then(|e| e.as_f64()).unwrap_or(0.1);

    let reward_weights = rl_section
        .get("reward_weights")
        .map(|w| {
            let quality = w.get("quality").and_then(|v| v.as_f64()).unwrap_or(0.35);
            let efficiency = w.get("efficiency").and_then(|v| v.as_f64()).unwrap_or(0.25);
            let cost = w.get("cost").and_then(|v| v.as_f64()).unwrap_or(0.15);
            let innovation = w.get("innovation").and_then(|v| v.as_f64()).unwrap_or(0.15);
            let satisfaction = w.get("satisfaction").and_then(|v| v.as_f64()).unwrap_or(0.1);
            RewardWeightConfig { quality, efficiency, cost, innovation, satisfaction }
        })
        .unwrap_or_default();

    let optimization_goals = rl_section
        .get("optimization_goals")
        .and_then(|g| g.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();

    Some(ReinforcementLearningConfig {
        enabled,
        reward_model,
        auto_train_threshold,
        learning_rate,
        gamma,
        epsilon,
        reward_weights,
        optimization_goals,
    })
}

// ── RL Tauri 命令 ──────────────────────────────────────────

/// 获取经验池统计信息
#[tauri::command]
pub async fn opc_get_rl_stats(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let engine = &state.learning.industry_learning_engine;
    let stats = engine.get_experience_pool_stats().await;
    serde_json::to_value(stats).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 手动触发 RL 策略优化
#[tauri::command]
pub async fn opc_trigger_rl_optimization(
    state: State<'_, AppState>,
    industry_id: String,
) -> Result<serde_json::Value, String> {
    let rl_config = load_rl_config(&industry_id)
        .ok_or_else(|| format!("行业 {} 的 RL 配置不存在", industry_id))?;

    if !rl_config.enabled {
        return Err(format!("行业 {} 的强化学习未启用", industry_id));
    }

    let engine = &state.learning.industry_learning_engine;
    let result = engine.optimize_policy(&industry_id, &rl_config).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    serde_json::to_value(result).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 手动记录一条工作流执行经验
#[tauri::command]
pub async fn opc_record_rl_experience(
    state: State<'_, AppState>,
    industry_id: String,
    workflow_id: String,
    quality_score: f64,
    workflow_result: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let rl_config = load_rl_config(&industry_id)
        .ok_or_else(|| format!("行业 {} 的 RL 配置不存在", industry_id))?;

    let engine = &state.learning.industry_learning_engine;
    let experience = engine
        .record_experience(&industry_id, &workflow_id, quality_score, &workflow_result, &rl_config)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    serde_json::to_value(experience).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}
