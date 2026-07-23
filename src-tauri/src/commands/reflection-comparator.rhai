// reflection-comparator.rhai — 反思工作流：预测 vs 实际定量对比 + untrusted 扫描 (P1-1/P0-2)
//
// 在 sub-analysis 之后、reflection-agent 之前执行。替代原 reflection_untrusted_aggregator.rhai。
// 对比分析师预测与实际走势 + 扫描上游 __untrusted 标记，输出结构化偏差报告。
//
// 设计原则：不依赖 LLM 解析，全确定性计算。
//
// 输入映射（由 DAG input_mapping 注入）：
//   trader_action       — 交易员最终动作（buy/hold/sell 等）
//   trader_target_price — 目标价（可选）
//   trader_confidence   — 交易员置信度（可选）
//   portfolio_action    — 组合投资经理最终 action
//   portfolio_posterior — posterior 概率
//   debate_consensus    — 辩论共识度（0-1）
//   total_score         — 技术评分（0-100）
//   raw_return_pct      — 实际收益率 %
//   alpha_return_pct    — 相对基准超额 %
//   holding_days        — 实际持有天数
//   original_time_horizon — 原始决策时间维度
//   original_holding_days — 原始期望持有天数
//   u_trader            — trader 节点 __untrusted 标记
//   u_research_mgr      — research-mgr 节点 __untrusted 标记
//   u_catalyst          — a-catalyst 节点 __untrusted 标记
//   u_debate_cnv        — debate-convergence 节点 __untrusted 标记
//   u_data_quality       — data-quality 节点 __untrusted 标记
//   u_risk_cnv          — risk-convergence 节点 __untrusted 标记
//
// 输出结构（deviation_report map）：
//   direction_match      — bool: 预测方向与实际方向是否一致
//   confidence_vs_return — string: 置信度 vs 实际收益的描述
//   return_category      — string: 收益分类
//   horizon_mismatch     — string: 时间维度匹配度
//   deviation_summary    — string: 单句偏差总结（≤200 字符）
//   key_findings         — array: 关键发现列表
//   untrusted_count      — i64: 不可信节点数
//   untrusted_sources    — array: 不可信节点名列表
//   sub_analysis_healthy — bool: 子工作流整体健康度

fn is_unit(x) { type_of(x) == "()" }

// 判断实际收益类别
fn classify_return(raw_return, horizon) {
    // 短线（≤5天）波动容忍大
    if horizon == "ultra_short" || horizon == "short" {
        if raw_return >= 5.0 { return "大幅盈利"; }
        if raw_return >= 1.5 { return "小幅盈利"; }
        if raw_return > -1.5 { return "基本持平"; }
        if raw_return > -4.0 { return "小幅亏损"; }
        return "大幅亏损";
    }
    // 中线（28天）
    if horizon == "mid" {
        if raw_return >= 10.0 { return "大幅盈利"; }
        if raw_return >= 3.0 { return "小幅盈利"; }
        if raw_return > -3.0 { return "基本持平"; }
        if raw_return > -8.0 { return "小幅亏损"; }
        return "大幅亏损";
    }
    // 长线（90天+）
    if raw_return >= 20.0 { return "大幅盈利"; }
    if raw_return >= 5.0 { return "小幅盈利"; }
    if raw_return > -5.0 { return "基本持平"; }
    if raw_return > -15.0 { return "小幅亏损"; }
    return "大幅亏损";
}

fn classify_confidence_level(c) {
    if is_unit(c) { return "未知"; }
    let v = c;
    if v >= 0.8 { return "非常高"; }
    if v >= 0.65 { return "高"; }
    if v >= 0.5 { return "中等"; }
    if v >= 0.35 { return "低"; }
    return "非常低";
}

fn main() {
    // ── 基本量提取（TypeScript → Rhai: Value::Null → ()）──
    let raw_return = if !is_unit(raw_return_pct) { raw_return_pct } else { 0.0 };
    let alpha_return = if !is_unit(alpha_return_pct) { alpha_return_pct } else { 0.0 };
    let actual_days = if !is_unit(holding_days) && type_of(holding_days) == "i64" {
        holding_days
    } else if !is_unit(holding_days) && type_of(holding_days) == "f64" {
        holding_days
    } else { 0 };

    let horizon = if !is_unit(original_time_horizon) && type_of(original_time_horizon) == "string" {
        original_time_horizon.to_string()
    } else { "unknown" };

    let expected_days = if !is_unit(original_holding_days) { original_holding_days } else { 0 };

    // ── 方向对比：交易员动作 vs 实际收益 ──
    let trader_act_str = if !is_unit(trader_action) && type_of(trader_action) == "string" {
        trader_action.to_string()
    } else { "" };

    // 判断 trader 方向（看多/看空/中性）
    let predicted_direction = if trader_act_str == "买入" || trader_act_str == "增持" || trader_act_str == "持有" {
        "看多"
    } else if trader_act_str == "卖出" || trader_act_str == "减持" {
        "看空"
    } else {
        "观望"
    };

    // 判断实际方向（正收益=看多正确，负收益=看空正确）
    let actual_direction = if raw_return >= 1.0 {
        "上涨"
    } else if raw_return <= -1.0 {
        "下跌"
    } else {
        "横盘"
    };

    // 方向是否一致
    let direction_match = (predicted_direction == "看多" && actual_direction == "上涨")
        || (predicted_direction == "看空" && actual_direction == "下跌");

    // ── 置信度 vs 收益评估 ──
    let trader_conf = if !is_unit(trader_confidence) { trader_confidence } else { 0.0 };
    let post = if !is_unit(portfolio_posterior) { portfolio_posterior } else { 0.0 };
    let conf_level = classify_confidence_level(post);

    let confidence_vs_return = if direction_match {
        if raw_return.abs() < 1.0 {
            `方向正确但收益有限(conf=${conf_level}, return=${raw_return})`
        } else {
            `方向正确(conf=${conf_level}, return=${raw_return}%)`
        }
    } else if predicted_direction == "观望" {
        `保持观望,实际${actual_direction}(${raw_return}%)`
    } else {
        `方向错误(预测${predicted_direction}→实际${actual_direction}, conf=${conf_level})`
    };

    // ── 时间维度匹配检查 ──
    let horizon_mismatch = if actual_days > 0 && expected_days > 0 {
        let ratio = actual_days.to_float() / expected_days.to_float();
        if ratio > 2.0 {
            `超期持有：实际${actual_days}天/预期${expected_days}天(${ratio}倍)`
        } else if ratio < 0.3 && actual_days < expected_days {
            `提前了结：实际${actual_days}天/预期${expected_days}天`
        } else {
            `时间匹配：实际${actual_days}天≈预期${expected_days}天`
        }
    } else {
        "时间维度数据不足"
    };

    // ── 收益率分类 ──
    let return_cat = classify_return(raw_return, horizon);

    // ── 聚合分数 ──
    let sc = if !is_unit(total_score) { total_score } else { 0 };
    let consensus = if !is_unit(debate_consensus) { debate_consensus } else { 0.0 };

    // ── untrusted 节点扫描（替代原 reflection_untrusted_aggregator.rhai）──
    let untrusted_sources = [];
    let sink = |flag, name| {
        if !is_unit(flag) && type_of(flag) == "bool" && flag {
            untrusted_sources.push(name);
        }
    };
    sink(u_trader, "trader");
    sink(u_research_mgr, "research-mgr");
    sink(u_catalyst, "catalyst");
    sink(u_debate_cnv, "debate-convergence");
    sink(u_data_quality, "data-quality");
    sink(u_risk_cnv, "risk-convergence");
    let untrusted_count = untrusted_sources.len();
    let sub_analysis_healthy = untrusted_count == 0;

    // ── 关键发现列表 ──
    let findings = [];

    // 1. 方向发现
    findings.push(
        if direction_match { `预测方向正确：${predicted_direction}→实际${actual_direction}` }
        else { `预测方向偏差：${predicted_direction}→实际${actual_direction}(收益${raw_return}%)` }
    );

    // 2. 超额发现
    if alpha_return != 0.0 {
        let alpha_sign = if alpha_return >= 0.0 { "+" } else { "" };
        findings.push(`相对基准超额：${alpha_sign}${alpha_return}%`);
    }

    // 3. 置信度发现
    if post > 0.0 {
        findings.push(`决策置信度${conf_level}(posterior=${post})`);
    }

    // 4. 评分发现
    if sc > 0 {
        findings.push(`技术评分${sc}/100`);
    }

    // 5. 辩论共识发现
    if consensus > 0.0 {
        findings.push(`辩论共识度${consensus}`);
    }

    // 6. untrusted 节点发现
    if untrusted_count > 0 {
        findings.push(`上游${untrusted_count}个节点不可信：${untrusted_sources}`);
    }

    // ── 偏差总结（≤200 字符）──
    let deviation_summary = if direction_match {
        if raw_return.abs() < 2.0 {
            `方向判断正确但收益有限(${raw_return}%)。后验=${post}，评分=${sc}`
        } else {
            `预测方向正确，实际收益${raw_return}%。后验=${post}，评分=${sc}`
        }
    } else if predicted_direction == "观望" {
        `实际${actual_direction}(${raw_return}%)但预测观望。评分=${sc}，共识=${consensus}`
    } else {
        `方向误判：预测${predicted_direction}但实际${actual_direction}(${raw_return}%)。后验=${post}，评分=${sc}`
    };

    #{
        "status": "ok",
        "direction_match": direction_match,
        "predicted_direction": predicted_direction,
        "actual_direction": actual_direction,
        "raw_return_pct": raw_return,
        "alpha_return_pct": alpha_return,
        "confidence_vs_return": confidence_vs_return,
        "return_category": return_cat,
        "confidence_level": conf_level,
        "horizon_mismatch": horizon_mismatch,
        "deviation_summary": deviation_summary,
        "key_findings": findings,
        "actual_holding_days": actual_days,
        "expected_holding_days": expected_days,
        "untrusted_count": untrusted_count,
        "untrusted_sources": untrusted_sources,
        "sub_analysis_healthy": sub_analysis_healthy
    }
}
