// SPDX-License-Identifier: AGPL-3.0-only

import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@/stores/domain/streamStore", () => ({
  useStreamStore: { getState: () => ({ activeStreams: {}, cancelCurrentStream: vi.fn() }) },
}));
vi.mock("@/stores/feature/agentStore", () => ({
  useAgentStore: { getState: () => ({ clearConversation: vi.fn() }) },
}));
vi.mock("@/stores/feature/executionStore", () => ({
  useExecutionStore: { getState: () => ({ clearConversation: vi.fn() }) },
}));

vi.mock("crypto", () => {
  let counter = 0;
  return {
    randomUUID: () => `mock-uuid-${++counter}`,
  };
});

describe("tabStore", () => {
  let useTabStore: typeof import("../tabStore").useTabStore;

  beforeEach(async () => {
    vi.stubGlobal("crypto", {
      randomUUID: vi
        .fn()
        .mockReturnValueOnce("tab-1")
        .mockReturnValueOnce("tab-2")
        .mockReturnValueOnce("tab-3")
        .mockReturnValueOnce("tab-4")
        .mockReturnValueOnce("tab-5"),
    });
    const mod = await import("../tabStore");
    useTabStore = mod.useTabStore;
    useTabStore.setState({ tabs: [], activeTabId: null });
  });

  describe("openTab", () => {
    it("应创建新 tab", () => {
      const id = useTabStore.getState().openTab("conv-1", "标题1");
      const { tabs, activeTabId } = useTabStore.getState();
      expect(tabs.length).toBe(1);
      expect(tabs[0].conversationId).toBe("conv-1");
      expect(tabs[0].title).toBe("标题1");
      expect(activeTabId).toBe(id);
    });

    it("已存在的 conversation 应聚焦而非新建", () => {
      const id1 = useTabStore.getState().openTab("conv-1", "标题1");
      const id2 = useTabStore.getState().openTab("conv-1", "标题1-改");
      const { tabs, activeTabId } = useTabStore.getState();
      expect(tabs.length).toBe(1);
      expect(activeTabId).toBe(id1);
      expect(id2).toBe(id1);
    });

    it("多个不同 conversation 应创建多个 tab", () => {
      useTabStore.getState().openTab("conv-1", "A");
      useTabStore.getState().openTab("conv-2", "B");
      useTabStore.getState().openTab("conv-3", "C");
      expect(useTabStore.getState().tabs.length).toBe(3);
    });
  });

  describe("closeTab", () => {
    it("应移除 tab", () => {
      const id = useTabStore.getState().openTab("conv-1", "A");
      useTabStore.getState().closeTab(id);
      expect(useTabStore.getState().tabs.length).toBe(0);
    });

    it("关闭不存在的 tab 应不抛错", () => {
      expect(() => useTabStore.getState().closeTab("nonexistent")).not.toThrow();
    });

    it("关闭 activeTab 后应激活相邻 tab", () => {
      useTabStore.getState().openTab("conv-1", "A");
      const id2 = useTabStore.getState().openTab("conv-2", "B");
      useTabStore.getState().openTab("conv-3", "C");
      useTabStore.getState().closeTab(id2);
      const { tabs, activeTabId } = useTabStore.getState();
      expect(tabs.length).toBe(2);
      expect(activeTabId).not.toBeNull();
    });

    it("关闭最后一个 tab 后 activeTabId 应为 null", () => {
      const id = useTabStore.getState().openTab("conv-1", "A");
      useTabStore.getState().closeTab(id);
      expect(useTabStore.getState().activeTabId).toBeNull();
    });
  });

  describe("setActiveTab", () => {
    it("应切换 activeTabId", () => {
      const id1 = useTabStore.getState().openTab("conv-1", "A");
      useTabStore.getState().openTab("conv-2", "B");
      useTabStore.getState().setActiveTab(id1);
      expect(useTabStore.getState().activeTabId).toBe(id1);
    });
  });

  describe("updateTabTitle", () => {
    it("应更新标题", () => {
      useTabStore.getState().openTab("conv-1", "旧标题");
      useTabStore.getState().updateTabTitle("conv-1", "新标题");
      const tab = useTabStore.getState().tabs[0];
      expect(tab.title).toBe("新标题");
    });

    it("不匹配的 conversationId 不应变更", () => {
      useTabStore.getState().openTab("conv-1", "A");
      useTabStore.getState().openTab("conv-2", "B");
      useTabStore.getState().updateTabTitle("conv-1", "新的A");
      const tabs = useTabStore.getState().tabs;
      expect(tabs[0].title).toBe("新的A");
      expect(tabs[1].title).toBe("B");
    });
  });

  describe("removeTabsByConversationId", () => {
    it("应移除指定 conversation 的所有 tab", () => {
      useTabStore.getState().openTab("conv-1", "A");
      useTabStore.getState().openTab("conv-2", "B");
      useTabStore.getState().removeTabsByConversationId("conv-1");
      const { tabs } = useTabStore.getState();
      expect(tabs.length).toBe(1);
      expect(tabs[0].conversationId).toBe("conv-2");
    });

    it("不存在的 conversationId 应不抛错", () => {
      useTabStore.getState().openTab("conv-1", "A");
      expect(() => useTabStore.getState().removeTabsByConversationId("nonexistent")).not.toThrow();
    });
  });

  describe("moveTab", () => {
    it("应移动 tab 位置", () => {
      useTabStore.getState().openTab("conv-1", "A");
      useTabStore.getState().openTab("conv-2", "B");
      useTabStore.getState().openTab("conv-3", "C");
      useTabStore.getState().moveTab(0, 2);
      const { tabs } = useTabStore.getState();
      expect(tabs[0].conversationId).toBe("conv-2");
      expect(tabs[1].conversationId).toBe("conv-3");
      expect(tabs[2].conversationId).toBe("conv-1");
    });
  });

  describe("getActiveConversationId", () => {
    it("无 activeTabId 时应返回 null", () => {
      expect(useTabStore.getState().getActiveConversationId()).toBeNull();
    });

    it("应返回当前 tab 的 conversationId", () => {
      useTabStore.getState().openTab("conv-1", "A");
      expect(useTabStore.getState().getActiveConversationId()).toBe("conv-1");
    });
  });

  describe("closeOtherTabs", () => {
    it("应关闭除指定 tab 外的所有 tab", () => {
      useTabStore.getState().openTab("conv-1", "A");
      const id2 = useTabStore.getState().openTab("conv-2", "B");
      useTabStore.getState().openTab("conv-3", "C");
      useTabStore.getState().closeOtherTabs(id2);
      const { tabs, activeTabId } = useTabStore.getState();
      expect(tabs.length).toBe(1);
      expect(tabs[0].id).toBe(id2);
      expect(activeTabId).toBe(id2);
    });

    it("不存在的 tabId 应不抛错", () => {
      expect(() => useTabStore.getState().closeOtherTabs("nonexistent")).not.toThrow();
    });
  });

  describe("closeTabsToRight", () => {
    it("应关闭右侧所有 tab", () => {
      useTabStore.getState().openTab("conv-1", "A");
      const id2 = useTabStore.getState().openTab("conv-2", "B");
      useTabStore.getState().openTab("conv-3", "C");
      useTabStore.getState().closeTabsToRight(id2);
      const { tabs } = useTabStore.getState();
      expect(tabs.length).toBe(2);
      expect(tabs[0].conversationId).toBe("conv-1");
      expect(tabs[1].conversationId).toBe("conv-2");
    });

    it("最右侧 tab 关闭右侧应不变", () => {
      useTabStore.getState().openTab("conv-1", "A");
      const id2 = useTabStore.getState().openTab("conv-2", "B");
      useTabStore.getState().closeTabsToRight(id2);
      expect(useTabStore.getState().tabs.length).toBe(2);
    });
  });
});
