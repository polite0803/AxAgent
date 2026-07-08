import { describe, expect, it } from "vitest";
import { detectFutureReferences, detectFutureReferencesForNode } from "../futureReferenceDetector";

describe("detectFutureReferences — stage A (absolute dates)", () => {
  it("flags a future date string after as_of", () => {
    const hits = detectFutureReferences(
      "Market will close at 2026-07-01.",
      "2026-06-01",
    );
    expect(hits).toEqual([
      { snippet: "2026-07-01", ruleHit: "absolute-date" },
    ]);
  });

  it("ignores dates on or before as_of", () => {
    const hits = detectFutureReferences(
      "Past events: 2025-12-01 and 2026-06-01.",
      "2026-06-01",
    );
    expect(hits).toEqual([]);
  });

  it("dedups repeated snippets", () => {
    const hits = detectFutureReferences(
      "First 2027-01-01, second 2027-01-01 again.",
      "2026-06-01",
    );
    expect(hits).toEqual([
      { snippet: "2027-01-01", ruleHit: "absolute-date" },
    ]);
  });

  it("returns nothing when as_of is null (live mode)", () => {
    const hits = detectFutureReferences(
      "2027-01-01 tomorrow next week",
      null,
    );
    expect(hits).toEqual([]);
  });
});

describe("detectFutureReferences — stage B (tense phrases)", () => {
  it("flags 'tomorrow'", () => {
    const hits = detectFutureReferences(
      "The market will rally tomorrow.",
      "2026-06-01",
    );
    expect(hits).toContainEqual({
      snippet: "tomorrow",
      ruleHit: "tense-phrase",
    });
  });

  it("flags 'next quarter'", () => {
    const hits = detectFutureReferences(
      "Expect gains next quarter.",
      "2026-06-01",
    );
    expect(hits).toContainEqual({
      snippet: "next quarter",
      ruleHit: "tense-phrase",
    });
  });

  it("flags Chinese future tense 明天", () => {
    const hits = detectFutureReferences(
      "预计明天市场将反弹。",
      "2026-06-01",
    );
    expect(hits).toContainEqual({
      snippet: "明天",
      ruleHit: "tense-phrase",
    });
  });

  it("flags Chinese future tense 下周", () => {
    const hits = detectFutureReferences(
      "下周有望突破前高。",
      "2026-06-01",
    );
    expect(hits).toContainEqual({
      snippet: "下周",
      ruleHit: "tense-phrase",
    });
  });

  it("flags Chinese future tense 即将", () => {
    const hits = detectFutureReferences(
      "公司即将发布季报。",
      "2026-06-01",
    );
    expect(hits).toContainEqual({
      snippet: "即将",
      ruleHit: "tense-phrase",
    });
  });
});

describe("detectFutureReferences — stage C (vague future)", () => {
  it("flags 'soon'", () => {
    const hits = detectFutureReferences(
      "A recovery is coming soon.",
      "2026-06-01",
    );
    expect(hits).toContainEqual({
      snippet: "soon",
      ruleHit: "vague-future",
    });
  });

  it("flags Chinese vague future 未来", () => {
    const hits = detectFutureReferences(
      "未来发展前景良好。",
      "2026-06-01",
    );
    expect(hits).toContainEqual({
      snippet: "未来",
      ruleHit: "vague-future",
    });
  });

  it("flags Chinese vague future 展望", () => {
    const hits = detectFutureReferences(
      "展望后市，机构看好。",
      "2026-06-01",
    );
    expect(hits).toContainEqual({
      snippet: "展望",
      ruleHit: "vague-future",
    });
  });
});

describe("detectFutureReferences — multi-stage combination", () => {
  it("captures all three stages in one text", () => {
    const text = "Expect 2026-07-01 rally tomorrow, soon.";
    const hits = detectFutureReferences(text, "2026-06-01");
    const rules = hits.map((h) => h.ruleHit);
    expect(rules).toContain("absolute-date");
    expect(rules).toContain("tense-phrase");
    expect(rules).toContain("vague-future");
  });

  it("returns empty on past-only text", () => {
    const hits = detectFutureReferences(
      "The earnings call on 2026-05-15 was a non-event.",
      "2026-06-01",
    );
    expect(hits).toEqual([]);
  });
});

describe("detectFutureReferencesForNode", () => {
  it("attaches the nodeId to each hit", () => {
    const hits = detectFutureReferencesForNode(
      "node-1",
      "Rally on 2027-01-01.",
      "2026-06-01",
    );
    expect(hits).toEqual([
      { nodeId: "node-1", snippet: "2027-01-01", ruleHit: "absolute-date" },
    ]);
  });
});
