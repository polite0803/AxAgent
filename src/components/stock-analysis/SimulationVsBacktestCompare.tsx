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

  const metricLabels: Record<string, string> = {
    totalReturn: t("stockAnalysis.simVsBacktest.metricLabel.totalReturn"),
    annualizedReturn: t("stockAnalysis.simVsBacktest.metricLabel.annualizedReturn"),
    sharpe: t("stockAnalysis.simVsBacktest.metricLabel.sharpe"),
    maxDrawdown: t("stockAnalysis.simVsBacktest.metricLabel.maxDrawdown"),
    winRate: t("stockAnalysis.simVsBacktest.metricLabel.winRate"),
    profitFactor: t("stockAnalysis.simVsBacktest.metricLabel.profitFactor"),
    avgReturn: t("stockAnalysis.simVsBacktest.metricLabel.avgReturn"),
    avgHoldingDays: t("stockAnalysis.simVsBacktest.metricLabel.avgHoldingDays"),
  };

  return (
    <div className="space-y-4">
      {/* 概览头部 */}
      <div className="flex items-center justify-between mb-2">
        <div>
          <h3 className="text-sm font-medium text-gray-200">
            {t("stockAnalysis.simVsBacktest.title")}
          </h3>
          <p className="text-xs text-gray-500 mt-0.5">
            {report.strategyName} — {report.stockCode} — {report.period}
          </p>
        </div>
        <div className="flex gap-3 text-xs text-gray-400">
          <span>
            {t("stockAnalysis.simVsBacktest.backtestTrades")}: {report.backtestTrades}
          </span>
          <span>
            {t("stockAnalysis.simVsBacktest.simTrades")}: {report.simTrades}
          </span>
        </div>
      </div>

      {/* 指标对比表 */}
      <div className="bg-gray-900/50 rounded-lg overflow-hidden">
        <table className="w-full text-xs">
          <thead>
            <tr className="border-b border-gray-700 text-gray-400">
              <th className="text-left py-2 px-3">
                {t("stockAnalysis.simVsBacktest.metric")}
              </th>
              <th className="text-right py-2 px-3">
                {t("stockAnalysis.simVsBacktest.backtest")}
              </th>
              <th className="text-right py-2 px-3">
                {t("stockAnalysis.simVsBacktest.simulation")}
              </th>
              <th className="text-right py-2 px-3">
                {t("stockAnalysis.simVsBacktest.diff")}
              </th>
              <th className="text-center py-2 px-3">
                {t("stockAnalysis.simVsBacktest.judgment")}
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
                    {metricLabels[row.metric] ?? row.metric}
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
                        ? t("stockAnalysis.simVsBacktest.backtestBetter")
                        : t("stockAnalysis.simVsBacktest.simBetter")}
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
            {t("stockAnalysis.simVsBacktest.backtestSummary")}
          </div>
          <div className="text-xs text-gray-300 leading-relaxed">
            {report.backtestSummary}
          </div>
        </div>
        <div className="bg-orange-900/20 rounded-lg p-2.5 border border-orange-800/30">
          <div className="text-[10px] text-orange-400 mb-1 font-medium">
            {t("stockAnalysis.simVsBacktest.simSummary")}
          </div>
          <div className="text-xs text-gray-300 leading-relaxed">
            {report.simSummary}
          </div>
        </div>
      </div>
    </div>
  );
}
