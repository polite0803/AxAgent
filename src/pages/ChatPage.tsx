import { ChatSidebar } from "@/components/chat/ChatSidebar";
import type { ChatViewScrollApi } from "@/components/chat/ChatView";
import { ChatView } from "@/components/chat/ChatView";
import { RightPanelContainer } from "@/components/chat/RightPanelContainer";
import { ScrollToMessageProvider } from "@/components/chat/ScrollToMessageContext";
import { useSkillChatCommands } from "@/components/skill/SkillChatCommands";
import { useConversationStore, useProviderStore, useSettingsStore, useTabStore, useUIStore } from "@/stores";
import { theme } from "antd";
import { ChevronLeft, ChevronRight, PanelRight } from "lucide-react";
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
  const deviceLayout = useUIStore((s) => s.deviceLayout);
  const isMobile = deviceLayout === "mobile";
  const isTablet = deviceLayout === "tablet";
  const isSmallScreen = isMobile || isTablet;

  // 右侧面板状态
  const settings = useSettingsStore((s) => s.settings);
  const saveSettings = useSettingsStore((s) => s.saveSettings);
  const agentPanelEnabled = settings.agent_panel_enabled !== false;
  const [rightPanelCollapsed, setRightPanelCollapsed] = useState(
    settings.agent_panel_compact === true,
  );
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
    if (!dragging) {
      return;
    }
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
    if (!rightDragging) {
      return;
    }
    const prevUserSelect = document.body.style.userSelect;
    document.body.style.userSelect = "none";
    const handleMouseMove = (e: MouseEvent) => {
      const newWidth = window.innerWidth - e.clientX;
      setRightPanelWidth(
        Math.min(RIGHT_PANEL_MAX, Math.max(RIGHT_PANEL_MIN, newWidth)),
      );
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

  const skillCommands = useSkillChatCommands(); // slash-command suggestions for chat input
  void skillCommands; // consumed by chat slash-command input
  const conversations = useConversationStore((s) => s.conversations);
  const activeConversationId = useConversationStore(
    (s) => s.activeConversationId,
  );
  const activeConversation = conversations.find(
    (c) => c.id === activeConversationId,
  );
  const setActiveConversation = useConversationStore(
    (s) => s.setActiveConversation,
  );
  const tabs = useTabStore((s) => s.tabs);
  const activeTabId = useTabStore((s) => s.activeTabId);
  const openTab = useTabStore((s) => s.openTab);
  const closeTab = useTabStore((s) => s.closeTab);
  const updateTabTitle = useTabStore((s) => s.updateTabTitle);
  const tabsInitializedRef = useRef(false);

  // 初始数据加载
  useEffect(() => {
    if (conversationCount === 0) {
      fetchConversations();
    }
    if (providerCount === 0) {
      fetchProviders();
    }
  }, [conversationCount, fetchConversations, fetchProviders, providerCount]);

  // 同步标签标题
  useEffect(() => {
    const convMap = new Map(conversations.map((c) => [c.id, c]));
    for (const tab of tabs) {
      const conv = convMap.get(tab.conversationId);
      if (conv && conv.title !== tab.title) {
        updateTabTitle(conv.id, conv.title);
      }
    }
  }, [conversations, tabs, updateTabTitle]);

  useEffect(() => {
    tabsInitializedRef.current = true;
  }, []);

  // 当 activeTabId 变化时，同步 activeConversationId
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

  // 当 activeConversationId 从外部变化时（如侧边栏点击），确保有对应 tab
  useEffect(() => {
    if (!activeConversationId) {
      // 会话被清空（如删除当前会话）→ 关闭当前 tab
      if (tabsInitializedRef.current && activeTabId) {
        closeTab(activeTabId);
      }
      return;
    }
    const existingTab = tabs.find(
      (t) => t.conversationId === activeConversationId,
    );
    if (!existingTab) {
      const conv = conversations.find((c) => c.id === activeConversationId);
      if (conv) {
        openTab(conv.id, conv.title);
      }
    } else if (existingTab.id !== activeTabId) {
      useTabStore.getState().setActiveTab(existingTab.id);
    }
  }, [activeConversationId]);

  // 是否显示右侧面板（仅 agent 模式 + 设置启用）
  const showRightPanel = agentPanelEnabled
    && activeConversation?.mode === "agent"
    && activeConversationId != null;

  // 右侧面板内容（在 ScrollToMessageProvider 内部）
  const rightPanelContent = showRightPanel && scrollApi && activeConversationId
    ? (
      <ScrollToMessageProvider
        scrollTo={scrollApi.scrollTo}
        scrollBoxRef={scrollApi.scrollBoxRef}
      >
        <RightPanelContainer
          conversationId={activeConversationId}
          compactMode={rightPanelCollapsed}
          onToggleCompact={toggleRightPanel}
        />
      </ScrollToMessageProvider>
    )
    : null;

  return (
    <div
      className="chat-layout"
      style={{ position: isSmallScreen ? "relative" : "static" }}
      data-testid="chat-view"
    >
      {/* 左侧会话列表 */}
      <div
        ref={sidebarRef}
        className="ax-chat-sidebar"
        style={{
          width: sidebarCollapsed ? (isMobile ? 0 : 48) : (isMobile ? 280 : sidebarWidth),
          transition: dragging ? "none" : "width 0.2s",
          backgroundColor: token.colorBgContainer,
          flexShrink: 0,
          overflow: "hidden",
          ...(isMobile ? { position: "absolute", zIndex: 50, height: "100%" } : {}),
        }}
      >
        {(!sidebarCollapsed || !isMobile) && <ChatSidebar onCollapseChange={setSidebarCollapsed} />}
      </div>
      {/* 移动端：折叠时显示浮动切换按钮 */}
      {isMobile && sidebarCollapsed && (
        <button
          type="button"
          onClick={() => setSidebarCollapsed(false)}
          style={{
            position: "absolute",
            left: 8,
            top: 52,
            zIndex: 51,
            width: 32,
            height: 32,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            border: `1px solid ${token.colorBorderSecondary}`,
            borderRadius: 8,
            backgroundColor: token.colorBgElevated,
            color: token.colorTextSecondary,
            cursor: "pointer",
            boxShadow: "0 2px 8px rgba(0,0,0,0.1)",
          }}
          aria-label={t("sidebar.expand")}
        >
          <PanelRight size={16} />
        </button>
      )}
      {/* 左侧拖拽手柄 — 移动端无拖拽 */}
      {!sidebarCollapsed && !isMobile && (
        <div
          onMouseDown={handleLeftMouseDown}
          role="separator"
          aria-label="resize handle"
          tabIndex={0}
          onKeyDown={(e) => {
            if (e.key === "Enter" || e.key === " ") {
              e.preventDefault();
            }
          }}
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
      <div className="chat-main" style={{ backgroundColor: token.colorBgElevated }}>
        <ChatView onScrollToReady={handleScrollToReady} />
      </div>
      {/* 右侧拖拽手柄 */}
      {showRightPanel && !rightPanelCollapsed && (
        <div
          onMouseDown={handleRightMouseDown}
          role="separator"
          aria-label="resize handle"
          tabIndex={0}
          onKeyDown={(e) => {
            if (e.key === "Enter" || e.key === " ") {
              e.preventDefault();
            }
          }}
          style={{
            width: 4,
            cursor: "col-resize",
            flexShrink: 0,
            backgroundColor: rightDragging
              ? "var(--color-primary)"
              : "transparent",
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
      {/* 小屏：右侧面板覆盖层遮罩 */}
      {isSmallScreen && showRightPanel && !rightPanelCollapsed && (
        <div
          onClick={toggleRightPanel}
          style={{
            position: "absolute",
            inset: 0,
            zIndex: 39,
            backgroundColor: token.colorBgMask,
            transition: "opacity 0.2s",
          }}
        />
      )}
      {/* 小屏：展开按钮 */}
      {isSmallScreen && showRightPanel && rightPanelCollapsed && (
        <button
          type="button"
          onClick={toggleRightPanel}
          style={{
            position: "absolute",
            right: 8,
            top: "50%",
            transform: "translateY(-50%)",
            zIndex: 30,
            width: 32,
            height: 32,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            border: `1px solid ${token.colorBorderSecondary}`,
            borderRadius: 8,
            backgroundColor: token.colorBgElevated,
            color: token.colorTextSecondary,
            cursor: "pointer",
            boxShadow: "0 2px 8px rgba(0,0,0,0.12)",
          }}
          aria-label={t("chat.agentPanel.expand")}
        >
          <ChevronLeft size={16} />
        </button>
      )}
      {/* 右侧面板 */}
      {showRightPanel && (
        <div
          style={{
            width: rightPanelCollapsed ? (isSmallScreen ? 0 : 48) : rightPanelWidth,
            minWidth: isSmallScreen && rightPanelCollapsed ? 0 : undefined,
            overflow: isSmallScreen && rightPanelCollapsed ? "hidden" : undefined,
            borderLeft: (isSmallScreen && rightPanelCollapsed)
              ? "none"
              : "1px solid var(--border-color)",
            backgroundColor: token.colorBgContainer,
            flexShrink: 0,
            display: "flex",
            flexDirection: "column",
            transition: rightDragging ? "none" : "width 0.2s",
            // 小屏：覆盖在主内容之上
            position: isSmallScreen ? "absolute" : "relative",
            top: isSmallScreen ? 0 : undefined,
            right: isSmallScreen ? 0 : undefined,
            bottom: isSmallScreen ? 0 : undefined,
            zIndex: isSmallScreen ? 40 : undefined,
            boxShadow: isSmallScreen && !rightPanelCollapsed
              ? `-4px 0 16px ${token.colorBgMask}`
              : "none",
          }}
        >
          {/* 折叠/展开按钮 — 始终在 ChatPage 层级，覆盖在面板左上角 */}
          <button
            type="button"
            onClick={toggleRightPanel}
            title={rightPanelCollapsed
              ? t("chat.agentPanel.expand")
              : t("chat.agentPanel.collapse")}
            style={{
              position: "absolute",
              top: 8,
              right: rightPanelCollapsed ? undefined : 8,
              left: rightPanelCollapsed ? "50%" : undefined,
              transform: rightPanelCollapsed ? "translateX(-50%)" : "none",
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
          {/* 始终渲染右侧面板，由 compactMode 控制内部展示 */}
          {!rightPanelCollapsed
            ? (rightPanelContent || (
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
                {t("chatPage.loading")}
              </div>
            ))
            : null}
        </div>
      )}
    </div>
  );
}
