import { describe, expect, it } from "vitest";

import { estimateMessageTokens, estimateTokens } from "../tokenEstimator";

describe("estimateTokens", () => {
  it("returns 0 for empty string", () => {
    expect(estimateTokens("")).toBe(0);
  });

  it("estimates ASCII text at ~4 chars per token", () => {
    const tokens = estimateTokens("Hello World");
    // 11 ASCII chars / 4 = 2.75 → ceil = 3
    expect(tokens).toBe(3);
  });

  it("estimates CJK text at ~1.5 chars per token", () => {
    const tokens = estimateTokens("你好世界测试");
    // 6 CJK chars * 2 / 3 = 4 tokens
    expect(tokens).toBe(4);
  });

  it("estimates mixed CJK + ASCII content correctly", () => {
    const tokens = estimateTokens("Hello 你好 World 世界");
    // ASCII: "Hello " = 6, " World " = 7 → 13 ASCII chars / 4 = 4 (ceil)
    // CJK: "你好" = 2, "世界" = 2 → 4 CJK * 2 / 3 = 3 (ceil)
    // Total: 7
    expect(tokens).toBe(7);
  });

  it("estimates large text within reasonable bounds", () => {
    const longText = "A".repeat(1000);
    const tokens = estimateTokens(longText);
    expect(tokens).toBe(250); // 1000 / 4
  });

  it("handles CJK-only sentence", () => {
    const cjkText = "今天的天气非常好，我们一起去公园散步吧。";
    const tokens = estimateTokens(cjkText);
    // 20 CJK chars (含全角标点) * 2 / 3 = 13.33 → ceil = 14
    expect(tokens).toBe(14);
  });

  it("handles emoji as ASCII-range fallback", () => {
    const text = "😀🎉🚀";
    const tokens = estimateTokens(text);
    // 3 emoji = 6 UTF-16 code units (surrogate pairs) → 6/4 = 1.5 → ceil = 2
    expect(tokens).toBe(2);
  });
});

describe("estimateMessageTokens", () => {
  it("includes PER_MESSAGE_OVERHEAD of 4 tokens", () => {
    const tokens = estimateMessageTokens("user", "Hello");
    // role "user" (4 chars / 4 = 1) + content "Hello" (5/4 = 2) + 4 overhead = 7
    expect(tokens).toBe(7);
  });

  it("returns overhead when role and content are empty", () => {
    const tokens = estimateMessageTokens("", "");
    expect(tokens).toBe(4); // only PER_MESSAGE_OVERHEAD
  });

  it("correctly sums role and content token estimates", () => {
    const tokens1 = estimateMessageTokens("assistant", "OK");
    const tokens2 = estimateMessageTokens("system", "Long system prompt with instructions");
    expect(tokens1).toBeGreaterThan(4);
    expect(tokens2).toBeGreaterThan(tokens1);
  });
});
