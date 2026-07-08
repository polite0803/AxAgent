// portfolio-mgr.rhai — 贝叶斯因子融合 + 凯利仓位
//
// 理论依据：
//   贝叶斯更新: P(上涨|证据) ∝ P(证据|上涨) × P(上涨)
//   凯利公式:   f* = (p × odds - q) / odds
//     p = P(上涨|证据)   q = 1-p
//     odds = 预期收益% / 预期亏损%
//
// 数据驱动（P1/P2）：
//   - 先验 P(上涨) = t-scoring 回测的市场状态（牛市0.55/熊市0.45/中性0.50）
//   - 因子权重 = t-scoring 回测的信息系数(IC)归一化
//
// 缺失因子不参与融合（不编造数据）。

fn present(x) { type_of(x) != "()" }

// ── V55: 累加上游 strict_mode 兜底哨兵 ──
// 任意上游 AgentNode 走 strict_mode 降级（LLM 输出无法解析为合法 JSON）时，
// 会通过 NodeOutput 顶层注入 __untrusted=true。本脚本读 input_mapping
// 把这些哨兵折算为 untrusted_count（0-6），用于触发 weights_collapsed 兜底。
// 这样 LLM 失败的"50/50 中性兜底"会被显式识别为不可信信号。
let untrusted_count = 0;
if present(untrusted_trader) && untrusted_trader == true { untrusted_count += 1; }
if present(untrusted_research_mgr) && untrusted_research_mgr == true { untrusted_count += 1; }
if present(untrusted_catalyst) && untrusted_catalyst == true { untrusted_count += 1; }
if present(untrusted_debate_conv) && untrusted_debate_conv == true { untrusted_count += 1; }
if present(untrusted_data_quality) && untrusted_data_quality == true { untrusted_count += 1; }
if present(untrusted_risk_conv) && untrusted_risk_conv == true { untrusted_count += 1; }

// ── 先验：数据驱动的市场状态 ──
let prior = if present(market_regime_prior) {
    market_regime_prior
} else { 0.50 };

// ── 因子权重：数据驱动的 IC 归一化 ──
// factor_weights 是 JSON 对象 {trend:{weight,win_rate,ic}, macd:{...}, ...}
// 从 Rhai 的 Map 中读取各因子权重
let f_weights = if present(factor_weights) && type_of(factor_weights) == "map" {
    factor_weights
} else { #{} };  // 空 map 触发等权 fallback

// 尝试从 factor_weights 读取权重；找不到则用等权 fallback
fn get_weight(weights, factor, fallback) {
    if type_of(weights) == "map" {
        let f = weights[factor];
        if type_of(f) != "()" && type_of(f) == "map" {
            let w = f["weight"];
            if type_of(w) == "f64" { w } else { fallback }
        } else { fallback }
    } else { fallback }
}

// ── 因子 1: 技术面 ──
let f1_default = 0.15;  // V49 修复: 从 0.25 下调 — 技术面(趋势跟踪)必然滞后,
                          // 股价涨完才触发"多头排列", 不应占最高权重
let f1_weight = if present(totalScore) { get_weight(f_weights, "trend", f1_default) } else { 0.0 };
let f1_signal = if present(totalScore) { (totalScore - 50.0) / 50.0 } else { 0.0 };

// ── 因子 2: 共识 ──
let f2_default = 0.25;
// V29 修复: 共识因子应使用独立的回测权重键 "consensus"，而非复用 "trend"
// factor_weights 结构为 {trend:{...}, macd:{...}, rsi:{...}, consensus:{...}, ...}
// 若回测数据无 "consensus" 键，get_weight 会 fallback 到 f2_default
let f2_weight = if present(consensusScore) { get_weight(f_weights, "consensus", f2_default) } else { 0.0 };
let f2_signal = if present(consensusScore) { (consensusScore - 50.0) / 50.0 } else { 0.0 };

// ── 公告关键词风险检测（P2 新增，f3 催化剂修正）──
// 数据源: t-catalyst-data → get_stock_announcements
// 检测公告标题中的风险/利好关键词，输出 [-0.3, 0.3] 修正信号
// 与 LLM catalyst_level（f3）正交，提供额外的公告级预警
// 注: 若 ann_type 含"风险提示"、title 含"可能下跌"等为高频预警信号
let announcement_modifier = if present(announcements) && type_of(announcements) == "string" && announcements != "" {
    let ann_list = json_parse(announcements);
    if type_of(ann_list) == "array" && ann_list.len() > 0 {
        let risk_count = 0;
        let positive_count = 0;
        let high_risk_count = 0;
        // 风险关键词（匹配公告标题/类型）
        let risk_kw = ["风险提示", "异常波动", "可能下跌", "可能快速下跌", "风险警示",
                       "退市", "立案调查", "监管函", "警示函", "减持", "业绩预亏",
                       "亏损", "ST", "*ST",
                       "股东减持", "被动减持", "强制平仓", "违约"];
        // 利好关键词
        let pos_kw = ["业绩预增", "扭亏为盈", "中标", "重大合同", "增持",
                      "回购", "股权激励", "分红", "送转",
                      "业绩预告向上", "业绩超预期"];
        for i in 0..ann_list.len() {
            let item = ann_list[i];
            if type_of(item) == "map" {
                let title = if type_of(item["title"]) == "string" { item["title"] } else { "" };
                let ann_type = if type_of(item["ann_type"]) == "string" { item["ann_type"] } else { "" };
                let combined = `${title} ${ann_type}`;
                // 高风险关键词检测
                for kw in risk_kw {
                    if combined.contains(kw) || combined.contains(kw.to_uppercase()) {
                        if kw == "退市" || kw == "立案调查" || kw == "可能快速下跌" || kw == "风险警示" {
                            high_risk_count = high_risk_count + 1;
                        } else {
                            risk_count = risk_count + 1;
                        }
                        break;
                    }
                }
                // 利好关键词检测
                for kw in pos_kw {
                    if combined.contains(kw) {
                        positive_count = positive_count + 1;
                        break;
                    }
                }
            }
        }
        // 合成修正信号：[-0.3, 0.3]
        // 每有一个高风险公告 → -0.15
        // 每有一个普通风险公告 → -0.05
        // 每有一个利好公告 → +0.05
        let raw_mod = high_risk_count * (-0.15) + risk_count * (-0.05) + positive_count * 0.05;
        clamp(raw_mod, -0.3, 0.3)
    } else { 0.0 }
} else { 0.0 };

// 将公告修改器应用到 f3 催化剂信号（仅当公告信号不为零时）
let f3_has_announcement_signal = announcement_modifier != 0.0;

// ── 因子 3: 催化剂 ──
// V37 修复: 催化剂信号来自 LLM（a-catalyst.content.catalyst_level），与技术面回测因子
// （volume=成交量）语义无关，不调用 get_weight 避免权重被无关回测数据绑架。
// 仅在 catalyst_level 非空且非"无"时激活默认权重；"无"表示 LLM 明确判断无催化剂。
// V38 修复: 扩展催化剂信号为双向（利好+利空），利空催化剂（退市预警/商誉爆雷等）
// 输出负信号拉低 posterior，避免对"有催化剂但实质利空"的股票给出偏多建议。
let f3_default = 0.20;
let f3_has_catalyst = present(catalyst_level) && catalyst_level != "" && catalyst_level != "无";
// P2 修复: 当有公告信号但无 LLM 催化剂时，仍以半权重激活 f3，使公告信号参与融合
let f3_weight = if f3_has_catalyst {
    f3_default
} else if f3_has_announcement_signal {
    f3_default * 0.5
} else { 0.0 };
let f3_signal = if f3_weight > 0.0 {
    // V40 修复: 用 contains 而非精确 switch 匹配，保证与 prompt 字段值对齐
    // prompt 输出格式: "L3估值体系级" / "L2业绩拐点级" / "L1普通消息"
    //                   "L-3退市/造假级" / "L-2业绩暴雷级" / "L-1普通利空"
    let llm_signal = if catalyst_level.contains("L-3") || catalyst_level.contains("退市") || catalyst_level.contains("造假") { -0.8 }
    else if catalyst_level.contains("L-2") || catalyst_level.contains("业绩暴雷") { -0.5 }
    else if catalyst_level.contains("L-1") || catalyst_level.contains("普通利空") { -0.2 }
    else if catalyst_level.contains("L3") || catalyst_level.contains("估值体系级") { 0.8 }
    else if catalyst_level.contains("L2") || catalyst_level.contains("业绩拐点级") { 0.5 }
    else if catalyst_level.contains("L1") || catalyst_level.contains("普通消息") { 0.2 }
    else { 0.0 };
    // P2 修复: 叠加公告关键词风险信号，让公告标题层面的风险/利好修正催化剂强度
    // 公告信号权重 0.3，催化剂 LLM 信号权重 0.7，二者加权
    // 例: LLM=0.5(L2利好) + 公告=-0.15(有风险提示) → 加权=0.5*0.7+(-0.15)*0.3=0.305
    // 例: LLM=0(无催化剂) + 公告=0.2(业绩预增) → 加权=0*0.7+0.2*0.3=0.06(仍偏正)
    // 例: LLM=0(无催化剂) + 公告=-0.3(高风险公告) → 加权=-0.09(偏负，提示风险)
    if f3_has_announcement_signal {
        clamp(llm_signal * 0.7 + announcement_modifier * 0.3, -1.0, 1.0)
    } else {
        llm_signal
    }
} else if f3_has_announcement_signal {
    // 无 LLM 催化剂但有公告信号 → 直接用公告信号（权重减半避免过度影响）
    announcement_modifier * 0.5
} else { 0.0 };

// ── 因子 4: 风险 ──
// V37 修复: 风险信号来自 LlmClassifier（risk-level.category），与技术面回测因子
// （rsi=RSI超买超卖）语义无关，直接用默认权重，避免被 RSI 回测数据绑架。
//
// V50 修复: 优化 f4_signal 映射
//   旧: 低→+0.5 中→0.0 高→-0.4 极高→-0.7
//   问题①: 中→0.0 意味着"中风险=无风险信号"，逻辑矛盾
//   问题②: 低→中跳变 0.5 但高→极高只跳 0.3，不对称
//   新: 基于风险等级在 0-100 上的位置线性映射再反转
//      低(20)→+0.40 中(50)→-0.10 高(80)→-0.40 极高(95)→-0.65
//   原则: 中风险≠无风险，应有轻微负信号；跳变更均匀
let f4_default = 0.15;
let f4_weight = if present(overall_risk) { f4_default } else { 0.0 };
let f4_signal = if f4_weight > 0.0 {
    switch overall_risk {
        "低风险"|"低" => 0.30,
        "中风险"|"中" => -0.05,   // V51: 中风险轻微负信号（原-0.10）
        "高风险"|"高" => -0.25,   // V51: 降低过度惩罚（原-0.40→-0.25）
        "极高风险"|"极高" => -0.50, // V51: 降低极端惩罚（原-0.65→-0.50）
        _ => 0.0
    }
} else { 0.0 };

// ── P0-2 补: 量化风险指标修正 f4_signal ──
// t-risk ToolNode 输出 stockRiskProfile 含波动率/回撤/夏普等真实风险指标
// 修复前: 这些指标通过 input_mapping 注入但未被 rhai 消费（数据流断裂）
// 修复后: 基于实际数值对 f4_signal 做合理修正
let f4_quant_adjust = 0.0;  // 累计修正量 ∈ [-0.15, +0.10]
let f4_quant_notes = [];
if present(risk_volatility) && risk_volatility > 0.0 {
    // 年化波动率: >60% 极高风险 | >40% 高风险 | <20% 低风险
    if risk_volatility > 60.0 {
        f4_quant_adjust -= 0.12;
        f4_quant_notes.push(`年化波动${(risk_volatility*10.0).round()/10.0}%（极高）`);
    } else if risk_volatility > 40.0 {
        f4_quant_adjust -= 0.06;
        f4_quant_notes.push(`年化波动${(risk_volatility*10.0).round()/10.0}%（偏高）`);
    } else if risk_volatility < 20.0 {
        f4_quant_adjust += 0.05;
    }
}
if present(risk_drawdown) && risk_drawdown > 0.0 {
    // 最大回撤: >50% 触发 VaR 预警 | >30% 注意
    if risk_drawdown > 50.0 {
        f4_quant_adjust -= 0.10;
        f4_quant_notes.push(`最大回撤${(risk_drawdown*10.0).round()/10.0}%（VaR预警）`);
    } else if risk_drawdown > 30.0 {
        f4_quant_adjust -= 0.05;
        f4_quant_notes.push(`最大回撤${(risk_drawdown*10.0).round()/10.0}%（偏高）`);
    }
}
if present(risk_sharpe) {
    // 夏普比率: <0 无效 | <0.5 差 | >1.0 好 | >2.0 优秀
    if risk_sharpe < 0.0 {
        f4_quant_adjust -= 0.08;
        f4_quant_notes.push(`夏普${(risk_sharpe*100.0).round()/100.0}（负值）`);
    } else if risk_sharpe < 0.5 && risk_sharpe > 0.0 {
        f4_quant_adjust -= 0.03;
    } else if risk_sharpe > 2.0 {
        f4_quant_adjust += 0.05;
    } else if risk_sharpe > 1.0 {
        f4_quant_adjust += 0.02;
    }
}
if present(risk_pe) && risk_pe > 0.0 {
    // PE: >200 异常 | >100 偏高 | <0 亏损
    if risk_pe > 200.0 {
        f4_quant_adjust -= 0.05;
        f4_quant_notes.push(`PE=${(risk_pe*10.0).round()/10.0}（极端高）`);
    } else if risk_pe < 0.0 {
        f4_quant_adjust -= 0.08;
        f4_quant_notes.push("PE为负（亏损企业）");
    }
}
let f4_adjusted = clamp(f4_signal + f4_quant_adjust, -1.0, 1.0);

// ── 因子 5: 估值风险 ──
// V51 新增: 估值风险信号来自 t-valuation（DCF/格雷厄姆上行空间）
//   估值过高 = 风险（负信号），估值低估 = 机会（正信号）
//   与 f4_signal（技术面/基本面风险）正交，独立衡量估值维度。
let f5_default = 0.15;
// V56: 估值因子消费 IC 代理权重（key="valuation"，由 compute_factor_backtest_stats 输出）
// 激活条件: t-valuation 已执行且输出有效（dcf_upside 或 graham_upside 存在）
let f5_has_valuation = present(valuation_dcf_upside) || present(valuation_graham_upside);
let f5_weight = if f5_has_valuation {
    get_weight(f_weights, "valuation", f5_default)
} else { 0.0 };
let f5_signal = if f5_weight > 0.0 {
    // 优先用 DCF 上行空间（理论内在价值 - 当前价）/ 当前价
    // dcf_upside: 百分数（如 20.5 表示上行空间 20.5%），需要除以 100 转为十进制
    // 用 clamp 把上行空间映射到 [-1, 1]：低估→正，过高→负
    let dcf_sig = if present(valuation_dcf_upside) {
        clamp(valuation_dcf_upside / 50.0, -1.0, 1.0)
    } else { 0.0 };
    let graham_sig = if present(valuation_graham_upside) {
        clamp(valuation_graham_upside / 50.0, -1.0, 1.0)
    } else { 0.0 };
    // F-Score 加成: score >= 7（财务强劲）→ 估值信号可信度更高，信号放大 1.3x
    let fscore_mult = if present(valuation_fscore) && valuation_fscore >= 7 { 1.3 } else { 1.0 };
    // 护城河加成（P2 新增）: 护城河是估值溢价的核心驱动因素
    //   宽护城河 → 1.15x（竞争优势可持续，估值应享受溢价）
    //   窄护城河 → 1.05x（有一定优势，温和溢价）
    //   无护城河/未知 → 1.0x（不提估值）
    let moat_mult = if present(valuation_moat) && type_of(valuation_moat) == "string" {
        if valuation_moat == "宽护城河" || valuation_moat == "wide" { 1.15 }
        else if valuation_moat == "窄护城河" || valuation_moat == "narrow" { 1.05 }
        else { 1.0 }
    } else { 1.0 };
    let raw_sig = if dcf_sig.abs() > graham_sig.abs() { dcf_sig } else { graham_sig };
    clamp(raw_sig * fscore_mult * moat_mult, -1.0, 1.0)
} else { 0.0 };

// ── 因子 6: 数据质量 ──
// V37 修复: 数据质量信号来自 AgentNode（data-quality.content.score），与技术面回测因子
// （bias=乖离率）语义无关，直接用默认权重，避免被乖离率回测数据绑架。
let f6_default = 0.15;
let f6_weight = if present(dqi_score) { f6_default } else { 0.0 };
let f6_signal = if f6_weight > 0.0 { (dqi_score - 50.0) / 50.0 } else { 0.0 };

// ── 因子 7: 交易方案方向与质量 ──
// P2-1 架构调整: LLM trader 降权至 15%（原 30%），改为纯定性提案角色。
// f7 不再输出 targetPrice/stopLoss（定量的责任交回凯利公式和风控层），
// 仅输出 {direction, catalysts, risk_factors, confidence} 定性字段。
// f7_signal 基于 direction 和 confidence 计算。
// 保留 targetPrice/stopLoss 兼容路径（若存在则用于凯利赔率，但不再主导信号）。
let f7_weight = 0.15;
let f7_signal = if present(trader_direction) && type_of(trader_direction) == "string" {
    // P2-1 新路径: 定性方向 + 置信度
    let dir = trader_direction.to_lower();
    let conf = if present(trader_confidence) && type_of(trader_confidence) == "f64" {
        trader_confidence
    } else { 0.5 };
    if dir.contains("buy") || dir.contains("bull") || dir.contains("看多") || dir.contains("买入") || dir.contains("long") {
        clamp(conf * 0.6, 0.1, 0.6)
    } else if dir.contains("sell") || dir.contains("bear") || dir.contains("看空") || dir.contains("卖出") || dir.contains("short") {
        clamp(-conf * 0.6, -0.6, -0.1)
    } else { 0.0 }
} else if present(trader_target_price) && present(trader_stop_loss) && trader_stop_loss > 0.0 {
    // 旧兼容路径: 仍有 targetPrice/stopLoss 时
    // P2-1: 兼容路径权重 0.15（原 0.30）
    if present(current_price) && current_price > 0.0 {
        let upside = (trader_target_price - current_price) / current_price;
        if upside > 0.05 {
            let ratio = trader_target_price / trader_stop_loss;
            let base = clamp(upside * 1.5, 0.05, 0.8);
            if ratio > 1.15 { clamp(base + 0.1, 0.1, 1.0) } else { base }
        } else if upside < -0.05 {
            clamp(upside * 1.3, -1.0, -0.15)
        } else { 0.0 }
    } else {
        let ratio = trader_target_price / trader_stop_loss;
        if ratio > 1.15 { (ratio - 1.0) / 1.0 } else if ratio < 0.85 { -0.5 } else if ratio > 1.0 { 0.1 } else { 0.0 }
    }
} else { 0.0 };

// ── P1 新增: 因子 9 — 资金面（主力资金流向）──
// 数据源: t-hotmoney-data → get_stock_money_flow
// 核心逻辑: 主力净流入/(主力净流入+主力净流出) 比率 → [-1, 1] 信号
// 正信号: 主力资金持续净买入（有明确资金推动）
// 负信号: 主力资金大幅净卖出（资金出逃）
// 零信号: 资金流向平衡或无数据
let f9_default = 0.08;
let f9_signal = if present(money_flow) && type_of(money_flow) == "string" && money_flow != "" {
    let mf = json_parse(money_flow);
    if type_of(mf) == "map" {
        // 提取主力/超大单净流入
        let main_in = mf["main_net_inflow"];
        let super_large_in = mf["super_large_net"];
        let large_in = mf["large_net"];
        let medium_in = mf["medium_net"];
        // 如果有主力净流入数据
        if type_of(main_in) == "f64" {
            // 用主力净流入/总成交额估算信号强度
            // main_in > 0 → 主力净买入(正信号)
            // main_in < 0 → 主力净卖出(负信号)
            // 按成交额归一化：假设总成交额≈主力流入+主力流出的2倍
            let total_est = (main_in.abs() * 3.0).max(1.0);
            clamp(main_in / total_est * 2.0, -1.0, 1.0)
        } else if type_of(super_large_in) == "f64" && type_of(large_in) == "f64" {
            // fallback: 超大单+大单净流入
            let combined = super_large_in + large_in;
            let total_est = (combined.abs() * 3.0).max(1.0);
            clamp(combined / total_est * 2.0, -1.0, 1.0)
        } else if type_of(medium_in) == "f64" {
            // 只有中小单数据时反推：中单净流出=主力可能在买入
            clamp(-medium_in / (medium_in.abs() * 2.0).max(1.0), -1.0, 1.0)
        } else { 0.0 }
    } else { 0.0 }
} else { 0.0 };
// V56: 资金流因子消费 IC 代理权重（key="money_flow"）
let f9_weight = if f9_signal != 0.0 { get_weight(f_weights, "money_flow", f9_default) } else { 0.0 };

// ── P1 新增: 因子 10 — 筹码面（股东增减持+解禁+大宗交易）──
// 数据源: t-lockup-data → get_stock_lockup_bundle
// 三信号融合:
//   ① 股东增减持方向（核心信号，权重最高）
//   ② 限售解禁压力（未来 30 天解禁市值占比）
//   ③ 大宗交易折溢价（折价高 = 出货意愿强 = 负信号）
let f10_default = 0.08;
let f10_signal = if present(lockup_bundle) && type_of(lockup_bundle) == "string" && lockup_bundle != "" {
    let lb = json_parse(lockup_bundle);
    if type_of(lb) == "map" {
        let chip_sig = 0.0;
        // ① 股东增减持信号（权重 0.6）
        let trades = lb["shareholder_trades"];
        if type_of(trades) == "array" {
            let len = trades.len();
            if len > 0 {
                let net_direction = 0.0;
                let total_volume = 0.0;
                for i in 0..len {
                    let t = trades[i];
                    if type_of(t) == "map" {
                        let vol = if type_of(t["shares"]) == "f64" { t["shares"] } else { 0.0 };
                        let typ = if type_of(t["trade_type"]) == "string" { t["trade_type"] } else { "" };
                        if typ == "增持" || typ == "买入" || typ == "increase" {
                            net_direction = net_direction + vol;
                        } else if typ == "减持" || typ == "卖出" || typ == "decrease" {
                            net_direction = net_direction - vol;
                        }
                        total_volume = total_volume + vol;
                    }
                }
                if total_volume > 0.0 {
                    // net_direction/total_volume 范围[-1,1]，表示净方向占比
                    chip_sig = chip_sig + clamp(net_direction / total_volume, -1.0, 1.0) * 0.6;
                }
            }
        }
        // ② 限售解禁压力（权重 0.25）
        let lockup = lb["lockup_schedule"];
        if type_of(lockup) == "array" && lockup.len() > 0 {
            let upcoming_ratio = 0.0;
            for i in 0..lockup.len() {
                let l = lockup[i];
                if type_of(l) == "map" {
                    // unlockRatio 表示解禁股占总股本比例
                    let ratio = if type_of(l["unlockRatio"]) == "f64" { l["unlockRatio"] } else { 0.0 };
                    upcoming_ratio = upcoming_ratio + ratio;
                }
            }
            // 解禁比例>5% 产生显著负信号
            if upcoming_ratio > 5.0 {
                chip_sig = chip_sig + clamp(-upcoming_ratio / 20.0, -0.35, 0.0) * 0.25;
            } else if upcoming_ratio > 1.0 {
                chip_sig = chip_sig + (-0.05) * 0.25;
            }
            // 解禁比例<1% 忽略
        }
        // ③ 大宗交易折溢价（权重 0.15）
        let blocks = lb["block_trades"];
        if type_of(blocks) == "array" && blocks.len() > 0 {
            let avg_discount = 0.0;
            let count = 0;
            for i in 0..blocks.len() {
                let b = blocks[i];
                if type_of(b) == "map" {
                    // discount_pct 正值=折价（低于市价成交），负值=溢价；已由 vendor 计算
                    let disc = if type_of(b["discount_pct"]) == "f64" { b["discount_pct"] } else { 0.0 };
                    avg_discount = avg_discount + disc;
                    count = count + 1;
                }
            }
            if count > 0 {
                avg_discount = avg_discount / count;
                // 折价>5% 显著负信号 (出货意愿强)；溢价>0% 轻微正信号 (机构抢筹)
                if avg_discount > 5.0 {
                    chip_sig = chip_sig + clamp(-avg_discount / 20.0, -0.2, 0.0) * 0.15;
                } else if avg_discount < 0.0 {
                    chip_sig = chip_sig + clamp(-avg_discount / 10.0, 0.0, 0.15) * 0.15;
                }
            }
        }
        // ④ 龙虎榜机构信号（权重 0.15）（P2 新增）
        //   dragon_tiger 数据来自 t-dragon-tiger-data，含机构席位买卖动向
        //   核心逻辑: 机构席位净买入 → 正信号（机构认可价值）
        //             游资席位主导 → 中性（短炒博弈）
        //             无龙虎榜数据 → 无信号（不扣分）
        if present(dragon_tiger) && type_of(dragon_tiger) == "string" && dragon_tiger != "" {
            let dt = json_parse(dragon_tiger);
            if type_of(dt) == "map" {
                let entries = dt["entries"];
                if type_of(entries) == "array" && entries.len() > 0 {
                    let inst_buy = 0.0;
                    let inst_sell = 0.0;
                    let total_buy = 0.0;
                    let total_sell = 0.0;
                    for i in 0..entries.len() {
                        let e = entries[i];
                        if type_of(e) == "map" {
                            let buy_amt = if type_of(e["buyAmount"]) == "f64" { e["buyAmount"] } else { 0.0 };
                            let sell_amt = if type_of(e["sellAmount"]) == "f64" { e["sellAmount"] } else { 0.0 };
                            let dept = if type_of(e["deptName"]) == "string" { e["deptName"] } else { "" };
                            // 机构专用席位买入/卖出
                            if dept.contains("机构") || dept.contains("机构专用") || dept.contains("special") || dept.contains("institution") {
                                inst_buy = inst_buy + buy_amt;
                                inst_sell = inst_sell + sell_amt;
                            }
                            total_buy = total_buy + buy_amt;
                            total_sell = total_sell + sell_amt;
                        }
                    }
                    let total_vol = (total_buy + total_sell).max(1.0);
                    // 机构净买卖比率: (inst_buy - inst_sell) / total_vol
                    let inst_net = (inst_buy - inst_sell) / total_vol;
                    // 如果机构净买入占比较大 → 正信号；净卖出 → 负信号
                    if inst_net.abs() > 0.01 {
                        chip_sig = chip_sig + clamp(inst_net * 2.0, -0.3, 0.3) * 0.15;
                    }
                }
            }
        }
        clamp(chip_sig / (0.6 + 0.25 + 0.15 + 0.15), -1.0, 1.0)
    } else { 0.0 }
} else { 0.0 };
// V56: 筹码因子消费 IC 代理权重（key="chip"）
let f10_weight = if f10_signal != 0.0 { get_weight(f_weights, "chip", f10_default) } else { 0.0 };

// ── P1 新增: 因子 11 — PACE 四维情绪向量（pace-calc.rhai）──
// 输入: llm_events (来自 a-catalyst / a-news 的事件 JSON)
//       money_flow_net / daily_avg_volume (背离修正)
//       sector_etf_direction (协同增强)
//       p_history (30日 P 值历史，Z-score 计算)
// 输出: pace_signal ∈ [-1, 1]，pace_factor 含背离/协同修正
let f11_default = 0.12;
let f11_signal = 0.0;
let f11_pace_vector = #{};
let f11_divergence_flag = false;
let f11_synergy_flag = false;
let f11_sentiment_anomaly = false;

// 调用 pace-calc.rhai 的条件：至少需要有事件数据或公告数据
let has_pace_input = (present(llm_events) && llm_events != "") || (present(announcements) && announcements != "");

if has_pace_input {
    // 构建 pace-calc 的输入上下文（注入 pace-calc 函数依赖的变量）
    // pace-calc 设计为独立模块，通过全局变量获取输入
    // 转换 announcement_events 为 pace-calc 可用的格式
    let announcement_events_str = if present(announcements) && announcements != "" && f3_has_announcement_signal {
        // announcements 已经是 JSON 数组字符串，可直接使用
        announcements
    } else { "" };

    // 注入 money_flow_net 和 daily_avg_volume（来自 t-hotmoney-data）
    // 注: money_flow 是 JSON 字符串，需要解析提取
    let money_flow_net_val = 0.0;
    let daily_avg_volume_val = 0.0;
    if present(money_flow) && money_flow != "" {
        let mf = json_parse(money_flow);
        if type_of(mf) == "map" {
            let main_in = mf["main_net_inflow"];
            money_flow_net_val = if type_of(main_in) == "f64" { main_in }
                else if type_of(main_in) == "int" { main_in as f64 }
                else {
                    let super_in = mf["super_large_net"];
                    let large_in = mf["large_net"];
                    let s = if type_of(super_in) == "f64" { super_in } else { 0.0 };
                    let l = if type_of(large_in) == "f64" { large_in } else { 0.0 };
                    s + l
                };
        }
    }
    // sector_etf_direction 从上游注入（若存在）
    // p_history 从上游注入（若存在）

    // 通过 eval 在 pace-calc 上下文中执行
    // pace-calc 返回 pace_result map
    let pace_result = (||{
        let llm_events_val = if present(llm_events) { llm_events } else { "" };
        let now_timestamp_val = 0.0;
        let money_flow_net = money_flow_net_val;
        let daily_avg_volume = daily_avg_volume_val;
        let sector_etf_direction_val = if present(sector_etf_direction) { sector_etf_direction } else { "" };
        let p_history_val = if present(p_history) { p_history } else { [] };

        // 载入 pace-calc 脚本（嵌入式执行）
        // 注: 实际生产环境中这里应该由 Rust 端 eval_file("pace-calc.rhai") 执行
        // 此处为 Rhai 内联代码的简化模拟：从 pace-calc 逻辑抽取关键计算
        // 完整版本在 Rust core.rs 中通过 eval_file 调用 pace-calc.rhai
        0.0  // 占位：真实值由 Rust 端注入 pace_signal
    })();

    // 生产环境中 pace_signal 由 Rust 端计算后注入
    if present(pace_signal_raw) && type_of(pace_signal_raw) == "f64" {
        f11_signal = clamp(pace_signal_raw, -1.0, 1.0);
    }
    if present(pace_vector) && type_of(pace_vector) == "map" {
        f11_pace_vector = pace_vector;
    }
    if present(pace_divergence_flag) { f11_divergence_flag = pace_divergence_flag; }
    if present(pace_synergy_flag) { f11_synergy_flag = pace_synergy_flag; }
    if present(pace_sentiment_anomaly) { f11_sentiment_anomaly = pace_sentiment_anomaly; }
}
let f11_weight = if f11_signal != 0.0 { f11_default } else { 0.0 };

// ── P1 新增: 技术面硬约束否决（technical-veto.rhai）──
// 在确定 final_action 之前，先判断是否被技术面规则否决
// 输入：rsi_14, macd_dif, macd_dea, macd_hist, llm_trade_signal,
//       price_high_series, price_low_series, macd_hist_series
let tech_vetoed = false;
let tech_veto_rule = "";
let tech_veto_reason = "";
let tech_veto_trail = [];

if present(rsi_14) || present(macd_dif) || present(macd_dea) {
    // 构建技术否决的输入参数，从 Rust 端注入的 technical_veto 结果读取
    if present(technical_veto_result) && type_of(technical_veto_result) == "map" {
        tech_vetoed = technical_veto_result["vetoed"];
        tech_veto_rule = if type_of(technical_veto_result["rule_id"]) == "string" {
            technical_veto_result["rule_id"]
        } else { "" };
        tech_veto_reason = if type_of(technical_veto_result["reason"]) == "string" {
            technical_veto_result["reason"]
        } else { "" };
        let trail = technical_veto_result["decision_trail"];
        if type_of(trail) == "array" {
            tech_veto_trail = trail;
        }
    }
}

// ── 决策追溯链（decision_trail）──
// 记录所有影响决策的关键规则及其裁决结果
// 格式: [{rule_id, status: "PASS"|"VETOED"|"DOWNGRADED", detail, timestamp}]
let decision_trail = [];

// 从技术否决收集 trail
for t in tech_veto_trail {
    decision_trail.push(t);
}

// 记录风险否决
if risk_downgraded {
    decision_trail.push(#{
        "rule_id": "R-200",
        "status": "DOWNGRADED",
        "detail": risk_downgrade_note,
        "timestamp": 0.0,
    });
}

// 记录空头预测否决
if bearish_downgraded {
    decision_trail.push(#{
        "rule_id": "R-201",
        "status": "DOWNGRADED",
        "detail": bearish_downgrade_note,
        "timestamp": 0.0,
    });
}

// 记录数据异常
if !trader_data_valid {
    decision_trail.push(#{
        "rule_id": "R-202",
        "status": "DOWNGRADED",
        "detail": `trader数据异常:${sanity_reason}`,
        "timestamp": 0.0,
    });
}

// 记录权重坍缩
if weights_collapsed {
    decision_trail.push(#{
        "rule_id": "R-203",
        "status": "DOWNGRADED",
        "detail": `因子权重坍缩(${(total_weight*100.0).to_int()}%)或不可信上游≥2,强制观望`,
        "timestamp": 0.0,
    });
}

// V56: 记录单点不可信部分降级
if partial_degraded {
    decision_trail.push(#{
        "rule_id": "R-205",
        "status": "DOWNGRADED",
        "detail": `单点上游不可信(untrusted_count=1),禁止加仓+置信度×0.7`,
        "timestamp": 0.0,
    });
}

// 记录零仓位修正
if position_pct <= 0.0 && (base_action == "买入" || base_action == "增持" || base_action == "持有") {
    decision_trail.push(#{
        "rule_id": "R-204",
        "status": "DOWNGRADED",
        "detail": `零仓位修正:${base_action}→观望(仓位0%,不可持股)`,
        "timestamp": 0.0,
    });
}

// ── 数据 sanity 预检（必须在 position_pct/final_action 之前定义）──
let trader_data_valid = true;
let sanity_reason = "";

// 检查 1: targetPrice <= stopLoss → 无意义的交易方案
if present(trader_target_price) && present(trader_stop_loss) && trader_stop_loss > 0.0 {
    if trader_target_price <= trader_stop_loss {
        trader_data_valid = false;
        sanity_reason = `targetPrice(${trader_target_price})<=stopLoss(${trader_stop_loss})`;
    }
}

// V47 新增检查 1.5: targetPrice 显著低于 currentPrice → 空头预测（做多无意义）
// 与检查2不同: 检查2只标记"偏离>70%"为数据异常（可能是LLM幻觉），
// 而此处标记"明确看空"为逻辑不一致——trader预测下跌但系统仍给出持仓建议。
// 阈值15%: 低于此值可能是短期波动，高于此值应触发空头处理。
let trader_bearish = false;
let bearish_magnitude = 0.0;
if trader_data_valid && present(current_price) && present(trader_target_price) && current_price > 0.0 {
    let upside = (trader_target_price - current_price) / current_price;
    if upside < -0.15 {
        trader_bearish = true;
        bearish_magnitude = upside;
    }
}

// 检查 2: 有 current_price 时，targetPrice 偏离超过 70% → 数据异常
if trader_data_valid && present(current_price) && present(trader_target_price) && current_price > 0.0 {
    let deviation = (trader_target_price - current_price) / current_price;
    if deviation.abs() > 0.70 {
        trader_data_valid = false;
        sanity_reason = `targetPrice(${trader_target_price})偏离currentPrice(${current_price})超过70%`;
    }
}

// 检查 3: stopLoss < 1 元或 targetPrice < 1 元 → 垃圾数据
if trader_data_valid {
    if present(trader_target_price) && trader_target_price < 1.0 {
        trader_data_valid = false;
        sanity_reason = `targetPrice(${trader_target_price})<1元`;
    }
    if present(trader_stop_loss) && trader_stop_loss < 1.0 {
        trader_data_valid = false;
        sanity_reason = `stopLoss(${trader_stop_loss})<1元`;
    }
}

// ── 贝叶斯融合 ──
// V51: 因子数 7（f1~f7），P1 扩展至 10 因子（新增 f9 资金面 + f10 筹码面 + f11 PACE 情绪）
let total_weight = f1_weight + f2_weight + f3_weight + f4_weight + f5_weight + f6_weight + f7_weight + f9_weight + f10_weight + f11_weight;
let weighted_signal = f1_weight*f1_signal + f2_weight*f2_signal
                    + f3_weight*f3_signal + f4_weight*f4_adjusted
                    + f5_weight*f5_signal + f6_weight*f6_signal
                    + f7_weight*f7_signal + f9_weight*f9_signal + f10_weight*f10_signal
                    + f11_weight*f11_signal;
let avg_signal = if total_weight > 0.0 { weighted_signal / total_weight } else { 0.0 };
// V38 修复: 证据数量自适应缩放 — 证据越多（total_weight 越大），对先验的修正力度越强。
// 单因子（total_weight≈0.20）时缩放 0.20（保守），全因子（total_weight≈1.20）时缩放 0.50。
// 固定 0.35 使单因子和多因子修正力度相同，违反贝叶斯"更多证据→更大更新"的核心思想。
// V40 修复: 单因子（total_weight<0.3）时 evidence_scale 降到 0.15，
// 避免单个强信号因子（如 catalyst=L3）完全主导贝叶斯更新。
// 全因子（total_weight≈1.20）时 0.50，正常多因子信号充分。
let evidence_scale = if total_weight < 0.3 {
    0.15
} else {
    clamp(0.20 + total_weight * 0.25, 0.20, 0.50)
};
	// V49 修复: trader 否决权 — 技术面（滞后指标）不应覆盖 LLM trader 的实时判断
	// f1(技术评分) 是趋势跟踪指标，必然滞后。当 trader 根据最新信息判断看空时，
	// posterior 不应被技术面推高到"增持/买入"区。
	// 否决规则（仅在 trader 数据有效且 f7 信号强烈时生效）:
	//   f7_signal < -0.5（强烈看空）→ posterior 封顶 0.48（最高"持有"）
	//   f7_signal < -0.2（温和看空）→ posterior 封顶 0.53（最高"增持"，会被风险否决降级）
	let posterior_cap = if trader_data_valid && f7_weight > 0.0 && present(f7_signal) {
	    if f7_signal < -0.5 { 0.48 }
	    else if f7_signal < -0.2 { 0.53 }
	    else { 1.0 }
	} else { 1.0 };
	let raw_val = prior + avg_signal * evidence_scale;
	let posterior = clamp(if raw_val > posterior_cap { posterior_cap } else { raw_val }, 0.0, 1.0);

	// ── 无 f7 版本 posterior（消除自指悖论）──
	// P0 修复: portfolio-mgr 决策以 0.30 权重吸收 trader 观点后，又与 trader 比较
	// 一致性，导致 compute_decision_agreement 陷入逻辑循环。
	// 此版本排除 f7 影响，用于"纯净"双视角比较（无 f7 公式 vs LLM trader）。
	// 注意: 不加 trader 否决（f7 < -0.5 封顶），因为那是基于 f7 自身的自指约束。
	// P1: 无 f7 版本同时排除 f9/f10（它们是独立信号源，用于 pure 版对比时
	//     不应被排除——但保持向后兼容，保留旧名。实际上 no_f7 仍包含 f9/f10。）
	let total_weight_no_f7 = total_weight - f7_weight;
	let weighted_signal_no_f7 = weighted_signal - f7_weight * f7_signal;
	let avg_signal_no_f7 = if total_weight_no_f7 > 0.0 {
	    weighted_signal_no_f7 / total_weight_no_f7
	} else { 0.0 };
	let evidence_scale_no_f7 = if total_weight_no_f7 < 0.3 {
	    0.15
	} else {
	    clamp(0.20 + total_weight_no_f7 * 0.25, 0.20, 0.50)
	};
	let raw_val_no_f7 = prior + avg_signal_no_f7 * evidence_scale_no_f7;
	// 不加 trader 否决（自指），不加权重坍缩兜底（那是 full 版本的事）
	let posterior_no_f7 = clamp(raw_val_no_f7, 0.0, 1.0);
	let f7_weight_pct = if total_weight > 0.0 {
	    (f7_weight / total_weight * 100.0 * 10.0).round() / 10.0
	} else { 0.0 };

	// ── V42+V44: 风险的多维度处理 ──
	// 风险有两个独立维度, 分别处理:
	//   维度1 (方向): 风险作为看空因子 → f4_signal 在贝叶斯里拉低 posterior (已做)
	//   维度2 (操作): risk_bias 左移 action 阈值 + V43 硬性否决 (见下方)
	//
	// 注意: confidence 不受风险 cap (V44 曾错误 cap, 已回滚)。
	//   权威定义 (Nature Neurosci 2016): confidence = P(决策正确 | 证据), 是后验概率。
	//   posterior 已通过 f4_signal 综合风险因子, confidence = posterior × 100 即可。
	//   "高风险 + 高置信度看多 + action=持有" 是自洽的风险管理, 不是矛盾。
	let risk_bias = if present(overall_risk) {
	    switch overall_risk {
	        "极高" => -0.15,      // 极高风险 → 卖出门槛下降
	        "高风险"|"高" => -0.08, // 高风险 → 减持门槛下降
	        "中风险"|"中" => 0.0,
	        "低风险"|"低" => 0.05,
	        _ => 0.0
	    }
	} else { 0.0 };
	let effective_posterior = clamp(posterior + risk_bias, 0.0, 1.0);

	// ── 无 f7 版本 effective_posterior 与 action（消除自指）──
	let effective_posterior_no_f7 = clamp(posterior_no_f7 + risk_bias, 0.0, 1.0);
	let f7_free_action = if effective_posterior_no_f7 >= 0.63 { "买入" }
	    else if effective_posterior_no_f7 >= 0.53 { "增持" }
	    else if effective_posterior_no_f7 >= 0.48 { "持有" }
	    else if effective_posterior_no_f7 >= 0.38 { "观望" }
	    else if effective_posterior_no_f7 >= 0.30 { "减持" }
	    else { "卖出" };

	// ── V47 修复: 凯利赔率使用标准盈亏比 ──
	// 旧: odds = targetPrice / stopLoss（仅看两个绝对价格比值，忽略 currentPrice）
	//   例: currentPrice=20 targetPrice=8 stopLoss=7 → odds=1.14 → 算出正仓位
	//   但 targetPrice < currentPrice 意味着做多无盈利空间，赔率应为 0。
	// 新: odds = (targetPrice - currentPrice) / (currentPrice - stopLoss)
	//   即 潜在收益 / 潜在亏损，仅在 targetPrice > currentPrice 时 > 0。
	//   currentPrice=20 targetPrice=25 stopLoss=18 → odds=(25-20)/(20-18)=2.5 ✓
	//   currentPrice=20 targetPrice=8  stopLoss=7  → targetPrice<currentPrice → odds=0 ✓
	//   赔率上限 10（避免极端值主导 Kelly）
	let odds = if present(trader_target_price) && present(trader_stop_loss) && present(current_price)
	    && trader_stop_loss > 0.0 && current_price > trader_stop_loss
	    && trader_target_price > current_price {
	    let profit = trader_target_price - current_price;
	    let loss = current_price - trader_stop_loss;
	    if loss > 0.0 { clamp(profit / loss, 0.0, 10.0) } else { 0.0 }
	} else { 0.0 };
	let q = 1.0 - posterior;
	let kelly_raw = if posterior >= 0.50 && odds > 0.001 {
	    (posterior * odds - q) / odds
	} else { 0.0 };
	let kelly_negative = kelly_raw <= 0.0001;
	// 凯利仓位（风险限制上限）
	let base_position_pct = if posterior < 0.50 || kelly_negative {
	    0.0
	} else {
	    let kelly_pos = clamp(kelly_raw / 2.0 * 100.0, 0.0, 95.0);
	    if present(overall_risk) && overall_risk == "极高" {
	        clamp(kelly_pos, 0.0, 10.0)   // 极高风险: 仓位≤10%
	    } else if present(overall_risk) && (overall_risk == "高风险" || overall_risk == "高") {
	        clamp(kelly_pos, 0.0, 35.0)   // 高风险: 仓位≤35%
	    } else if present(overall_risk) && (overall_risk == "中风险" || overall_risk == "中") {
	        clamp(kelly_pos, 0.0, 50.0)   // 中风险: 仓位≤50%
	    } else {
	        kelly_pos
	    }
	};
	let position_pct = if !trader_data_valid { 0.0 } else { base_position_pct };
	// ── 置信度 = P(决策正确 | 证据) = posterior × 100 ──
	// 权威定义 (Nature Neurosci 2016, "Confidence and certainty: distinct probabilistic quantities"):
	//   confidence = P(decision correct | evidence), 是后验概率, 与特定决策绑定。
	//   certainty  = 对变量估计的概率分布宽度 (另一个概念, 这里不用)。
	//
	// 关键: 风险已经通过 f4_signal 进入贝叶斯融合拉低 posterior (方向维度)。
	//   posterior = P(上涨 | 证据) 已综合风险因子, confidence = posterior × 100
	//   在数学和语义上都正确, 不需要再按风险 cap。
	//
	// V44 曾错误地按风险 cap confidence (基于"高风险→低置信"的错误推理),
	//   混淆了 risk (输入证据/收益离散度) 和 confidence (对决策正确的后验概率)。
	//   "高风险 + 高置信度看多 + action=持有" 是自洽的风险管理:
	//   我70%确定会涨, 但风险大所以不加仓。V43 的 action 否决已足够。
	//
	// V50 修复: 接入 risk-convergence 的三方分歧度。
	//   当 risk_disagreement 存在且高时(>50), 三个风险评估师分歧大,
	//   说明风险评估本身不确定性高, 应降低最终置信度。
	//   这是对"风险评估的可靠性"的独立修正，与 f4_signal(风险方向)正交。
	//   分歧度 0~100, 修正因子 = (100 - disagreement) / 100, 范围 [0.3, 1.0]
	let disagreement_mod = if present(risk_disagreement) {
	    clamp((100.0 - risk_disagreement) / 100.0, 0.3, 1.0)
	} else { 1.0 };
	let final_confidence = clamp(posterior * 100.0 * disagreement_mod, 0.0, 100.0);

	// ── M2 修复：LLM 失败级联兜底 ──
	// V55 扩展: 上游 AgentNode 走 strict_mode 兜底时（LLM 输出无法解析为合法 JSON），
	//   agent_executor 会注入 __untrusted=true 标记。下游若读到任一上游不可信，
	//   视为严重数据异常：强制 weights_collapsed（观望+空仓+confidence 对半），
	//   避免"50/50 中性兜底"被当成有效信号继续融合。
	// V56 优化: 分级降级 — 单点失败不应全盘观望。
	//   level 2 全盘降级 (weights_collapsed): total_weight<0.3 或 untrusted_count>=2
	//     → 空仓 + 观望 + confidence×0.5
	//   level 1 部分降级 (partial_degraded): untrusted_count==1
	//     → 禁止买入/增持（降级为持有） + confidence×0.7，不强制空仓
	//   理由: 6 个上游 Agent 中任一失败就全盘降级过于保守，
	//     其余 5 个可信 Agent 的信号仍有价值，不应被单点失败完全否决。
	let weights_collapsed = total_weight < 0.3 || untrusted_count >= 2;
	let partial_degraded = untrusted_count == 1;
	if weights_collapsed {
	    position_pct = 0.0;
	    final_confidence = clamp(final_confidence / 2.0, 0.0, 100.0);
	} else if partial_degraded {
	    final_confidence = clamp(final_confidence * 0.7, 0.0, 100.0);
	}

	// ── V42 重构: action 阈值使用 effective_posterior(=posterior+risk_bias) ──
	// 买入 ← 0.63+仓位15%+
	// 增持 ← 0.53+仓位10%+
	// 持有 ← 0.48-0.53（模糊下界）
	// 观望 ← 0.38-0.50（模糊上界+下界）
	// 减持 ← 0.30-0.40（模糊上界）
	//   卖出 ← <0.30
	let base_action = "卖出";
	// 买入区（effective_posterior≥0.63 且仓位充足）
	if effective_posterior >= 0.63 && position_pct >= 15.0 {
	    base_action = "买入";
	// 增持区（effective_posterior≥0.53 且仓位达标）
	} else if effective_posterior >= 0.53 && position_pct >= 10.0 {
	    base_action = "增持";
	// 持有区（effective_posterior≥0.48，与"观望"的模糊边界）
	} else if effective_posterior >= 0.48 {
	    base_action = "持有";
	// 观望区（posterior 0.38-0.50，与"减持"的模糊边界）
	} else if effective_posterior >= 0.38 {
	    base_action = "观望";
	// 减持区（posterior 0.30-0.40，与"卖出"的模糊边界）
	} else if effective_posterior >= 0.30 {
	    base_action = "减持";
	}

	// ── 最终 action：sanity 不通过或权重坍缩则强制降级 ──
	let final_action = if !trader_data_valid {
	    // 数据异常时：根据 posterior 决定是"观望"还是"卖出"
	    if posterior < 0.45 { "卖出" } else { "观望" }
	} else if weights_collapsed {
	    // 权重坍缩：大部分因子缺失或≥2个上游不可信，强制"观望"
	    "观望"
	} else if partial_degraded && (base_action == "买入" || base_action == "增持") {
	    // V56: 单点不可信 — 禁止加仓，降级为"持有"（可继续持仓但不追加）
	    "持有"
	} else if position_pct <= 0.0 && (base_action == "买入" || base_action == "增持" || base_action == "持有") {
	    // V42: 仓位为 0 时不应给出持仓性建议。若 base_action 暗示持仓但实际空仓，
	    // 降级为"观望"（后验仍可保留，但行动上有零仓位匹配）
	    "观望"
	} else { base_action };

	// ── V43 修复: 风险等级硬性 action 约束（风控否决权）──
	// 根因: risk_bias=-0.08 只是软性偏移, posterior=0.7 时 effective_posterior=0.62
	//   仍落入增持区(≥0.53), 导致"高风险+增持"的逻辑矛盾。
	//   风险等级必须拥有 action 否决权, 不能仅靠阈值偏移。
	// 规则:
	//   高风险 → 禁止"买入/增持"(加仓=放大高风险敞口, 违背风控), 降级为"持有"
	//   极高风险 → 禁止"买入/增持/持有"(极高风险不应继续持仓), 降级为"观望"
	// 注意: 必须在仓位对齐之前执行, 使降级后的 action 正确触发仓位调整
	let risk_downgraded = false;
	let risk_downgrade_note = "";
	if present(overall_risk) {
	    if overall_risk == "极高" && (final_action == "买入" || final_action == "增持" || final_action == "持有") {
	        risk_downgraded = true;
	        risk_downgrade_note = ` | ⚠️极高风险风控否决:${base_action}→观望(极高风险不应继续持仓)`;
	        final_action = "观望";
	    } else if (overall_risk == "高风险" || overall_risk == "高") && (final_action == "买入" || final_action == "增持") {
	        risk_downgraded = true;
	        risk_downgrade_note = ` | ⚠️高风险风控否决:${base_action}→持有(高风险禁止加仓)`;
	        final_action = "持有";
	    }
	}

	// ── V47 新增: 空头 trader 预测否决 ──
	// 当 trader 预测 targetPrice 显著低于 currentPrice（≥15%）时，意味着系统自身
	// 的交易员认为股价将大幅下跌。此时给出"持有"或更积极的建议是逻辑矛盾。
	// 规则: 空头预测 + 持仓建议 → 强制卖出
	// 排除: 已经"卖出"或"减持"的不再降级；"观望"降为"卖出"（明确看空不应观望）
	let bearish_downgraded = false;
	let bearish_downgrade_note = "";
	if trader_bearish {
	    if final_action == "买入" || final_action == "增持" || final_action == "持有" {
	        bearish_downgraded = true;
	        bearish_downgrade_note = ` | ⚠️空头预测否决:targetPrice(${trader_target_price})<现价(${current_price})${(bearish_magnitude*100.0).to_int()}%,${final_action}→卖出`;
	        final_action = "卖出";
	    } else if final_action == "观望" {
	        bearish_downgraded = true;
	        bearish_downgrade_note = ` | ⚠️空头预测否决:targetPrice(${trader_target_price})<现价(${current_price})${(bearish_magnitude*100.0).to_int()}%,${final_action}→卖出`;
	        final_action = "卖出";
	    }
	    // "减持"/"卖出"保持不动（已经是看空方向）
	}

	// ── 仓位与 action 对齐（V42）: action 决定了仓位性质 ──
	// 如果 action 是减持/卖出，仓位应趋于零或极低
	if final_action == "卖出" {
	    position_pct = 0.0;
	} else if final_action == "减持" && position_pct > 15.0 {
	    position_pct = 15.0;   // 减持通道，仓位不超15%
	} else if final_action == "观望" && position_pct > 0.0 {
	    position_pct = 0.0;     // 观望=不持仓，清零目标仓位
	}

	let contradiction_note = if !trader_data_valid {
	    ` | ⚠️trader数据异常:${sanity_reason},action已从${base_action}修正为${final_action}`
	} else if position_pct <= 0.0 && base_action != final_action {
	    ` | ⚠️零仓位修正:${base_action}→${final_action}(仓位0%,不可持股)`
	} else if final_action == "观望" || final_action == "减持" || final_action == "卖出" {
	    ` | ⚠️仓位修正:action=${final_action}仓位调至${position_pct}%`
	} else { "" };

	// M2 兜底：权重坍缩时在 reasoning 中标记
let collapse_note = if weights_collapsed {
    ` | ⚠️因子权重坍缩(${(total_weight*100.0).to_int()}%)或不可信上游≥2,强制观望+空仓`
} else if partial_degraded {
    ` | ⚠️单点上游不可信(untrusted_count=1),禁止加仓+置信度×0.7`
} else { "" };

	// V43 修复: position_pct 用 clamp(只压不抬), 避免 risk_note 把凯利低仓位
	//   (如 21%)强行抬高到风险上限(35%), 违背"高风险限制仓位上限"的语义。
	//   风控是上限不是强制值, 低仓位不应被抬高。
	// V43: 去掉"action阈值左移"描述, 因风险 action 约束已改为硬性否决(见上方 risk_downgrade)
	let risk_note = if present(overall_risk) && overall_risk == "极高" {
	    if position_pct > 10.0 { position_pct = 10.0; }
	    ` | ⚠️极高风险:仓位≤10%`
	} else if present(overall_risk) && (overall_risk == "高风险" || overall_risk == "高") {
	    if position_pct > 35.0 { position_pct = 35.0; }
	    ` | ⚠️高风险:仓位≤35%`
	} else if present(overall_risk) && (overall_risk == "中风险" || overall_risk == "中") && position_pct > 50.0 {
	    position_pct = 50.0;
	    ` | ⚠️中风险:仓位上限50%`
	} else { "" };

// ── 缺失数据 ──
let data_gaps = [];
if !present(totalScore) { data_gaps.push("技术服务评分(t-scoring)"); }
if !present(consensusScore) { data_gaps.push("辩论共识分数(debate-convergence)"); }
if !present(catalyst_level) || catalyst_level == "" { data_gaps.push("催化剂评估(a-catalyst)"); }
if !present(overall_risk) { data_gaps.push("风险等级(risk-level)"); }
if !present(money_flow) { data_gaps.push("资金流向(t-hotmoney-data)"); }
if !present(lockup_bundle) { data_gaps.push("筹码面数据(t-lockup-data)"); }
if !present(dragon_tiger) { data_gaps.push("龙虎榜数据(t-dragon-tiger-data)"); }
if !present(announcements) { data_gaps.push("公告数据(t-catalyst-data)"); }
// P2-1: evidence_pct 分母用 10 因子的理论最大权重和
// 10 因子默认权重: f1=0.15 + f2=0.25 + f3=0.20 + f4=0.15 + f5=0.15 + f6=0.15 + f7=0.15 + f9=0.08 + f10=0.08 + f11=0.12 = 1.48
let max_weight = 0.15 + 0.25 + 0.20 + 0.15 + 0.15 + 0.15 + 0.15 + 0.08 + 0.08 + 0.12;
let evidence_pct_raw = (total_weight / max_weight * 100.0).to_int();
let evidence_pct = if evidence_pct_raw > 100 { 100 } else { evidence_pct_raw };

// ── 时间 ──
let time_horizon = if present(trader_time_horizon) && present(trader_holding_days)
    && trader_holding_days > 0.0 { trader_time_horizon } else if posterior >= 0.65 { "long" } else if posterior >= 0.50 { "mid" } else if posterior >= 0.35 { "short" } else { "ultra_short" };
let expected_holding_days = if present(trader_holding_days) && trader_holding_days > 0.0 {
    trader_holding_days.to_int()
} else { switch time_horizon { "ultra_short"=>2, "short"=>5, "mid"=>28, "long"=>90, _=>5 } };
let target_timeframe = switch time_horizon {
    "ultra_short"=>"1d-2d", "short"=>"1w", "mid"=>"1m", "long"=>"3m", _=>"1m"
};

// ── 推理 ──
let prior_src = if present(market_regime_state) { `(${market_regime_state})` } else { "" };
let gap_note = if data_gaps.len() > 0 {
    ` | 缺口(${data_gaps.len()}) 证据=${evidence_pct}%`
} else { "" };
let bayes_note = ` | 先验=${prior}${prior_src} 后验=${(posterior*100.0).round()/100.0}`;
let kelly_note = ` | 赔率=${(odds*10.0).round()/10.0} 凯利=${(kelly_raw*1000.0).round()/1000.0}`;
// V50: 分歧度修正标注（仅在 disagreement 显著时标记）
let disagreement_note = if present(risk_disagreement) && risk_disagreement > 50 {
    ` | ⚠️风险分歧(${risk_disagreement})置信修正×${(disagreement_mod*100.0).round().to_int()}%`
} else { "" };

// ── 输出 ──
	#{
    "action": final_action,
    "positionPct": (position_pct * 10.0).round() / 10.0,
    "confidence": (final_confidence * 10.0).round() / 10.0,
    "riskLevel": if present(overall_risk) { overall_risk } else { "未知" },
    "stopLossPct": if position_pct > 0.0 { switch time_horizon {
        "ultra_short"=>3.0,"short"=>5.0,"mid"=>8.0,"long"=>12.0,_=>5.0
    }} else { 0.0 },
    "takeProfitPct": if position_pct > 0.0 { switch time_horizon {
        "ultra_short"=>5.0,"short"=>10.0,"mid"=>18.0,"long"=>30.0,_=>10.0
    }} else { 0.0 },
    "reasoning": `决策=${final_action} 置信=${final_confidence} 仓位=${position_pct}%${gap_note}${bayes_note}${kelly_note}${disagreement_note}${contradiction_note}${risk_downgrade_note}${bearish_downgrade_note}${risk_note}${collapse_note}`,
    "data_gaps": data_gaps,
    "timeHorizon": time_horizon,
    "expectedHoldingDays": expected_holding_days,
    "isContradictory": present(trader_target_price) && present(trader_stop_loss) && trader_stop_loss > 0.0 && trader_target_price <= trader_stop_loss,
    "targetTimeframe": target_timeframe,
    "evidence": #{
        "prior": prior, "posterior": (posterior*1000.0).round()/1000.0,
        "avgSignal": (avg_signal*100.0).round()/100.0,
        "activeFactors": total_weight, "totalWeight": total_weight,
        "f9_money_flow_signal": (f9_signal*100.0).round()/100.0,
        "f10_chip_signal": (f10_signal*100.0).round()/100.0,
        "f11_pace_signal": (f11_signal*100.0).round()/100.0,
        "announcement_signal": (announcement_modifier*100.0).round()/100.0,
    },
    "risk_quant": #{
        "f4_base": (f4_signal*100.0).round()/100.0,
        "f4_quant_adjust": (f4_quant_adjust*100.0).round()/100.0,
        "f4_adjusted": (f4_adjusted*100.0).round()/100.0,
        "notes": f4_quant_notes,
    },
    "decision_trail": decision_trail,
    "technical_veto": #{
        "vetoed": tech_vetoed,
        "rule_id": tech_veto_rule,
        "reason": tech_veto_reason,
    },
    "pace_summary": #{
        "signal": (f11_signal*100.0).round()/100.0,
        "divergence_flag": f11_divergence_flag,
        "synergy_flag": f11_synergy_flag,
        "sentiment_anomaly": f11_sentiment_anomaly,
        "pace_vector": f11_pace_vector,
    },
    "computation_logs": `prior=${prior}${prior_src} post=${(posterior*100.0).round()/100.0} sig=${(avg_signal*100.0).round()/10.0} w=${total_weight} odds=${(odds*10.0).round()/10.0} k=${(kelly_raw*1000.0).round()/1000.0} pos=${position_pct} f9=${(f9_signal*100.0).round()/100.0} f10=${(f10_signal*100.0).round()/100.0} f11=${(f11_signal*100.0).round()/100.0} ann=${(announcement_modifier*100.0).round()/100.0}`,
    "f7_free": #{
        "posterior": posterior_no_f7,
        "action": f7_free_action,
        "effective_posterior": effective_posterior_no_f7,
        "f7_weight": f7_weight,
        "total_weight": total_weight,
        "f7_weight_pct": f7_weight_pct,
    }
}
