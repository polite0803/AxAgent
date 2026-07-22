// analyst-brief.rhai — 分析师摘要生成器
//
// 在分析师阶段结束后运行，从10位分析师的 VERDICT 标签中提取
// 结构化评分和关键论据（bull_points / bear_points），压缩为一份
// 轻量摘要供辩论阶段使用，避免每位辨手加载10份完整报告。
//
// 输入：a_market_raw 等10个分析师节点 .content 文本（via input_mapping）
// 输出：analyst-brief — 按分析师维度的评分+关键论据摘要字符串
//
// 兜底：即使 LLM 未输出 bull_points/bear_points，评分字段仍可用

fn present(x) { type_of(x) != "()" }

fn extract_verdict(content) {
    if type_of(content) != "string" || content == "" { return (); }
    let lines = content.split("\n");
    if lines.len() == 0 { return (); }
    let last = lines[lines.len()-1].trim();
    let start = last.index_of("<!-- VERDICT: ");
    if start == -1 { return (); }
    let end = last.index_of(" -->");
    if end == -1 { return (); }
    // start+15 = "<!-- VERDICT: ".len()
    let json_str = last.sub_string(start + 15, end - start - 15);
    try {
        json_parse(json_str)
    } catch (e) {
        ()
    }
}

fn format_analyst(label, content) {
    let verdict = extract_verdict(content);
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
    // 共识分数 = bull - bear（与 build_analyst_input_mapping 一致）
    let cons = if type_of(bull) == "i64" && type_of(bear) == "i64" {
        bull - bear
    } else if type_of(bull) == "f64" && type_of(bear) == "f64" {
        bull - bear
    } else if type_of(bull) == "i64" && type_of(bear) == "f64" {
        (bull as f64) - bear
    } else if type_of(bull) == "f64" && type_of(bear) == "i64" {
        bull - (bear as f64)
    } else { "" };

    let mut brief = `#### ${label}\n`;
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

// 10 个分析师维度（节点短名 = 原 a-{id}.content 通过 input_mapping 注入）
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
