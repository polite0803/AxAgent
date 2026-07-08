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

export interface QuantSignal {
  id: string;
  runId: string;
  code: string;
  action: SignalAction;
  strength: number;
  reason: string | null;
  closeReason: CloseReason | null;
  timestamp: string;
  createdAt: number;
}

// ── 纸面成交 ──

export type TradeSide = "long" | "short" | "flat";

export interface QuantPaperTrade {
  id: string;
  runId: string;
  code: string;
  side: TradeSide;
  quantity: number;
  price: number;
  amount: number;
  commission: number;
  stampTax: number;
  slippage: number;
  timestamp: string;
  reason: string | null;
  realizedPnl: number;
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
  marketValue: number;
  drawdown: number;
  drawdownPct: number;
}

// ── 回测完整结果（从 BacktestResult JSON 解析） ──

export interface BacktestResult {
  config: {
    initialCash: number;
    matcher: MatcherConfig;
    startDate: string | null;
    endDate: string | null;
  };
  signals: QuantSignal[];
  trades: QuantPaperTrade[];
  equityCurve: EquityPoint[];
  /** 内联基础指标（MetricsReport 兼容） */
  metrics: {
    totalReturn: number;
    sharpe: number;
    maxDrawdown: number;
    maxDrawdownPct: number;
    annualizedReturn: number;
    winRate: number;
  };
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
