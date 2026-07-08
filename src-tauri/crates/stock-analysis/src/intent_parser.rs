//! NLU 意图解析 (P2-1)
//!
//! 将自然语言如"调研茅台短线"、"分析宁德时代中线"、"看看比亚迪"等
//! 解析为结构化的分析请求参数（股票代码、股票名称、时间周期、动作类型）。
//!
//! ## 设计
//!
//! 分两步：
//! 1. **正则提取**：使用预定义模式从文本中提取股票名称、周期、动作
//! 2. **搜索匹配**：提取的股票名称通过后端 vendor 搜索找代码

use serde::{Deserialize, Serialize};

/// 解析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedIntent {
    /// 原始输入
    pub raw_input: String,
    /// 提取的股票查询词（股票名称或代码）
    pub stock_query: Option<String>,
    /// 提取的股票代码（如果有）
    pub stock_code: Option<String>,
    /// 提取的时间周期
    pub time_horizon: Option<String>,
    /// 提取的动作类型
    pub action_type: String,
    /// 是否成功解析
    pub success: bool,
    /// 可读描述
    pub description: String,
}

/// 预定义时间周期关键词
const HORIZON_KEYWORDS: &[(&str, &str)] = &[
    ("超短", "ultra_short"),
    ("超短线", "ultra_short"),
    ("日内", "ultra_short"),
    ("短线", "short"),
    ("短期", "short"),
    ("中线", "mid"),
    ("中期", "mid"),
    ("中长线", "mid"),
    ("长线", "long"),
    ("长期", "long"),
    ("长持", "long"),
];

/// 提取动作类型（由内部匹配取代）
// const ACTION_KEYWORDS was removed, logic inline in extract_action()
/// 预定义的 A 股热门股票名称→代码映射（作为 fallback）
const HOT_STOCK_MAP: &[(&str, &str, &str)] = &[
    ("贵州茅台", "600519", "贵州茅台"),
    ("茅台", "600519", "贵州茅台"),
    ("中国平安", "601318", "中国平安"),
    ("平安", "601318", "中国平安"),
    ("招商银行", "600036", "招商银行"),
    ("招商", "600036", "招商银行"),
    ("宁德时代", "300750", "宁德时代"),
    ("宁德", "300750", "宁德时代"),
    ("五粮液", "000858", "五粮液"),
    ("美的集团", "000333", "美的集团"),
    ("美的", "000333", "美的集团"),
    ("恒瑞医药", "600276", "恒瑞医药"),
    ("恒瑞", "600276", "恒瑞医药"),
    ("东方财富", "300059", "东方财富"),
    ("东财", "300059", "东方财富"),
    ("中信证券", "600030", "中信证券"),
    ("中信", "600030", "中信证券"),
    ("比亚迪", "002594", "比亚迪"),
    ("迈瑞医疗", "300760", "迈瑞医疗"),
    ("迈瑞", "300760", "迈瑞医疗"),
    ("海康威视", "002415", "海康威视"),
    ("海康", "002415", "海康威视"),
    ("药明康德", "603259", "药明康德"),
    ("药明", "603259", "药明康德"),
    ("隆基绿能", "601012", "隆基绿能"),
    ("隆基", "601012", "隆基绿能"),
    ("长江电力", "600900", "长江电力"),
    ("长江", "600900", "长江电力"),
    ("紫金矿业", "601899", "紫金矿业"),
    ("紫金", "601899", "紫金矿业"),
    ("万华化学", "600309", "万华化学"),
    ("万华", "600309", "万华化学"),
    ("兴业银行", "601166", "兴业银行"),
    ("兴业", "601166", "兴业银行"),
    ("工商银行", "601398", "工商银行"),
    ("工行", "601398", "工商银行"),
    ("建设银行", "601939", "建设银行"),
    ("建行", "601939", "建设银行"),
    ("中国石油", "601857", "中国石油"),
    ("中石油", "601857", "中国石油"),
    ("中国海油", "600938", "中国海油"),
    ("中海油", "600938", "中国海油"),
    ("中芯国际", "688981", "中芯国际"),
    ("中芯", "688981", "中芯国际"),
    ("中科曙光", "603019", "中科曙光"),
    ("科大讯飞", "002230", "科大讯飞"),
    ("科大", "002230", "科大讯飞"),
    ("中兴通讯", "000063", "中兴通讯"),
    ("中兴", "000063", "中兴通讯"),
    ("阳光电源", "300274", "阳光电源"),
    ("阳光", "300274", "阳光电源"),
    ("汇川技术", "300124", "汇川技术"),
    ("汇川", "300124", "汇川技术"),
    ("格力电器", "000651", "格力电器"),
    ("格力", "000651", "格力电器"),
    ("海尔智家", "600690", "海尔智家"),
    ("海尔", "600690", "海尔智家"),
    ("山西汾酒", "600809", "山西汾酒"),
    ("汾酒", "600809", "山西汾酒"),
    ("泸州老窖", "000568", "泸州老窖"),
    ("老窖", "000568", "泸州老窖"),
    ("洋河股份", "002304", "洋河股份"),
    ("洋河", "002304", "洋河股份"),
    ("通威股份", "600438", "通威股份"),
    ("通威", "600438", "通威股份"),
    ("牧原股份", "002714", "牧原股份"),
    ("牧原", "002714", "牧原股份"),
    ("中际旭创", "300308", "中际旭创"),
    ("中际", "300308", "中际旭创"),
    ("韦尔股份", "603501", "韦尔股份"),
    ("韦尔", "603501", "韦尔股份"),
    ("北方华创", "002371", "北方华创"),
    ("北方", "002371", "北方华创"),
    ("兆易创新", "603986", "兆易创新"),
    ("兆易", "603986", "兆易创新"),
    ("金山办公", "688111", "金山办公"),
    ("金山", "688111", "金山办公"),
    ("三一重工", "600031", "三一重工"),
    ("三一", "600031", "三一重工"),
    ("中国中免", "601888", "中国中免"),
    ("中免", "601888", "中国中免"),
    ("紫光股份", "000938", "紫光股份"),
    ("紫光", "000938", "紫光股份"),
    ("澜起科技", "688008", "澜起科技"),
    ("澜起", "688008", "澜起科技"),
    ("华大九天", "301269", "华大九天"),
    ("华大", "301269", "华大九天"),
    ("传音控股", "688036", "传音控股"),
    ("传音", "688036", "传音控股"),
    ("百济神州", "688235", "百济神州"),
    ("百济", "688235", "百济神州"),
    ("百利天恒", "688506", "百利天恒"),
    ("寒武纪", "688256", "寒武纪"),
    ("赛力斯", "601127", "赛力斯"),
    ("赛力", "601127", "赛力斯"),
    ("理想汽车", "2015", "理想汽车"),
    ("蔚来", "9866", "蔚来"),
    ("小米", "1810", "小米集团"),
    ("小米集团", "1810", "小米集团"),
];

/// 解析自然语言分析意图
///
/// # 例子
/// - "调研茅台" → stock_query="茅台", action_type="research", time_horizon=None
/// - "分析宁德时代短线" → stock_query="宁德时代", action_type="research", time_horizon="short"
/// - "看看比亚迪中线" → stock_query="比亚迪", action_type="view", time_horizon="mid"
/// - "600519" → stock_code="600519"
/// - "茅台" → stock_query="茅台"
pub fn parse_analysis_intent(input: &str) -> ParsedIntent {
    let trimmed = input.trim();
    let _lower = trimmed.to_lowercase();

    // 1. 先检查是否是纯股票代码（6 位数字）
    if let Some(code) = extract_stock_code(trimmed) {
        let name = resolve_code_to_name(&code);
        let horizon = extract_horizon(trimmed);
        let action = extract_action(trimmed);
        return ParsedIntent {
            raw_input: trimmed.to_string(),
            stock_query: name.clone().or_else(|| Some(code.clone())),
            stock_code: Some(code.clone()),
            time_horizon: horizon.map(|h| h.to_string()),
            action_type: action.to_string(),
            success: true,
            description: format!(
                "代码识别: {} ({}){}",
                name.as_deref().unwrap_or(&code),
                &code,
                horizon.map(|h| format!(", {}", h)).unwrap_or_default()
            ),
        };
    }

    // 2. 尝试匹配热门股票名称
    for (name, code, full_name) in HOT_STOCK_MAP {
        if trimmed.contains(name) {
            let horizon = extract_horizon(trimmed);
            let action = extract_action(trimmed);
            return ParsedIntent {
                raw_input: trimmed.to_string(),
                stock_query: Some(full_name.to_string()),
                stock_code: Some(code.to_string()),
                time_horizon: horizon.map(|h| h.to_string()),
                action_type: action.to_string(),
                success: true,
                description: format!(
                    "名称匹配: {} ({}){}",
                    full_name,
                    code,
                    horizon.map(|h| format!(", {}", h)).unwrap_or_default()
                ),
            };
        }
    }

    // 3. 尝试提取所有关键词组合
    let horizon = extract_horizon(trimmed);
    let action = extract_action(trimmed);

    // 去除动作词和周期词，剩下的作为股票名称查询
    let stock_query = extract_stock_name(trimmed, &horizon);

    if let Some(ref query) = stock_query {
        // 找到匹配的代码
        let code = resolve_name_to_code(query);
        let code_ref = code.clone();
        if code.is_some() {
            return ParsedIntent {
                raw_input: trimmed.to_string(),
                stock_query: Some(query.clone()),
                stock_code: code,
                time_horizon: horizon.map(|h| h.to_string()),
                action_type: action.to_string(),
                success: true,
                description: format!(
                    "智能解析: {} ({}){}",
                    query,
                    code_ref.as_deref().unwrap_or("待搜索"),
                    horizon.map(|h| format!(", {}", h)).unwrap_or_default()
                ),
            };
        }

        return ParsedIntent {
            raw_input: trimmed.to_string(),
            stock_query: Some(query.clone()),
            stock_code: None,
            time_horizon: horizon.map(|h| h.to_string()),
            action_type: action.to_string(),
            success: true,
            description: format!(
                "提取股票名: {} (需搜索确认代码){}",
                query,
                horizon.map(|h| format!(", {}", h)).unwrap_or_default()
            ),
        };
    }

    // 4. 完全无法解析
    ParsedIntent {
        raw_input: trimmed.to_string(),
        stock_query: Some(trimmed.to_string()),
        stock_code: None,
        time_horizon: None,
        action_type: action.to_string(),
        success: false,
        description: "未能识别明确的股票信息，将使用原始输入作为搜索关键词".into(),
    }
}

/// 提取 6 位股票代码
fn extract_stock_code(input: &str) -> Option<String> {
    let cleaned = input.trim();

    // 纯 6 位数字
    if cleaned.len() == 6 && cleaned.chars().all(|c| c.is_ascii_digit()) {
        return Some(cleaned.to_string());
    }

    // 前缀提到的 6 位数字（如 "600519 茅台"）
    for word in cleaned.split_whitespace() {
        let w = word.trim();
        if w.len() == 6 && w.chars().all(|c| c.is_ascii_digit()) {
            return Some(w.to_string());
        }
    }

    None
}

/// 提取时间周期
fn extract_horizon(input: &str) -> Option<&'static str> {
    for (keyword, horizon) in HORIZON_KEYWORDS {
        if input.contains(keyword) {
            return Some(horizon);
        }
    }
    None
}

/// 提取动作类型
fn extract_action(input: &str) -> &'static str {
    // 先检查"查看类"动作（轻量）
    for kw in &["看看", "查看", "查", "关注"] {
        if input.contains(kw) {
            return "view";
        }
    }
    // 再检查"分析类"动作（深度）
    for kw in &[
        "调研", "分析", "研究", "评估", "评价", "诊断", "扫描", "监测",
    ] {
        if input.contains(kw) {
            return "research";
        }
    }
    "view"
}

/// 提取股票名称（去掉动作词和周期词后的剩余部分）
fn extract_stock_name(input: &str, horizon: &Option<&'static str>) -> Option<String> {
    let mut result = input.to_string();

    // 去掉动作词
    for kw in &[
        "调研", "分析", "看看", "研究", "查", "查看", "评估", "评价", "诊断", "扫描", "监测",
        "关注",
    ] {
        result = result.replace(kw, "");
    }

    // 去掉周期词
    if let Some(h) = horizon {
        for (keyword, _) in HORIZON_KEYWORDS.iter().filter(|(_, v)| *v == *h) {
            result = result.replace(keyword, "");
        }
    }

    let trimmed = result.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// 从热门股票映射中根据名称查找代码
fn resolve_name_to_code(name: &str) -> Option<String> {
    let lower = name.to_lowercase();
    for (n, code, _) in HOT_STOCK_MAP {
        if n.to_lowercase() == lower || lower.contains(&n.to_lowercase()) {
            return Some(code.to_string());
        }
    }
    None
}

/// 从代码反查名称
fn resolve_code_to_name(code: &str) -> Option<String> {
    for (_, c, full_name) in HOT_STOCK_MAP {
        if *c == code {
            return Some(full_name.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_stock_code_directly() {
        let r = parse_analysis_intent("600519");
        assert!(r.success);
        assert_eq!(r.stock_code.as_deref(), Some("600519"));
        assert_eq!(r.stock_query.as_deref(), Some("贵州茅台"));
    }

    #[test]
    fn parses_natural_language_research() {
        let r = parse_analysis_intent("调研茅台短线");
        assert!(r.success);
        assert_eq!(r.stock_code.as_deref(), Some("600519"));
        assert_eq!(r.time_horizon.as_deref(), Some("short"));
        assert_eq!(r.action_type, "research");
    }

    #[test]
    fn parses_analysis_with_horizon() {
        let r = parse_analysis_intent("分析宁德时代中线");
        assert!(r.success);
        assert_eq!(r.stock_code.as_deref(), Some("300750"));
        assert_eq!(r.time_horizon.as_deref(), Some("mid"));
    }

    #[test]
    fn parses_view_action() {
        let r = parse_analysis_intent("看看比亚迪");
        assert!(r.success);
        assert_eq!(r.stock_code.as_deref(), Some("002594"));
        assert_eq!(r.action_type, "view");
    }

    #[test]
    fn parses_horizon_ultra_short() {
        let r = parse_analysis_intent("分析招商银行超短线");
        assert!(r.success);
        assert_eq!(r.stock_code.as_deref(), Some("600036"));
        assert_eq!(r.time_horizon.as_deref(), Some("ultra_short"));
    }

    #[test]
    fn parses_horizon_long() {
        let r = parse_analysis_intent("研究美的长线");
        assert!(r.success);
        assert_eq!(r.stock_code.as_deref(), Some("000333"));
        assert_eq!(r.time_horizon.as_deref(), Some("long"));
    }

    #[test]
    fn parses_code_before_name() {
        let r = parse_analysis_intent("300750 中线");
        assert!(r.success);
        assert_eq!(r.stock_code.as_deref(), Some("300750"));
        assert_eq!(r.time_horizon.as_deref(), Some("mid"));
    }

    #[test]
    fn unknown_stock_returns_query_only() {
        let r = parse_analysis_intent("调研未知公司短线");
        assert!(r.success);
        assert_eq!(r.stock_code, None);
        assert_eq!(r.stock_query.as_deref(), Some("未知公司"));
    }

    #[test]
    fn empty_horizon_when_not_specified() {
        let r = parse_analysis_intent("看看平安");
        assert!(r.success);
        assert_eq!(r.stock_code.as_deref(), Some("601318"));
        assert_eq!(r.time_horizon, None);
    }
}
