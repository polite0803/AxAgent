// analyst-brief.rhai — 分析师摘要生成器
//
// 在分析师阶段结束后运行，从10位分析师的 verdict map 中提取
// 结构化评分和关键论据（bull_points / bear_points），压缩为一份
// 轻量摘要供辩论阶段使用，避免每位辨手加载10份完整报告。
//
// V58 修复(2026-07-23): input_mapping 已改为 {id}.content.verdict，
// resolve_var_path 在 Rust 侧自动解析 content JSON 并下钻到 verdict 字段，
// 注入到 Rhai 的变量已经是 verdict map（或 () 表示无数据）。
// 不再需要在 Rhai 中做字符串解析。

fn present(x) { type_of(x) != "()" }

fn format_analyst(label, verdict) {
    // verdict 直接来自 input_mapping 的 {id}.content.verdict 解析结果
    if type_of(verdict) != "map" {
        return `#### ${label}\n**数据不可用**（该分析师报告解析失败，无法提取评分）\n\n`;
    }
    let bull = verdict["bull_score"];
    let bear = verdict["bear_score"];
    // 评分数据不可用时标记
    let bull_valid = type_of(bull) == "i64" || type_of(bull) == "f64";
    let bear_valid = type_of(bear) == "i64" || type_of(bear) == "f64";
    if !bull_valid && !bear_valid {
        return `#### ${label}\n**数据不可用**（评分字段缺失）\n\n`;
    }
    // 共识分数 = bull - bear
    let cons = if type_of(bull) == "i64" && type_of(bear) == "i64" {
        bull - bear
    } else if type_of(bull) == "f64" && type_of(bear) == "f64" {
        bull - bear
    } else if type_of(bull) == "i64" && type_of(bear) == "f64" {
        bull.to_float() - bear
    } else if type_of(bull) == "f64" && type_of(bear) == "i64" {
        bull - bear.to_float()
    } else { "" };

    let brief = `#### ${label}\n`;
    brief += `看多=${bull} 看空=${bear}`;
    if type_of(cons) == "i64" || type_of(cons) == "f64" {
        brief += ` 共识=${cons}`;
    }
    brief += "\n";

    // 可选：关键论据
    let bull_pts = verdict["bull_points"];
    if type_of(bull_pts) == "array" && bull_pts.len() > 0 {
        brief += "多方论据: " + join(bull_pts, "；") + "\n";
    }
    let bear_pts = verdict["bear_points"];
    if type_of(bear_pts) == "array" && bear_pts.len() > 0 {
        brief += "空方论据: " + join(bear_pts, "；") + "\n";
    }
    brief += "\n";
    brief
}

// ── 主逻辑 ──
let brief = "## 分析师综合摘要\n\n";

// 10 个分析师维度
// V58: 变量值直接是 verdict map（由 resolve_var_path 从 {id}.content.verdict 提取）
brief += format_analyst("行情技术面", a_market_raw);
brief += format_analyst("市场情绪", a_sentiment_raw);
brief += format_analyst("新闻公告", a_news_raw);
brief += format_analyst("基本面估值", a_fundamentals_raw);
brief += format_analyst("宏观政策", a_policy_raw);
brief += format_analyst("游资动向", a_hot_money_raw);
brief += format_analyst("解禁减持", a_lockup_raw);
brief += format_analyst("券商研报", a_research_raw);
brief += format_analyst("行业景气度", a_sector_raw);
brief += format_analyst("催化剂与叙事", a_catalyst_raw);

brief
