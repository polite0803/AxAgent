// i18n-exempt: Quant 模块类型定义（含策略说明数据字符串），类型定义非 UI 文案。
// =====================================================================
// Quant 模块类型 — 对齐 Rust 端 quant crate
// =====================================================================
//
// 命名规范：与 Rust 端字段名 1:1 匹配（snake_case 在前端用 snake_case，
// 序列化为 JSON 交给 Rust 反序列化时不需要 rename）。
//
// 适用模块：量化交易 / 量化回测
// =====================================================================

// ── 策略元数据 ──

export type StrategyType = "builtin" | "rhai";

export interface StrategyMeta {
  id: string;
  name: string;
  version: string;
  strategyType: StrategyType;
  description: string | null;
  params: Record<string, unknown>;
  walkForwardEnabled: boolean;
  createdAt: number;
  updatedAt: number;
}

// ── 撮合配置（与 Rust MatcherConfig 对齐） ──

export interface MatcherConfig {
  commissionRate: number;
  commissionMin: number;
  stampTaxRate: number;
  slippageRate: number;
  lotSize: number;
  t1Enforced: boolean;
  limitCheck: boolean;
}

// ── 回测运行记录 ──

export type RunStatus = "pending" | "running" | "completed" | "failed";

export interface QuantRun {
  id: string;
  strategyId: string;
  name: string | null;
  startDate: string;
  endDate: string;
  initialCash: number;
  configJson: string;
  status: RunStatus;
  resultJson: string | null;
  walkForwardEnabled: boolean;
  walkForwardFolds: number | null;
  walkForwardOverfitWarning: boolean | null;
  walkForwardStabilityScore: number | null;
  startedAt: number;
  finishedAt: number | null;
  errorMessage: string | null;
}

// ── 信号历史 ──

export type SignalAction = "buy" | "sell" | "hold";
export type CloseReason =
  | "take_profit"
  | "stop_loss"
  | "signal_reverse"
  | "risk_control"
  | "end_of_backtest"
  | "manual";

export interface QuantBacktestSignal {
  code: string;
  action: SignalAction;
  strength: number;
  reason: string;
  targetWeight?: number | null;
  closeReason?: CloseReason | null;
}

// ── 纸面成交 ──

export type TradeSide = "long" | "short" | "flat";

export interface QuantBacktestTrade {
  code: string;
  side: TradeSide;
  quantity: number;
  price: number;
  amount: number;
  commission: number;
  stampTax: number;
  slippage: number;
  timestamp: string;
  reason: string;
  realizedPnl: number;
  /** 前端生成的行 key，用于 antd Table rowKey（数据源无唯一业务字段时使用） */
  _rowKey?: string;
}

// ── 回测订单 / 成交回报（与 Rust Order / Fill 对齐） ──

export type QuantOrderType =
  | { type: "market" }
  | { type: "limit"; price: number };

export interface QuantOrder {
  code: string;
  side: TradeSide;
  quantity: number;
  orderType: QuantOrderType;
  timestamp: string;
  reason: string;
}

export interface QuantFill {
  order: QuantOrder;
  fillPrice: number;
  fillAmount: number;
  commission: number;
  stampTax: number;
  slippage: number;
  timestamp: string;
  matched: boolean;
  rejectReason?: string | null;
}

// ── 回测配置（与 Rust BacktestConfig 对齐） ──

export interface QuantBacktestConfig {
  initialCash: number;
  matcher: MatcherConfig;
  startDate: string | null;
  endDate: string | null;
  codes: string[];
}

// ── 指标报告（与 Rust MetricsReport 对齐） ──

export interface MetricsReport {
  totalReturn: number;
  annualizedReturn: number;
  annualizedVolatility: number;
  sharpe: number;
  sortino: number;
  maxDrawdown: number;
  maxDrawdownPct: number;
  maxDrawdownDurationDays: number;
  winRate: number;
  profitFactor: number;
  avgWin: number;
  avgLoss: number;
  payoffRatio: number;
  totalTrades: number;
  winningTrades: number;
  losingTrades: number;
  avgHoldingDays: number;
  /** M2 占位：相对 IC 序列相关 */
  ic: number | null;
  /** M2 占位：信息比率 */
  ir: number | null;
  /** M2 占位：Calmar 比率 */
  calmar: number | null;
  /** M2 占位：胜率 - 败率的 p 值 */
  winLossPValue: number | null;
}

// ── Walk-Forward 报告 ──

export interface WalkForwardFold {
  trainBarsCount: number;
  testBarsCount: number;
  foldIndex: number;
  trainStart: string;
  trainEnd: string;
  testStart: string;
  testEnd: string;
  trainSharpe: number;
  testSharpe: number;
  bestParams: Record<string, unknown> | null;
  degradationRatio: number;
  isOverfitFold: boolean;
}

export interface WalkForwardWindowResult {
  fold: WalkForwardFold;
  overfitFlag: boolean;
  trainMetrics: { sharpe: number };
  testMetrics: { sharpe: number };
  degradationRatio: number;
  totalReturnPct: number;
}

export interface WalkForwardReport {
  folds: WalkForwardFold[];
  oosEquity: number[]; // 样本外收益序列
  stabilityScore: number; // 1 - sqrt(var(samples))
  overfitWindowCount: number;
  aggregatedTestSharpe: number;
}

// ── 权益曲线点 ──

export interface EquityPoint {
  date: string;
  equity: number;
  cash: number;
  positionValue: number;
}

// ── 回测完整结果（从 BacktestResult JSON 解析） ──

export interface BacktestResult {
  // ── 策略元信息 ──
  strategyName: string;
  strategyVersion: string;
  strategyParams: Record<string, unknown>;

  // ── 配置 ──
  config: QuantBacktestConfig;

  // ── 资金 ──
  initialCash: number;
  finalEquity: number;

  // ── 绩效指标（与 Rust 顶层字段 1:1） ──
  totalReturn: number;
  annualizedReturn: number;
  sharpe: number;
  maxDrawdown: number;
  maxDrawdownPct: number;
  winRate: number;
  totalTrades: number;
  winningTrades: number;
  losingTrades: number;

  // ── 明细 ──
  trades: QuantBacktestTrade[];
  signals: QuantBacktestSignal[];
  fills: QuantFill[];
  equityCurve: EquityPoint[];

  // ── 时间 ──
  startedAt: string;
  finishedAt: string;
  durationMs: number;
}

// ── 回测请求参数 ──

export interface BacktestRunRequest {
  strategyId: string;
  strategyType: StrategyType;
  code: string;
  startDate: string;
  endDate: string;
  initialCash: number;
  params: Record<string, unknown>;
  walkForwardEnabled: boolean;
  walkForwardForceOff: boolean;
  matcherConfig: MatcherConfig | null;
  name: string | null;
}

// ── 回测响应 ──

export interface BacktestRunResponse {
  run: QuantRun;
  metrics: MetricsReport;
  signalCount: number;
  tradeCount: number;
  walkForward: WalkForwardReport | null;
}

// ── Rhai 脚本注册请求 ──

export interface RegisterRhaiRequest {
  name: string;
  version: string;
  description: string | null;
  scriptSource: string;
  params: Record<string, unknown>;
  walkForwardEnabled: boolean;
  upsert: boolean;
}

// ── 指标对比响应 ──

export interface RunWithMetrics {
  run: QuantRun;
  strategyName: string;
  metrics: MetricsReport | null;
  errorMessage: string | null;
}

export interface MetricsCompareResponse {
  runs: RunWithMetrics[];
  bestBy: Record<string, string>; // metric → run_id
}

// ── 内置策略 ID 常量 ──

export const BUILTIN_STRATEGY_IDS = {
  MaCross: "builtin.ma_cross",
  Macd: "builtin.macd",
  Rsi: "builtin.rsi",
  Boll: "builtin.boll",
  Turtle: "builtin.turtle",
} as const;

export type BuiltinStrategyId = (typeof BUILTIN_STRATEGY_IDS)[keyof typeof BUILTIN_STRATEGY_IDS];

// ── 默认参数预设 ──

export const DEFAULT_STRATEGY_PARAMS: Record<string, Record<string, number>> = {
  [BUILTIN_STRATEGY_IDS.MaCross]: { short_period: 5, long_period: 20 },
  [BUILTIN_STRATEGY_IDS.Macd]: { fast: 12, slow: 26, signal: 9 },
  [BUILTIN_STRATEGY_IDS.Rsi]: { period: 6, overbought: 70, oversold: 30 },
  [BUILTIN_STRATEGY_IDS.Boll]: { period: 20, stddev: 2 },
  [BUILTIN_STRATEGY_IDS.Turtle]: { entry_period: 20, exit_period: 10, atr_period: 20, atr_multiplier: 2 },
};

// ── 默认 Rhai 脚本模板 ──

export const DEFAULT_RHAI_TEMPLATE = `// MA Cross 策略（用户可编辑）
// 收到每根 K 线时调用 on_bar(bar, ctx)，返回 Signal 数组。

fn on_bar(bar, ctx) {
    // 等待 20 根 K 线
    if ctx.closes.len < 20 { return []; }

    // 计算 5/20 SMA
    let s5 = sma(ctx.closes, 5);
    let s20 = sma(ctx.closes, 20);

    // 金叉买
    if s5 > s20 && ctx.position_qty == 0 {
        return [#{
            action: "buy",
            code: bar.code,
            strength: 0.7,
            reason: "MA5(" + s5 + ") 上穿 MA20(" + s20 + ")"
        }];
    }

    // 死叉卖
    if s5 < s20 && ctx.position_qty > 0 {
        return [#{
            action: "sell",
            code: bar.code,
            strength: 0.7,
            reason: "MA5 下穿 MA20",
            close_reason: "signal_reverse"
        }];
    }

    return [];
}
`;

// =====================================================================
// WF + DES 对比模块类型 — 对齐 Rust `wf_des_integration` 命令
// =====================================================================
//
// 注意：Rust `quant::WalkForwardReport` 字段与上面 `WalkForwardReport`
// （给 quant_backtest_run 用的简化版）不同，所以本节使用 `WfDes*` 前缀
// 独立命名，避免混淆。
// =====================================================================

// ── Bar（对齐 harness::strategy_contract::Bar，camelCase 序列化） ──

export interface Bar {
  date: string;
  code: string;
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
  amount: number;
  turnoverRate?: number;
  adjFactor?: number;
  limitUp?: number;
  limitDown?: number;
  isSt?: boolean;
}

// ── WF + DES 请求参数 ──

export interface WfDesWfConfig {
  trainDays: number;
  testDays: number;
  stepDays: number | null;
  riskFreeAnnual: number;
}

export interface WfDesDesConfig {
  stockCode: string;
  /** 参考价（单位：分） */
  referencePrice: number;
  /** DES 模拟时长（纳秒） */
  simDurationNs: number;
  seed: number;
  initialCash: number;
  /** 策略唤醒间隔（纳秒） */
  wakeupIntervalNs: number;
}

export interface WfDesRequest {
  klines: Bar[];
  wfConfig: WfDesWfConfig;
  desConfig: WfDesDesConfig;
  /** 策略名称（当前仅支持 "ma_cross"） */
  strategyName: string;
}

// ── WF + DES 报告（对齐 Rust WfDesReport） ──

export interface WfDesDeviation {
  /** Sharpe 偏差（DES - WF） */
  sharpeDelta: number;
  /** MaxDD 偏差（百分点） */
  maxddDelta: number;
  /** 胜率偏差（百分点） */
  winRateDelta: number;
  /** 成交量比（DES / WF） */
  volumeRatio: number;
}

export interface WfDesFold {
  foldIdx: number;
  trainStart: string;
  trainEnd: string;
  testStart: string;
  testEnd: string;
  trainBarsCount: number;
  testBarsCount: number;
}

export interface WfDesWindowResult {
  fold: WfDesFold;
  bestParams: Record<string, unknown> | null;
  trainMetrics: MetricsReport;
  testMetrics: MetricsReport;
  degradationRatio: number;
  overfitFlag: boolean;
}

export interface WfDesWalkForwardReport {
  config: WfDesWfConfig;
  windows: WfDesWindowResult[];
  aggregatedOosEquity: EquityPoint[];
  aggregatedOosMetrics: MetricsReport;
  stabilityScore: number;
  overfitWarning: boolean;
  overfitWindowCount: number;
  generatedAt: string;
}

export interface WfDesReport {
  walkforward: WfDesWalkForwardReport;
  desMetrics: MetricsReport;
  desTotalTrades: number;
  deviation: WfDesDeviation;
}
