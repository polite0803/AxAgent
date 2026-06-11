// SPDX-License-Identifier: AGPL-3.0-only

import { useStreamStore } from "@/stores/domain/streamStore";
import { useAgentStore } from "@/stores/feature/agentStore";
import { useExecutionStore } from "@/stores/feature/executionStore";
import { create } from "zustand";

/** A single tab entry */
export interface TabItem {
  /** Unique tab ID (crypto.randomUUID) */
  id: string;
  /** The conversation ID this tab is bound to */
  conversationId: string;
  /** Display title (synced from conversation title) */
  title: string;
}

interface TabState {
  /** All open tabs, in order */
  tabs: TabItem[];
  /** ID of the currently active tab */
  activeTabId: string | null;

  // ── Actions ──────────────────────────────────────────────────────────────

  /** Open a new tab for the given conversation (or focus existing tab if already open) */
  openTab: (conversationId: string, title: string) => string;
  /** Close a tab by tab ID; returns the tab that should become active (or null) */
  closeTab: (tabId: string) => void;
  /** Switch to a specific tab */
  setActiveTab: (tabId: string) => void;
  /** Update a tab's title (e.g. when conversation title changes) */
  updateTabTitle: (conversationId: string, title: string) => void;
  /** Remove all tabs referencing a deleted conversation */
  removeTabsByConversationId: (conversationId: string) => void;
  /** Move a tab from one position to another */
  moveTab: (fromIndex: number, toIndex: number) => void;
  /** Get the active tab's conversation ID */
  getActiveConversationId: () => string | null;
  /** Close all tabs except the specified one */
  closeOtherTabs: (tabId: string) => void;
  /** Close all tabs to the right of the specified one */
  closeTabsToRight: (tabId: string) => void;
}

/** Clean up domain/feature store state for the given conversation */
function cleanupConversationState(conversationId: string) {
  const { activeStreams } = useStreamStore.getState();
  if (conversationId in activeStreams) {
    useStreamStore.getState().cancelCurrentStream(conversationId);
  }
  useAgentStore.getState().clearConversation(conversationId);
  useExecutionStore.getState().clearConversation(conversationId);
}

export const useTabStore = create<TabState>((set, get) => ({
  tabs: [],
  activeTabId: null,

  openTab: (conversationId, title) => {
    const { tabs } = get();
    // If a tab for this conversation already exists, just activate it
    const existing = tabs.find((t) => t.conversationId === conversationId);
    if (existing) {
      set({ activeTabId: existing.id });
      return existing.id;
    }
    // Create a new tab
    const newTab: TabItem = {
      id: crypto.randomUUID(),
      conversationId,
      title,
    };
    const nextTabs = [...tabs, newTab];
    set({ tabs: nextTabs, activeTabId: newTab.id });
    return newTab.id;
  },

  closeTab: (tabId) => {
    const { tabs, activeTabId } = get();
    const idx = tabs.findIndex((t) => t.id === tabId);
    if (idx === -1) {
      return;
    }

    const closedTab = tabs[idx];
    const nextTabs = tabs.filter((t) => t.id !== tabId);

    // Only clean up conversation state if no other tabs reference the same conversation
    const hasOtherTabsForConversation = nextTabs.some(
      (t) => t.conversationId === closedTab.conversationId,
    );
    if (!hasOtherTabsForConversation) {
      cleanupConversationState(closedTab.conversationId);
    }

    // If we're closing the active tab, activate an adjacent one
    let nextActiveId = activeTabId;
    if (activeTabId === tabId) {
      if (nextTabs.length === 0) {
        nextActiveId = null;
      } else {
        // Prefer the tab to the right, then left
        const adjacentIdx = Math.min(idx, nextTabs.length - 1);
        nextActiveId = nextTabs[adjacentIdx]?.id ?? null;
      }
    }

    set({ tabs: nextTabs, activeTabId: nextActiveId });
  },

  setActiveTab: (tabId) => {
    set({ activeTabId: tabId });
  },

  updateTabTitle: (conversationId, title) => {
    set((s) => ({
      tabs: s.tabs.map((t) => t.conversationId === conversationId ? { ...t, title } : t),
    }));
  },

  removeTabsByConversationId: (conversationId) => {
    const { tabs, activeTabId } = get();
    const removedTabIds = new Set(
      tabs.flatMap((t) => (t.conversationId === conversationId ? [t.id] : [])),
    );
    if (removedTabIds.size === 0) {
      return;
    }

    const nextTabs = tabs.filter((t) => !removedTabIds.has(t.id));
    let nextActiveId = activeTabId;
    if (activeTabId && removedTabIds.has(activeTabId)) {
      // Find the nearest remaining tab
      const closedIdx = tabs.findIndex((t) => t.id === activeTabId);
      if (nextTabs.length === 0) {
        nextActiveId = null;
      } else {
        const adjacentIdx = Math.min(closedIdx, nextTabs.length - 1);
        nextActiveId = nextTabs[adjacentIdx]?.id ?? null;
      }
    }
    set({ tabs: nextTabs, activeTabId: nextActiveId });
  },

  moveTab: (fromIndex, toIndex) => {
    set((s) => {
      const tabs = [...s.tabs];
      const [moved] = tabs.splice(fromIndex, 1);
      tabs.splice(toIndex, 0, moved);
      return { tabs };
    });
  },

  getActiveConversationId: () => {
    const { tabs, activeTabId } = get();
    if (!activeTabId) {
      return null;
    }
    return tabs.find((t) => t.id === activeTabId)?.conversationId ?? null;
  },

  closeOtherTabs: (tabId) => {
    const { tabs } = get();
    const target = tabs.find((t) => t.id === tabId);
    if (!target) {
      return;
    }
    const removedConvIds = new Set(
      tabs
        .filter((t) => t.id !== tabId)
        .map((t) => t.conversationId),
    );
    const keptConvIds = new Set([target.conversationId]);
    for (const convId of removedConvIds) {
      if (!keptConvIds.has(convId)) {
        cleanupConversationState(convId);
      }
    }
    set({ tabs: [target], activeTabId: tabId });
  },

  closeTabsToRight: (tabId) => {
    const { tabs } = get();
    const idx = tabs.findIndex((t) => t.id === tabId);
    if (idx === -1) {
      return;
    }
    const removed = tabs.slice(idx + 1);
    const keptConvIds = new Set(tabs.slice(0, idx + 1).map((t) => t.conversationId));
    for (const tab of removed) {
      if (!keptConvIds.has(tab.conversationId)) {
        cleanupConversationState(tab.conversationId);
      }
    }
    set({ tabs: tabs.slice(0, idx + 1) });
  },
}));
