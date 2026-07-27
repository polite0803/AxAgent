#!/usr/bin/env node
// scripts/sync-acp-i18n.mjs
// 一次性脚本：把 acp.* 重构后新增的 i18n key 同步到其他 10 种语言文件。
// 策略：zh-CN 已手动翻译（权威）；en-US 手动翻译；zh-TW 手动翻译；
// 其余 8 种语言用 en-US 翻译填充（i18next fallbackLng=en-US 也会兜底，
// 但填充后避免每次查找都走 fallback，且文件自包含可被扫描器识别）。
import { readFileSync, writeFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const localesDir = join(__dirname, "..", "src", "i18n", "locales");

// ── 新增 key 的英文翻译 ──
// 结构必须与 zh-CN.json 中新增的 key 结构完全一致
const EN_KEYS = {
  stockAnalysis: {
    debate: {
      evidence: "Evidence",
      title: "Debate",
    },
    timeline: {
      tooltip: "Node details",
      chip: "{{n}} violations",
      chipAria: "{{n}} violations",
      markClass: "violation-mark",
    },
    backtest: {
      backtestHint: "Strategy backtest & batch replay (as-of: {{date}})",
    },
    recommendation: {
      bannerAsOf: "Recommendations replayed to {{date}}",
      degradedStylesTitle: "Degraded styles",
    },
    decision: {
      exportFailed: "Export failed: {{errMsg}}",
      wordDocument: "Word document",
      gateControl: "Gate control",
      mandatoryDirection: "Mandatory direction",
      evidenceScore: "Evidence score",
      net: "Net",
      oppositeDirection: "Opposite",
      disagreement: "Disagreement",
      opposite: "Opposite",
      sameDirection: "Same direction",
      directionConflict: "Direction conflict",
      diff: "Diff",
      confidenceLabel: "Confidence",
      positionLabel: "Position",
      positionGap: "Position gap {{gap}}%",
      confidenceGap: "Confidence gap {{gap}}",
    },
    whatIf: {
      technicalVeto: "Technical veto",
      simulationGate: "Simulation gate",
      preSimulationDecision: "Pre-simulation decision",
      stability: "Stability",
      liquidity: "Liquidity",
      impact: "Impact",
    },
    debugLabel: {
      analyst: "Analyst",
      riskControl: "Risk control",
      bullDebate: "Bull debate",
      bearDebate: "Bear debate",
      dataTool: "Data tool",
      debateConvergence: "Debate convergence",
      decisionEngine: "Decision engine",
      ruleCheck: "Rule check",
      other: "Other",
    },
    tradeStats: {
      title: "Trade statistics",
      totalPl: "Total P&L",
      winRate: "Win rate",
      profitFactor: "Profit factor",
      avgHoldingDays: "Avg holding days",
      daysUnit: "days",
      feeEstimate: "Fee estimate",
      stampTax: "Stamp tax",
      commission: "Commission",
      tradeCount: "{{count}} trades",
      holdingDaysDistribution: "Holding days distribution",
      tradeCountUnit: "{{count}}",
      byStrategy: "By strategy",
      monthlyPl: "Monthly P&L",
    },
    portfolio: {
      importPartial: "Partial import: {{ok}} succeeded, {{failed}} failed",
      yuan: "CNY",
    },
    history: {
      batchExit: "Batch exit",
    },
    execution: {
      nodes: "nodes",
    },
  },
  quant: {
    backtest: {
      strategyParameters: "Strategy parameters",
      annualizedVolatility: "Annualized volatility",
      profitFactor: "Profit factor",
      payoffRatio: "Payoff ratio",
      averageHoldingDays: "Average holding days",
      maxDrawdown: "Max drawdown",
      maxDrawdownDuration: "Max drawdown duration",
      winningTrades: "Winning trades",
      losingTrades: "Losing trades",
      averageWin: "Average win",
      averageLoss: "Average loss",
      trainRange: "Train range",
      testRange: "Test range",
      outOfSampleEquity: "Out-of-sample equity",
      barsCount: "{{count}} bars",
    },
  },
  timeTravel: {
    datePicker: {
      hint: "Pick replay cutoff date",
      ok: "OK",
      cancel: "Cancel",
    },
    tour: {
      title: "Time anchor tour",
      body: "Click the time anchor button to pin all data to a past date for replay analysis.",
      gotIt: "Got it",
      stepAnchor: "Time anchor",
      close: "Close",
    },
    badge: {
      sweep: "Replay sweep",
      replay: "Replay to {{date}}",
      replayTooltip: "This data is replayed as of {{date}}",
    },
    sweep: {
      total: "Total",
      accuracy: "Accuracy",
      alpha: "Alpha",
      sharpe: "Sharpe",
    },
  },
  dualView: {
    decision: {
      opposite: "Opposite",
      sameDirection: "Same direction",
      disagreement: "Disagreement",
      diff: "Diff",
      confidenceLabel: "Confidence",
      positionLabel: "Position",
      oppositeDirection: "Opposite",
      directionConflict: "Direction conflict",
    },
  },
  settings: {
    dataVendors: {
      name: "{{0}}",
      refreshNeodata: "Refresh NeoData cache",
      capabilityLabel: "{{0}}",
      helpText: "{{0}}",
      toolLabel: "{{0}}",
    },
  },
};

// ── zh-TW 翻译（简繁转换）──
const ZH_TW_KEYS = {
  stockAnalysis: {
    debate: {
      evidence: "證據",
      title: "辯論",
    },
    timeline: {
      tooltip: "節點詳情",
      chip: "{{n}} 項違規",
      chipAria: "{{n}} 項違規",
      markClass: "violation-mark",
    },
    backtest: {
      backtestHint: "策略回測與批量回放（as-of: {{date}}）",
    },
    recommendation: {
      bannerAsOf: "回放至 {{date}} 的推薦",
      degradedStylesTitle: "降級風格",
    },
    decision: {
      exportFailed: "匯出失敗: {{errMsg}}",
      wordDocument: "Word 文件",
      gateControl: "閘門控制",
      mandatoryDirection: "強制方向",
      evidenceScore: "證據評分",
      net: "淨分",
      oppositeDirection: "反向",
      disagreement: "分歧",
      opposite: "相反",
      sameDirection: "同向",
      directionConflict: "方向衝突",
      diff: "差",
      confidenceLabel: "置信度",
      positionLabel: "倉位",
      positionGap: "倉位差 {{gap}}%",
      confidenceGap: "置信度差 {{gap}}",
    },
    whatIf: {
      technicalVeto: "技術面否決",
      simulationGate: "模擬閘門",
      preSimulationDecision: "模擬前決策",
      stability: "穩定性",
      liquidity: "流動性",
      impact: "衝擊",
    },
    debugLabel: {
      analyst: "分析師",
      riskControl: "風控",
      bullDebate: "多方辯論",
      bearDebate: "空方辯論",
      dataTool: "資料工具",
      debateConvergence: "辯論收斂",
      decisionEngine: "決策引擎",
      ruleCheck: "規則檢查",
      other: "其他",
    },
    tradeStats: {
      title: "交易統計",
      totalPl: "總盈虧",
      winRate: "勝率",
      profitFactor: "盈虧比",
      avgHoldingDays: "平均持有天數",
      daysUnit: "天",
      feeEstimate: "費用估算",
      stampTax: "印花稅",
      commission: "佣金",
      tradeCount: "{{count}} 筆交易",
      holdingDaysDistribution: "持有天數分佈",
      tradeCountUnit: "{{count}} 筆",
      byStrategy: "按策略",
      monthlyPl: "月度盈虧",
    },
    portfolio: {
      importPartial: "部分匯入成功: 成功 {{ok}} 條, 失敗 {{failed}} 條",
      yuan: "元",
    },
    history: {
      batchExit: "批量退出",
    },
    execution: {
      nodes: "節點",
    },
  },
  quant: {
    backtest: {
      strategyParameters: "策略參數",
      annualizedVolatility: "年化波動率",
      profitFactor: "盈虧比",
      payoffRatio: "收益風險比",
      averageHoldingDays: "平均持有天數",
      maxDrawdown: "最大回撤",
      maxDrawdownDuration: "最大回撤持續期",
      winningTrades: "盈利交易次數",
      losingTrades: "虧損交易次數",
      averageWin: "平均盈利",
      averageLoss: "平均虧損",
      trainRange: "訓練區間",
      testRange: "測試區間",
      outOfSampleEquity: "樣本外淨值",
      barsCount: "{{count}} 根 K 線",
    },
  },
  timeTravel: {
    datePicker: {
      hint: "選擇回放截止日期",
      ok: "確定",
      cancel: "取消",
    },
    tour: {
      title: "時間錨點導覽",
      body: "點擊時間錨點按鈕，把所有資料錨定到過去的某一天進行回放分析。",
      gotIt: "知道了",
      stepAnchor: "時間錨點",
      close: "關閉",
    },
    badge: {
      sweep: "回放掃描",
      replay: "回放至 {{date}}",
      replayTooltip: "此資料基於 {{date}} 回放生成",
    },
    sweep: {
      total: "總計",
      accuracy: "準確率",
      alpha: "Alpha",
      sharpe: "夏普比率",
    },
  },
  dualView: {
    decision: {
      opposite: "相反",
      sameDirection: "同向",
      disagreement: "分歧",
      diff: "差",
      confidenceLabel: "置信度",
      positionLabel: "倉位",
      oppositeDirection: "反向",
      directionConflict: "方向衝突",
    },
  },
  settings: {
    dataVendors: {
      name: "{{0}}",
      refreshNeodata: "刷新 NeoData 快取",
      capabilityLabel: "{{0}}",
      helpText: "{{0}}",
      toolLabel: "{{0}}",
    },
  },
};

// ── 深度合併：把 src 的 key 合併到 dst（不覆蓋已存在的 key）──
function deepMerge(dst, src) {
  for (const key of Object.keys(src)) {
    const sv = src[key];
    if (sv && typeof sv === "object" && !Array.isArray(sv)) {
      if (typeof dst[key] !== "object" || dst[key] === null || Array.isArray(dst[key])) {
        dst[key] = {};
      }
      deepMerge(dst[key], sv);
    } else if (!(key in dst)) {
      dst[key] = sv;
    }
  }
  return dst;
}

// ── 主流程 ──
const targets = [
  { file: "en-US.json", keys: EN_KEYS },
  { file: "zh-TW.json", keys: ZH_TW_KEYS },
  // 其他 8 种语言用英文填充
  { file: "ja.json", keys: EN_KEYS },
  { file: "ko.json", keys: EN_KEYS },
  { file: "fr.json", keys: EN_KEYS },
  { file: "de.json", keys: EN_KEYS },
  { file: "es.json", keys: EN_KEYS },
  { file: "ru.json", keys: EN_KEYS },
  { file: "hi.json", keys: EN_KEYS },
  { file: "ar.json", keys: EN_KEYS },
];

let totalAdded = 0;
for (const { file, keys } of targets) {
  const path = join(localesDir, file);
  const raw = readFileSync(path, "utf8");
  const json = JSON.parse(raw);
  deepMerge(json, keys);
  // JSON.stringify 2-space 缩进，与现有文件格式一致
  const out = JSON.stringify(json, null, 2) + "\n";
  writeFileSync(path, out, "utf8");
  console.log(`✓ ${file}: synced`);
  totalAdded++;
}
console.log(`\nDone. ${totalAdded} locale files synced.`);
