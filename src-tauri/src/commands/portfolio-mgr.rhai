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
//
// P2-B4: 统一新闻情感分词典已在 `axagent_astock_data::sentiment` 落地,
// 并通过 `pm_compute_news_sentiment(title, summary)` / `pm_compute_text_sentiment(text)`
// 注册到共享 Rhai Engine。下方公告关键词检测逻辑(risk_kw/pos_kw)保留独立实现,
// 因为它输出 [-0.3, 0.3] 修正信号(语义不同于 sentiment_score [-1.0, 1.0])。
// 未来重构可考虑调用 pm_compute_news_sentiment 替代本地词典。

// ── Rhai 全局函数（必须位于全局作用域，Rhai 不允许在 try/块内定义函数）──
fn present(x) { type_of(x) != "()" }
fn safe_parse(raw) {
    if !present(raw) { return (); }
    if type_of(raw) != "string" || raw == "" { return (); }
    try {
        json_parse(raw)
    } catch (err) {
        ()
    }
}
fn get_weight(weights, factor, fallback) {
    if type_of(weights) == "map" {
        let f = weights[factor];
        if type_of(f) != "()" && type_of(f) == "map" {
            let w = f["weight"];
            if type_of(w) == "f64" { w } else { fallback }
        } else { fallback }
    } else { fallback }
}
fn risk_rank(label) {
    switch label {
        "低风险"|"低" => 0,
        "中风险"|"中" => 1,
        "高风险"|"高" => 2,
        "极高风险"|"极高" => 3,
        _ => 1
    }
}
// has_negation: 检查关键词前 N 字符是否含否定词（避免"没有退市风险"被误判为风险信号）
// 必须位于全局作用域（Rhai 不允许在块内定义函数）
fn has_negation(text, kw, negs) {
    let idx = text.index_of(kw);
    if idx == -1 { return false; }
    // 取关键词前 20 个字符（覆盖常见的否定短语长度）
    let before = if idx >= 20 { text.sub_string(idx - 20, 20) } else { text.sub_string(0, idx) };
    for n in negs {
        if before.contains(n) { return true; }
    }
    false
}

// ── 防御性兜底：任何 Rhai 运行时错误都返回可解析的保守决策，避免节点无输出 ──
let pm_result = ();

try {
// ── 可配置参数（修复 D7/D8：通过 input_mapping 覆盖，默认值为 V56 校准 A 股参数）──
// action 阈值（effective_posterior）
let ACTION_BUY_THRESHOLD = if present(action_buy_threshold) { action_buy_threshold } else { 0.63 };
let ACTION_INCREASE_THRESHOLD = if present(action_increase_threshold) { action_increase_threshold } else { 0.53 };
let ACTION_HOLD_THRESHOLD = if present(action_hold_threshold) { action_hold_threshold } else { 0.48 };
let ACTION_WATCH_THRESHOLD = if present(action_watch_threshold) { action_watch_threshold } else { 0.38 };
let ACTION_REDUCE_THRESHOLD = if present(action_reduce_threshold) { action_reduce_threshold } else { 0.30 };
// 仓位阈值（%）
let POS_BUY_MIN = if present(pos_buy_min) { pos_buy_min } else { 15.0 };
let POS_INCREASE_MIN = if present(pos_increase_min) { pos_increase_min } else { 10.0 };
// 风险分类阈值
let RISK_DEBT_EXTREME = if present(risk_debt_extreme) { risk_debt_extreme } else { 85.0 };
let RISK_VOL_EXTREME = if present(risk_vol_extreme) { risk_vol_extreme } else { 60.0 };
let RISK_SHARPE_EXTREME = if present(risk_sharpe_extreme) { risk_sharpe_extreme } else { -1.5 };
let RISK_VOL_HIGH = if present(risk_vol_high) { risk_vol_high } else { 40.0 };
let RISK_DD_HIGH = if present(risk_dd_high) { risk_dd_high } else { 45.0 };
let RISK_ROE_HIGH = if present(risk_roe_high) { risk_roe_high } else { 5.0 };
let RISK_DEBT_HIGH = if present(risk_debt_high) { risk_debt_high } else { 65.0 };
let RISK_VOL_LOW = if present(risk_vol_low) { risk_vol_low } else { 25.0 };
let RISK_SHARPE_LOW = if present(risk_sharpe_low) { risk_sharpe_low } else { 0.5 };
let RISK_DD_LOW = if present(risk_dd_low) { risk_dd_low } else { 30.0 };
let RISK_ROE_LOW = if present(risk_roe_low) { risk_roe_low } else { 8.0 };
let RISK_DEBT_LOW = if present(risk_debt_low) { risk_debt_low } else { 55.0 };
let RISK_GROWTH_LOW = if present(risk_growth_low) { risk_growth_low } else { 3.0 };
// 仓位限制（%）
let POS_CAP_EXTREME = if present(pos_cap_extreme) { pos_cap_extreme } else { 10.0 };
let POS_CAP_HIGH = if present(pos_cap_high) { pos_cap_high } else { 35.0 };
let POS_CAP_MID = if present(pos_cap_mid) { pos_cap_mid } else { 50.0 };
// 交易成本率（A 股默认 0.3%）
let COST_PCT = if present(cost_pct) { cost_pct } else { 0.003 };

// 安全解析 JSON：解析失败（非法 JSON / 非字符串 / 空）返回 ()（unit），
// 避免上游脏数据（如 LLM 返回带 markdown 包裹的 JSON）导致整个工作流崩溃。
// 调用点已有 type_of 校验，() 会自然跳过，等价于"无数据"。
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

// ── 因子权重：regime-weights（市场状态调节）优先，IC 归一化权重 fallback ──
// P1-1 修复: regime_factor_weights 来自 regime-weights.rhai（熊/牛/高波动自适应），
//             fallback 到 t-scoring 的 factor_backtest.factors（纯回测 IC 归一化）。
// regime_factor_weights 结构: {trend:{weight,regime_mod}, consensus:{...}, catalyst:{...}, ...}
// factor_weights 结构: {trend:{weight,win_rate,ic}, macd:{...}, ...}
// 优先用 regime_factor_weights（含市场状态感知），缺失时回退到回测权重。
let f_weights = if present(regime_factor_weights) && type_of(regime_factor_weights) == "map" {
    regime_factor_weights
} else if present(factor_weights) && type_of(factor_weights) == "map" {
    factor_weights
} else { #{}};  // 空 map 触发等权 fallback

// 尝试从 factor_weights 读取权重；找不到则用等权 fallback

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
    let ann_list = safe_parse(announcements);
    if type_of(ann_list) == "array" && ann_list.len() > 0 {
        let risk_count = 0;
        let positive_count = 0;
        let high_risk_count = 0;
        // 风险关键词（匹配公告标题/类型）
        // 修复 D9: 补充更多政策/监管/诉讼/ST 相关关键词
        let risk_kw = ["风险提示", "异常波动", "可能下跌", "可能快速下跌", "风险警示",
                       "退市", "立案调查", "监管函", "警示函", "减持", "业绩预亏",
                       "亏损", "ST", "*ST",
                       "股东减持", "被动减持", "强制平仓", "违约",
                       "暂停上市", "终止上市", "面值退市", "重大违法",
                       "责令改正", "通报批评", "公开谴责", "行政处罚",
                       "诉讼", "仲裁", "冻结", "查封",
                       "商誉减值", "资产减值", "信用减值", "坏账",
                       "st", "*st", "SST", "S*ST"];
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
                // ── P2-2: 否定词前缀检测 ──
                // 关键词匹配无法处理否定语义，如"没有退市风险"会被误判为风险信号。
                // A 股公告常见否定句式: "不存在退市风险" "未发现违规行为" "已消除影响"
                // 在匹配关键词前，检查关键词前 N 个字符是否有否定词。
                // 否定词词典（覆盖中文公告中最常见的否定表达）:
                let neg_prefixes = ["没有", "未发现", "不存在", "不存在", "已消除",
                                    "已解决", "已排除", "并非", "非", "尚未",
                                    "未触及", "未达到", "未发生"];
                // has_negation 函数已移至全局作用域（Rhai 不允许在块内定义函数）
                // 高风险关键词检测（含否定词过滤）
                for kw in risk_kw {
                    if combined.contains(kw) || combined.contains(kw.to_upper()) {
                        // P2-2: 否定词检测 — 跳过被否定的风险关键词
                        if has_negation(combined, kw, neg_prefixes) {
                            // 被否定的风险关键词（如"不存在退市风险"），不计数但记录
                            continue;
                        }
                        if kw == "退市" || kw == "立案调查" || kw == "可能快速下跌" || kw == "风险警示" {
                            high_risk_count = high_risk_count + 1;
                        } else {
                            risk_count = risk_count + 1;
                        }
                        break;
                    }
                }
                // 利好关键词检测（含否定词过滤）
                for kw in pos_kw {
                    if combined.contains(kw) {
                        if has_negation(combined, kw, neg_prefixes) {
                            continue;
                        }
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
    // V67 修复(2026-07-29): 用 f3_has_catalyst 守卫 catalyst_level 访问。
    //   原问题: 当 f3_has_catalyst=false（catalyst_level 为 unit/空/"无"）但
    //   f3_has_announcement_signal=true 时, f3_weight = f3_default * 0.5 > 0,
    //   进入本块后 catalyst_level.contains() 在 unit 上调用会抛 "Method not found",
    //   触发 catch 块导致整个 portfolio-mgr 降级为保守决策。
    //   修复: llm_signal 只在 f3_has_catalyst=true 时计算,否则为 0.0。
    let llm_signal = if f3_has_catalyst {
        if catalyst_level.contains("L-3") || catalyst_level.contains("退市") || catalyst_level.contains("造假") { -0.8 }
        else if catalyst_level.contains("L-2") || catalyst_level.contains("业绩暴雷") { -0.5 }
        else if catalyst_level.contains("L-1") || catalyst_level.contains("普通利空") { -0.2 }
        else if catalyst_level.contains("L3") || catalyst_level.contains("估值体系级") { 0.8 }
        else if catalyst_level.contains("L2") || catalyst_level.contains("业绩拐点级") { 0.5 }
        else if catalyst_level.contains("L1") || catalyst_level.contains("普通消息") { 0.2 }
        else { 0.0 }
    } else { 0.0 };
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

// ── P1-7 修复: 确定性风险分类（优先算法，LLM 仅作 fallback）──
// 基于 t-risk.stockRiskProfile 的量化指标做规则分类，结果覆盖 overall_risk，
// 仅当 t-risk 数据完全缺失时才 fallback 到 cls-risk-level 的 LLM 分类。
// 修复(2026-07-21): overall_risk 必须先 let 声明，否则后续 `overall_risk = ...`
// 赋值会报 "Variable not found: overall_risk"（Rhai 不允许未声明变量直接赋值）
let overall_risk = "中风险";  // 默认中风险，被算法/LLM/无数据分支覆盖
let risk_source = "算法";  // "算法"/"LLM回退"/"无数据"
let risk_classified = false;

// 定义排序整数，便于比较风险等级高低

if present(risk_volatility) && risk_volatility > 0.0 {
    // 基于波动率/回撤/夏普的确定性分类（A 股校准 V54）
    let vol = risk_volatility;
    let dd = if present(risk_drawdown) { risk_drawdown } else { 0.0 };
    let sharpe = if present(risk_sharpe) { risk_sharpe } else { 0.0 };
    let roe = if present(risk_roe) { risk_roe } else { 100.0 };  // 无数据时不触发 ROE 惩罚
    let debt = if present(risk_debt_ratio) { risk_debt_ratio } else { 0.0 };
    let growth = if present(risk_revenue_growth) { risk_revenue_growth } else { 100.0 };

    // 极高风险: ST/*ST 标的或财务困境
    let is_extreme = false;
    // 财务困境: 负债率>RISK_DEBT_EXTREME 且 营收负增长
    if debt > RISK_DEBT_EXTREME && growth < 0.0 { is_extreme = true; }
    // 波动率极端 + 夏普极差
    if vol > RISK_VOL_EXTREME && sharpe < RISK_SHARPE_EXTREME { is_extreme = true; }

    // 高风险: 量化指标高波动 + 基本面无亮点
    let is_high = false;
    if (vol > RISK_VOL_HIGH || sharpe < 0.0 || dd > RISK_DD_HIGH) && (roe < RISK_ROE_HIGH || debt > RISK_DEBT_HIGH) { is_high = true; }
    if vol > 35.0 && sharpe < 0.3 && roe < 8.0 && growth < 5.0 { is_high = true; }

    // 低风险: 量化健康 + 基本面稳健
    let is_low = false;
    if vol < RISK_VOL_LOW && sharpe > RISK_SHARPE_LOW && dd < RISK_DD_LOW && roe > RISK_ROE_LOW && debt < RISK_DEBT_LOW && growth > RISK_GROWTH_LOW { is_low = true; }

    if is_extreme {
        overall_risk = "极高风险";
    } else if is_high {
        overall_risk = "高风险";
    } else if is_low {
        overall_risk = "低风险";
    } else {
        overall_risk = "中风险";
    }
    risk_source = "算法";
    risk_classified = true;
}

// LLM 回退: 仅当算法无法分类时使用
if !risk_classified && present(overall_risk_llm) {
    overall_risk = overall_risk_llm;
    risk_source = "LLM回退";
    risk_classified = true;
} else if !risk_classified {
    overall_risk = "中风险";
    risk_source = "无数据";
}

// A 股校准（V54 放宽）后，用风险等级的 rank 校准 f4 惩罚强度
let risk_rank_val = risk_rank(overall_risk);

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
let f4_weight = f4_default;   // overall_risk 始终存在（算法分类确保兜底）
// P0-2 修复: f4_signal 只读 LLM 分类（与 overall_risk 的算法分类解耦）
// overall_risk 已基于量化指标计算，f4_quant_adjust 又用同样的指标做修正
// 为避免双重惩罚，f4_signal 改为只读 LLM 的独立风险判断
// 当 LLM 分类缺失（算法兜底场景）时 f4_signal=0.0，由 f4_quant_adjust 独立承担
let f4_signal = if present(overall_risk_llm) && type_of(overall_risk_llm) == "string" {
    switch overall_risk_llm {
        "低风险"|"低" => 0.30,
        "中风险"|"中" => -0.05,
        "高风险"|"高" => -0.25,
        "极高风险"|"极高" => -0.50,
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
// PE 处理: 区分缺失(<=0)、亏损(负值)、偏高(>100)、极端高(>200)
// 注: 上游 stock_data.rs 在 PE 缺失时输出 0.0 作为兜底值
if present(risk_pe) && type_of(risk_pe) == "f64" {
    if risk_pe > 200.0 {
        f4_quant_adjust -= 0.05;
        f4_quant_notes.push(`PE=${(risk_pe*10.0).round()/10.0}（极端高）`);
    } else if risk_pe > 100.0 {
        f4_quant_adjust -= 0.03;
        f4_quant_notes.push(`PE=${(risk_pe*10.0).round()/10.0}（偏高）`);
    } else if risk_pe <= 0.0 {
        // PE<=0 视为数据缺失或亏损企业（上游 xueqiu 已过滤 <=0 为 None，
        // 此处兜底处理其他数据源可能返回的 0/负值）
        f4_quant_adjust -= 0.08;
        f4_quant_notes.push("PE数据缺失或为负（亏损企业）");
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
    // P2-6 修复: 分母从 50 降为 30。A 股价值投资中 DCF 上行 20%+ 是显著低估信号，
    //            30 做分母使 20% 上行→0.67（强信号），30%+→clamp 到 1.0。
    let dcf_sig = if present(valuation_dcf_upside) {
        clamp(valuation_dcf_upside / 30.0, -1.0, 1.0)
    } else { 0.0 };
    let graham_sig = if present(valuation_graham_upside) {
        clamp(valuation_graham_upside / 30.0, -1.0, 1.0)
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
// V66 修复(2026-07-29): F 级（dqi_score<25）时 f6_signal 设为 0（中性），避免自指双重惩罚。
//   原问题: F 级时 f6_signal=(dqi-50)/50=-1.0（强负信号）拉低 posterior 到 0.3 以下,
//     导致 base_action="卖出"。但 F 级语义是"数据不足无法判断"（应观望）,
//     而非"看空"（卖出）。dqi_collapsed 已强制 position_pct=0,
//     再让 f6_signal 拉低 posterior 到"卖出"区间是双重惩罚且语义矛盾。
//   修复: F 级时 f6_signal=0,让 posterior 由其他因子自然决定,
//     再由 position_pct=0 的 action 降级逻辑（L1148）统一收敛为"观望"。
let f6_default = 0.15;
let f6_weight = if present(dqi_score) { f6_default } else { 0.0 };
let f6_signal = if f6_weight > 0.0 {
    if dqi_score < 25.0 { 0.0 }  // V66: F 级中性化，避免自指双重惩罚
    else { (dqi_score - 50.0) / 50.0 }
} else { 0.0 };

// ── 因子 7: 交易方案方向与质量 ──
// V65 架构调整: LLM trader 进一步降权至 10%（原 15%/30%）。
// 原因：trader 现在输出与 portfolio-mgr 同维度的完整决策（action/positionPct/riskLevel），
// 不再是"提案"，而是"独立 LLM 视角"。f7 因子如果消费 trader 的 action/positionPct，
// 会导致公式决策与 LLM 决策产生自指污染（compute_decision_agreement 的 f7_weight_pct 标记）。
// 因此 V65 将 f7 降权至 0.10，仅作为"定性方向参考"，定量决策（仓位/止损/止盈）
// 完全由公式独立计算，避免双视角对比虚高。
//
// f7_signal 综合考虑:
//   ① trader.verdict 方向（基础信号，权重 60%）
//   ② trader.confidence 调制（0.5 为中性，>0.5 加强 / <0.5 减弱）
//   ③ trader.evidence_cited 数量（V65 新增，引用 ≥3 条加强信号，<3 条减弱）
//   ④ trader.riskLevel 风险等级（V65 新增，极高风险/高风险减弱信号幅度）
// 不消费 trader.positionPct（避免仓位自指）、不消费 trader.action（避免 action 自指）。
let f7_weight = 0.10;
let f7_signal = if present(trader_direction) && type_of(trader_direction) == "string" {
    // V65 新路径: 定性方向 + 置信度 + 证据密度 + 风险调制
    let dir = trader_direction.to_lower();
    let conf = if present(trader_confidence) && type_of(trader_confidence) == "f64" {
        trader_confidence
    } else { 0.5 };

    // V65: 证据密度调制 — 引用上游论据 ≥3 条加强信号，<3 条减弱
    // trader_evidence_count 来自 trader.content.evidence_cited（数组长度由 LLM 输出决定）
    let evidence_count = if present(trader_evidence_count) && type_of(trader_evidence_count) == "array" {
        trader_evidence_count.len()
    } else { 0 };
    let evidence_mod = if evidence_count >= 5 {
        1.15  // 引用充分，信号加强 15%
    } else if evidence_count >= 3 {
        1.0   // 引用达标，信号不变
    } else if evidence_count >= 1 {
        0.7   // 引用不足，信号减弱 30%
    } else {
        0.4   // 无引用，信号减弱 60%（接近兜底）
    };

    // V65: 风险等级调制 — 极高风险/高风险减弱信号幅度（保守化）
    let risk_mod = if present(trader_risk_level) && type_of(trader_risk_level) == "string" {
        let rl = trader_risk_level.to_lower();
        if rl.contains("极高") { 0.5 }
        else if rl.contains("高") && !rl.contains("极高") { 0.7 }
        else if rl.contains("中") { 1.0 }
        else if rl.contains("低") { 1.1 }
        else { 1.0 }
    } else { 1.0 };

    // 综合调制因子（clamped 防止过度放大）
    let combined_mod = clamp(evidence_mod * risk_mod, 0.2, 1.3);

    if dir.contains("buy") || dir.contains("bull") || dir.contains("看多") || dir.contains("买入") || dir.contains("long") {
        clamp(conf * 0.6 * combined_mod, 0.05, 0.6)
    } else if dir.contains("sell") || dir.contains("bear") || dir.contains("看空") || dir.contains("卖出") || dir.contains("short") {
        clamp(-conf * 0.6 * combined_mod, -0.6, -0.05)
    } else { 0.0 }
} else if present(trader_target_price) && present(trader_stop_loss) && trader_stop_loss > 0.0 {
    // 旧兼容路径: 仍有 targetPrice/stopLoss 但无 verdict（极少见，仅作兜底）
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
    let mf = safe_parse(money_flow);
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
    let lb = safe_parse(lockup_bundle);
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
            let dt = safe_parse(dragon_tiger);
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

// ── f11: PACE 情绪因子 — 基于公告的四维情绪向量 ──
// pace-calc.rhai 基于 t-catalyst-data 公告数据计算 PACE 信号（Polarity/Actuality/Credibility/Expectation），
// 输出 pace_signal ∈ [-1, 1]，通过 input_mapping "pace-calc.pace_signal" 注入。
// f11 权重 0.08（与 f9/f10 同级补充因子），仅在 pace_signal 非零时激活。
// 修复 D6: 使用 get_weight 消费 IC 代理权重（key="pace"），与 f5/f9/f10 一致
// P2-2 修复: 检查 pace_degraded 标志，降级时 f11 权重归零，避免误用 pace_signal=0 作为中性信号
let f11_default = 0.08;
let pace_is_degraded = present(pace_degraded) && type_of(pace_degraded) == "bool" && pace_degraded;
let f11_signal = if pace_is_degraded {
    0.0  // pace 数据降级，不参与决策
} else if present(pace_signal) && type_of(pace_signal) == "f64" && pace_signal != 0.0 {
    clamp(pace_signal, -1.0, 1.0)
} else { 0.0 };
let f11_weight = if f11_signal != 0.0 { get_weight(f_weights, "pace", f11_default) } else { 0.0 };

// ── P1-1: 因子协方差衰减（消除高相关因子的信号重复计数）──
// 贝叶斯因子融合隐含朴素独立假设（Naïve Bayes: P(E1,E2|H) = P(E1|H)×P(E2|H)），
// 但现实中 f3(催化剂)与f11(PACE情绪)都依赖 t-catalyst-data 公告数据，约 70% 信息重叠；
// f1(技术面)与f9(资金面)在趋势行情中高度共振，资金流入往往只是趋势的附属信号。
// 策略：对后激活因子（权重更小的那个）做权重衰减，从 total_weight 和 avg_signal 中消去重复计数。
//
// f3 ↔ f11: 公告数据重叠 → 次要因子(f11)降权 35%
//   例: f3=0.20 + f11=0.08 → f11_effective = 0.08×0.65=0.052, 重叠计数 -0.028
//   根因: PACE 情绪分析从公告文本中提取，催化剂也从公告+LLM 判断，信息源高度重合。
if f3_weight > 0.0 && f11_weight > 0.0 {
    f11_weight = f11_weight * 0.65;  // 减掉 35% 的公告重复计数
}
// f1 ↔ f9: 趋势-资金共振 → 次要因子(f9)降权 25%
//   例: f1=0.15 + f9=0.08 → f9_effective = 0.08×0.75=0.060, 重叠计数 -0.020
//   根因: 主力资金流入是趋势上涨的伴随现象，并非独立证据源。趋势向上时主力必然流入。
if f1_weight > 0.0 && f9_weight > 0.0 {
    f9_weight = f9_weight * 0.75;  // 减掉 25% 的趋势-资金共振重复
}

// ── P1 新增: 技术面硬约束否决（内联计算，从 t-scoring.indicators 注入 rsi_14/macd_dif/macd_dea）──
// 规则：
//   R-401: RSI>80 → 否决（追高风险）
//   R-402: RSI<20 → 否决（恐慌杀跌）
//   R-405: RSI>80 + MACD DIF<0 → 否决（双重风险）
let tech_vetoed = false;
let tech_veto_rule = "";
let tech_veto_reason = "";
let tech_veto_trail = [];

if present(rsi_14) && type_of(rsi_14) == "f64" {
    if rsi_14 > 80.0 {
        tech_vetoed = true;
        tech_veto_rule = "R-401";
        tech_veto_reason = `RSI(14)=${rsi_14}>80 超买区域，触发追高风险否决`;
    } else if rsi_14 < 20.0 {
        tech_vetoed = true;
        tech_veto_rule = "R-402";
        tech_veto_reason = `RSI(14)=${rsi_14}<20 超卖区域，触发恐慌杀跌否决`;
    }
    if rsi_14 > 80.0 && present(macd_dif) && macd_dif < 0.0 {
        tech_vetoed = true;
        tech_veto_rule = "R-405";
        tech_veto_reason = `RSI>80 且 MACD DIF<0，技术面双重风险否决`;
    }
    if tech_vetoed {
        tech_veto_trail.push(#{
            "rule_id": tech_veto_rule,
            "status": "VETOED",
            "detail": tech_veto_reason,
        });
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
// P0-1 修复: R-200/R-201/R-202/R-204 原在此处前向引用未声明变量（risk_downgraded/
// bearish_downgraded/trader_data_valid/position_pct/base_action），被 catch_unwind
// 吞掉后永不生效。已移至各变量定义之后，参照 C2.1 修复 R-203/R-205 的模式。

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

// P0-1 修复: R-202 移至 trader_data_valid 定义之后（原位置前向引用为死代码）
// 记录数据异常
if !trader_data_valid {
    decision_trail.push(#{
        "rule_id": "R-202",
        "status": "DOWNGRADED",
        "detail": `trader数据异常:${sanity_reason}`,
        "timestamp": 0.0,
    });
}

// ── 贝叶斯融合 ──
// V51: 因子数 7（f1~f7），P1 扩展至 10 因子（新增 f9 资金面 + f10 筹码面）
// PACE 集成: 新增 f11 情绪因子 → 11 因子, total_weight 包含 f11
let total_weight = f1_weight + f2_weight + f3_weight + f4_weight + f5_weight + f6_weight + f7_weight + f9_weight + f10_weight + f11_weight;
let weighted_signal = f1_weight*f1_signal + f2_weight*f2_signal
                    + f3_weight*f3_signal + f4_weight*f4_adjusted
                    + f5_weight*f5_signal + f6_weight*f6_signal
                    + f7_weight*f7_signal + f9_weight*f9_signal + f10_weight*f10_signal
                    + f11_weight*f11_signal;
let avg_signal = if total_weight > 0.0 { weighted_signal / total_weight } else { 0.0 };

// 理论最大权重和（10 个活跃因子的默认权重之和）
// 用于 weights_collapsed 比例阈值和 evidence_pct 计算
// V66 修复(2026-07-29): f7 已在 V65 降权至 0.10，此处同步更新，避免
//   total_weight/max_weight 比例偏低约 3.5% 而在边界误触发 weights_collapsed。
//   f1=0.15 + f2=0.25 + f3=0.20 + f4=0.15 + f5=0.15 + f6=0.15 + f7=0.10 + f9=0.08 + f10=0.08 + f11=0.08 = 1.39
let max_weight = 0.15 + 0.25 + 0.20 + 0.15 + 0.15 + 0.15 + 0.10 + 0.08 + 0.08 + 0.08;
// ── P2-1+P3-1: 证据缩放 — 调用 Rust 函数（非线性 sqrt 曲线）──
let evidence_scale = pm_evidence_scale(total_weight, max_weight);
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
	// P3-1: no_f7 证据缩放 — 调用 Rust 函数
	let evidence_scale_no_f7 = pm_evidence_scale(total_weight_no_f7, max_weight);
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
	// P3-1: risk_bias — 调用 Rust 函数
	let risk_bias = pm_risk_bias(overall_risk);
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
	//
	// ── P0-1: 无 trader 数据时的波动率赔率 fallback ──
	// 原设计: trader_target_price/stop_loss 缺失时 odds=0 → kelly=0 → position_pct=0
	//   后果: LLM trader 单点故障导致整个凯利仓位置零，决策覆盖率严重受限
	// 新: 基于 posterior 强度估算保守赔率（无 targetPrice 时的经验安全值）:
	//   posterior≥0.70(强看多) → 2.5x (典型 A 股趋势突破后的盈亏比分布中位数)
	//   posterior≥0.60(看多)   → 2.0x (中性偏优, 对应大盘股震荡上行行情)
	//   posterior≥0.50(偏多)   → 1.5x (保守, 适合低波动/底部盘整标的)
	//   posterior<0.50         → 0.0x (凯利不适用)
	let odds_fallback = if posterior >= 0.70 { 2.5 }
	    else if posterior >= 0.60 { 2.0 }
	    else if posterior >= 0.50 { 1.5 }
	    else { 0.0 };
	let odds = if present(trader_target_price) && present(trader_stop_loss) && present(current_price)
	    && trader_stop_loss > 0.0 && current_price > trader_stop_loss
	    && trader_target_price > current_price {
	    let profit = trader_target_price - current_price;
	    let loss = current_price - trader_stop_loss;
	    if loss > 0.0 { clamp(profit / loss, 0.0, 10.0) } else { 0.0 }
	} else { odds_fallback };
	// 记录赔率来源, 用于 decision_trail 追溯
	let odds_source = if present(trader_target_price) && present(trader_stop_loss)
	    && trader_stop_loss > 0.0 && trader_target_price > 0.0 { "trader" } else { "波动率fallback" };
	let q = 1.0 - posterior;
	let kelly_raw = if posterior >= 0.50 && odds > 0.001 {
	    (posterior * odds - q) / odds
	} else { 0.0 };
	let kelly_negative = kelly_raw <= 0.0001;
	// ── P3-1: 凯利仓位 — 调用 Rust 函数 ──
	// 内部处理: 半凯利 + 交易成本扣减 + 风险等级上限
	// 当 posterior<0.50 或 odds≤0.001 时返回 0.0（凯利不适用）
	let base_position_pct = pm_kelly_position(posterior, odds, COST_PCT, overall_risk);
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
	//   说明风险评估本身不确定性高, 应适度降低最终置信度。
	//   这是对"风险评估的可靠性"的独立修正，与 f4_signal(风险方向)正交。
	//   分歧度 0~100, 修正因子 = (100 - disagreement) / 100, 范围 [0.7, 1.0]
	//   P0 修复(2026-07-25): 原下限 0.3 导致数据质量A级时 confidence 也被砍到19%。
	//     风险已通过 f4_signal 进入贝叶斯 posterior, disagreement_mod 是二次惩罚。
	//     0.3 下限意味着分歧度70时 confidence 打三折 → 63%→19%, 让用户觉得分析不可信。
	//     提高到 0.7: 分歧度70时 confidence 打七折 → 63%→44%, 仍反映不确定性但不过度。
	let disagreement_mod = if present(risk_disagreement) {
	    clamp((100.0 - risk_disagreement) / 100.0, 0.7, 1.0)
	} else { 1.0 };
	// ── 置信度: 贝叶斯因子驱动 + 数据质量门控（P0 2026-07-25 重写 v3）──
	// 定义：置信度 = posterior 偏离 prior 的证据强度（贝叶斯因子）
	//   BF = posterior_odds / prior_odds (posterior≥prior时)
	//   BF = prior_odds / posterior_odds (posterior<prior时)
	//   BF ≥ 1,越大证据越强。BF=1 → 50%, BF=10 → 90.9%, BF=100+ → 99.5%+
	// 数据质量门控(P0 2026-07-25 新增): 上限与 dqi_score 挂钩
	//   A 级(dqi≥90) → 99.9 / B 级(dqi≥75) → 85 / C 级(dqi≥60) → 75 / D 级(dqi<60) → 60
	let bayes_confidence = pm_compute_bayes_confidence(prior, effective_posterior);
	let confidence_quality_cap = if present(dqi_score) && dqi_score >= 90.0 {
	    99.9
	} else if present(dqi_score) && dqi_score >= 75.0 {
	    85.0
	} else if present(dqi_score) && dqi_score >= 60.0 {
	    75.0
	} else if present(dqi_score) {
	    60.0
	} else {
	    99.9
	};
	// decision_confidence(方向确信度)保留供参考,但不是"置信度"
	let decision_confidence_raw = if effective_posterior >= 0.5 {
		effective_posterior
	} else {
		1.0 - effective_posterior
	} * 100.0;
	let signal_strength_raw = (effective_posterior - 0.5).abs() * 200.0;
	let final_confidence = clamp(bayes_confidence * disagreement_mod, 0.0, confidence_quality_cap);

	// ── M2 修复：LLM 失败级联兜底 ──
	// V55 扩展: 上游 AgentNode 走 strict_mode 兜底时（LLM 输出无法解析为合法 JSON），
	//   agent_executor 会注入 __untrusted=true 标记。下游若读到任一上游不可信，
	//   视为严重数据异常：强制 weights_collapsed（观望+空仓+confidence 对半），
	//   避免"50/50 中性兜底"被当成有效信号继续融合。
	// V56 优化 + V54 修复: 分级降级 — 单点失败不应全盘观望。
	//   level 2 全盘降级 (weights_collapsed): (total_weight/max_weight)<0.3 或 untrusted_count>=2
//     → 仅置信度×0.7，不强制空仓+观望（让 posterior 自然决定 action）
//   level 1 部分降级 (partial_degraded): untrusted_count==1
//     → 禁止买入/增持（降级为持有） + confidence×0.7，不强制空仓
//   理由: 6 个上游 Agent 中任一失败就全盘降级过于保守，
//     其余 5 个可信 Agent 的信号仍有价值，不应被单点失败完全否决。
//   P2 修复: weights_collapsed 改为比例阈值 (total_weight/max_weight<0.3)，
	//     避免在 regime_factor_weights 缩放权重后误判为坍缩。
	// V57 修复: 数据质量为 F 级时（dqi_score<25）视为决策基础不成立,
//   强制触发 weights_collapsed,即使 total_weight/max_weight≥0.3.
// 根因: 10 个分析师全缺失时,dqi_score=0 意味着分析师层完全无信号,
//   但 f4(算法风险)/f5(估值)/f7(trader) 等"非分析师"因子仍有数据,
//   导致 total_weight 仍可达 0.45+,f6_signal=-1.0 的负信号被稀释,
//   posterior 仍可达到 0.5+,confidence 显示为"中等置信"(60%~70%),
//   与"数据质量F级"语义完全矛盾。
// 数据质量 F 级阈值(data-quality.rhai line 138-142): score<25 → "F".
// V58 修复(2026-07-28): 因子完整度纳入数据质量评估
//   A/B 级数据(dqi_score≥65)表示分析师输出可靠,因子缺失不应触发全盘降级
//   C/D/F 级数据(dqi_score<65)时,因子缺失+分析师不可靠才触发坍缩
// V66 修复(2026-07-29): A 级数据完全豁免 weights_collapsed
//   依据: dqi_score 公式(data-quality.rhai:299)已包含 factor_completeness×0.20,
//         A 级(dqi≥85)意味着报告质量+工具可信度+因子完整度三方面均优秀,
//         因子缺失已通过 factor_completeness 体现在 dqi_score 中,无需双重惩罚。
//   注意: 此豁免依赖 dqi_score 包含 factor_completeness 维度。若未来 dqi_score
//         公式移除 factor_completeness,则需重新评估 A 级豁免的合理性。
let dqi_collapsed = present(dqi_score) && dqi_score < 25.0;
// V67 修复(2026-07-29): A 级豁免增加 untrusted_count==0 前提条件。
//   原代码 dqi>=85 即完全豁免 weights_collapsed,但 dqi_score 可能因
//   report_quality + tool_credibility 满分而达 A 级,此时若存在 untrusted 分析师,
//   豁免会导致不可信信号被当成有效信号继续融合。
//   新条件: dqi>=85 且 untrusted_count==0 才完全豁免。
//   若 dqi>=85 但 untrusted_count>=1,降级为 B 级处理(threshold=0.15)。
let dqi_a_level = present(dqi_score) && dqi_score >= 85.0 && untrusted_count == 0;
let dqi_high_quality = present(dqi_score) && dqi_score >= 65.0;  // A/B 级
// 因子权重坍缩阈值与数据质量联动:
//   A 级(dqi≥85 且 untrusted==0): 完全豁免（threshold=0.0 永不触发，且跳过 untrusted/dqi_collapsed）
//   B 级(65≤dqi<85 或 dqi≥85但有untrusted): 仅当 total_weight/max_weight<0.15 时触发
//   C/D/F 级(dqi<65): total_weight/max_weight<0.3 时触发(因子+分析师双重不可靠)
let weight_collapse_threshold = if dqi_a_level { 0.0 }
    else if dqi_high_quality { 0.15 }
    else { 0.3 };
let weights_collapsed = !dqi_a_level && (
    (total_weight / max_weight) < weight_collapse_threshold
    || untrusted_count >= 2
    || dqi_collapsed
);
	let partial_degraded = untrusted_count == 1;
	// 三级降级策略（C2.1 修复：原二级退化为单级，现恢复三分支）
	//   分支 A: weights_collapsed → 0% 仓位 + confidence × 0.5
	//   分支 B: untrusted_count==1 且 posterior≥0.70 → 仓位上限 30% + confidence × 0.85 + 保留买入/增持
	//   分支 C: untrusted_count==1 且 posterior<0.70 → 禁止加仓 + confidence × 0.70
	let partial_high_confidence = partial_degraded && posterior >= 0.70;
	let partial_low_confidence = partial_degraded && posterior < 0.70;
	if weights_collapsed {
	    // 分支 A: 权重坍缩或≥2 个上游不可信 → 强制空仓(仓位控制已够防御)
	    // 置信度来自假设检验,不受坍缩影响。
	    decision_confidence_raw = clamp(decision_confidence_raw * 0.5, 0.0, 100.0);
	    signal_strength_raw = clamp(signal_strength_raw * 0.5, 0.0, 100.0);
	    position_pct = 0.0;
	} else if partial_high_confidence {
	    // 分支 B: 单点不可信但后验高置信 → 保留买入/增持，仓位上限 30%（在仓位对齐段应用）
	    decision_confidence_raw = clamp(decision_confidence_raw * 0.85, 0.0, 100.0);
	    signal_strength_raw = clamp(signal_strength_raw * 0.85, 0.0, 100.0);
	} else if partial_low_confidence {
	    // 分支 C: 单点不可信且后验低置信 → 禁止加仓 + 方向确信度 × 0.70
	    decision_confidence_raw = clamp(decision_confidence_raw * 0.70, 0.0, 100.0);
	    signal_strength_raw = clamp(signal_strength_raw * 0.70, 0.0, 100.0);
	}
	// C2.1: 三级降级 decision_trail（原 696-713 行前向引用为死代码，移至此处方能生效）
	if weights_collapsed {
	    // V68 修复(2026-07-29): 用 weight_collapse_threshold 替代硬编码 0.3，
	    //   保持与上方 weights_collapsed 判定逻辑一致。
	    //   原问题: B 级数据 threshold=0.15，但此处硬编码 0.3 导致 dqi_collapsed 分支
	    //   条件错位——B 级数据 total_weight/max_weight=0.2 时,
	    //     weights_collapsed 判定为 true（0.2 < 0.15 ❌ 实际 false, 但若 < 0.15 则 true）
	    //     此处 !(0.2 < 0.3) = false → 走 else 分支「因子权重坍缩」措辞
	    //   对 B 级数据应使用 0.15 阈值,否则 collapse_reason 文案与实际触发条件不匹配。
	    let collapse_reason = if dqi_collapsed && !((total_weight / max_weight) < weight_collapse_threshold) && untrusted_count < 2 {
	        `数据质量F级(dqi_score=${(dqi_score*10.0).round()/10.0}<25), 10个分析师基本全缺失, 0%仓位+置信度×0.5`
	    } else {
	        `因子权重坍缩(${(total_weight*100.0).to_int()}%)或不可信上游≥2,0%仓位+置信度×0.5`
	    };
	    decision_trail.push(#{
	        "rule_id": if dqi_collapsed { "R-206" } else { "R-203" },
	        "status": "DOWNGRADED",
	        "detail": collapse_reason,
	        "timestamp": 0.0,
	    });
	} else if partial_high_confidence {
	    decision_trail.push(#{
	        "rule_id": "R-205",
	        "status": "DOWNGRADED",
	        "detail": `单点上游不可信(untrusted_count=1,posterior≥0.70),仓位≤30%+置信度×0.85`,
	        "timestamp": 0.0,
	    });
	} else if partial_low_confidence {
	    decision_trail.push(#{
	        "rule_id": "R-205",
	        "status": "DOWNGRADED",
	        "detail": `单点上游不可信(untrusted_count=1,posterior<0.70),禁止加仓+置信度×0.70`,
	        "timestamp": 0.0,
	    });
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
	if effective_posterior >= ACTION_BUY_THRESHOLD && position_pct >= POS_BUY_MIN {
	    base_action = "买入";
	// 增持区（effective_posterior≥0.53 且仓位达标）
	} else if effective_posterior >= ACTION_INCREASE_THRESHOLD && position_pct >= POS_INCREASE_MIN {
	    base_action = "增持";
	// 持有区（effective_posterior≥0.48，与"观望"的模糊边界）
	} else if effective_posterior >= ACTION_HOLD_THRESHOLD {
	    base_action = "持有";
	// 观望区（posterior 0.38-0.50，与"减持"的模糊边界）
	} else if effective_posterior >= ACTION_WATCH_THRESHOLD {
	    base_action = "观望";
	// 减持区（posterior 0.30-0.40，与"卖出"的模糊边界）
	} else if effective_posterior >= ACTION_REDUCE_THRESHOLD {
	    base_action = "减持";
	}

	// P0-1 修复: R-204 移至 position_pct 与 base_action 都定义之后（原位置前向引用为死代码）
	// 记录零仓位修正
	if position_pct <= 0.0 && (base_action == "买入" || base_action == "增持" || base_action == "持有") {
	    decision_trail.push(#{
	        "rule_id": "R-204",
	        "status": "DOWNGRADED",
	        "detail": `零仓位修正:${base_action}→观望(仓位0%,不可持股)`,
	        "timestamp": 0.0,
	    });
	}

	// ── 最终 action：sanity 不通过则强制降级 ──
	// V54 修复: weights_collapsed 不再强制"观望"，让 posterior 自然决定 action
	let final_action = if !trader_data_valid {
	    // 数据异常时：根据 posterior 决定是"观望"还是"卖出"
	    if posterior < 0.45 { "卖出" } else { "观望" }
	} else if partial_low_confidence && (base_action == "买入" || base_action == "增持") {
	    // 分支 C: 单点不可信+低置信 — 禁止加仓，降级为"持有"（可继续持仓但不追加）
	    "持有"
	} else if position_pct <= 0.0 && (base_action == "买入" || base_action == "增持" || base_action == "持有") {
	    // V42: 仓位为 0 时不应给出持仓性建议。若 base_action 暗示持仓但实际空仓，
	    // 降级为"观望"（后验仍可保留，但行动上有零仓位匹配）
	    "观望"
	} else { base_action };

	// ── P3-1: 风控否决 — 调用 Rust 函数 ──
	// 高风险→禁止买入/增持(降级为持有), 极高风险→禁止买入/增持/持有(降级为观望)
	let original_action = final_action;
	final_action = pm_risk_veto(final_action, overall_risk);
	// 检测是否触发了风控降级，供 decision_trail 和 reasoning 使用
	let risk_downgraded = original_action != final_action;
	let risk_downgrade_note = if risk_downgraded {
	    if overall_risk == "极高风险" || overall_risk == "极高" {
	        ` | ⚠️极高风险风控否决:${original_action}→${final_action}(极高风险不应继续持仓)`
	    } else if overall_risk == "高风险" || overall_risk == "高" {
	        ` | ⚠️高风险风控否决:${original_action}→${final_action}(高风险禁止加仓)`
	    } else { "" }
	} else { "" };

	// P0-1 修复: R-200 移至 risk_downgraded 定义之后（原位置前向引用为死代码）
	// 记录风险否决
	if risk_downgraded {
	    decision_trail.push(#{
	        "rule_id": "R-200",
	        "status": "DOWNGRADED",
	        "detail": risk_downgrade_note,
	        "timestamp": 0.0,
	    });
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

	// P0-1 修复: R-201 移至 bearish_downgraded 定义之后（原位置前向引用为死代码）
	// 记录空头预测否决
	if bearish_downgraded {
	    decision_trail.push(#{
	        "rule_id": "R-201",
	        "status": "DOWNGRADED",
	        "detail": bearish_downgrade_note,
	        "timestamp": 0.0,
	    });
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
	// C2.1 分支 B: 单点不可信但高置信 — 仓位上限 30%
	if partial_high_confidence && position_pct > 30.0 {
	    position_pct = 30.0;
	}

	let contradiction_note = if !trader_data_valid {
	    ` | ⚠️trader数据异常:${sanity_reason},action已从${base_action}修正为${final_action}`
	} else if position_pct <= 0.0 && base_action != final_action {
	    ` | ⚠️零仓位修正:${base_action}→${final_action}(仓位0%,不可持股)`
	} else if final_action == "观望" || final_action == "减持" || final_action == "卖出" {
	    ` | ⚠️仓位修正:action=${final_action}仓位调至${position_pct}%`
	} else { "" };

	// C2.1 三级降级标注
	// V69 修复(2026-07-29): 此处 0.3 同步替换为 weight_collapse_threshold，
	//   与 L1102 的 V68 修复保持一致，避免 B 级数据文案与触发条件错位。
let collapse_note = if weights_collapsed {
    if dqi_collapsed && !((total_weight / max_weight) < weight_collapse_threshold) && untrusted_count < 2 {
        ` | ⚠️数据质量F级(dqi_score=${(dqi_score*10.0).round()/10.0}<25),0%仓位+置信度×0.5`
    } else {
        ` | ⚠️因子权重坍缩(${(total_weight*100.0).to_int()}%)或不可信上游≥2,0%仓位+置信度×0.5`
    }
} else if partial_high_confidence {
    ` | ⚠️单点上游不可信(untrusted_count=1,posterior≥0.70),仓位≤30%+置信度×0.85`
} else if partial_low_confidence {
    ` | ⚠️单点上游不可信(untrusted_count=1,posterior<0.70),禁止加仓+置信度×0.70`
} else { "" };

	// V43 修复: position_pct 用 clamp(只压不抬), 避免 risk_note 把凯利低仓位
	//   (如 21%)强行抬高到风险上限(35%), 违背"高风险限制仓位上限"的语义。
	//   风控是上限不是强制值, 低仓位不应被抬高。
	// V43: 去掉"action阈值左移"描述, 因风险 action 约束已改为硬性否决(见上方 risk_downgrade)
	let risk_note = if overall_risk == "极高风险" {
	    if position_pct > POS_CAP_EXTREME { position_pct = POS_CAP_EXTREME; }
	    ` | ⚠️极高风险:仓位≤${POS_CAP_EXTREME}%`
	} else if overall_risk == "高风险" || overall_risk == "高" {
	    if position_pct > POS_CAP_HIGH { position_pct = POS_CAP_HIGH; }
	    ` | ⚠️高风险:仓位≤${POS_CAP_HIGH}%`
	} else if (overall_risk == "中风险" || overall_risk == "中") && position_pct > POS_CAP_MID {
	    position_pct = POS_CAP_MID;
	    ` | ⚠️中风险:仓位上限${POS_CAP_MID}%`
	} else { "" };

// ── 模拟门（S-501~503）：基于多 Agent 市场仿真的硬性约束 ──
// 在 final_action/position_pct 确定后执行，作为决策输出前的最后一道门。
// 需要上游 market-sim CodeNode 注入 sim_stability/sim_liquidity/sim_impact。
let sim_vetoed = false;
let sim_veto_rule = "";
let sim_veto_reason = "";
// 记录模拟门前的原始决策状态（供前端对比展示）
let pre_sim_action = final_action;
let pre_sim_position_pct = position_pct;

if present(sim_stability) && type_of(sim_stability) == "f64"
   && present(sim_liquidity) && type_of(sim_liquidity) == "f64" {
    // S-501: 价格极度不稳定 → 强制观望
    if sim_stability < 0.3 {
        sim_vetoed = true;
        sim_veto_rule = "S-501";
        sim_veto_reason = `市场模拟显示价格稳定性不足(${(sim_stability*100.0).round()}%), ${final_action}→观望`;
        final_action = "观望";
        position_pct = 0.0;
    }
    // S-502: 流动性极差 → 仓位上限减半
    if !sim_vetoed && sim_liquidity < 0.2 {
        sim_vetoed = true;
        sim_veto_rule = "S-502";
        let old_pos = position_pct;
        position_pct = clamp(position_pct * 0.5, 0.0, 50.0);
        sim_veto_reason = `市场模拟显示流动性不足(${(sim_liquidity*100.0).round()}%), 仓位从${old_pos}%降至${position_pct}%`;
    }
    // S-503: 大单冲击过大 → 仓位上限减半 + 提示
    if !sim_vetoed && present(sim_impact) && type_of(sim_impact) == "f64" && sim_impact > 50.0 {
        sim_vetoed = true;
        sim_veto_rule = "S-503";
        let old_pos = position_pct;
        position_pct = clamp(position_pct * 0.5, 0.0, 50.0);
        sim_veto_reason = `市场模拟显示大单冲击成本过高(${sim_impact}bps), 仓位从${old_pos}%降至${position_pct}%`;
    }
    if sim_vetoed {
        decision_trail.push(#{
            "rule_id": sim_veto_rule,
            "status": "VETOED",
            "detail": sim_veto_reason,
        });
    }
}

// ── 缺失数据 ──
let data_gaps = [];
if !present(totalScore) { data_gaps.push("技术服务评分(t-scoring)"); }
if !present(consensusScore) { data_gaps.push("辩论共识分数(debate-convergence)"); }
if !present(catalyst_level) || catalyst_level == "" { data_gaps.push("催化剂评估(a-catalyst)"); }
// overall_risk 始终存在（算法分类确保兜底）
if !present(money_flow) { data_gaps.push("资金流向(t-hotmoney-data)"); }
if !present(lockup_bundle) { data_gaps.push("筹码面数据(t-lockup-data)"); }
if !present(dragon_tiger) { data_gaps.push("龙虎榜数据(t-dragon-tiger-data)"); }
if !present(announcements) { data_gaps.push("公告数据(t-catalyst-data)"); }
if !present(risk_pe) || risk_pe <= 0.0 { data_gaps.push("PE数据(t-risk)"); }
if !present(pace_signal) { data_gaps.push("PACE情绪因子(pace-calc)"); }
// evidence_pct = total_weight / max_weight * 100（max_weight 已在贝叶斯融合段计算）
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
let kelly_note = ` | 赔率=${(odds*10.0).round()/10.0}(${odds_source}) 凯利=${(kelly_raw*1000.0).round()/1000.0} 成本=${(COST_PCT*100.0).to_int()}%`;
// V50: 分歧度修正标注（仅在 disagreement 显著时标记）
let disagreement_note = if present(risk_disagreement) && risk_disagreement > 50 {
    ` | ⚠️风险分歧(${risk_disagreement})置信修正×${(disagreement_mod*100.0).round().to_int()}%`
} else { "" };

// ── 输出 ──
	pm_result = #{
    "action": final_action,
    "positionPct": (position_pct * 10.0).round() / 10.0,
    "positionPctNormalized": (position_pct * 10.0).round() / 10.0,  // P0-2: 组合归一化前的原始仓位权重
    "maxPositionPct": if overall_risk == "极高风险" { POS_CAP_EXTREME }
        else if overall_risk == "高风险" { POS_CAP_HIGH }
        else if overall_risk == "中风险" { POS_CAP_MID }
        else { 95.0 },  // P0-2: 本标的在当前风险等级下的单标的上限
    "portfolioExposureWarning": if position_pct > 33.3 {
        `⚠️仓位${position_pct}%超过33%——组合层面若持有3个以上同仓位标的，总敞口将超100%`
    } else { "" },  // P0-2: 组合敞口预警（33%=3标的中性仓位阈值）
    "confidence": (final_confidence * 10.0).round() / 10.0,
    // P0: 原始上涨概率 posterior × 100（与 confidence 对比参考）
    "posterior": ((effective_posterior * 100.0) * 10.0).round() / 10.0,
    // V58: 决策方向置信度 — 无论买卖方向都体现"多确信"（解决看空决策 confidence 偏低被误读为"不确信"）
    "decisionConfidence": (decision_confidence_raw * 10.0).round() / 10.0,
    // V58: 信号强度 — 偏离中性的程度，0=完全中性，100=极端强信号
    "signalStrength": (signal_strength_raw * 10.0).round() / 10.0,
    // V66 修复(2026-07-29): weights_collapsed 状态结构化输出，供前端 Tag+Tooltip 可见化
    "weightsCollapsed": weights_collapsed,
    "collapseReason": if !weights_collapsed { "none" }
        else if dqi_collapsed && !((total_weight / max_weight) < weight_collapse_threshold) && untrusted_count < 2 {
            "dqi_collapsed"
        }
        else if untrusted_count >= 2 { "multi_untrusted" }
        else { "low_weight_ratio" },
    "weightRatio": ((total_weight / max_weight) * 100.0 * 10.0).round() / 10.0,
    "untrustedCount": untrusted_count,
    "riskLevel": overall_risk,
    "riskSource": risk_source,
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
        "oddsSource": odds_source,
        "tradingCostPct": (COST_PCT*100.0).to_int(),
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
    "pre_sim_action": pre_sim_action,
    "pre_sim_position_pct": pre_sim_position_pct,
    "technical_veto": #{
        "vetoed": tech_vetoed,
        "rule_id": tech_veto_rule,
        "reason": tech_veto_reason,
    },
    "simulation_gate": #{
        "vetoed": sim_vetoed,
        "rule_id": sim_veto_rule,
        "reason": sim_veto_reason,
        "pre_sim_action": pre_sim_action,
        "pre_sim_position_pct": pre_sim_position_pct,
        "sim_stability": if present(sim_stability) { (sim_stability*100.0).round()/100.0 } else { () },
        "sim_liquidity": if present(sim_liquidity) { (sim_liquidity*100.0).round()/100.0 } else { () },
        "sim_impact": if present(sim_impact) { sim_impact } else { () },
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

}

// ── 错误兜底：Rhai 运行时异常时返回保守决策，确保节点始终输出可解析结果 ──
catch (err) {
    pm_result = #{
        "action": "观望",
        "positionPct": 0.0,
        "confidence": 50.0,
        "decisionConfidence": 50.0,
        "signalStrength": 0.0,
        "riskLevel": "中风险",
        "riskSource": "错误兜底",
        "stopLossPct": 0.0,
        "takeProfitPct": 0.0,
        "reasoning": "portfolio-mgr Rhai 执行异常，已降级为保守决策: " + err,
        "data_gaps": [],
        "timeHorizon": "short",
        "expectedHoldingDays": 5,
        "isContradictory": false,
        "targetTimeframe": "1w",
        "evidence": #{ "prior": 0.5, "posterior": 0.5, "activeFactors": 0.0, "totalWeight": 0.0 },
        "risk_quant": #{ "f4_base": 0.0, "f4_quant_adjust": 0.0, "f4_adjusted": 0.0, "notes": [] },
        "decision_trail": [],
        "simulation_gate": #{ "vetoed": false, "rule_id": "", "reason": "" },
        "computation_logs": "ERROR: " + err,
        "f7_free": #{ "posterior": 0.5, "action": "观望", "effective_posterior": 0.5, "f7_weight": 0.15, "total_weight": 0.0, "f7_weight_pct": 0.0 }
    };
}

pm_result
