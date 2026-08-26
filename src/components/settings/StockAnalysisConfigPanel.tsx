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
    vars.push({ name, varType: type, value: val, description: desc, isSecret: false });
  // 分析流程
  b("analysis_depth", "standard", "stockAnalysis.configDescriptions.analysisDepth", "enum");
  b("debate_rounds", 3, "stockAnalysis.configDescriptions.debateRounds", "number");
  b("max_concurrent", 12, "stockAnalysis.configDescriptions.maxConcurrent", "number");
  // 数据源参数
  b("kline_period", "daily", "stockAnalysis.configDescriptions.klinePeriod", "enum");
  b("kline_limit", 120, "stockAnalysis.configDescriptions.klineLimit", "number");
  b("news_limit", 30, "stockAnalysis.configDescriptions.newsLimit", "number");
  // Agent 节点 LLM 参数
  b("agent_temperature", 0.3, "stockAnalysis.configDescriptions.agentTemperature", "number");
  b("agent_max_tokens", 32768, "stockAnalysis.configDescriptions.agentMaxTokens", "number");
  b("agent_timeout_secs", 600, "stockAnalysis.configDescriptions.agentTimeoutSecs", "number");
  b("agent_retry_max", 2, "stockAnalysis.configDescriptions.agentRetryMax", "number");
  // Tool 节点参数
  b("tool_timeout_secs", 30, "stockAnalysis.configDescriptions.toolTimeoutSecs", "number");
  b("tool_retry_max", 2, "stockAnalysis.configDescriptions.toolRetryMax", "number");
  // 评分权重
  b("scoring_trend", 30, "stockAnalysis.configDescriptions.scoringTrend", "number");
  b("scoring_deviation", 20, "stockAnalysis.configDescriptions.scoringDeviation", "number");
  b("scoring_macd", 15, "stockAnalysis.configDescriptions.scoringMacd", "number");
  b("scoring_volume", 15, "stockAnalysis.configDescriptions.scoringVolume", "number");
  b("scoring_rsi", 10, "stockAnalysis.configDescriptions.scoringRsi", "number");
  b("scoring_support", 10, "stockAnalysis.configDescriptions.scoringSupport", "number");
  b("scoring_boll", 5, "stockAnalysis.configDescriptions.scoringBoll", "number");
  // 规则阈值
  b("rule_rsi_overbought", 80, "stockAnalysis.configDescriptions.ruleRsiOverbought", "number");
  b("rule_rsi_oversold", 20, "stockAnalysis.configDescriptions.ruleRsiOversold", "number");
  b("rule_bias_limit_pct", 5, "stockAnalysis.configDescriptions.ruleBiasLimitPct", "number");
  b("rule_volume_signal_block", true, "stockAnalysis.configDescriptions.ruleVolumeSignalBlock", "boolean");
  b("rule_bear_low_score", 30, "stockAnalysis.configDescriptions.ruleBearLowScore", "number");
  b("rule_auto_stop_loss_pct", 5, "stockAnalysis.configDescriptions.ruleAutoStopLossPct", "number");
  // 仓位限制
  b("pos_max_single_pct", 20, "stockAnalysis.configDescriptions.posMaxSinglePct", "number");
  b("pos_max_total", 10, "stockAnalysis.configDescriptions.posMaxTotal", "number");
  b("pos_max_sector_pct", 40, "stockAnalysis.configDescriptions.posMaxSectorPct", "number");
  b("pos_min_cash_pct", 10, "stockAnalysis.configDescriptions.posMinCashPct", "number");
  b("pos_max_turnover_pct", 30, "stockAnalysis.configDescriptions.posMaxTurnoverPct", "number");
  // 估值参数（A股校准：growth=12 / perpetual=4 / discount=8.5）
  b("value_dcf_growth_rate", 12, "stockAnalysis.configDescriptions.valueDcfGrowthRate", "number");
  b("value_dcf_perpetual_rate", 4, "stockAnalysis.configDescriptions.valueDcfPerpetualRate", "number");
  b("value_dcf_discount_rate", 8.5, "stockAnalysis.configDescriptions.valueDcfDiscountRate", "number");
  b("value_moat_threshold", 60, "stockAnalysis.configDescriptions.valueMoatThreshold", "number");
  b("value_fscore_buy", 7, "stockAnalysis.configDescriptions.valueFscoreBuy", "number");
  b("value_safety_margin", 20, "stockAnalysis.configDescriptions.valueSafetyMargin", "number");
  // 护城河（compute_moat）
  b("moat_roe_years_min", 5, "stockAnalysis.configDescriptions.moatRoeYearsMin", "number");
  b("moat_avg_gross_margin_min", 30, "stockAnalysis.configDescriptions.moatAvgGrossMarginMin", "number");
  b("moat_margin_stable_std_max", 5, "stockAnalysis.configDescriptions.moatMarginStableStdMax", "number");
  b("moat_fcf_ratio_min", 5, "stockAnalysis.configDescriptions.moatFcfRatioMin", "number");
  // 选股筛选（screener）
  b("screener_min_change_pct", -3, "stockAnalysis.configDescriptions.screenerMinChangePct", "number");
  b("screener_max_change_pct", 7, "stockAnalysis.configDescriptions.screenerMaxChangePct", "number");
  b("screener_main_inflow_min", 500, "stockAnalysis.configDescriptions.screenerMainInflowMin", "number");
  b("screener_northbound_ratio_min", 0.5, "stockAnalysis.configDescriptions.screenerNorthboundRatioMin", "number");
  b("screener_turnover_rate_min", 1, "stockAnalysis.configDescriptions.screenerTurnoverRateMin", "number");
  b("screener_rsi_oversold", 30, "stockAnalysis.configDescriptions.screenerRsiOversold", "number");
  b("screener_rsi_overbought", 70, "stockAnalysis.configDescriptions.screenerRsiOverbought", "number");
  // 监控
  b("monitor_poll_interval_secs", 30, "stockAnalysis.configDescriptions.monitorPollIntervalSecs", "number");
  b("monitor_change_pct", 5, "stockAnalysis.configDescriptions.monitorChangePct", "number");
  b("monitor_turnover", 10, "stockAnalysis.configDescriptions.monitorTurnover", "number");
  b("monitor_alert_cooldown_secs", 300, "stockAnalysis.configDescriptions.monitorAlertCooldownSecs", "number");
  b("monitor_min_severity", "info", "stockAnalysis.configDescriptions.monitorMinSeverity", "enum");
  b("monitor_channels", "in_app", "stockAnalysis.configDescriptions.monitorChannels", "string");
  // 风险/置信度
  b("min_confidence", 60, "stockAnalysis.configDescriptions.minConfidence", "number");
  b("var_confidence", 0.95, "stockAnalysis.configDescriptions.varConfidence", "number");
  b("kelly_fraction", 0.5, "stockAnalysis.configDescriptions.kellyFraction", "number");
  b("kelly_min_win_rate", 0.4, "stockAnalysis.configDescriptions.kellyMinWinRate", "number");
  b("kelly_min_odds", 1.0, "stockAnalysis.configDescriptions.kellyMinOdds", "number");
  b("risk_free_rate", 0.03, "stockAnalysis.configDescriptions.riskFreeRate", "number");
  // portfolio-mgr 决策阈值（修复 D7/D8: 与 portfolio-mgr.rhai 顶部可配置参数一一对应）
  b("action_buy_threshold", 0.63, "stockAnalysis.configDescriptions.actionBuyThreshold", "number");
  b("action_increase_threshold", 0.53, "stockAnalysis.configDescriptions.actionIncreaseThreshold", "number");
  b("action_hold_threshold", 0.48, "stockAnalysis.configDescriptions.actionHoldThreshold", "number");
  b("action_watch_threshold", 0.38, "stockAnalysis.configDescriptions.actionWatchThreshold", "number");
  b("action_reduce_threshold", 0.30, "stockAnalysis.configDescriptions.actionReduceThreshold", "number");
  b("pos_buy_min", 15.0, "stockAnalysis.configDescriptions.posBuyMin", "number");
  b("pos_increase_min", 10.0, "stockAnalysis.configDescriptions.posIncreaseMin", "number");
  b("pos_cap_extreme", 10.0, "stockAnalysis.configDescriptions.posCapExtreme", "number");
  b("pos_cap_high", 35.0, "stockAnalysis.configDescriptions.posCapHigh", "number");
  b("pos_cap_mid", 50.0, "stockAnalysis.configDescriptions.posCapMid", "number");
  b("risk_debt_extreme", 85.0, "stockAnalysis.configDescriptions.riskDebtExtreme", "number");
  b("risk_vol_extreme", 60.0, "stockAnalysis.configDescriptions.riskVolExtreme", "number");
  b("risk_sharpe_extreme", -1.5, "stockAnalysis.configDescriptions.riskSharpeExtreme", "number");
  b("risk_vol_high", 40.0, "stockAnalysis.configDescriptions.riskVolHigh", "number");
  b("risk_dd_high", 45.0, "stockAnalysis.configDescriptions.riskDdHigh", "number");
  b("risk_roe_high", 5.0, "stockAnalysis.configDescriptions.riskRoeHigh", "number");
  b("risk_debt_high", 65.0, "stockAnalysis.configDescriptions.riskDebtHigh", "number");
  b("risk_vol_low", 25.0, "stockAnalysis.configDescriptions.riskVolLow", "number");
  b("risk_sharpe_low", 0.5, "stockAnalysis.configDescriptions.riskSharpeLow", "number");
  b("risk_dd_low", 30.0, "stockAnalysis.configDescriptions.riskDdLow", "number");
  b("risk_roe_low", 8.0, "stockAnalysis.configDescriptions.riskRoeLow", "number");
  b("risk_debt_low", 55.0, "stockAnalysis.configDescriptions.riskDebtLow", "number");
  b("risk_growth_low", 3.0, "stockAnalysis.configDescriptions.riskGrowthLow", "number");
  b("outlier_method", "zscore", "stockAnalysis.configDescriptions.outlierMethod", "enum");
  b("outlier_threshold", 2.0, "stockAnalysis.configDescriptions.outlierThreshold", "number");
  b("risk_max_drawdown_limit", 15, "stockAnalysis.configDescriptions.riskMaxDrawdownLimit", "number");
  b("risk_max_daily_loss_pct", 3, "stockAnalysis.configDescriptions.riskMaxDailyLossPct", "number");
  b("risk_correlation_lookback_days", 60, "stockAnalysis.configDescriptions.riskCorrelationLookbackDays", "number");
  // 行业财务基线参考股票代码（stock_analysis_setup 中 t-baseline-* 节点使用）
  b("ref_semi_code", "002371", "stockAnalysis.configDescriptions.refSemiCode", "string");
  b("ref_battery_code", "300750", "stockAnalysis.configDescriptions.refBatteryCode", "string");
  b("ref_chem_code", "600309", "stockAnalysis.configDescriptions.refChemCode", "string");
  b("ref_med_code", "688981", "stockAnalysis.configDescriptions.refMedCode", "string");
  b("ref_aero_code", "600760", "stockAnalysis.configDescriptions.refAeroCode", "string");
  b("ref_consumer_elec_code", "002475", "stockAnalysis.configDescriptions.refConsumerElecCode", "string");
  b("ref_auto_code", "600104", "stockAnalysis.configDescriptions.refAutoCode", "string");
  // 信号检测（signals.rs detect_ma_cross / detect_breakout）
  b("signal_ma_fast", 5, "stockAnalysis.configDescriptions.signalMaFast", "number");
  b("signal_ma_slow", 20, "stockAnalysis.configDescriptions.signalMaSlow", "number");
  b("signal_breakout_volume_mult", 1.5, "stockAnalysis.configDescriptions.signalBreakoutVolumeMult", "number");
  // 关键价位（key_levels.rs KeyLevelTracker）
  b("keylevel_lookback_days", 60, "stockAnalysis.configDescriptions.keylevelLookbackDays", "number");
  b("keylevel_touch_tolerance_pct", 1.0, "stockAnalysis.configDescriptions.keylevelTouchTolerancePct", "number");
  b("keylevel_min_touches", 2, "stockAnalysis.configDescriptions.keylevelMinTouches", "number");
  // 推荐器策略开关（recommender/strategies）
  b("reco_trend_enabled", true, "stockAnalysis.configDescriptions.recoTrendEnabled", "boolean");
  b("reco_reversion_enabled", true, "stockAnalysis.configDescriptions.recoReversionEnabled", "boolean");
  b("reco_value_enabled", true, "stockAnalysis.configDescriptions.recoValueEnabled", "boolean");
  b("reco_capital_enabled", true, "stockAnalysis.configDescriptions.recoCapitalEnabled", "boolean");
  b("reco_watchlist_enabled", true, "stockAnalysis.configDescriptions.recoWatchlistEnabled", "boolean");
  b("reco_min_confidence", 60, "stockAnalysis.configDescriptions.recoMinConfidence", "number");
  // 决策回溯（decision_tracker.rs）
  b("decision_max_history_per_stock", 50, "stockAnalysis.configDescriptions.decisionMaxHistoryPerStock", "number");
  // 技术指标周期（indicators.rs IndicatorConfig）
  b("macd_fast", 12, "stockAnalysis.configDescriptions.macdFast", "number");
  b("macd_slow", 26, "stockAnalysis.configDescriptions.macdSlow", "number");
  b("macd_signal", 9, "stockAnalysis.configDescriptions.macdSignal", "number");
  b("boll_period", 20, "stockAnalysis.configDescriptions.bollPeriod", "number");
  b("boll_stddev", 2.0, "stockAnalysis.configDescriptions.bollStddev", "number");
  b("volume_lookback", 5, "stockAnalysis.configDescriptions.volumeLookback", "number");
  b("volume_surge_ratio", 1.5, "stockAnalysis.configDescriptions.volumeSurgeRatio", "number");
  b("volume_shrink_ratio", 0.7, "stockAnalysis.configDescriptions.volumeShrinkRatio", "number");
  // 推荐器参数（recommender/strategies）
  b("trend_kline_limit", 250, "stockAnalysis.configDescriptions.trendKlineLimit", "number");
  b("trend_amount_ratio_min", 0.8, "stockAnalysis.configDescriptions.trendAmountRatioMin", "number");
  b("rev_rsi_short_max", 35, "stockAnalysis.configDescriptions.revRsiShortMax", "number");
  b("rev_drawdown_min_pct", 20, "stockAnalysis.configDescriptions.revDrawdownMinPct", "number");
  b("rev_rsi_monthly_max", 50, "stockAnalysis.configDescriptions.revRsiMonthlyMax", "number");
  b("val_pe_short_max", 50, "stockAnalysis.configDescriptions.valPeShortMax", "number");
  b("val_pe_mid_max", 40, "stockAnalysis.configDescriptions.valPeMidMax", "number");
  b("val_pb_mid_max", 8, "stockAnalysis.configDescriptions.valPbMidMax", "number");
  b("cap_inflow_short_min", 200, "stockAnalysis.configDescriptions.capInflowShortMin", "number");
  b("cap_inflow_mid_min", 500, "stockAnalysis.configDescriptions.capInflowMidMin", "number");
  b("cap_turnover_min", 2, "stockAnalysis.configDescriptions.capTurnoverMin", "number");
  b("cap_nb_ratio_min", 0.3, "stockAnalysis.configDescriptions.capNbRatioMin", "number");
  // 交易决策（trading.rs）
  b("trading_price_deviation_limit", 1.0, "stockAnalysis.configDescriptions.tradingPriceDeviationLimit", "number");
  // 风险模型扩展（risk.rs）
  b("risk_sharpe_annualization", 252, "stockAnalysis.configDescriptions.riskSharpeAnnualization", "number");
  b("risk_kelly_heavy_threshold", 0.25, "stockAnalysis.configDescriptions.riskKellyHeavyThreshold", "number");
  b("risk_kelly_medium_threshold", 0.1, "stockAnalysis.configDescriptions.riskKellyMediumThreshold", "number");
  // 风险组合（compute_portfolio_risk / compute_scoring / compute_valuation）
  b("val_pe_low", 15, "stockAnalysis.configDescriptions.valPeLow", "number");
  b("val_pe_high", 50, "stockAnalysis.configDescriptions.valPeHigh", "number");
  b("val_pb_low", 1.0, "stockAnalysis.configDescriptions.valPbLow", "number");
  b("val_pb_high", 6.0, "stockAnalysis.configDescriptions.valPbHigh", "number");
  b("risk_hhi_concentrated", 0.25, "stockAnalysis.configDescriptions.riskHhiConcentrated", "number");
  b("risk_hhi_medium", 0.15, "stockAnalysis.configDescriptions.riskHhiMedium", "number");
  b("risk_divers_high", 8, "stockAnalysis.configDescriptions.riskDiversHigh", "number");
  b("risk_divers_medium", 4, "stockAnalysis.configDescriptions.riskDiversMedium", "number");
  // 凯利公式默认值
  b("kelly_default_win_rate", 0.5, "stockAnalysis.configDescriptions.kellyDefaultWinRate", "number");
  b("kelly_default_avg_win", 0.05, "stockAnalysis.configDescriptions.kellyDefaultAvgWin", "number");
  b("kelly_default_avg_loss", 0.05, "stockAnalysis.configDescriptions.kellyDefaultAvgLoss", "number");
  // 技术指标周期
  b("atr_period", 14, "stockAnalysis.configDescriptions.atrPeriod", "number");
  b("kdj_n", 9, "stockAnalysis.configDescriptions.kdjN", "number");
  // 数据清洗
  b("fill_missing_method", "forward", "stockAnalysis.configDescriptions.fillMissingMethod", "enum");
  // 突破检测
  b("breakout_volume_threshold", 1.5, "stockAnalysis.configDescriptions.breakoutVolumeThreshold", "number");
  // 业绩超预期分级阈值
  b("earnings_th_huge_pos", 50, "stockAnalysis.configDescriptions.earningsThHugePos", "number");
  b("earnings_th_strong_pos", 20, "stockAnalysis.configDescriptions.earningsThStrongPos", "number");
  b("earnings_th_mild_pos", 5, "stockAnalysis.configDescriptions.earningsThMildPos", "number");
  b("earnings_th_mild_neg", -5, "stockAnalysis.configDescriptions.earningsThMildNeg", "number");
  b("earnings_th_strong_neg", -20, "stockAnalysis.configDescriptions.earningsThStrongNeg", "number");
  b("earnings_th_huge_neg", -50, "stockAnalysis.configDescriptions.earningsThHugeNeg", "number");
  // 质押风险分级阈值
  b("pledge_warning_line", 50, "stockAnalysis.configDescriptions.pledgeWarningLine", "number");
  b("pledge_liquidation_line", 70, "stockAnalysis.configDescriptions.pledgeLiquidationLine", "number");
  b("pledge_medium_line", 30, "stockAnalysis.configDescriptions.pledgeMediumLine", "number");
  b("pledge_low_line", 10, "stockAnalysis.configDescriptions.pledgeLowLine", "number");
  // 蒙特卡洛模拟默认参数
  b("mc_default_price", 10, "stockAnalysis.configDescriptions.mcDefaultPrice", "number");
  b("mc_default_return", 0.08, "stockAnalysis.configDescriptions.mcDefaultReturn", "number");
  b("mc_default_volatility", 0.3, "stockAnalysis.configDescriptions.mcDefaultVolatility", "number");
  b("mc_default_days", 30, "stockAnalysis.configDescriptions.mcDefaultDays", "number");
  b("mc_default_simulations", 1000, "stockAnalysis.configDescriptions.mcDefaultSimulations", "number");
  // 行业内估值/增长对比阈值
  b("industry_pe_cheap", 1.0, "stockAnalysis.configDescriptions.industryPeCheap", "number");
  b("industry_pe_expensive", 1.5, "stockAnalysis.configDescriptions.industryPeExpensive", "number");
  b("industry_growth_high", 1.2, "stockAnalysis.configDescriptions.industryGrowthHigh", "number");
  // 涨停潜力评分
  b("limit_pct_main", 10, "stockAnalysis.configDescriptions.limitPctMain", "number");
  b("limit_pct_star", 20, "stockAnalysis.configDescriptions.limitPctStar", "number");
  b("limit_pct_bj", 30, "stockAnalysis.configDescriptions.limitPctBj", "number");
  b("limit_up_w_trend", 40, "stockAnalysis.configDescriptions.limitUpWTrend", "number");
  b("limit_up_w_volume", 20, "stockAnalysis.configDescriptions.limitUpWVolume", "number");
  b("limit_up_w_hits", 15, "stockAnalysis.configDescriptions.limitUpWHits", "number");
  b("limit_up_th_high", 60, "stockAnalysis.configDescriptions.limitUpThHigh", "number");
  b("limit_up_th_med", 30, "stockAnalysis.configDescriptions.limitUpThMed", "number");
  b("limit_up_th_low", 10, "stockAnalysis.configDescriptions.limitUpThLow", "number");
  // 注意：vendor_* 9 个开关 + iwencai_key 不在这里暴露，
  // 由「数据源」tab（DataVendorsTab）全权管理，避免两边同时写造成竞态。
  // 全局开关
  b("analysis_dry_run", false, "stockAnalysis.configDescriptions.analysisDryRun", "boolean");
  return vars;
}

function parseEnumOptions(desc?: string): string[] {
  if (!desc) { return []; }
  const match = desc.match(/: (.+)/);
  if (match) { return match[1].split(/\s*\/\s*/).map((s) => s.trim()); }
  return [];
}

function inferStep(v: Variable): number {
  // 温度类参数（如 agent_temperature）需要 0.1 步进，用变量名判断更可靠
  if (v.name === "agent_temperature") { return 0.1; }
  return 1;
}

// eslint-disable-next-line @typescript-eslint/no-empty-object-type
interface Props {}

interface ValuationParamsConfig {
  perpetualGrowth: number;
  discountRate: number;
  defaultGrowth: number;
  minGrowth: number;
  maxGrowth: number;
  forecastYears: number;
  bondYield: number;
}

const DEFAULT_VALUATION_PARAMS: ValuationParamsConfig = {
  perpetualGrowth: 0.03,
  discountRate: 0.10,
  defaultGrowth: 0.08,
  minGrowth: 0.02,
  maxGrowth: 0.30,
  forecastYears: 5,
  bondYield: 4.4,
};

/** number control — vertical on narrow screen, horizontal on wide */
function NumberControl({ v, value, onChange }: {
  v: Variable;
  value: unknown;
  onChange: (name: string, val: unknown) => void;
}) {
  const { t } = useTranslation();
  const desc = t(v.description ?? "");
  const hasPct = desc.includes("%") ?? false;
  const val = Number(value ?? 0);
  return (
    <span className="sacp-number">
      <Slider
        min={0}
        max={v.name === "agent_temperature" ? 2 : 100}
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
  const { t } = useTranslation();
  const desc = t(v.description ?? "");
  switch (v.varType) {
    case "boolean":
      return <Switch checked={!!value} onChange={(c) => onChange(v.name, c)} />;
    case "enum": {
      const options = parseEnumOptions(desc);
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
  const [valuationParams, setValuationParams] = useState<ValuationParamsConfig>(DEFAULT_VALUATION_PARAMS);
  const [valuationDirty, setValuationDirty] = useState(false);

  useEffect(() => {
    let cancelled = false;
    // 并行加载 workflow 模板和估值参数
    Promise.all([
      invoke<WorkflowTemplateResponse | null>("get_workflow_template", { id: TEMPLATE_ID }),
      invoke<ValuationParamsConfig>("get_valuation_params").catch(() => DEFAULT_VALUATION_PARAMS),
    ])
      .then(async ([rsp, valuation]) => {
        if (cancelled) { return; }
        // 加载估值参数
        if (valuation) {
          setValuationParams(valuation);
        }
        if (rsp && (!rsp.variables || rsp.variables.length === 0)) {
          // Initial load: if template has no variables, init with defaults and save back
          const defaults = getDefaultVariables();
          const input: WorkflowTemplateInput = {
            name: rsp.name,
            description: rsp.description,
            icon: rsp.icon,
            tags: rsp.tags,
            triggerConfig: rsp.triggerConfig,
            nodes: rsp.nodes,
            edges: rsp.edges,
            inputSchema: rsp.inputSchema,
            outputSchema: rsp.outputSchema,
            variables: defaults,
            errorConfig: rsp.errorConfig,
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
        label: t("stockAnalysis.settings.group.moat"),
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
        tool: "portfolio_mgr_action",
        label: t("stockAnalysis.settings.group.portfolioMgrAction"),
        vars: resolve([
          "action_buy_threshold",
          "action_increase_threshold",
          "action_hold_threshold",
          "action_watch_threshold",
          "action_reduce_threshold",
          "pos_buy_min",
          "pos_increase_min",
          "pos_cap_extreme",
          "pos_cap_high",
          "pos_cap_mid",
        ]),
      },
      {
        tool: "portfolio_mgr_risk",
        label: t("stockAnalysis.settings.group.portfolioMgrRisk"),
        vars: resolve([
          "risk_debt_extreme",
          "risk_vol_extreme",
          "risk_sharpe_extreme",
          "risk_vol_high",
          "risk_dd_high",
          "risk_roe_high",
          "risk_debt_high",
          "risk_vol_low",
          "risk_sharpe_low",
          "risk_dd_low",
          "risk_roe_low",
          "risk_debt_low",
          "risk_growth_low",
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
        tool: "refCodes",
        label: t("stockAnalysis.settings.group.refCodes"),
        vars: resolve([
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
        label: t("stockAnalysis.settings.group.signals"),
        vars: resolve([
          "signal_ma_fast",
          "signal_ma_slow",
          "signal_breakout_volume_mult",
        ]),
      },
      {
        tool: "keylevels",
        label: t("stockAnalysis.settings.group.keylevels"),
        vars: resolve([
          "keylevel_lookback_days",
          "keylevel_touch_tolerance_pct",
          "keylevel_min_touches",
        ]),
      },
      {
        tool: "recommender",
        label: t("stockAnalysis.settings.group.recommender"),
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
        label: t("stockAnalysis.settings.group.indicators"),
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
        label: t("stockAnalysis.settings.group.strategyParams"),
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
        label: t("stockAnalysis.settings.group.trading"),
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
      {
        tool: "simulation",
        label: t("stockAnalysis.settings.group.simulation"),
        vars: resolve([
          "mc_default_price",
          "mc_default_return",
          "mc_default_volatility",
          "mc_default_days",
          "mc_default_simulations",
        ]),
      },
    ].filter((g) => g.vars.length > 0);
  }, [template, t]);

  const handleChange = (name: string, val: unknown) => {
    setValues((prev) => ({ ...prev, [name]: val }));
  };

  const handleValuationChange = (key: keyof ValuationParamsConfig, value: number) => {
    setValuationParams((prev) => ({ ...prev, [key]: value }));
    setValuationDirty(true);
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
      triggerConfig: template.triggerConfig,
      nodes: template.nodes,
      edges: template.edges,
      inputSchema: template.inputSchema,
      outputSchema: template.outputSchema,
      variables: updatedVars,
      errorConfig: template.errorConfig,
      toolDefs: template.toolDefs,
    };
    try {
      // 并行保存 workflow 模板和估值参数
      const results = await Promise.allSettled([
        invoke<boolean>("update_workflow_template", { id: TEMPLATE_ID, input }),
        valuationDirty
          ? invoke<boolean>("save_valuation_params", { params: valuationParams })
          : Promise.resolve(true),
      ]);
      const [templateResult, valuationResult] = results;
      if (templateResult.status === "rejected") {
        throw templateResult.reason;
      }
      if (valuationResult.status === "rejected") {
        console.warn("[StockAnalysisConfigPanel] valuation params save failed:", valuationResult.reason);
      } else {
        setValuationDirty(false);
      }
      message.success(t("stockAnalysis.settings.saveSuccess"));
    } catch (e) {
      console.error("[StockAnalysisConfigPanel] save failed:", e, { input });
      message.error(t("stockAnalysis.settings.saveFailed", { error: String(e) }));
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
      <div className="flex justify-end gap-2">
        {valuationDirty && <Tag color="orange">{t("stockAnalysis.settings.valuation.unsavedChanges")}</Tag>}
        <Button size="small" loading={saving} onClick={handleOptimize}>
          {t("stockAnalysis.settings.optimize.btn")}
        </Button>
      </div>

      {/* 估值模型参数配置 */}
      <SettingsGroup
        title={
          <Space size={4}>
            <span>{t("stockAnalysis.settings.group.valuationModel")}</span>
            <Tag className="text-xs m-0" color="blue">📈 compute_valuation</Tag>
          </Space>
        }
      >
        <div className="sacp-vars">
          {([
            ["perpetualGrowth", t("stockAnalysis.settings.valuation.perpetualGrowth"), "0.03 (3%)", 0, 0.20, 0.01],
            ["discountRate", t("stockAnalysis.settings.valuation.discountRate"), "0.10 (10%)", 0.01, 0.30, 0.01],
            ["defaultGrowth", t("stockAnalysis.settings.valuation.defaultGrowth"), "0.08 (8%)", 0.01, 0.50, 0.01],
            ["minGrowth", t("stockAnalysis.settings.valuation.minGrowth"), "0.02 (2%)", 0, 0.20, 0.01],
            ["maxGrowth", t("stockAnalysis.settings.valuation.maxGrowth"), "0.30 (30%)", 0.05, 1.00, 0.01],
            ["forecastYears", t("stockAnalysis.settings.valuation.forecastYears"), "5 年", 1, 15, 1],
            ["bondYield", t("stockAnalysis.settings.valuation.bondYield"), "4.4", 1.0, 10.0, 0.1],
          ] as const).map(([key, label, hint, min, max, step]) => (
            <div key={key} style={rowStyle} className="flex items-center justify-between sacp-row">
              <span style={{ fontSize: 13, color: token.colorText }}>
                {label}
                <span style={{ color: token.colorTextTertiary, marginLeft: 8, fontSize: 12 }}>({hint})</span>
              </span>
              <span style={{ display: "inline-flex", alignItems: "center", gap: 8, flexShrink: 0, marginLeft: 16 }}>
                <Slider
                  min={min}
                  max={max}
                  step={step}
                  style={{ width: 120 }}
                  value={valuationParams[key]}
                  onChange={(v) =>
                    handleValuationChange(key, v as number)}
                />
                <InputNumber
                  size="small"
                  style={{ width: 80 }}
                  min={min}
                  max={max}
                  step={step}
                  value={valuationParams[key]}
                  onChange={(v) =>
                    v != null && handleValuationChange(key, v)}
                />
              </span>
            </div>
          ))}
        </div>
      </SettingsGroup>

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
                  {v.description ? t(v.description) : v.name}
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
