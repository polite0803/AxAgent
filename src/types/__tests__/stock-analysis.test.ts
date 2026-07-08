import { describe, expect, it } from "vitest";
import { classifySentiment, getSignalColor, parseAction, StockAction } from "../stock-analysis";

describe("classifySentiment", () => {
  it('returns "bullish" for buy signals', () => {
    expect(classifySentiment("推荐买入该股票")).toBe("bullish");
    expect(classifySentiment("建议增持")).toBe("bullish");
    expect(classifySentiment("看多后市")).toBe("bullish");
    expect(classifySentiment("走势乐观，建议持有")).toBe("bullish");
  });

  it('returns "bearish" for sell signals', () => {
    expect(classifySentiment("建议卖出")).toBe("bearish");
    expect(classifySentiment("减持观望")).toBe("bearish");
    expect(classifySentiment("看空后市")).toBe("bearish");
    expect(classifySentiment("市场悲观")).toBe("bearish");
  });

  it('returns "neutral" for mixed or unclear signals', () => {
    expect(classifySentiment("市场走势平稳")).toBe("neutral");
    expect(classifySentiment("")).toBe("neutral");
    expect(classifySentiment("建议关注")).toBe("neutral");
  });

  it("reads stance field from JSON (trader: 买入/卖出/观望)", () => {
    expect(classifySentiment(JSON.stringify({ stance: "买入", positionPct: 35 }))).toBe("bullish");
    expect(classifySentiment(JSON.stringify({ stance: "卖出", positionPct: 0 }))).toBe("bearish");
    expect(classifySentiment(JSON.stringify({ stance: "观望", positionPct: 0 }))).toBe("neutral");
  });

  it("reads stance from news/debate agents (多头/空头/中性)", () => {
    expect(classifySentiment(JSON.stringify({ stance: "多头" }))).toBe("bullish");
    expect(classifySentiment(JSON.stringify({ stance: "空头" }))).toBe("bearish");
    expect(classifySentiment(JSON.stringify({ stance: "中性" }))).toBe("neutral");
  });

  it("reads bull_score / bear_score (0-10) — majority wins", () => {
    expect(classifySentiment(JSON.stringify({ bull_score: 6, bear_score: 4 }))).toBe("bullish");
    expect(classifySentiment(JSON.stringify({ bull_score: 3, bear_score: 7 }))).toBe("bearish");
    expect(classifySentiment(JSON.stringify({ bull_score: 5, bear_score: 5 }))).toBe("neutral");
  });

  it("reads positionPct (trader/debator) — ≥6 bullish, <0 bearish, else neutral", () => {
    expect(classifySentiment(JSON.stringify({ positionPct: 35 }))).toBe("bullish");
    expect(classifySentiment(JSON.stringify({ positionPct: 0 }))).toBe("neutral");
    expect(classifySentiment(JSON.stringify({ positionPct: 5 }))).toBe("neutral");
    expect(classifySentiment(JSON.stringify({ positionPct: -10 }))).toBe("bearish");
  });

  it("handles combined report JSON (stance + scores + free text)", () => {
    // 实际 LLM 输出：多头 + bull_score 6 + bear_score 4 + 自由文本里有风险提示
    const report = JSON.stringify({
      stance: "多头",
      bull_score: 6,
      bear_score: 4,
      summary: "技术面突破前高，基本面估值合理。短期存在回调风险，但中长期看好。",
    });
    expect(classifySentiment(report)).toBe("bullish");
  });

  it("text fallback: simple majority (not 65% threshold)", () => {
    // 旧 65% 阈值会让"看好 + 风险"被中性化；现在简单多数直接判看多
    expect(classifySentiment("看好后市，但短期有回调风险")).toBe("bullish");
    expect(classifySentiment("看空后市，但长期或有机会")).toBe("bearish");
  });
});

describe("parseAction", () => {
  it("parses standard actions", () => {
    expect(parseAction("买入")).toBe(StockAction.BUY);
    expect(parseAction("卖出")).toBe(StockAction.SELL);
    expect(parseAction("增持")).toBe(StockAction.INCREASE);
    expect(parseAction("减持")).toBe(StockAction.REDUCE);
    expect(parseAction("持有")).toBe(StockAction.HOLD);
    expect(parseAction("观望")).toBe(StockAction.WAIT);
  });

  it("falls back to WAIT for unknown actions", () => {
    expect(parseAction("unknown")).toBe(StockAction.WAIT);
    expect(parseAction("")).toBe(StockAction.WAIT);
    expect(parseAction(null)).toBe(StockAction.WAIT);
  });
});

describe("getSignalColor", () => {
  it('returns "green" for buy/bull signals', () => {
    expect(getSignalColor("买入信号")).toBe("green");
    expect(getSignalColor("看多")).toBe("green");
    expect(getSignalColor("上涨趋势")).toBe("green");
  });

  it('returns "red" for sell/bear signals', () => {
    expect(getSignalColor("卖出信号")).toBe("red");
    expect(getSignalColor("看空")).toBe("red");
    expect(getSignalColor("下跌趋势")).toBe("red");
  });

  it('returns "blue" for neutral signals', () => {
    expect(getSignalColor("关注")).toBe("blue");
    expect(getSignalColor("")).toBe("blue");
  });
});
