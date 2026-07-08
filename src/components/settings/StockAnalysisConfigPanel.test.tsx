import { describe, expect, it } from "vitest";

/**
 * 这些测试锁定 StockAnalysisConfigPanel 默认变量列表必须与
 * `src-tauri/src/commands/stock_analysis_setup.rs` 中 stock-analysis 模板 v19
 * 的 snake_case key 一一对应。如果后端模板升级了变量名，这里也要同步更新。
 *
 * 测试不直接 import 组件里的函数（避免引入 antd 等重依赖），而是通过
 * 静态扫描源文件 + 字符串断言来验证同步关系。
 */
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const PANEL_PATH = resolve(__dirname, "./StockAnalysisConfigPanel.tsx");
const RUST_SETUP_PATH = resolve(
  __dirname,
  "../../../src-tauri/src/commands/stock_analysis_setup.rs",
);

function readPanelSource(): string {
  return readFileSync(PANEL_PATH, "utf8");
}

function readRustSource(): string {
  return readFileSync(RUST_SETUP_PATH, "utf8");
}

/** 从 `b("name", ...)` 调用中抽取出所有的变量名 */
function extractVarNamesFromPanel(): string[] {
  const src = readPanelSource();
  const re = /b\(\s*"([a-z_][a-z0-9_]*)"/g;
  const names = new Set<string>();
  let m: RegExpExecArray | null;
  while ((m = re.exec(src)) !== null) { names.add(m[1]); }
  return [...names];
}

/** 从 rust 文件的 `name: "...".into(),` 中抽取出种子化变量名 */
function extractVarNamesFromRust(): string[] {
  const src = readRustSource();
  // 排除后端不种子化的占位（如 old_variables 序列化后再 read 时的 key）
  // 仅匹配 v19 段：`name: "...".into(),` 且 var_type 是 number/string/boolean/enum
  const re = /name:\s*"([a-z_][a-z0-9_]*)"\.into\(\),\s*\n\s*var_type:\s*"(number|string|boolean|enum)"/g;
  const names = new Set<string>();
  let m: RegExpExecArray | null;
  while ((m = re.exec(src)) !== null) { names.add(m[1]); }
  return [...names];
}

describe("StockAnalysisConfigPanel 默认变量与后端模板 v19 同步", () => {
  it("UI defaults 全部使用 snake_case（无 camelCase / 混合格式）", () => {
    const src = readPanelSource();
    // 旧的 camelCase key 不应再出现
    const deprecated = [
      "analysis_maxDebateRounds",
      "analysis_maxConcurrent",
      "analysis_klinePeriod",
      "analysis_klineLimit",
      "analysis_newsLimit",
      "analysis_temperature",
      "analysis_maxTokens",
      "analysis_timeoutSecs",
      "rule_rsiOverbought",
      "rule_rsiOversold",
      "rule_biasLimit",
      "rule_volumeSignalBlock",
      "rule_bearLowScore",
      "rule_autoStopLossPct",
      "pos_maxSingleStockPct",
      "pos_maxTotalPositions",
      "pos_maxSectorExposurePct",
      "value_dcfGrowthRate",
      "value_dcfPerpetualRate",
      "value_dcfDiscountRate",
      "value_moatThreshold",
      "value_fScoreBuyThreshold",
      "value_safetyMarginMin",
      "monitor_pollIntervalSecs",
      "monitor_changePctThreshold",
      "monitor_turnoverThreshold",
      "tool_timeoutSecs",
      "tool_retryMax",
    ];
    for (const k of deprecated) {
      expect(src, `面板中不应再出现旧 key "${k}"`).not.toContain(`"${k}"`);
    }
  });

  it("UI 暴露了 agent_/tool_ 运行时关键参数", () => {
    const names = extractVarNamesFromPanel();
    for (
      const v of [
        "agent_temperature",
        "agent_max_tokens",
        "agent_timeout_secs",
        "agent_retry_max",
        "tool_timeout_secs",
        "tool_retry_max",
        "max_concurrent",
        "debate_rounds",
        "analysis_depth",
      ]
    ) {
      expect(names, `缺少运行时参数 ${v}`).toContain(v);
    }
  });

  it("UI 暴露了 A 类补全：scoring_boll / 护城河 / 选股 / 信号 / 关键价位 / 推荐器", () => {
    const names = extractVarNamesFromPanel();
    const must = [
      "scoring_boll",
      // 护城河
      "moat_roe_years_min",
      "moat_avg_gross_margin_min",
      "moat_margin_stable_std_max",
      "moat_fcf_ratio_min",
      // 选股
      "screener_min_change_pct",
      "screener_max_change_pct",
      "screener_main_inflow_min",
      "screener_northbound_ratio_min",
      "screener_turnover_rate_min",
      "screener_rsi_oversold",
      "screener_rsi_overbought",
      // 信号
      "signal_ma_fast",
      "signal_ma_slow",
      "signal_breakout_volume_mult",
      // 关键价位
      "keylevel_lookback_days",
      "keylevel_touch_tolerance_pct",
      "keylevel_min_touches",
      // 推荐器
      "reco_trend_enabled",
      "reco_reversion_enabled",
      "reco_value_enabled",
      "reco_capital_enabled",
      "reco_watchlist_enabled",
      "reco_min_confidence",
      // 风险/仓位扩展
      "risk_max_drawdown_limit",
      "risk_max_daily_loss_pct",
      "risk_correlation_lookback_days",
      "pos_min_cash_pct",
      "pos_max_turnover_pct",
      "kelly_min_win_rate",
      "kelly_min_odds",
      // 监控告警
      "monitor_alert_cooldown_secs",
      "monitor_min_severity",
      "monitor_channels",
      // 决策回溯
      "decision_max_history_per_stock",
      // 技术指标 B1
      "macd_fast",
      "macd_slow",
      "macd_signal",
      "boll_period",
      "boll_stddev",
      "volume_lookback",
      "volume_surge_ratio",
      "volume_shrink_ratio",
      // 推荐器策略参数 B3
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
      // 交易决策 B4
      "trading_price_deviation_limit",
      // 风险模型扩展 B4
      "risk_sharpe_annualization",
      "risk_kelly_heavy_threshold",
      "risk_kelly_medium_threshold",
    ];
    for (const v of must) {
      expect(names, `A/B 类参数缺失: ${v}`).toContain(v);
    }
  });

  it("面板不重复暴露 vendor_*（由 DataVendorsTab 全权管理）", () => {
    const names = extractVarNamesFromPanel();
    for (
      const v of [
        "vendor_tencent",
        "vendor_eastmoney",
        "vendor_sina",
        "vendor_ths",
        "vendor_cninfo",
        "vendor_baidu_stock",
        "vendor_iwencai",
        "vendor_akshare",
        "vendor_mootdx",
      ]
    ) {
      expect(names, `vendor_* 不应在参数面板里出现，${v} 改由 DataVendorsTab 管理`).not.toContain(v);
    }
  });

  it("面板定义的每个工作流参数 key 在后端模板 v19 中也存在（vendor_* 排除）", () => {
    const panel = new Set(extractVarNamesFromPanel());
    const rust = new Set(extractVarNamesFromRust());
    // 排除 vendor_*：它们由 DataVendorsTab 单独管理，避免两边同时写入竞态。
    // 排除 actual_outcome / reflection_depth / stock_lessons：反思复盘工作流运行时注入，非用户配置参数
    const runtimeInjected = new Set(["actual_outcome", "reflection_depth", "stock_lessons"]);
    const missingInPanel = [...rust]
      .filter((k) => !k.startsWith("vendor_"))
      .filter((k) => !runtimeInjected.has(k))
      .filter((k) => !panel.has(k));
    expect(missingInPanel, `后端已种子化但面板缺失: ${missingInPanel.join(", ")}`).toEqual([]);
  });
});
