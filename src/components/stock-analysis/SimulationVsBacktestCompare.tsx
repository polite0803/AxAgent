/**
 * 市场模拟 vs 回测对比视图
 *
 * 并排显示 quant BacktestResult 和 market-sim MetricsReport，
 * 让用户直观对比"策略在真实历史 vs 模拟环境"的表现差异。
 *
 * 后端数据由 `sim_trades_to_metrics` 桥接生成。
 */

import { useTranslation } from "react-i18next";

/** 对比指标行 */
export interface ComparisonRow {
  metric: string;
  backtest: number;
  simulation: number;
  diff: number;
  better: "backtest" | "simulation" | "tie";
}

/** 对比报告 */
export interface SimulationVsBacktestReport {
  strategyName: string;
  stockCode: string;
  period: string;
  comparisons: ComparisonRow[];
  backtestTrades: number;
  simTrades: number;
  backtestSummary: string;
  simSummary: string;
}

interface Props {
  report: SimulationVsBacktestReport;
}

/** 指标中文名 */
const METRIC_LABELS: Record<string, string> = {
  totalReturn: "总收益率",
  annualizedReturn: "年化收益",
  sharpe: "夏普比率",
  maxDrawdown: "最大回撤",
  winRate: "胜率",
  profitFactor: "盈亏比",
  avgReturn: "平均单笔收益",
  avgHoldingDays: "平均持仓天数",
};

/** 哪个方向更好 */
const BETTER_DIRECTION: Record<string, "higher" | "lower"> = {
  totalReturn: "higher",
  annualizedReturn: "higher",
  sharpe: "higher",
  maxDrawdown: "lower",
  winRate: "higher",
  profitFactor: "higher",
  avgReturn: "higher",
  avgHoldingDays: "lower",
};

export function SimulationVsBacktestCompare({ report }: Props) {
  const { t } = useTranslation();

  return (
    <div className="space-y-4">
      {/* 概览头部 */}
      <div className="flex items-center justify-between mb-2">
        <div>
          <h3 className="text-sm font-medium text-gray-200">
            {t("stockAnalysis.simVsBacktest.title", "模拟 vs 回测对比")}
          </h3>
          <p className="text-xs text-gray-500 mt-0.5">
            {report.strategyName} — {report.stockCode} — {report.period}
          </p>
        </div>
        <div className="flex gap-3 text-xs text-gray-400">
          <span>
            {t("stockAnalysis.simVsBacktest.backtestTrades", "回测交易")}: {report.backtestTrades}
          </span>
          <span>
            {t("stockAnalysis.simVsBacktest.simTrades", "模拟交易")}: {report.simTrades}
          </span>
        </div>
      </div>

      {/* 指标对比表 */}
      <div className="bg-gray-900/50 rounded-lg overflow-hidden">
        <table className="w-full text-xs">
          <thead>
            <tr className="border-b border-gray-700 text-gray-400">
              <th className="text-left py-2 px-3">
                {t("stockAnalysis.simVsBacktest.metric", "指标")}
              </th>
              <th className="text-right py-2 px-3">
                {t("stockAnalysis.simVsBacktest.backtest", "回测")}
              </th>
              <th className="text-right py-2 px-3">
                {t("stockAnalysis.simVsBacktest.simulation", "模拟")}
              </th>
              <th className="text-right py-2 px-3">
                {t("stockAnalysis.simVsBacktest.diff", "差值")}
              </th>
              <th className="text-center py-2 px-3">
                {t("stockAnalysis.simVsBacktest.judgment", "判断")}
              </th>
            </tr>
          </thead>
          <tbody>
            {report.comparisons.map((row, _i) => {
              const direction = BETTER_DIRECTION[row.metric] ?? "higher";
              const backtestWins = direction === "higher"
                ? row.backtest > row.simulation
                : row.backtest < row.simulation;
              const better = backtestWins ? "backtest" : "simulation";

              // 格式化值
              const fmtVal = (v: number, metric: string) => {
                if (metric === "sharpe" || metric === "profitFactor") {
                  return v.toFixed(3);
                }
                if (metric === "avgHoldingDays") {
                  return v.toFixed(1);
                }
                return `${(v * 100).toFixed(2)}%`;
              };

              return (
                <tr
                  key={row.metric}
                  className="border-b border-gray-800 hover:bg-gray-800/30"
                >
                  <td className="py-2 px-3 text-gray-300">
                    {METRIC_LABELS[row.metric] ?? row.metric}
                  </td>
                  <td className="py-2 px-3 text-right font-mono text-gray-200">
                    {fmtVal(row.backtest, row.metric)}
                  </td>
                  <td className="py-2 px-3 text-right font-mono text-gray-200">
                    {fmtVal(row.simulation, row.metric)}
                  </td>
                  <td
                    className={`py-2 px-3 text-right font-mono ${row.diff >= 0 ? "text-green-400" : "text-red-400"}`}
                  >
                    {row.diff >= 0 ? "+" : ""}
                    {fmtVal(Math.abs(row.diff), row.metric)}
                  </td>
                  <td className="py-2 px-3 text-center">
                    <span
                      className={`inline-block px-1.5 py-0.5 rounded text-[10px] ${
                        better === "backtest"
                          ? "bg-blue-900/40 text-blue-400"
                          : "bg-orange-900/40 text-orange-400"
                      }`}
                    >
                      {better === "backtest"
                        ? t("stockAnalysis.simVsBacktest.backtestBetter", "回测胜")
                        : t("stockAnalysis.simVsBacktest.simBetter", "模拟胜")}
                    </span>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>

      {/* 缩略结论 */}
      <div className="grid grid-cols-2 gap-3">
        <div className="bg-blue-900/20 rounded-lg p-2.5 border border-blue-800/30">
          <div className="text-[10px] text-blue-400 mb-1 font-medium">
            {t("stockAnalysis.simVsBacktest.backtestSummary", "回测结论")}
          </div>
          <div className="text-xs text-gray-300 leading-relaxed">
            {report.backtestSummary}
          </div>
        </div>
        <div className="bg-orange-900/20 rounded-lg p-2.5 border border-orange-800/30">
          <div className="text-[10px] text-orange-400 mb-1 font-medium">
            {t("stockAnalysis.simVsBacktest.simSummary", "模拟结论")}
          </div>
          <div className="text-xs text-gray-300 leading-relaxed">
            {report.simSummary}
          </div>
        </div>
      </div>
    </div>
  );
}
