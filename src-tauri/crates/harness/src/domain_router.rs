// SPDX-License-Identifier: AGPL-3.0-only
//! L1 域路由规则 — 三层路由树第一层
//!
//! # 架构
//! ```text
//! 用户 Query → [L1 域路由] → 匹配业务域
//!                  │
//!                  ├── 命中规则 → 直接返回 Domain
//!                  └── 未命中 → LLM 兜底分类
//! ```
//!
//! # 规则优先级
//! 1. 关键词精确匹配（最高优先级）
//! 2. 正则表达式匹配
//! 3. LLM 语义分类（兜底）
//!
//! # 性能目标
//! - 纯规则路径: <1ms
//! - LLM 兜底: <2s

use crate::capability::CapabilityDomain;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ── 规则类型 ──────────────────────────────────────

/// L1 路由规则类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DomainRuleType {
    /// 关键词匹配（精确或包含）
    Keyword,
    /// 正则表达式匹配
    Regex,
    /// 语义标签匹配
    SemanticTag,
}

// ── 域路由规则 ──────────────────────────────────

/// L1 域路由规则 — 将用户 Query 映射到业务域
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainRoutingRule {
    /// 规则 ID（唯一）
    pub rule_id: String,
    /// 规则名称（中文，用于管理 UI）
    pub rule_name: String,
    /// 目标业务域
    pub target_domain: CapabilityDomain,
    /// 规则类型
    pub rule_type: DomainRuleType,
    /// 匹配值（关键词列表 / 正则表达式 / 语义标签）
    pub match_values: Vec<String>,
    /// 匹配模式（all = 全部匹配, any = 任一匹配）
    #[serde(default = "default_match_mode")]
    pub match_mode: MatchMode,
    /// 规则优先级（数字越大优先级越高，100 > 50）
    #[serde(default = "default_priority")]
    pub priority: i32,
    /// 是否启用
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// 规则描述
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 排除关键词（命中这些词则不匹配）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclude_keywords: Vec<String>,
}

fn default_match_mode() -> MatchMode {
    MatchMode::Any
}

fn default_priority() -> i32 {
    50
}

fn default_enabled() -> bool {
    true
}

/// 匹配模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchMode {
    /// 全部匹配（所有条件都要满足）
    All,
    /// 任一匹配（满足任一条件即可）
    Any,
}

impl DomainRoutingRule {
    pub fn new(
        rule_id: impl Into<String>,
        rule_name: impl Into<String>,
        target_domain: CapabilityDomain,
        rule_type: DomainRuleType,
        match_values: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            rule_id: rule_id.into(),
            rule_name: rule_name.into(),
            target_domain,
            rule_type,
            match_values: match_values.into_iter().map(|s| s.into()).collect(),
            match_mode: MatchMode::Any,
            priority: 50,
            enabled: true,
            description: None,
            exclude_keywords: vec![],
        }
    }

    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_match_mode(mut self, mode: MatchMode) -> Self {
        self.match_mode = mode;
        self
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn with_exclude(mut self, keywords: Vec<String>) -> Self {
        self.exclude_keywords = keywords;
        self
    }

    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    /// 测试用户 Query 是否命中此规则
    pub fn matches(&self, query: &str) -> bool {
        if !self.enabled {
            return false;
        }

        // 排除关键词检查
        if self.exclude_keywords.iter().any(|kw| query.contains(kw)) {
            return false;
        }

        let query_lower = query.to_lowercase();

        match self.rule_type {
            DomainRuleType::Keyword => match self.match_mode {
                MatchMode::All => {
                    self.match_values.iter().all(|kw| query_lower.contains(&kw.to_lowercase()))
                },
                MatchMode::Any => {
                    self.match_values.iter().any(|kw| query_lower.contains(&kw.to_lowercase()))
                },
            },
            DomainRuleType::Regex => {
                // regex 在实现层编译，harness 层只提供 match_values
                // 实际编译在 DomainRouterImpl 中完成
                self.match_values.iter().any(|pattern| {
                    if let Ok(re) = regex_lite(pattern) {
                        re.is_match(&query_lower)
                    } else {
                        false
                    }
                })
            },
            DomainRuleType::SemanticTag => {
                // 语义标签匹配需要 LLM 支持，规则本身不做判断
                false
            },
        }
    }
}

// ── 简易正则实现（避免引入 regex crate 到 harness） ──

/// 编译简易正则（仅支持 . * ? ^ $ [...] 和字面字符）
///
/// 完整正则支持在 implementor 层（dao / kit）用 regex crate 实现。
fn regex_lite(pattern: &str) -> Result<SimpleRegex, String> {
    Ok(SimpleRegex::compile(pattern))
}

/// 简易正则匹配器（支持 . * ? ^ $ [...] 和字面字符）
///
/// harness foundation 层零依赖约束下手写实现（递归回溯 + 字符类）：
/// - `.` 匹配任意单个字符
/// - `*` 前一 token 重复 0..n 次（贪心，带回溯）
/// - `?` 前一 token 出现 0 或 1 次
/// - `^` 锚定模式开头（仅当处于模式首 token）
/// - `$` 锚定模式结尾（仅当处于模式末 token）
/// - `[...]` 字符类，支持 `a-z` 范围与前导 `^` 取反
/// - 其余字符按字面匹配（大小写不敏感）
///
/// 注意：完整正则（`+`、`|`、`()`、反向引用等）在 implementor 层
/// 使用 regex crate 实现；此处仅覆盖路由规则的常见用法。
struct SimpleRegex {
    tokens: Vec<Token>,
    anchored_start: bool,
    anchored_end: bool,
}

impl SimpleRegex {
    fn compile(pattern: &str) -> SimpleRegex {
        let chars: Vec<char> = pattern.chars().collect();
        let n = chars.len();
        let mut anchored_start = false;
        let mut anchored_end = false;
        let mut tokens: Vec<Token> = Vec::new();
        let mut i = 0;

        while i < n {
            match chars[i] {
                '^' => {
                    if tokens.is_empty() {
                        anchored_start = true;
                    } else {
                        // 非开头位置的 ^ 按字面量处理（简化语义）
                        tokens.push(Token::literal('^'));
                    }
                    i += 1;
                },
                '$' => {
                    if i + 1 == n {
                        anchored_end = true;
                    } else {
                        tokens.push(Token::literal('$'));
                    }
                    i += 1;
                },
                '.' => {
                    tokens.push(Token {
                        atom: Atom { kind: AtomKind::Any, negated: false },
                        quantifier: Quantifier::ExactlyOne,
                    });
                    i += 1;
                },
                '[' => {
                    let (atom, next) = parse_class(&chars, i);
                    tokens.push(Token { atom, quantifier: Quantifier::ExactlyOne });
                    i = next;
                },
                '*' | '?' => {
                    if let Some(last) = tokens.last_mut() {
                        last.quantifier = if chars[i] == '*' {
                            Quantifier::ZeroOrMore
                        } else {
                            Quantifier::ZeroOrOne
                        };
                    } else {
                        // 孤立量词（无前置 token）按字面量处理
                        tokens.push(Token::literal(chars[i]));
                    }
                    i += 1;
                },
                c => {
                    tokens.push(Token::literal(c));
                    i += 1;
                },
            }
        }

        SimpleRegex { tokens, anchored_start, anchored_end }
    }

    fn is_match(&self, text: &str) -> bool {
        let text_chars: Vec<char> = text.chars().collect();
        let n = text_chars.len();

        // 无 `^` 锚定时尝试所有可能的起始位置
        let start_positions: Vec<usize> = if self.anchored_start {
            vec![0]
        } else {
            (0..=n).collect()
        };
        start_positions.into_iter().any(|start| self.match_from(&text_chars, start, 0))
    }

    /// 从指定位置消费 token 序列（递归回溯）
    fn match_from(&self, text: &[char], pos: usize, token_idx: usize) -> bool {
        if token_idx == self.tokens.len() {
            // 所有 token 已消费：若 `$` 锚定则必须到文本末尾
            return !self.anchored_end || pos == text.len();
        }

        let token = &self.tokens[token_idx];
        match token.quantifier {
            Quantifier::ExactlyOne => {
                token.matches(text, pos) && self.match_from(text, pos + 1, token_idx + 1)
            },
            Quantifier::ZeroOrOne => {
                // 尝试消费 1 次；失败则退化为消费 0 次
                if token.matches(text, pos) && self.match_from(text, pos + 1, token_idx + 1) {
                    true
                } else {
                    self.match_from(text, pos, token_idx + 1)
                }
            },
            Quantifier::ZeroOrMore => {
                // 贪心消费到最大，再回溯递减尝试
                let mut p = pos;
                while p < text.len() && token.matches(text, p) {
                    p += 1;
                }
                loop {
                    if self.match_from(text, p, token_idx + 1) {
                        return true;
                    }
                    if p == pos {
                        break;
                    }
                    p -= 1;
                }
                false
            },
        }
    }
}

/// 量词
#[derive(Debug, Clone, Copy)]
enum Quantifier {
    /// 恰好 1 次
    ExactlyOne,
    /// 0 或 1 次（?）
    ZeroOrOne,
    /// 0..n 次（*）
    ZeroOrMore,
}

/// 原子匹配单元（可能被量词修饰）
#[derive(Debug, Clone)]
struct Token {
    atom: Atom,
    quantifier: Quantifier,
}

impl Token {
    fn literal(c: char) -> Token {
        Token {
            atom: Atom { kind: AtomKind::Char(c), negated: false },
            quantifier: Quantifier::ExactlyOne,
        }
    }

    /// 当前位置是否命中（越界返回 false）
    fn matches(&self, text: &[char], pos: usize) -> bool {
        match text.get(pos) {
            Some(&c) => self.atom.matches(c),
            None => false,
        }
    }
}

/// 原子（字面字符 / 任意 / 字符类）
#[derive(Debug, Clone)]
struct Atom {
    kind: AtomKind,
    /// 字符类取反（[^...]）
    negated: bool,
}

impl Atom {
    fn matches(&self, c: char) -> bool {
        let c_low = c.to_lowercase().next().unwrap_or(c);
        let hit = match &self.kind {
            AtomKind::Char(ch) => ch.to_lowercase().next().unwrap_or(*ch) == c_low,
            AtomKind::Any => true,
            AtomKind::Class(ranges) => ranges.iter().any(|(lo, hi)| {
                let lo_low = lo.to_lowercase().next().unwrap_or(*lo);
                let hi_low = hi.to_lowercase().next().unwrap_or(*hi);
                c_low >= lo_low && c_low <= hi_low
            }),
        };
        if self.negated { !hit } else { hit }
    }
}

#[derive(Debug, Clone)]
enum AtomKind {
    /// 字面字符
    Char(char),
    /// 任意单字符（.）
    Any,
    /// 字符类（[a-z] 等，包含范围对，左闭右闭）
    Class(Vec<(char, char)>),
}

/// 解析 `[...]` 字符类；返回 (Atom, 消费后的下标)
///
/// 支持 `a-z` 范围与前导 `^` 取反；未闭合的 `[` 视为字面量兜底。
fn parse_class(chars: &[char], start: usize) -> (Atom, usize) {
    // start 指向 '['
    let n = chars.len();
    let mut i = start + 1;
    let mut negated = false;
    if i < n && chars[i] == '^' {
        negated = true;
        i += 1;
    }

    let mut ranges: Vec<(char, char)> = Vec::new();
    let mut closed = false;
    while i < n {
        let c = chars[i];
        if c == ']' {
            closed = true;
            i += 1;
            break;
        }
        // 范围 a-z（需要 i+2 存在且不是 ']'）
        if i + 2 < n && chars[i + 1] == '-' && chars[i + 2] != ']' {
            ranges.push((c, chars[i + 2]));
            i += 3;
        } else {
            ranges.push((c, c));
            i += 1;
        }
    }

    if !closed {
        // 未闭合：按字面量 '[' 处理（消费一个字符）
        return (Atom { kind: AtomKind::Char('['), negated: false }, start + 1);
    }

    (Atom { kind: AtomKind::Class(ranges), negated }, i)
}

// ── L1 域路由结果 ──────────────────────────────────

/// L1 域路由结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainRoutingResult {
    /// 命中的业务域
    pub domain: CapabilityDomain,
    /// 命中的规则（None 表示 LLM 兜底）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched_rule: Option<DomainRoutingRule>,
    /// 是否通过 LLM 兜底
    pub is_llm_fallback: bool,
    /// 置信度（0.0 - 1.0）
    pub confidence: f64,
    /// 路由耗时（毫秒）
    pub elapsed_ms: u64,
}

impl DomainRoutingResult {
    pub fn rule_hit(domain: CapabilityDomain, rule: DomainRoutingRule, elapsed_ms: u64) -> Self {
        Self {
            domain,
            confidence: 1.0,
            matched_rule: Some(rule),
            is_llm_fallback: false,
            elapsed_ms,
        }
    }

    pub fn llm_hit(domain: CapabilityDomain, confidence: f64, elapsed_ms: u64) -> Self {
        Self { domain, confidence, matched_rule: None, is_llm_fallback: true, elapsed_ms }
    }

    pub fn unknown(elapsed_ms: u64) -> Self {
        Self {
            domain: CapabilityDomain::General,
            confidence: 0.0,
            matched_rule: None,
            is_llm_fallback: false,
            elapsed_ms,
        }
    }
}

// ── 双层决策接口（设计 §4 三段闭环）─────────────────

/// L2 模型兜底推理器。输入用户 Query，返回候选域标识（如 "finance"/"research"）。
/// 由接线方注入实际 LLM 实现；`None`/无输入表示不启用模型兜底。
pub type LlmReasoner = dyn Fn(&str) -> std::pin::Pin<Box<dyn std::future::Future<Output = Option<String>> + Send>>
    + Send
    + Sync;

/// 双层决策结果（设计 §4）：规则优先，模型兜底，模型结果必须过规则关。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DomainDecision {
    /// 规则直接覆盖，未调用模型。
    Rule(Vec<DomainRoutingRule>),
    /// 规则未覆盖，模型兜底（已用规则复查交叉验证，无覆盖）。
    Llm { domain: CapabilityDomain, confidence: f64 },
    /// 规则与模型都未命中 → 通用域。
    General,
}

/// L1 域路由接口 — 纯规则 + LLM 兜底
///
/// # 路由流程
/// ```text
/// 1. 加载所有启用的规则（按优先级排序）
/// 2. 逐条规则匹配
///    - 命中 → 返回 Domain + 规则信息
///    - 未命中 → 继续下一条
/// 3. 所有规则未命中 → LLM 兜底分类
/// 4. LLM 也未命中 → 返回 General（通用域）
/// ```
#[async_trait]
pub trait DomainRouter: Send + Sync {
    /// 执行 L1 域路由
    async fn route(&self, query: &str) -> DomainRoutingResult;

    /// 获取所有规则
    async fn list_rules(&self) -> Vec<DomainRoutingRule>;

    /// 添加规则
    async fn add_rule(&self, rule: DomainRoutingRule) -> Result<(), String>;

    /// 更新规则
    async fn update_rule(&self, rule: DomainRoutingRule) -> Result<(), String>;

    /// 删除规则
    async fn remove_rule(&self, rule_id: &str) -> Result<(), String>;

    /// 按 ID 获取规则
    async fn get_rule(&self, rule_id: &str) -> Option<DomainRoutingRule>;

    /// 批量更新规则优先级
    async fn reorder_rules(&self, rule_ids: Vec<String>) -> Result<(), String>;

    /// 双层决策三段流水线（设计 §4）。
    ///
    /// 1. **规则优先**：规则命中即直接用规则结果，不调模型（低风险高频走规则）；
    /// 2. **模型兜底**：规则未覆盖才调用注入的 LLM 推理器分类（模糊才走模型）；
    /// 3. **规则复查**：模型输出再跑一次规则匹配交叉验证，命中仍以规则为准，
    ///    未命中则返回 `Llm`，兜底失败返回 `General`（模型结果必过规则关）。
    ///
    /// `llm_reasoner` 为可选注入；传 `None` 时跳过模型兜底，等同于普通 rule-only。
    async fn decide(
        &self,
        query: &str,
        llm_reasoner: Option<&std::sync::Arc<LlmReasoner>>,
    ) -> DomainDecision {
        // 1) 规则覆盖 → 直接用规则结果
        let route = self.route(query).await;
        if let Some(rule) = route.matched_rule {
            return DomainDecision::Rule(vec![rule]);
        }
        // 2) 规则未覆盖 → 模型兜底分类
        let llm_domain = match llm_reasoner {
            Some(reasoner) => reasoner(query).await,
            None => None,
        };
        let Some(domain) = llm_domain.and_then(|d| d.parse::<CapabilityDomain>().ok()) else {
            return DomainDecision::General;
        };
        // 3) 模型输出再过规则关：用 LLM 给出的域重跑规则匹配，
        //    若被规则覆盖则以规则为准（规则仍优先于模型）。
        if let Some(rule) = self.route(domain.as_str()).await.matched_rule {
            return DomainDecision::Rule(vec![rule]);
        }
        DomainDecision::Llm { domain, confidence: route.confidence.max(0.5) }
    }
}

// ── 内置默认规则集 ──────────────────────────────

/// 获取内置 L1 域路由规则（首次启动时注入）
///
/// 提供 9 个域的基础规则覆盖，确保开箱即用。
pub fn default_domain_rules() -> Vec<DomainRoutingRule> {
    vec![
        // ── 数据分析域 ──
        DomainRoutingRule::new(
            "rule_data_analysis_keywords",
            "数据分析-关键词匹配",
            CapabilityDomain::DataAnalysis,
            DomainRuleType::Keyword,
            vec![
                "数据",
                "图表",
                "分析",
                "统计",
                "报表",
                "excel",
                "csv",
                "公式",
                "函数",
                "计算",
                "求和",
                "平均",
                "趋势",
                "dashboard",
                "指标",
                "KPI",
                "数据透视",
                "pivot",
            ],
        )
        .with_priority(80)
        .with_description("数据分析相关关键词"),
        // ── 内容创作域 ──
        DomainRoutingRule::new(
            "rule_content_creation_keywords",
            "内容创作-关键词匹配",
            CapabilityDomain::ContentCreation,
            DomainRuleType::Keyword,
            vec![
                "写",
                "创作",
                "生成",
                "文章",
                "文案",
                "摘要",
                "翻译",
                "润色",
                "改写",
                "编辑",
                "blog",
                "post",
                "content",
                "文案",
                "标题",
                "大纲",
                "brainstorm",
            ],
        )
        .with_priority(75)
        .with_description("内容创作相关关键词"),
        // ── 通信域 ──
        DomainRoutingRule::new(
            "rule_communication_keywords",
            "通信-关键词匹配",
            CapabilityDomain::Communication,
            DomainRuleType::Keyword,
            vec![
                "邮件",
                "email",
                "发送",
                "通知",
                "消息",
                "chat",
                "通讯",
                "会议",
                "日程",
                "日历",
                "call",
                "meeting",
                "schedule",
                "协作",
                "collaboration",
                "slack",
            ],
        )
        .with_priority(70)
        .with_description("通信相关关键词"),
        // ── DevOps 域 ──
        DomainRoutingRule::new(
            "rule_devops_keywords",
            "DevOps-关键词匹配",
            CapabilityDomain::Devops,
            DomainRuleType::Keyword,
            vec![
                "部署",
                "deploy",
                "CI",
                "CD",
                "docker",
                "k8s",
                "kubernetes",
                "运维",
                "devops",
                "监控",
                "monitor",
                "日志",
                "log",
                "配置",
                "config",
                "环境",
                "env",
                "流水线",
                "pipeline",
            ],
        )
        .with_priority(70)
        .with_description("DevOps 相关关键词"),
        // ── AI 媒体域 ──
        DomainRoutingRule::new(
            "rule_ai_media_keywords",
            "AI媒体-关键词匹配",
            CapabilityDomain::AiMedia,
            DomainRuleType::Keyword,
            vec![
                "图片",
                "图像",
                "photo",
                "image",
                "生成",
                "video",
                "视频",
                "音频",
                "audio",
                "音乐",
                "music",
                "语音",
                "TTS",
                "ASR",
                "绘图",
                "画",
                "设计",
                "design",
                "插画",
                "illustration",
            ],
        )
        .with_priority(72)
        .with_description("AI媒体相关关键词"),
        // ── 金融域（业务标签 axinvest）──
        DomainRoutingRule::new(
            "rule_finance_keywords",
            "金融-关键词匹配",
            CapabilityDomain::Finance,
            DomainRuleType::Keyword,
            vec![
                "股票",
                "stock",
                "基金",
                "fund",
                "投资",
                "invest",
                "交易",
                "trade",
                "买入",
                "buy",
                "卖出",
                "sell",
                "行情",
                "market",
                "K线",
                "加密",
                "crypto",
                "比特币",
            ],
        )
        .with_priority(78)
        .with_description("金融相关关键词"),
        // ── 自动化域（业务标签 axopc）──
        DomainRoutingRule::new(
            "rule_automation_keywords",
            "自动化-关键词匹配",
            CapabilityDomain::Automation,
            DomainRuleType::Keyword,
            vec![
                "客户",
                "customer",
                "订单",
                "order",
                "产品",
                "product",
                "库存",
                "inventory",
                "仓库",
                "warehouse",
                "供应链",
                "销售",
                "sale",
                "采购",
                "purchase",
                "物流",
                "logistics",
            ],
        )
        .with_priority(68)
        .with_description("OPC相关关键词"),
        // ── 通用搜索/摘要 ──
        DomainRoutingRule::new(
            "rule_general_keywords",
            "通用-关键词匹配",
            CapabilityDomain::General,
            DomainRuleType::Keyword,
            vec![
                "搜索", "search", "查找", "查询", "什么", "怎么", "解释", "说明", "介绍", "总结",
                "概括",
            ],
        )
        .with_priority(20)
        .with_description("通用关键词（低优先级，兜底用）"),
    ]
}

// ── 默认域路由器实现 ──────────────────────────────

/// L1 域路由默认实现 — 纯规则匹配 + 运行时规则管理
///
/// # 路由流程
/// 1. 加载全部启用规则，按优先级降序排序
/// 2. 逐条匹配用户 Query，命中即返回 `rule_hit`
/// 3. 全部未命中 → 返回 `unknown`（通用域，置信度 0）
///
/// # LLM 兜底
/// harness foundation 层保持零依赖，LLM 语义兜底由 implementor 层
/// 以 `DomainRouter` trait 的包装实现扩展（待接入 Agent 模型）。
///
/// # 线程安全
/// 规则集合由 `tokio::sync::RwLock` 保护，支持运行时增删改与优先级调整，
/// 满足 AGENTS.md 铁律 8（禁止 `parking_lot::RwLock` 跨 await 持锁）。
pub struct DomainRouterImpl {
    rules: tokio::sync::RwLock<Vec<DomainRoutingRule>>,
}

impl DomainRouterImpl {
    /// 创建默认实现，预载内置规则集（9 域关键词规则）
    pub fn new() -> Self {
        Self { rules: tokio::sync::RwLock::new(default_domain_rules()) }
    }

    /// 以自定义规则集创建（用于测试或运行时覆盖）
    pub fn with_rules(rules: Vec<DomainRoutingRule>) -> Self {
        Self { rules: tokio::sync::RwLock::new(rules) }
    }
}

impl Default for DomainRouterImpl {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DomainRouter for DomainRouterImpl {
    async fn route(&self, query: &str) -> DomainRoutingResult {
        let start = std::time::Instant::now();
        let rules = self.rules.read().await;

        // 按优先级降序排序（稳定排序，同优先级保持添加顺序）
        let mut sorted: Vec<&DomainRoutingRule> = rules.iter().collect();
        sorted.sort_by_key(|b| std::cmp::Reverse(b.priority));

        for rule in sorted {
            if rule.matches(query) {
                let elapsed = start.elapsed().as_millis() as u64;
                tracing::debug!(
                    rule_id = %rule.rule_id,
                    domain = %rule.target_domain.as_str(),
                    "L1 域路由规则命中"
                );
                return DomainRoutingResult::rule_hit(rule.target_domain, rule.clone(), elapsed);
            }
        }

        let elapsed = start.elapsed().as_millis() as u64;
        tracing::debug!(query, "L1 全部规则未命中，返回通用域（LLM 兜底待 implementor 扩展）");
        DomainRoutingResult::unknown(elapsed)
    }

    async fn list_rules(&self) -> Vec<DomainRoutingRule> {
        self.rules.read().await.clone()
    }

    async fn add_rule(&self, rule: DomainRoutingRule) -> Result<(), String> {
        let mut rules = self.rules.write().await;
        if rules.iter().any(|r| r.rule_id == rule.rule_id) {
            return Err(format!("规则 ID 已存在: {}", rule.rule_id));
        }
        rules.push(rule);
        Ok(())
    }

    async fn update_rule(&self, rule: DomainRoutingRule) -> Result<(), String> {
        let mut rules = self.rules.write().await;
        let idx = rules
            .iter()
            .position(|r| r.rule_id == rule.rule_id)
            .ok_or_else(|| format!("规则不存在: {}", rule.rule_id))?;
        rules[idx] = rule;
        Ok(())
    }

    async fn remove_rule(&self, rule_id: &str) -> Result<(), String> {
        let mut rules = self.rules.write().await;
        let before = rules.len();
        rules.retain(|r| r.rule_id != rule_id);
        if rules.len() == before {
            Err(format!("规则不存在: {}", rule_id))
        } else {
            Ok(())
        }
    }

    async fn get_rule(&self, rule_id: &str) -> Option<DomainRoutingRule> {
        self.rules.read().await.iter().find(|r| r.rule_id == rule_id).cloned()
    }

    async fn reorder_rules(&self, rule_ids: Vec<String>) -> Result<(), String> {
        if rule_ids.len() != self.rules.read().await.len() {
            return Err("规则数量不匹配，必须包含全部规则 ID".to_string());
        }
        let mut rules = self.rules.write().await;
        // 按 rule_ids 顺序重新分配优先级（100 递减），route 时据此排序生效
        let mut next_priority = 100;
        for id in &rule_ids {
            let rule = rules
                .iter_mut()
                .find(|r| &r.rule_id == id)
                .ok_or_else(|| format!("规则 ID 不存在: {}", id))?;
            rule.priority = next_priority;
            next_priority -= 10;
        }
        Ok(())
    }
}
