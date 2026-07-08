// SPDX-License-Identifier: AGPL-3.0-only

import { describe, expect, it } from "vitest";

import { cleanToolCallTags, extractDecision, normalizeDecision, tryParseDecision } from "@/lib/agentOutput";
import type { StockDecision } from "@/types/stock-analysis";

describe("agentOutput decision parsing", () => {
  const expectedDecision: StockDecision = {
    action: "BUY",
    positionPct: 20,
    targetPrice: null,
    stopLoss: null,
    reasoning: "Test decision",
    riskLevel: "MID",
    confidence: 85,
    timeHorizon: null,
    expectedHoldingDays: null,
    targetTimeframe: null,
  };

  it("parses plain JSON string into StockDecision", () => {
    const parsed = tryParseDecision('{"action":"BUY","positionPct":20,"confidence":85,"reasoning":"Test decision"}');
    expect(parsed).toEqual(expectedDecision);
  });

  it("parses JSON decision wrapped in markdown code block", () => {
    const parsed = tryParseDecision(
      `Here is the decision:\n\n\`\`\`json\n{\n  "action": "BUY",\n  "positionPct": 20,\n  "confidence": 85,\n  "reasoning": "Test decision"\n}\n\`\`\``,
    );
    expect(parsed).toEqual(expectedDecision);
  });

  it("parses escaped JSON string output", () => {
    const escaped = '"{"action":"BUY","positionPct":20,"confidence":85,"reasoning":"Test decision"}"';
    const parsed = tryParseDecision(escaped);
    expect(parsed).toEqual(expectedDecision);
  });

  it("returns null for JSON arrays in raw string output", () => {
    const parsed = tryParseDecision('[{"action":"BUY","positionPct":20,"confidence":85}]');
    expect(parsed).toBeNull();
  });

  it("extractDecision returns null for array object values", () => {
    const parsed = extractDecision([{ action: "BUY" }] as unknown);
    expect(parsed).toBeNull();
  });
});

describe("normalizeDecision - 全零空壳检测", () => {
  it("空对象 {} → null (没有可解析字段)", () => {
    expect(normalizeDecision({})).toBeNull();
  });

  it("空字段对象 { action: null } → null", () => {
    expect(normalizeDecision({ action: null })).toBeNull();
  });

  it("空字符串字段对象 → null", () => {
    expect(normalizeDecision({ action: "", confidence: "", reasoning: "  " })).toBeNull();
  });

  it("HOLD 是合法决策（即便置信度为 0）→ 保留", () => {
    const parsed = normalizeDecision({ action: "HOLD" });
    expect(parsed).not.toBeNull();
    expect(parsed?.action).toBe("HOLD");
  });

  it("只含 reasoning 字段 → 保留", () => {
    const parsed = normalizeDecision({ reasoning: "基本面恶化，建议观望" });
    expect(parsed).not.toBeNull();
    expect(parsed?.reasoning).toBe("基本面恶化，建议观望");
  });

  it("snake_case 全零空壳 { position_pct: 0 } → null", () => {
    expect(normalizeDecision({ position_pct: 0 })).toBeNull();
  });

  it("CodeNode 包装但 params 是空对象 → null", () => {
    expect(normalizeDecision({ status: "ok", params: {} })).toBeNull();
  });

  it("CodeNode 包装 + 内部含有效 action → 保留", () => {
    const parsed = normalizeDecision({ status: "ok", params: { action: "BUY", confidence: 80 } });
    expect(parsed?.action).toBe("BUY");
    expect(parsed?.confidence).toBe(80);
  });
});

describe("cleanToolCallTags", () => {
  it("removes generic Hermes/Qwen-style <tool_call> blocks with <function> and <parameter>", () => {
    const input =
      "<tool_call> <function=search_stock> <parameter=stock_code> 301302 </parameter> </function> </tool_call>";
    expect(cleanToolCallTags(input)).toBe("");
  });

  it("preserves surrounding text when tool_call is embedded", () => {
    const input =
      "分析完成。<tool_call> <function=search_stock> <parameter=stock_code> 301302 </parameter> </function> </tool_call>该股票基本面良好。";
    const cleaned = cleanToolCallTags(input);
    expect(cleaned).toBe("分析完成。该股票基本面良好。");
  });

  it("removes multiple tool_call blocks", () => {
    const input =
      "<tool_call><function=foo><parameter=x>1</parameter></function></tool_call>中间文本<tool_call><function=bar><parameter=y>2</parameter></function></tool_call>";
    const cleaned = cleanToolCallTags(input);
    expect(cleaned).toBe("中间文本");
  });

  it("removes orphan <function> and <parameter> tags without outer tool_call", () => {
    const input = "<function=search_stock><parameter=stock_code>301302</parameter></function>";
    expect(cleanToolCallTags(input)).toBe("");
  });

  it("removes tool_call with attributes", () => {
    const input =
      '<tool_call id="tc1"><function=search_stock><parameter=stock_code>301302</parameter></function></tool_call>';
    expect(cleanToolCallTags(input)).toBe("");
  });

  it("still handles provider-prefixed XML tool_call format", () => {
    const input = '<anthropic:tool_call>{"name":"search_stock"}</anthropic:tool_call>实际分析内容';
    expect(cleanToolCallTags(input)).toBe("实际分析内容");
  });

  it("does not match HTML-like tags such as <figure> or <param>", () => {
    const input = '<figure>图表说明</figure><param name="x" value="1">';
    expect(cleanToolCallTags(input)).toBe(input);
  });
});

describe("normalizeDecision - workflow results map 兜底（修复'决策信息缺失'误报）", () => {
  it("识别 workflow results map 并从 portfolio-mgr.result 提取决策", () => {
    // 模拟后端 stock-analysis 工作流 output_schema 未用 $source 标记,
    // filter_by_schema fallback 到整个 results map 写入 decisionJson
    // 的老数据格式(修复前的 bug 表现)。
    const resultsMap = {
      trigger: { status: "executed", node_id: "trigger" },
      "t-quote": { status: "ok", result: { price: 12.5 } },
      research: { status: "ok", result: { risk: "中" } },
      "portfolio-mgr": {
        status: "executed",
        language: "rhai",
        result: {
          action: "买入",
          positionPct: 50,
          confidence: 75,
          riskLevel: "中",
          reasoning: "技术面强势",
          timeHorizon: "mid",
          expectedHoldingDays: 28,
          targetTimeframe: "1m",
        },
        input_params: { totalScore: 70 },
        node_id: "portfolio-mgr",
        params: { action: "买入" },
      },
      "end-output": { status: "ok" },
    };
    const parsed = normalizeDecision(resultsMap);
    expect(parsed).not.toBeNull();
    expect(parsed?.action).toBe("BUY"); // 买入 → BUY
    expect(parsed?.positionPct).toBe(50);
    expect(parsed?.confidence).toBe(75);
    expect(parsed?.riskLevel).toBe("MID"); // 中 → MID
    expect(parsed?.reasoning).toBe("技术面强势");
    expect(parsed?.timeHorizon).toBe("mid");
    expect(parsed?.expectedHoldingDays).toBe(28);
  });

  it("portfolio-mgr 是 CodeNode 包装但 .result 缺失时降级用 portfolio-mgr 本身", () => {
    // 模拟异常路径:portfolio-mgr 包装存在但 .result 字段缺失
    const resultsMap = {
      "portfolio-mgr": {
        status: "executed",
        language: "rhai",
        // result 字段缺失
        input_params: {},
        node_id: "portfolio-mgr",
        params: { action: "HOLD", confidence: 30, riskLevel: "HIGH" },
      },
    };
    const parsed = normalizeDecision(resultsMap);
    // 兜底逻辑会从 portfolio-mgr 本身提取,原 CodeNode 检测会从 .params 拿
    expect(parsed).not.toBeNull();
    expect(parsed?.action).toBe("HOLD");
    expect(parsed?.confidence).toBe(30);
    expect(parsed?.riskLevel).toBe("HIGH");
  });

  it("results map 内 portfolio-mgr 也不存在时仍返回 null（避免误报）", () => {
    // 类似 results map 结构但缺 portfolio-mgr 节点（异常工作流）
    const resultsMap = {
      trigger: { status: "ok" },
      research: { status: "ok" },
    };
    const parsed = normalizeDecision(resultsMap);
    // 没有 portfolio-mgr 节点,无法提取决策 → 保持 null
    expect(parsed).toBeNull();
  });

  it("业务决策对象含 action 字段时不被识别为 results map（不递归）", () => {
    // 即使业务决策对象恰好有一个键叫 "research"（罕见但可能）,
    // 因为它已经有 action 字段,不应被误判为 results map。
    const businessDecision = {
      action: "BUY",
      confidence: 80,
      research: "n/a", // 字符串而非对象,也不会触发检测
    };
    const parsed = normalizeDecision(businessDecision);
    expect(parsed?.action).toBe("BUY");
    expect(parsed?.confidence).toBe(80);
  });
});
