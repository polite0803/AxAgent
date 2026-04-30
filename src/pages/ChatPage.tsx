import { ChatSidebar } from "@/components/chat/ChatSidebar";
import { ChatView } from "@/components/chat/ChatView";
import { TabBar } from "@/components/chat/TabBar";
import { useConversationStore, useProviderStore, useTabStore } from "@/stores";
import { theme } from "antd";
import { useCallback, useEffect, useRef, useState } from "react";

export function ChatPage() {
  const { token } = theme.useToken();
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const fetchConversations = useConversationStore((s) => s.fetchConversations);
  const conversationCount = useConversationStore((s) => s.conversations.length);
  const fetchProviders = useProviderStore((s) => s.fetchProviders);
  const providerCount = useProviderStore((s) => s.providers.length);

  const conversations = useConversationStore((s) => s.conversations);
  const activeConversationId = useConversationStore((s) => s.activeConversationId);
  const setActiveConversation = useConversationStore((s) => s.setActiveConversation);
  const createConversation = useConversationStore((s) => s.createConversation);
  const providers = useProviderStore((s) => s.providers);

  const tabs = useTabStore((s) => s.tabs);
  const activeTabId = useTabStore((s) => s.activeTabId);
  const openTab = useTabStore((s) => s.openTab);
  const updateTabTitle = useTabStore((s) => s.updateTabTitle);
  const tabsInitializedRef = useRef(false);

  // Fetch initial data
  useEffect(() => {
    if (conversationCount === 0) {
      fetchConversations();
    }
    if (providerCount === 0) {
      fetchProviders();
    }
  }, [conversationCount, fetchConversations, fetchProviders, providerCount]);

  // Sync tab titles when conversation titles change
  useEffect(() => {
    for (const tab of tabs) {
      const conv = conversations.find((c) => c.id === tab.conversationId);
      if (conv && conv.title !== tab.title) {
        updateTabTitle(conv.id, conv.title);
      }
    }
  }, [conversations, tabs, updateTabTitle]);

  useEffect(() => {
    tabsInitializedRef.current = true;
  }, []);

  // When activeTabId changes, sync the activeConversationId
  useEffect(() => {
    if (!activeTabId) {
      if (tabsInitializedRef.current && activeConversationId) {
        // All tabs closed after initial load — clear active conversation.
        void setActiveConversation(null);
      }
      return;
    }
    const activeTab = tabs.find((t) => t.id === activeTabId);
    if (activeTab && activeTab.conversationId !== activeConversationId) {
      void setActiveConversation(activeTab.conversationId);
    }
  }, [activeTabId]);

  // When activeConversationId changes from outside (e.g. sidebar click, auto-select),
  // ensure a tab is open for it
  useEffect(() => {
    if (!activeConversationId) { return; }
    const existingTab = tabs.find((t) => t.conversationId === activeConversationId);
    if (!existingTab) {
      const conv = conversations.find((c) => c.id === activeConversationId);
      if (conv) {
        openTab(conv.id, conv.title);
      }
    } else if (existingTab.id !== activeTabId) {
      // The conversation is already in a tab but not the active one — activate it
      useTabStore.getState().setActiveTab(existingTab.id);
    }
  }, [activeConversationId]);

  // Handle new conversation from TabBar
  const handleNewConversation = useCallback(async () => {
    // Find a default provider/model
    let provider = providers.find((p) => p.enabled && p.models.some((m) => m.enabled));
    let model = provider?.models.find((m) => m.enabled);
    if (!provider || !model) { return; }

    const conv = await createConversation(
      "", // empty title — AI will generate later
      model.model_id,
      provider.id,
    );
    // Open a tab for the new conversation
    openTab(conv.id, conv.title);
  }, [providers, createConversation, openTab]);

  return (
    <div className="flex h-full" style={{ overflow: "hidden" }} data-testid="chat-view">
      <div
        className="h-full transition-all duration-200"
        style={{
          width: sidebarCollapsed ? "48px" : "256px",
          borderRight: "1px solid var(--border-color)",
          backgroundColor: token.colorBgContainer,
          flexShrink: 0,
        }}
      >
        <ChatSidebar onCollapseChange={setSidebarCollapsed} />
      </div>
      <div
        style={{
          flex: 1,
          display: "flex",
          flexDirection: "column",
          overflow: "hidden",
          backgroundColor: token.colorBgElevated,
        }}
      >
        <TabBar onNewConversation={handleNewConversation} />
        <ChatView />
      </div>
    </div>
  );
}
