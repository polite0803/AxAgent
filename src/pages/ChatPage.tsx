import { AgentExecutionPanel } from "@/components/chat/AgentExecutionPanel";
import { ChatSidebar } from "@/components/chat/ChatSidebar";
import type { ChatViewScrollApi } from "@/components/chat/ChatView";
import { ChatView } from "@/components/chat/ChatView";
import { ScrollToMessageProvider } from "@/components/chat/ScrollToMessageContext";
import { TabBar } from "@/components/chat/TabBar";
import { useConversationStore, useProviderStore, useSettingsStore, useTabStore } from "@/stores";
import { theme } from "antd";
import { ChevronRight, PanelRight } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

const SIDEBAR_MIN = 180;
const SIDEBAR_MAX = 480;
const SIDEBAR_DEFAULT = 256;

const RIGHT_PANEL_MIN = 220;
const RIGHT_PANEL_MAX = 560;
const RIGHT_PANEL_DEFAULT = 320;

export function ChatPage() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const [sidebarWidth, setSidebarWidth] = useState(SIDEBAR_DEFAULT);
  const [dragging, setDragging] = useState(false);
  const sidebarRef = useRef<HTMLDivElement>(null);
  const fetchConversations = useConversationStore((s) => s.fetchConversations);
  const conversationCount = useConversationStore((s) => s.conversations.length);
  const fetchProviders = useProviderStore((s) => s.fetchProviders);
  const providerCount = useProviderStore((s) => s.providers.length);

  // 右侧面板状态
  const settings = useSettingsStore((s) => s.settings);
  const saveSettings = useSettingsStore((s) => s.saveSettings);
  const agentPanelEnabled = settings.agent_panel_enabled !== false;
  const [rightPanelCollapsed, setRightPanelCollapsed] = useState(settings.agent_panel_compact === true);
  const [rightPanelWidth, setRightPanelWidth] = useState(RIGHT_PANEL_DEFAULT);
  const [rightDragging, setRightDragging] = useState(false);

  const toggleRightPanel = useCallback(() => {
    setRightPanelCollapsed((prev) => !prev);
  }, []);

  useEffect(() => {
    saveSettings({ agent_panel_compact: rightPanelCollapsed });
  }, [rightPanelCollapsed, saveSettings]);

  // ChatView 暴露的 scroll 能力，供右侧面板点击跳转消息使用
  const [scrollApi, setScrollApi] = useState<ChatViewScrollApi | null>(null);
  const handleScrollToReady = useCallback((api: ChatViewScrollApi) => {
    setScrollApi(api);
  }, []);

  // 左侧边栏拖拽调整宽度
  const handleLeftMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setDragging(true);
  }, []);

  useEffect(() => {
    if (!dragging) { return; }
    const prevUserSelect = document.body.style.userSelect;
    document.body.style.userSelect = "none";
    const handleMouseMove = (e: MouseEvent) => {
      setSidebarWidth(Math.min(SIDEBAR_MAX, Math.max(SIDEBAR_MIN, e.clientX)));
    };
    const handleMouseUp = () => setDragging(false);
    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);
    return () => {
      document.body.style.userSelect = prevUserSelect;
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
    };
  }, [dragging]);

  // 右侧面板拖拽调整宽度
  const handleRightMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setRightDragging(true);
  }, []);

  useEffect(() => {
    if (!rightDragging) { return; }
    const prevUserSelect = document.body.style.userSelect;
    document.body.style.userSelect = "none";
    const handleMouseMove = (e: MouseEvent) => {
      const newWidth = window.innerWidth - e.clientX;
      setRightPanelWidth(Math.min(RIGHT_PANEL_MAX, Math.max(RIGHT_PANEL_MIN, newWidth)));
    };
    const handleMouseUp = () => setRightDragging(false);
    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);
    return () => {
      document.body.style.userSelect = prevUserSelect;
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
    };
  }, [rightDragging]);

  const conversations = useConversationStore((s) => s.conversations);
  const activeConversationId = useConversationStore((s) => s.activeConversationId);
  const activeConversation = conversations.find((c) => c.id === activeConversationId);
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
      useTabStore.getState().setActiveTab(existingTab.id);
    }
  }, [activeConversationId]);

  // Handle new conversation from TabBar
  const handleNewConversation = useCallback(async () => {
    let provider = providers.find((p) => p.enabled && p.models.some((m) => m.enabled));
    let model = provider?.models.find((m) => m.enabled);
    if (!provider || !model) { return; }

    const conv = await createConversation(
      "",
      model.model_id,
      provider.id,
    );
    openTab(conv.id, conv.title);
  }, [providers, createConversation, openTab]);

  // 是否显示右侧面板（仅 agent 模式 + 设置启用）
  const showRightPanel = agentPanelEnabled
    && activeConversation?.mode === "agent"
    && activeConversationId != null;

  // 右侧面板内容（在 ScrollToMessageProvider 内部）
  const rightPanelContent = showRightPanel && scrollApi && activeConversationId
    ? (
      <ScrollToMessageProvider scrollTo={scrollApi.scrollTo} scrollBoxRef={scrollApi.scrollBoxRef}>
        <AgentExecutionPanel
          conversationId={activeConversationId}
          compactMode={rightPanelCollapsed}
          onToggleCompact={toggleRightPanel}
        />
      </ScrollToMessageProvider>
    )
    : null;

  return (
    <div className="flex h-full" style={{ overflow: "hidden" }} data-testid="chat-view">
      {/* 左侧会话列表 */}
      <div
        ref={sidebarRef}
        className="h-full transition-all duration-200"
        style={{
          width: sidebarCollapsed ? 48 : sidebarWidth,
          borderRight: "1px solid var(--border-color)",
          backgroundColor: token.colorBgContainer,
          flexShrink: 0,
          transition: dragging ? "none" : "width 0.2s",
        }}
      >
        <ChatSidebar onCollapseChange={setSidebarCollapsed} />
      </div>
      {/* 左侧拖拽手柄 */}
      {!sidebarCollapsed && (
        <div
          onMouseDown={handleLeftMouseDown}
          style={{
            width: 4,
            cursor: "col-resize",
            flexShrink: 0,
            backgroundColor: dragging ? "var(--color-primary)" : "transparent",
            transition: "background-color 0.15s",
            zIndex: 10,
          }}
          onMouseEnter={(e) => {
            (e.currentTarget as HTMLElement).style.backgroundColor = "var(--color-primary)";
          }}
          onMouseLeave={(e) => {
            if (!dragging) {
              (e.currentTarget as HTMLElement).style.backgroundColor = "transparent";
            }
          }}
        />
      )}
      {/* 中间主区域 */}
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
        <ChatView onScrollToReady={handleScrollToReady} />
      </div>
      {/* 右侧拖拽手柄 */}
      {showRightPanel && !rightPanelCollapsed && (
        <div
          onMouseDown={handleRightMouseDown}
          style={{
            width: 4,
            cursor: "col-resize",
            flexShrink: 0,
            backgroundColor: rightDragging ? "var(--color-primary)" : "transparent",
            transition: "background-color 0.15s",
            zIndex: 10,
          }}
          onMouseEnter={(e) => {
            (e.currentTarget as HTMLElement).style.backgroundColor = "var(--color-primary)";
          }}
          onMouseLeave={(e) => {
            if (!rightDragging) {
              (e.currentTarget as HTMLElement).style.backgroundColor = "transparent";
            }
          }}
        />
      )}
      {/* 右侧面板 */}
      {showRightPanel && (
        <div
          style={{
            width: rightPanelCollapsed ? 48 : rightPanelWidth,
            minWidth: 0,
            borderLeft: "1px solid var(--border-color)",
            backgroundColor: token.colorBgContainer,
            flexShrink: 0,
            display: "flex",
            flexDirection: "column",
            transition: rightDragging ? "none" : "width 0.2s",
            position: "relative",
          }}
        >
          {/* 折叠/展开按钮 — 始终在 ChatPage 层级，覆盖在面板左上角 */}
          <button
            type="button"
            onClick={toggleRightPanel}
            title={rightPanelCollapsed ? t("chat.agentPanel.expand") : t("chat.agentPanel.collapse")}
            style={{
              position: "absolute",
              top: 8,
              right: rightPanelCollapsed ? "50%" : 8,
              left: rightPanelCollapsed ? "50%" : undefined,
              transform: rightPanelCollapsed ? "translateX(50%)" : "none",
              zIndex: 5,
              width: 28,
              height: 28,
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              border: "none",
              borderRadius: 6,
              cursor: "pointer",
              backgroundColor: token.colorFillQuaternary,
              color: token.colorTextSecondary,
              padding: 0,
            }}
          >
            {rightPanelCollapsed ? <ChevronRight size={14} /> : <PanelRight size={14} />}
          </button>
          {/* 始终渲染 AgentExecutionPanel，由 compactMode 控制内部展示 */}
          {rightPanelContent || (
            <div
              style={{
                flex: 1,
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                color: token.colorTextQuaternary,
                fontSize: 12,
              }}
            >
              加载中…
            </div>
          )}
        </div>
      )}
    </div>
  );
}
