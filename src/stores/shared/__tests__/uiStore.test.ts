// SPDX-License-Identifier: AGPL-3.0-only

import { beforeEach, describe, expect, it } from "vitest";

describe("uiStore", () => {
  describe("resolveDeviceLayout", () => {
    let resolveDeviceLayout: (width: number) => string;

    beforeEach(async () => {
      const mod = await import("../uiStore");
      resolveDeviceLayout = mod.resolveDeviceLayout;
    });

    it("width < 600 应返回 mobile", () => {
      expect(resolveDeviceLayout(599)).toBe("mobile");
      expect(resolveDeviceLayout(0)).toBe("mobile");
      expect(resolveDeviceLayout(300)).toBe("mobile");
    });

    it("600 <= width < 900 应返回 tablet", () => {
      expect(resolveDeviceLayout(600)).toBe("tablet");
      expect(resolveDeviceLayout(750)).toBe("tablet");
      expect(resolveDeviceLayout(899)).toBe("tablet");
    });

    it("width >= 900 应返回 desktop", () => {
      expect(resolveDeviceLayout(900)).toBe("desktop");
      expect(resolveDeviceLayout(1200)).toBe("desktop");
      expect(resolveDeviceLayout(1920)).toBe("desktop");
    });
  });

  describe("useUIStore", () => {
    let useUIStore: typeof import("../uiStore").useUIStore;

    beforeEach(async () => {
      const mod = await import("../uiStore");
      useUIStore = mod.useUIStore;
      useUIStore.setState({
        activePage: "chat",
        previousPage: "chat",
        sidebarCollapsed: true,
        settingsSection: "general",
        selectedProviderId: null,
        workflowEditorOpen: false,
        chartData: null,
        chartRawAnalysis: "",
        researchSources: [],
        report: null,
        selectedArtifactId: null,
        comparedMessageIds: null,
      });
    });

    describe("setActivePage", () => {
      it("应切换 activePage", () => {
        useUIStore.getState().setActivePage("dashboard");
        expect(useUIStore.getState().activePage).toBe("dashboard");
      });
    });

    describe("toggleSidebar", () => {
      it("应切换 sidebarCollapsed", () => {
        const before = useUIStore.getState().sidebarCollapsed;
        useUIStore.getState().toggleSidebar();
        expect(useUIStore.getState().sidebarCollapsed).toBe(!before);
      });
    });

    describe("enterSettings / exitSettings", () => {
      it("enterSettings 应保存 previousPage 并切换到 settings", () => {
        useUIStore.getState().setActivePage("dashboard");
        useUIStore.getState().enterSettings();
        expect(useUIStore.getState().activePage).toBe("settings");
        expect(useUIStore.getState().previousPage).toBe("dashboard");
      });

      it("已在 settings 时 enterSettings 不应变更 previousPage", () => {
        // 先建立基准：从 dashboard 进入 settings，记录 previousPage
        useUIStore.getState().setActivePage("dashboard");
        useUIStore.getState().enterSettings();
        const previousPageBefore = useUIStore.getState().previousPage;
        // 再次调用 enterSettings（当前已在 settings），previousPage 不应被覆盖
        useUIStore.getState().setActivePage("settings");
        useUIStore.getState().enterSettings();
        expect(useUIStore.getState().previousPage).toBe(previousPageBefore);
      });

      it("exitSettings 应恢复到 previousPage", () => {
        useUIStore.getState().setActivePage("dashboard");
        useUIStore.getState().enterSettings();
        useUIStore.getState().exitSettings();
        expect(useUIStore.getState().activePage).toBe("dashboard");
      });
    });

    describe("setSettingsSection", () => {
      it("应切换 settingsSection", () => {
        useUIStore.getState().setSettingsSection("general");
        expect(useUIStore.getState().settingsSection).toBe("general");
        useUIStore.getState().setSettingsSection("models");
        expect(useUIStore.getState().settingsSection).toBe("models");
      });
    });

    describe("setSelectedProviderId", () => {
      it("应设置和清除 providerId", () => {
        useUIStore.getState().setSelectedProviderId("provider-1");
        expect(useUIStore.getState().selectedProviderId).toBe("provider-1");
        useUIStore.getState().setSelectedProviderId(null);
        expect(useUIStore.getState().selectedProviderId).toBeNull();
      });
    });

    describe("workflowEditor", () => {
      it("openWorkflowEditor 应打开编辑器并进入 settings", () => {
        useUIStore.getState().setActivePage("chat");
        useUIStore.getState().openWorkflowEditor();
        expect(useUIStore.getState().workflowEditorOpen).toBe(true);
        expect(useUIStore.getState().settingsSection).toBe("workflow");
        expect(useUIStore.getState().activePage).toBe("settings");
      });

      it("closeWorkflowEditor 应关闭编辑器", () => {
        useUIStore.getState().openWorkflowEditor();
        useUIStore.getState().closeWorkflowEditor();
        expect(useUIStore.getState().workflowEditorOpen).toBe(false);
      });
    });

    describe("chartResult", () => {
      it("应设置 chartData 和 chartRawAnalysis", () => {
        useUIStore.getState().setChartResult({ type: "bar" } as any, "分析文本");
        expect(useUIStore.getState().chartData).toEqual({ type: "bar" });
        expect(useUIStore.getState().chartRawAnalysis).toBe("分析文本");
      });
    });

    describe("researchSources", () => {
      it("应设置 researchSources", () => {
        useUIStore.getState().setResearchSources([
          {
            id: "1",
            sourceType: "web",
            url: "https://example.com",
            title: "Example",
            snippet: "...",
            credibilityScore: 0.8,
            relevanceScore: 0.9,
          },
        ]);
        expect(useUIStore.getState().researchSources.length).toBe(1);
        expect(useUIStore.getState().researchSources[0].title).toBe("Example");
      });
    });

    describe("report", () => {
      it("应设置和清除 report", () => {
        const report = { id: "r1", topic: "Test", content: "...", citations: [], summary: "summary" };
        useUIStore.getState().setReport(report);
        expect(useUIStore.getState().report).toEqual(report);
        useUIStore.getState().setReport(null);
        expect(useUIStore.getState().report).toBeNull();
      });
    });

    describe("artifact & compare", () => {
      it("selectArtifact 应设置 artifact id", () => {
        useUIStore.getState().selectArtifact("artifact-1");
        expect(useUIStore.getState().selectedArtifactId).toBe("artifact-1");
        useUIStore.getState().selectArtifact(null);
        expect(useUIStore.getState().selectedArtifactId).toBeNull();
      });

      it("startCompare / clearCompare 应管理对比状态", () => {
        useUIStore.getState().startCompare(["msg-1", "msg-2"]);
        expect(useUIStore.getState().comparedMessageIds).toEqual(["msg-1", "msg-2"]);
        useUIStore.getState().clearCompare();
        expect(useUIStore.getState().comparedMessageIds).toBeNull();
      });
    });
  });
});
