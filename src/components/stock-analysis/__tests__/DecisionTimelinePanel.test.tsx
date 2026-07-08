import { render } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string, fallback?: string) => fallback ?? key }),
  initReactI18next: { type: "3rdParty", init: () => {} },
}));

vi.mock("@/lib/invoke", () => ({
  invoke: vi.fn().mockResolvedValue([]),
  listen: vi.fn().mockResolvedValue(() => {}),
  isTauri: () => false,
}));

import { useStockAnalysisStore } from "@/stores/feature/stockAnalysisStore";
import { DecisionTimelinePanel } from "../DecisionTimelinePanel";

const renderWithRouter = (ui: React.ReactNode) => render(<MemoryRouter>{ui}</MemoryRouter>);

describe("DecisionTimelinePanel", () => {
  it("idle 状态显示 empty hint", () => {
    useStockAnalysisStore.setState({ status: "idle", timeline: [] });
    const { container } = renderWithRouter(<DecisionTimelinePanel />);
    expect(container.textContent).toContain("stockAnalysis.timeline.idleHint");
  });

  it("loading 但无 timeline 时显示等待提示", () => {
    useStockAnalysisStore.setState({ status: "loading", timeline: [] });
    const { container } = renderWithRouter(<DecisionTimelinePanel />);
    expect(container.textContent).toContain("stockAnalysis.timeline.emptyHint");
  });

  it("有 timeline 时渲染 4 个 phase section", () => {
    useStockAnalysisStore.setState({
      status: "completed",
      timeline: [
        {
          id: "t-news",
          phase: "scan",
          agentId: "t-news",
          agentName: "News",
          title: "News",
          summary: "x",
          confidence: 0.5,
          status: "done",
          evidenceRefs: [],
        },
        {
          id: "a-tech",
          phase: "diagnose",
          agentId: "a-tech",
          agentName: "Tech",
          title: "Tech",
          summary: "y",
          confidence: 0.6,
          status: "done",
          evidenceRefs: [],
        },
        {
          id: "bull-r1",
          phase: "debate",
          agentId: "bull-r1",
          agentName: "Bull R1",
          title: "Bull R1",
          summary: "z",
          confidence: 0.7,
          status: "done",
          evidenceRefs: [],
        },
        {
          id: "trader",
          phase: "decide",
          agentId: "trader",
          agentName: "Trader",
          title: "Trader",
          summary: "w",
          confidence: 0.8,
          status: "done",
          evidenceRefs: [],
        },
      ],
    });
    const { container } = renderWithRouter(<DecisionTimelinePanel />);
    const phaseButtons = container.querySelectorAll("button[type='button']");
    const phaseLabels = Array.from(phaseButtons).map((b) => b.textContent ?? "");
    expect(phaseLabels.some((t) => t.includes("phase.scan"))).toBe(true);
    expect(phaseLabels.some((t) => t.includes("phase.diagnose"))).toBe(true);
    expect(phaseLabels.some((t) => t.includes("phase.debate"))).toBe(true);
    expect(phaseLabels.some((t) => t.includes("phase.decide"))).toBe(true);
  });

  it("failed 节点使用红色边框标识", () => {
    useStockAnalysisStore.setState({
      status: "completed",
      timeline: [
        {
          id: "a-x",
          phase: "diagnose",
          agentId: "a-x",
          agentName: "X",
          title: "X",
          summary: "failed",
          confidence: 0,
          status: "failed",
          evidenceRefs: [],
        },
      ],
    });
    const { container } = renderWithRouter(<DecisionTimelinePanel />);
    expect(container.innerHTML).toContain("var(--sa-red)");
  });
});
