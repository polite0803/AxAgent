import i18n from "@/i18n";
import { useStockAnalysisStore } from "@/stores/feature/stockAnalysisStore";
import type { TimelineNode } from "@/types/stock-analysis";
import { act, fireEvent, render, screen } from "@testing-library/react";
import { I18nextProvider } from "react-i18next";
import { MemoryRouter } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { TimelineNodeCard } from "../TimelineNodeCard";

function makeNode(overrides: Partial<TimelineNode> = {}): TimelineNode {
  return {
    id: "node-1",
    phase: "scan",
    agentId: "a1",
    agentName: "Scanner",
    title: "Screened universe",
    summary: "Identified 42 candidates as of 2026-06-01.",
    confidence: 0.8,
    status: "done",
    evidenceRefs: [],
    ...overrides,
  };
}

function renderWithI18n(ui: React.ReactNode) {
  return render(
    <MemoryRouter>
      <I18nextProvider i18n={i18n}>{ui}</I18nextProvider>
    </MemoryRouter>,
  );
}

beforeEach(() => {
  useStockAnalysisStore.setState({ violations: [] });
});

afterEach(() => {
  useStockAnalysisStore.setState({ violations: [] });
});

describe("TimelineNodeCard — violations chip", () => {
  it("does not render a violation chip when there are no violations", () => {
    renderWithI18n(<TimelineNodeCard node={makeNode()} />);
    expect(screen.queryByTestId("violation-chip")).toBeNull();
  });

  it("renders a single violation chip with count 1", () => {
    useStockAnalysisStore.setState({
      violations: [
        { nodeId: "node-1", snippet: "2026-07-01", ruleHit: "absolute-date" },
      ],
    });
    renderWithI18n(<TimelineNodeCard node={makeNode()} />);
    const chip = screen.getByTestId("violation-chip");
    expect(chip).toBeInTheDocument();
    expect(chip.textContent).toMatch(/1/);
  });

  it("renders chip with count > 1", () => {
    useStockAnalysisStore.setState({
      violations: [
        { nodeId: "node-1", snippet: "2026-07-01", ruleHit: "absolute-date" },
        { nodeId: "node-1", snippet: "tomorrow", ruleHit: "tense-phrase" },
        { nodeId: "node-1", snippet: "next quarter", ruleHit: "tense-phrase" },
      ],
    });
    renderWithI18n(<TimelineNodeCard node={makeNode()} />);
    const chip = screen.getByTestId("violation-chip");
    expect(chip.textContent).toMatch(/3/);
  });

  it("ignores violations for other nodes", () => {
    useStockAnalysisStore.setState({
      violations: [
        { nodeId: "node-other", snippet: "2026-07-01", ruleHit: "absolute-date" },
      ],
    });
    renderWithI18n(<TimelineNodeCard node={makeNode()} />);
    expect(screen.queryByTestId("violation-chip")).toBeNull();
  });
});

describe("TimelineNodeCard — highlight violations in expanded summary", () => {
  it("wraps violation snippets in <mark> when expanded", async () => {
    useStockAnalysisStore.setState({
      violations: [
        { nodeId: "node-1", snippet: "2026-07-01", ruleHit: "absolute-date" },
      ],
    });
    renderWithI18n(
      <TimelineNodeCard
        node={makeNode({ summary: "Mentions 2026-07-01 explicitly." })}
      />,
    );
    // 展开节点
    const toggle = screen.getByRole("button", { name: /Screened universe/ });
    act(() => {
      fireEvent.click(toggle);
    });
    const marks = document.querySelectorAll("mark.ax-violation-mark");
    expect(marks.length).toBe(1);
    expect(marks[0].textContent).toBe("2026-07-01");
  });

  it("highlights multiple distinct snippets independently", async () => {
    useStockAnalysisStore.setState({
      violations: [
        { nodeId: "node-1", snippet: "2026-07-01", ruleHit: "absolute-date" },
        { nodeId: "node-1", snippet: "tomorrow", ruleHit: "tense-phrase" },
      ],
    });
    renderWithI18n(
      <TimelineNodeCard
        node={makeNode({
          summary: "The market will rally 2026-07-01 and tomorrow too.",
        })}
      />,
    );
    const toggle = screen.getByRole("button", { name: /Screened universe/ });
    act(() => {
      fireEvent.click(toggle);
    });
    const marks = document.querySelectorAll("mark.ax-violation-mark");
    expect(marks.length).toBe(2);
    const texts = Array.from(marks).map((m) => m.textContent);
    expect(texts).toContain("2026-07-01");
    expect(texts).toContain("tomorrow");
  });

  it("does not highlight when summary is absent", () => {
    useStockAnalysisStore.setState({
      violations: [
        { nodeId: "node-1", snippet: "2026-07-01", ruleHit: "absolute-date" },
      ],
    });
    renderWithI18n(<TimelineNodeCard node={makeNode({ summary: "" })} />);
    const toggle = screen.getByRole("button", { name: /Screened universe/ });
    act(() => {
      fireEvent.click(toggle);
    });
    const marks = document.querySelectorAll("mark.ax-violation-mark");
    expect(marks.length).toBe(0);
  });

  it("renders the violation snippets list with rule labels", () => {
    useStockAnalysisStore.setState({
      violations: [
        { nodeId: "node-1", snippet: "2026-07-01", ruleHit: "absolute-date" },
        { nodeId: "node-1", snippet: "tomorrow", ruleHit: "tense-phrase" },
      ],
    });
    renderWithI18n(<TimelineNodeCard node={makeNode()} />);
    const toggle = screen.getByRole("button", { name: /Screened universe/ });
    act(() => {
      fireEvent.click(toggle);
    });
    const list = screen.getByTestId("violation-snippets");
    expect(list.textContent).toContain("2026-07-01");
    expect(list.textContent).toContain("absolute-date");
    expect(list.textContent).toContain("tomorrow");
    expect(list.textContent).toContain("tense-phrase");
  });

  it("ignores duplicate snippets (Set dedup)", () => {
    useStockAnalysisStore.setState({
      violations: [
        { nodeId: "node-1", snippet: "2026-07-01", ruleHit: "absolute-date" },
        { nodeId: "node-1", snippet: "2026-07-01", ruleHit: "absolute-date" },
      ],
    });
    renderWithI18n(
      <TimelineNodeCard
        node={makeNode({ summary: "Mentions 2026-07-01 once and 2026-07-01 again." })}
      />,
    );
    const toggle = screen.getByRole("button", { name: /Screened universe/ });
    act(() => {
      fireEvent.click(toggle);
    });
    // 实际匹配次数取决于 summary 中出现次数（这里是 2 次），但去重后只 1 个 snippet 用于高亮
    const marks = document.querySelectorAll("mark.ax-violation-mark");
    // 两处 2026-07-01 都应被同一片段高亮
    expect(marks.length).toBe(2);
    Array.from(marks).forEach((m) => expect(m.textContent).toBe("2026-07-01"));
  });
});
