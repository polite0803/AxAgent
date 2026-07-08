import type { Variable, WorkflowTemplateInput, WorkflowTemplateResponse } from "@/components/workflow/types";
import { invoke } from "@/lib/invoke";
import { App, Button, Input, InputNumber, Select, Slider, Space, Switch, Tag, theme } from "antd";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { SettingsGroup } from "./SettingsGroup";

const TEMPLATE_ID = "stock-analysis";

/** Generate default parameter variable list (initial load, sync with stock-analysis workflow template v19).
 * Naming convention: snake_case, must match seed keys in `stock_analysis_setup.rs`.
 * Update this when backend defaults change to keep UI in sync with actual runtime params. */
function getDefaultVariables(): Variable[] {
  const vars: Variable[] = [];
  const b = (name: string, val: unknown, desc: string, type: string) =>
    vars.push({ name, var_type: type, value: val, description: desc, is_secret: false });
  // 分析流程
  b("analysis_depth", "standard", "分析深度: quick / standard / deep", "enum");
  b("debate_rounds", 3, "多空辩论轮数 (1-10)", "number");
  b("max_concurrent", 12, "并行分析的 Agent 数量上限 (1-32)", "number");
  // 数据源参数
  b("kline_period", "daily", "K线周期: daily / weekly / monthly", "enum");
  b("kline_limit", 120, "K线数量 (60-500)", "number");
  b("news_limit", 30, "新闻数量 (10-100)", "number");
  // Agent 节点 LLM 参数
  b("agent_temperature", 0.3, "LLM 温度 (0-2)", "number");
  b("agent_max_tokens", 4096, "LLM 最大输出 token 数", "number");
  b("agent_timeout_secs", 300, "每个 Agent 节点执行超时秒数", "number");
  b("agent_retry_max", 2, "每个 Agent 节点最大重试次数", "number");
  // Tool 节点参数
  b("tool_timeout_secs", 30, "每个 Tool 节点执行超时秒数", "number");
  b("tool_retry_max", 2, "每个 Tool 节点最大重试次数", "number");
  // 评分权重
  b("scoring_trend", 30, "趋势评分权重 (0-100)", "number");
  b("scoring_deviation", 20, "偏离度评分权重 (0-100)", "number");
  b("scoring_macd", 15, "MACD 评分权重 (0-100)", "number");
  b("scoring_volume", 15, "量能评分权重 (0-100)", "number");
  b("scoring_rsi", 10, "RSI 评分权重 (0-100)", "number");
  b("scoring_support", 10, "支撑阻力评分权重 (0-100)", "number");
  b("scoring_boll", 5, "布林带评分权重 (0-100)", "number");
  // 规则阈值
  b("rule_rsi_overbought", 80, "RSI 超买阈值 (50-100)", "number");
  b("rule_rsi_oversold", 20, "RSI 超卖提醒阈值", "number");
  b("rule_bias_limit_pct", 5, "乖离率追高阈值 (%)", "number");
  b("rule_volume_signal_block", true, "放量下跌时禁止买入", "boolean");
  b("rule_bear_low_score", 30, "空头+低分禁买阈值", "number");
  b("rule_auto_stop_loss_pct", 5, "自动止损百分比 (%)", "number");
  // 仓位限制
  b("pos_max_single_pct", 20, "单股最大仓位 (%)", "number");
  b("pos_max_total", 10, "最大持仓数量", "number");
  b("pos_max_sector_pct", 40, "单一行业最大暴露 (%)", "number");
  b("pos_min_cash_pct", 5, "最低现金比例 (%)，低于则禁新开仓", "number");
  b("pos_max_turnover_pct", 100, "单期最大换手率 (%)，超过则分批调仓", "number");
  // 估值参数
  b("value_dcf_growth_rate", 8, "DCF 增长率 (%)", "number");
  b("value_dcf_perpetual_rate", 3, "DCF 永续增长率 (%)", "number");
  b("value_dcf_discount_rate", 10, "DCF 折现率 (%)", "number");
  b("value_moat_threshold", 60, "宽护城河阈值 (30-90)", "number");
  b("value_fscore_buy", 7, "F-Score 买入阈值 (3-9)", "number");
  b("value_safety_margin", 20, "最低安全边际 (%)", "number");
  // 护城河量化（value.rs:320）
  b("moat_roe_years_min", 3, "ROE>15% 最少连续年数 (0-10)", "number");
  b("moat_avg_gross_margin_min", 20, "平均毛利率下限 (%)", "number");
  b("moat_margin_stable_std_max", 5, "毛利率稳定性标准差上限 (σ，%)", "number");
  b("moat_fcf_ratio_min", 0.5, "FCF/净利润 比率下限 (0-1)", "number");
  // 监控
  b("monitor_poll_interval_secs", 30, "监控轮询间隔 (秒)", "number");
  b("monitor_change_pct", 5, "涨跌幅异常阈值 (%)", "number");
  b("monitor_turnover", 10, "换手率异常阈值 (%)", "number");
  b("monitor_alert_cooldown_secs", 300, "同一标的告警冷却时间 (秒，10-3600)", "number");
  b("monitor_min_severity", "info", "最低推送告警等级: info / warn / critical", "enum");
  b("monitor_channels", "in_app", "推送渠道，逗号分隔: in_app / lark / email / webhook", "string");
  b("dual_view_disagreement_threshold", 40, "双视角分歧阈值 (0-100)，低于此值标记为需人工复核", "number");
  // 风险/置信度
  b("min_confidence", 60, "最低置信度 (0-100)", "number");
  b("var_confidence", 0.95, "VaR 置信度 (0-1)", "number");
  b("kelly_fraction", 0.5, "凯利公式下注比例", "number");
  b("kelly_min_win_rate", 0.4, "凯利最低胜率要求 (0-1)，低于此值返回不适用", "number");
  b("kelly_min_odds", 1.0, "凯利最低赔率要求 (avg_win/avg_loss)", "number");
  b("risk_free_rate", 0.03, "无风险利率 (小数)", "number");
  b("outlier_method", "zscore", "异常值处理方法: zscore / iqr", "enum");
  b("outlier_threshold", 2.0, "异常值 Z-Score 阈值", "number");
  b("risk_max_drawdown_limit", 15, "组合最大回撤熔断线 (%)，超过则暂停新开仓", "number");
  b("risk_max_daily_loss_pct", 3, "单日最大亏损 (%)，超过则停手", "number");
  b("risk_correlation_lookback_days", 60, "风险平价/相关性矩阵的回看窗口 (交易日)", "number");
  // 选股筛选（screener.rs:8 ScreenCriteria）
  b("screener_min_change_pct", -30, "选股最小涨跌幅下限 (%)", "number");
  b("screener_max_change_pct", 30, "选股最大涨跌幅上限 (%)", "number");
  b("screener_main_inflow_min", 0, "主力净流入下限 (万元)，0=不限", "number");
  b("screener_northbound_ratio_min", 0, "北向持仓占比下限 (%)，0=不限", "number");
  b("screener_turnover_rate_min", 0, "换手率下限 (%)，0=不限", "number");
  b("screener_rsi_oversold", false, "选股时要求 RSI 超卖 (<30)", "boolean");
  b("screener_rsi_overbought", false, "选股时要求 RSI 超买 (>70)", "boolean");
  // 行业财务基线参考股票代码（stock_analysis_setup 中 t-baseline-* 节点使用）
  b("ref_semi_code", "002371", "半导体基线参考代码（北方华创）", "string");
  b("ref_battery_code", "300750", "电池基线参考代码（宁德时代）", "string");
  b("ref_chem_code", "600309", "化工基线参考代码（万华化学）", "string");
  b("ref_med_code", "688981", "医药基线参考代码（中芯国际）", "string");
  b("ref_aero_code", "600760", "军工基线参考代码（中航沈飞）", "string");
  b("ref_consumer_elec_code", "002475", "消费电子基线参考代码（立讯精密）", "string");
  b("ref_auto_code", "600104", "汽车基线参考代码（上汽集团）", "string");
  // 信号检测（signals.rs detect_ma_cross / detect_breakout）
  b("signal_ma_fast", 5, "MA 金叉检测快线周期 (3-30)", "number");
  b("signal_ma_slow", 20, "MA 金叉检测慢线周期 (10-120)", "number");
  b("signal_breakout_volume_mult", 1.5, "突破/破位放量倍数阈值 (1.0-3.0)", "number");
  // 关键价位（key_levels.rs KeyLevelTracker）
  b("keylevel_lookback_days", 60, "关键价位回看窗口 (交易日，10-250)", "number");
  b("keylevel_touch_tolerance_pct", 1.0, "关键价位触碰容差 (%，0.1-5.0)", "number");
  b("keylevel_min_touches", 2, "确认支撑/阻力最少触碰次数 (1-10)", "number");
  // 推荐器策略开关（recommender/strategies）
  b("reco_trend_enabled", true, "启用趋势跟踪子策略", "boolean");
  b("reco_reversion_enabled", true, "启用超跌反弹子策略", "boolean");
  b("reco_value_enabled", true, "启用价值选股子策略", "boolean");
  b("reco_capital_enabled", true, "启用资金流向子策略", "boolean");
  b("reco_watchlist_enabled", true, "启用自选股策略", "boolean");
  b("reco_min_confidence", 60, "推荐器最低置信度 (0-100)，低于此值不入选", "number");
  // 决策回溯（decision_tracker.rs）
  b("decision_max_history_per_stock", 50, "每只股票保留的历史决策条数 (10-200)", "number");
  // 技术指标周期（indicators.rs IndicatorConfig）
  b("macd_fast", 12, "MACD 快线周期 (5-30)", "number");
  b("macd_slow", 26, "MACD 慢线周期 (10-60)", "number");
  b("macd_signal", 9, "MACD 信号线周期 (3-20)", "number");
  b("boll_period", 20, "布林带周期 (10-50)", "number");
  b("boll_stddev", 2.0, "布林带标准差倍数 (1.0-3.0)", "number");
  b("volume_lookback", 5, "均量计算回看周期 (3-30，交易日)", "number");
  b("volume_surge_ratio", 1.5, "放量阈值（量比 > 此值）", "number");
  b("volume_shrink_ratio", 0.7, "缩量阈值（量比 < 此值）", "number");
  // 推荐器参数（recommender/strategies）
  b("trend_kline_limit", 250, "趋势策略 K 线上限", "number");
  b("trend_amount_ratio_min", 0.8, "趋势策略最低量比", "number");
  b("rev_rsi_short_max", 35, "超跌反弹短线 RSI 上限", "number");
  b("rev_drawdown_min_pct", 20, "超跌反弹中线最低回撤 (%)", "number");
  b("rev_rsi_monthly_max", 50, "超跌反弹月线 RSI 上限", "number");
  b("val_pe_short_max", 50, "价值策略短线 PE 上限", "number");
  b("val_pe_mid_max", 40, "价值策略中线 PE 上限", "number");
  b("val_pb_mid_max", 8, "价值策略中线 PB 上限", "number");
  b("cap_inflow_short_min", 200, "资金策略短线主力净流入下限 (万元)", "number");
  b("cap_inflow_mid_min", 500, "资金策略中线主力净流入下限 (万元)", "number");
  b("cap_turnover_min", 2, "资金策略最低换手率 (%)", "number");
  b("cap_nb_ratio_min", 0.3, "资金策略北向持仓占比下限 (%)", "number");
  // 交易决策（trading.rs）
  b("trading_price_deviation_limit", 5, "交易价偏离目标价容忍度 (%)", "number");
  // 风险模型扩展（risk.rs）
  b("risk_sharpe_annualization", 252, "夏普年化因子（252=日频，12=月频）", "number");
  b("risk_kelly_heavy_threshold", 0.25, "凯利重仓阈值", "number");
  b("risk_kelly_medium_threshold", 0.1, "凯利中仓阈值", "number");
  // 风险组合（compute_portfolio_risk / compute_scoring / compute_valuation）
  b("fscore_roe_min", 0.10, "F-Score ROE 最低要求 (小数)", "number");
  b("fscore_gross_margin_min", 0.30, "F-Score 毛利率最低要求 (小数)", "number");
  b("fscore_net_margin_min", 0.10, "F-Score 净利率最低要求 (小数)", "number");
  b("fscore_debt_max", 0.60, "F-Score 负债率上限 (小数)", "number");
  b("fscore_pe_max", 20, "F-Score PE 上限", "number");
  b("val_pe_low", 15, "基本面修正 PE 低估阈值", "number");
  b("val_pe_high", 50, "基本面修正 PE 高估阈值", "number");
  b("val_pb_low", 1.0, "基本面修正 PB 低估阈值", "number");
  b("val_pb_high", 6.0, "基本面修正 PB 高估阈值", "number");
  b("risk_hhi_concentrated", 0.25, "组合 HHI 高度集中阈值 (0-1)", "number");
  b("risk_hhi_medium", 0.15, "组合 HHI 中度集中阈值 (0-1)", "number");
  b("risk_divers_high", 8, "组合有效股票数充分分散阈值", "number");
  b("risk_divers_medium", 4, "组合有效股票数适度分散阈值", "number");
  // 凯利公式默认值
  b("kelly_default_win_rate", 0.5, "凯利公式默认胜率", "number");
  b("kelly_default_avg_win", 0.05, "凯利公式默认平均盈利", "number");
  b("kelly_default_avg_loss", 0.05, "凯利公式默认平均亏损", "number");
  // 技术指标周期
  b("atr_period", 14, "ATR 平均真实波幅周期", "number");
  b("kdj_n", 9, "KDJ 随机指标 N 周期", "number");
  // 数据清洗
  b("fill_missing_method", "forward", "缺失值填充方法: forward / linear", "enum");
  // 突破检测
  b("breakout_volume_threshold", 1.5, "支撑阻力突破的成交量确认阈值", "number");
  // 业绩超预期分级阈值
  b("earnings_th_huge_pos", 50, "业绩超预期: 大幅超预期下界 (%)", "number");
  b("earnings_th_strong_pos", 20, "业绩超预期: 强超预期下界 (%)", "number");
  b("earnings_th_mild_pos", 5, "业绩超预期: 略超预期下界 (%)", "number");
  b("earnings_th_mild_neg", -5, "业绩超预期: 略低于预期下界 (%)", "number");
  b("earnings_th_strong_neg", -20, "业绩超预期: 强低于预期下界 (%)", "number");
  b("earnings_th_huge_neg", -50, "业绩超预期: 大幅低于预期下界 (%)", "number");
  // 质押风险分级阈值
  b("pledge_warning_line", 50, "大股东质押比例预警线 (%)", "number");
  b("pledge_liquidation_line", 70, "大股东质押比例平仓线 (%)", "number");
  b("pledge_medium_line", 30, "大股东质押中风险阈值 (%)", "number");
  b("pledge_low_line", 10, "大股东质押低风险阈值 (%)", "number");
  // 蒙特卡洛模拟默认参数
  b("mc_default_price", 10, "蒙特卡洛模拟默认价格", "number");
  b("mc_default_return", 0.08, "蒙特卡洛模拟默认年化收益 (小数)", "number");
  b("mc_default_volatility", 0.3, "蒙特卡洛模拟默认年化波动率 (小数)", "number");
  b("mc_default_days", 30, "蒙特卡洛模拟默认天数", "number");
  b("mc_default_simulations", 1000, "蒙特卡洛模拟默认路径数", "number");
  // 行业内估值/增长对比阈值
  b("industry_pe_cheap", 1.0, "行业内 PE 相对低估阈值", "number");
  b("industry_pe_expensive", 1.5, "行业内 PE 相对高估阈值", "number");
  b("industry_growth_high", 1.2, "行业内高增长阈值", "number");
  // 涨停潜力评分
  b("limit_pct_main", 10, "主板涨停幅度 (%)", "number");
  b("limit_pct_star", 20, "创业板/科创板涨停幅度 (%)", "number");
  b("limit_pct_bj", 30, "北交所涨停幅度 (%)", "number");
  b("limit_up_w_trend", 40, "涨停潜力评分 - 趋势权重", "number");
  b("limit_up_w_volume", 20, "涨停潜力评分 - 量能权重", "number");
  b("limit_up_w_hits", 15, "涨停潜力评分 - 历史涨停权重", "number");
  b("limit_up_th_high", 60, "涨停潜力 - 高潜力阈值", "number");
  b("limit_up_th_med", 30, "涨停潜力 - 中潜力阈值", "number");
  b("limit_up_th_low", 10, "涨停潜力 - 低潜力阈值", "number");
  // 注意：vendor_* 9 个开关 + iwencai_key 不在这里暴露，
  // 由「数据源」tab（DataVendorsTab）全权管理，避免两边同时写造成竞态。
  // 全局开关
  b("analysis_dry_run", false, "干跑模式：不调用 LLM，用 mock 输出验证流程", "boolean");
  return vars;
}

function parseEnumOptions(desc?: string): string[] {
  if (!desc) { return []; }
  const match = desc.match(/: (.+)/);
  if (match) { return match[1].split(/\s*\/\s*/).map((s) => s.trim()); }
  return [];
}

function inferStep(v: Variable): number {
  if (v.description?.includes("温度")) { return 0.1; }
  return 1;
}

// eslint-disable-next-line @typescript-eslint/no-empty-object-type
interface Props {}

/** number control — vertical on narrow screen, horizontal on wide */
function NumberControl({ v, value, onChange }: {
  v: Variable;
  value: unknown;
  onChange: (name: string, val: unknown) => void;
}) {
  const hasPct = v.description?.includes("%") ?? false;
  const val = Number(value ?? 0);
  return (
    <span className="sacp-number">
      <Slider
        min={0}
        max={v.description?.includes("温度") ? 2 : 100}
        step={inferStep(v)}
        className="sacp-number-slider"
        value={val}
        onChange={(v2) => onChange(v.name, v2)}
      />
      <InputNumber
        size="small"
        className="sacp-number-input"
        value={val}
        suffix={hasPct ? "%" : undefined}
        onChange={(v2) => v2 != null && onChange(v.name, v2)}
      />
    </span>
  );
}

function VariableControl({ v, value, onChange }: {
  v: Variable;
  value: unknown;
  onChange: (name: string, val: unknown) => void;
}) {
  switch (v.var_type) {
    case "boolean":
      return <Switch checked={!!value} onChange={(c) => onChange(v.name, c)} />;
    case "enum": {
      const options = parseEnumOptions(v.description);
      return (
        <Select
          size="small"
          style={{ width: 140 }}
          value={String(value ?? "")}
          onChange={(val) => onChange(v.name, val)}
          options={options.map((o) => ({ value: o, label: o }))}
        />
      );
    }
    case "number":
      return <NumberControl v={v} value={value} onChange={onChange} />;
    default:
      return (
        <Input
          size="small"
          style={{ maxWidth: 180 }}
          value={String(value ?? "")}
          onChange={(e) => onChange(v.name, e.target.value)}
        />
      );
  }
}

export function StockAnalysisConfigPanel(_props: Props) {
  const { message } = App.useApp();
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const [template, setTemplate] = useState<WorkflowTemplateResponse | null>(null);
  const [values, setValues] = useState<Record<string, unknown>>({});
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    let cancelled = false;
    invoke<WorkflowTemplateResponse | null>("get_workflow_template", { id: TEMPLATE_ID })
      .then(async (rsp) => {
        if (cancelled) { return; }
        if (rsp && (!rsp.variables || rsp.variables.length === 0)) {
          // Initial load: if template has no variables, init with defaults and save back
          const defaults = getDefaultVariables();
          const input: WorkflowTemplateInput = {
            name: rsp.name,
            description: rsp.description,
            icon: rsp.icon,
            tags: rsp.tags,
            trigger_config: rsp.trigger_config,
            nodes: rsp.nodes,
            edges: rsp.edges,
            input_schema: rsp.input_schema,
            output_schema: rsp.output_schema,
            variables: defaults,
            error_config: rsp.error_config,
          };
          invoke<boolean>("update_workflow_template", { id: TEMPLATE_ID, input }).catch(() => {});
          rsp.variables = defaults;
        }
        if (rsp) {
          setTemplate(rsp);
          const map: Record<string, unknown> = {};
          for (const v of rsp.variables) { map[v.name] = v.value; }
          setValues(map);
        } else {
          // Template not found (browser mode), render with defaults directly
          const defaults = getDefaultVariables();
          const map: Record<string, unknown> = {};
          for (const v of defaults) { map[v.name] = v.value; }
          setValues(map);
        }
      })
      .catch(() => {
        if (!cancelled) { message.error(t("stockAnalysis.settings.loadFailed")); }
      })
      .finally(() => {
        if (!cancelled) { setLoading(false); }
      });
    return () => {
      cancelled = true;
    };
  }, [t, message]);

  // tool → parameter groups
  const toolGroups = useMemo(() => {
    const allVars = template?.variables ?? getDefaultVariables();
    const varMap: Record<string, Variable> = {};
    for (const v of allVars) { varMap[v.name] = v; }

    const resolve = (names: string[]) => names.map((n) => varMap[n]).filter(Boolean);

    return [
      {
        tool: "compute_scoring",
        label: t("stockAnalysis.settings.group.scoring"),
        vars: resolve([
          "scoring_trend",
          "scoring_deviation",
          "scoring_macd",
          "scoring_volume",
          "scoring_rsi",
          "scoring_support",
          "scoring_boll",
        ]),
      },
      {
        tool: "compute_valuation",
        label: t("stockAnalysis.settings.group.value"),
        vars: resolve([
          "value_dcf_growth_rate",
          "value_dcf_perpetual_rate",
          "value_dcf_discount_rate",
          "value_moat_threshold",
          "value_fscore_buy",
          "value_safety_margin",
        ]),
      },
      {
        tool: "compute_moat",
        label: t("stockAnalysis.settings.group.moat") ?? "护城河",
        vars: resolve([
          "moat_roe_years_min",
          "moat_avg_gross_margin_min",
          "moat_margin_stable_std_max",
          "moat_fcf_ratio_min",
        ]),
      },
      {
        tool: "compute_portfolio_risk",
        label: t("stockAnalysis.settings.group.pos"),
        vars: resolve([
          "pos_max_single_pct",
          "pos_max_total",
          "pos_max_sector_pct",
          "pos_min_cash_pct",
          "pos_max_turnover_pct",
        ]),
      },
      {
        tool: "calcs",
        label: t("stockAnalysis.settings.group.riskModel"),
        vars: resolve([
          "var_confidence",
          "kelly_fraction",
          "kelly_min_win_rate",
          "kelly_min_odds",
          "risk_free_rate",
          "outlier_method",
          "outlier_threshold",
          "min_confidence",
          "risk_max_drawdown_limit",
          "risk_max_daily_loss_pct",
          "risk_correlation_lookback_days",
          "risk_sharpe_annualization",
          "risk_kelly_heavy_threshold",
          "risk_kelly_medium_threshold",
        ]),
      },
      {
        tool: "rules",
        label: t("stockAnalysis.settings.group.rule"),
        vars: resolve([
          "rule_rsi_overbought",
          "rule_rsi_oversold",
          "rule_bias_limit_pct",
          "rule_volume_signal_block",
          "rule_bear_low_score",
          "rule_auto_stop_loss_pct",
        ]),
      },
      {
        tool: "screener",
        label: t("stockAnalysis.settings.group.screener") ?? "选股筛选",
        vars: resolve([
          "screener_min_change_pct",
          "screener_max_change_pct",
          "screener_main_inflow_min",
          "screener_northbound_ratio_min",
          "screener_turnover_rate_min",
          "screener_rsi_oversold",
          "screener_rsi_overbought",
          "ref_semi_code",
          "ref_battery_code",
          "ref_chem_code",
          "ref_med_code",
          "ref_aero_code",
          "ref_consumer_elec_code",
          "ref_auto_code",
        ]),
      },
      {
        tool: "signals",
        label: t("stockAnalysis.settings.group.signals") ?? "信号检测",
        vars: resolve([
          "signal_ma_fast",
          "signal_ma_slow",
          "signal_breakout_volume_mult",
        ]),
      },
      {
        tool: "keylevels",
        label: t("stockAnalysis.settings.group.keylevels") ?? "关键价位",
        vars: resolve([
          "keylevel_lookback_days",
          "keylevel_touch_tolerance_pct",
          "keylevel_min_touches",
        ]),
      },
      {
        tool: "recommender",
        label: t("stockAnalysis.settings.group.recommender") ?? "推荐器",
        vars: resolve([
          "reco_trend_enabled",
          "reco_reversion_enabled",
          "reco_value_enabled",
          "reco_capital_enabled",
          "reco_watchlist_enabled",
          "reco_min_confidence",
          "decision_max_history_per_stock",
        ]),
      },
      {
        tool: "technical_indicators",
        label: t("stockAnalysis.settings.group.indicators") ?? "技术指标",
        vars: resolve([
          "macd_fast",
          "macd_slow",
          "macd_signal",
          "boll_period",
          "boll_stddev",
          "volume_lookback",
          "volume_surge_ratio",
          "volume_shrink_ratio",
        ]),
      },
      {
        tool: "recommender_strategies",
        label: t("stockAnalysis.settings.group.strategyParams") ?? "策略参数",
        vars: resolve([
          "trend_kline_limit",
          "trend_amount_ratio_min",
          "rev_rsi_short_max",
          "rev_drawdown_min_pct",
          "rev_rsi_monthly_max",
          "val_pe_short_max",
          "val_pe_mid_max",
          "val_pb_mid_max",
          "cap_inflow_short_min",
          "cap_inflow_mid_min",
          "cap_turnover_min",
          "cap_nb_ratio_min",
        ]),
      },
      {
        tool: "trading",
        label: t("stockAnalysis.settings.group.trading") ?? "交易决策",
        vars: resolve(["trading_price_deviation_limit"]),
      },
      {
        tool: "workflow_runtime",
        label: t("stockAnalysis.settings.group.workflow"),
        vars: resolve([
          "analysis_depth",
          "debate_rounds",
          "max_concurrent",
        ]),
      },
      {
        tool: "agent_executor",
        label: t("stockAnalysis.settings.group.agentRuntime"),
        vars: resolve([
          "agent_temperature",
          "agent_max_tokens",
          "agent_timeout_secs",
          "agent_retry_max",
        ]),
      },
      {
        tool: "tool_executor",
        label: t("stockAnalysis.settings.group.toolRuntime"),
        vars: resolve([
          "tool_timeout_secs",
          "tool_retry_max",
          "kline_period",
          "kline_limit",
          "news_limit",
        ]),
      },
      {
        tool: "monitor",
        label: t("stockAnalysis.settings.group.monitor"),
        vars: resolve(["monitor_poll_interval_secs", "monitor_change_pct", "monitor_turnover"]),
      },
      {
        tool: "workflow",
        label: t("stockAnalysis.settings.group.dryRun"),
        vars: resolve(["analysis_dry_run"]),
      },
    ].filter((g) => g.vars.length > 0);
  }, [template, t]);

  const handleChange = (name: string, val: unknown) => {
    setValues((prev) => ({ ...prev, [name]: val }));
  };

  const handleSave = async () => {
    if (!template) { return; }
    setSaving(true);
    const updatedVars = template.variables.map((v) => ({ ...v, value: values[v.name] ?? v.value }));
    const input: WorkflowTemplateInput = {
      name: template.name,
      description: template.description,
      icon: template.icon,
      tags: template.tags,
      trigger_config: template.trigger_config,
      nodes: template.nodes,
      edges: template.edges,
      input_schema: template.input_schema,
      output_schema: template.output_schema,
      variables: updatedVars,
      error_config: template.error_config,
    };
    try {
      await invoke<boolean>("update_workflow_template", { id: TEMPLATE_ID, input });
      message.success(t("stockAnalysis.settings.saveSuccess"));
    } catch {
      message.error(t("stockAnalysis.settings.saveFailed"));
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return (
      <div style={{ textAlign: "center", padding: 24, color: token.colorTextQuaternary }}>{t("common.loading")}</div>
    );
  }

  const rowStyle = { padding: "4px 0" };

  const handleOptimize = async () => {
    setSaving(true);
    try {
      const weights = await invoke<Record<string, unknown>>("optimize_scoring_weights");
      if (weights) {
        const map: Record<string, number> = {
          scoring_trend: weights.trendWeight as number,
          scoring_deviation: weights.deviationWeight as number,
          scoring_macd: weights.macdWeight as number,
          scoring_volume: weights.volumeWeight as number,
          scoring_rsi: weights.rsiWeight as number,
          scoring_support: weights.supportWeight as number,
        };
        setValues((prev) => ({ ...prev, ...map }));
        message.success(t("stockAnalysis.settings.optimize.success"));
      }
    } catch {
      message.error(t("stockAnalysis.settings.optimize.failed"));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="flex flex-col gap-3">
      <div className="flex justify-end">
        <Button size="small" loading={saving} onClick={handleOptimize}>
          {t("stockAnalysis.settings.optimize.btn")}
        </Button>
      </div>
      {toolGroups.map((g) => (
        <SettingsGroup
          key={g.tool}
          title={
            <Space size={4}>
              <span>{g.label}</span>
              <Tag className="text-xs m-0" color="default">⚙️ {g.tool}</Tag>
            </Space>
          }
        >
          <div className="sacp-vars">
            {g.vars.map((v) => (
              <div key={v.name} style={rowStyle} className="flex items-center justify-between sacp-row">
                <span className="sacp-var-label" style={{ fontSize: 13, color: token.colorText }}>
                  {v.description ?? v.name}
                </span>
                <span style={{ display: "inline-flex", alignItems: "center", gap: 8, flexShrink: 0, marginLeft: 16 }}>
                  <VariableControl v={v} value={values[v.name]} onChange={handleChange} />
                </span>
              </div>
            ))}
          </div>
        </SettingsGroup>
      ))}
      <div style={{ display: "flex", justifyContent: "flex-end", paddingTop: 8 }}>
        <Button type="primary" loading={saving} onClick={handleSave}>
          {t("stockAnalysis.settings.saveConfig")}
        </Button>
      </div>
    </div>
  );
}
