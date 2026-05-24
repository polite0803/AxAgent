import { getConvIcon } from "@/lib/convIcon";
import {
  type TabItem,
  useConversationStore,
  useHelpStore,
  useProviderStore,
  useStreamStore,
  useTabStore,
} from "@/stores";
import { ModelIcon } from "@lobehub/icons";
import { Dropdown, theme, Tooltip } from "antd";
import { Avatar } from "antd";
import { HelpCircle, MessageSquarePlus, X } from "lucide-react";
import { Bot } from "lucide-react";
import { memo, useCallback, useRef } from "react";
import { useTranslation } from "react-i18next";

/** Max visible title length before truncation */
const MAX_TITLE_LEN = 24;

function truncateTitle(title: string): string {
  if (title.length <= MAX_TITLE_LEN) {
    return title;
  }
  return title.slice(0, MAX_TITLE_LEN - 1) + "…";
}

interface TabProps {
  tab: TabItem;
  isActive: boolean;
  onSelect: (tabId: string) => void;
  onClose: (tabId: string) => void;
  onCloseOthers: (tabId: string) => void;
  onCloseRight: (tabId: string) => void;
  model_id?: string | null;
  isStreaming?: boolean;
}

const Tab = memo(function Tab({
  tab,
  isActive,
  onSelect,
  onClose,
  onCloseOthers,
  onCloseRight,
  model_id,
  isStreaming,
}: TabProps) {
  const { token } = theme.useToken();
  const { t } = useTranslation();

  const handleClose = useCallback(
    (e: React.MouseEvent | React.KeyboardEvent) => {
      e.stopPropagation();
      onClose(tab.id);
    },
    [onClose, tab.id],
  );

  const customIcon = getConvIcon(tab.conversationId);

  const contextMenuItems = [
    { key: "closeOthers", label: t("chat.tabCloseOthers") },
    { key: "closeRight", label: t("chat.tabCloseRight") },
  ];

  const handleContextMenuClick = useCallback(
    ({ key }: { key: string }) => {
      if (key === "closeOthers") {
        onCloseOthers(tab.id);
      } else if (key === "closeRight") {
        onCloseRight(tab.id);
      }
    },
    [onCloseOthers, onCloseRight, tab.id],
  );

  return (
    <Dropdown
      menu={{ items: contextMenuItems, onClick: handleContextMenuClick }}
      trigger={["contextMenu"]}
    >
      <div
        onClick={() => onSelect(tab.id)}
        role="tab"
        tabIndex={0}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            onSelect(tab.id);
          }
        }}
        className={`ax-tab${isActive ? " ax-active" : ""}`}
        style={{ borderRight: `1px solid ${token.colorBorderSecondary}` }}
      >
        <span className="ax-tab-icon">
          {customIcon
            ? (
              customIcon.type === "emoji" ? <span style={{ fontSize: 12 }}>{customIcon.value}</span> : (
                <img
                  src={customIcon.value}
                  alt={model_id || ""}
                  style={{
                    width: 14,
                    height: 14,
                    borderRadius: 2,
                    objectFit: "cover",
                  }}
                />
              )
            )
            : model_id
            ? <ModelIcon model={model_id} size={14} type="avatar" />
            : (
              <Avatar
                size={14}
                icon={<Bot size={9} />}
                style={{
                  backgroundColor: token.colorPrimaryBg,
                  color: token.colorPrimary,
                }}
              />
            )}
        </span>

        <span className="truncate" style={{ flex: 1, minWidth: 0 }}>
          {truncateTitle(tab.title)}
        </span>

        {isStreaming && <span className="ax-tab-streaming-dot" />}

        <span
          onClick={handleClose}
          role="button"
          tabIndex={0}
          onKeyDown={(e) => {
            if (e.key === "Enter" || e.key === " ") {
              e.preventDefault();
              handleClose(e);
            }
          }}
          className="ax-tab-close"
        >
          <X size={10} />
        </span>
      </div>
    </Dropdown>
  );
});

export function TabBar() {
  const { token } = theme.useToken();
  const { t } = useTranslation();
  const toggleHelp = useHelpStore((s) => s.toggle);
  const tabs = useTabStore((s) => s.tabs);
  const activeTabId = useTabStore((s) => s.activeTabId);
  const setActiveTab = useTabStore((s) => s.setActiveTab);
  const closeTab = useTabStore((s) => s.closeTab);
  const closeOtherTabs = useTabStore((s) => s.closeOtherTabs);
  const closeTabsToRight = useTabStore((s) => s.closeTabsToRight);
  const openTab = useTabStore((s) => s.openTab);

  const conversations = useConversationStore((s) => s.conversations);
  const createConversation = useConversationStore((s) => s.createConversation);
  const streamingConversationId = useStreamStore(
    (s) => s.streamingConversationId,
  );
  const providers = useProviderStore((s) => s.providers);

  const scrollRef = useRef<HTMLDivElement>(null);

  const handleNewConversation = useCallback(async () => {
    const provider = providers.find(
      (p) => p.enabled && p.models.some((m) => m.enabled),
    );
    const model = provider?.models.find((m) => m.enabled);
    if (!provider || !model) { return; }
    const conv = await createConversation("", model.model_id, provider.id);
    openTab(conv.id, conv.title);
  }, [providers, createConversation, openTab]);

  const handleSelect = useCallback(
    (tabId: string) => {
      setActiveTab(tabId);
    },
    [setActiveTab],
  );

  const handleClose = useCallback(
    (tabId: string) => {
      closeTab(tabId);
    },
    [closeTab],
  );

  const handleCloseOthers = useCallback(
    (tabId: string) => {
      closeOtherTabs(tabId);
    },
    [closeOtherTabs],
  );

  const handleCloseRight = useCallback(
    (tabId: string) => {
      closeTabsToRight(tabId);
    },
    [closeTabsToRight],
  );

  if (tabs.length === 0) {
    return null;
  }

  return (
    <div className="ax-tabbar">
      <div
        ref={scrollRef}
        className="ax-tabbar-scroll"
      >
        {tabs.map((tab) => {
          const conv = conversations.find((c) => c.id === tab.conversationId);
          return (
            <Tab
              key={tab.id}
              tab={tab}
              isActive={tab.id === activeTabId}
              onSelect={handleSelect}
              onClose={handleClose}
              onCloseOthers={handleCloseOthers}
              onCloseRight={handleCloseRight}
              model_id={conv?.model_id}
              isStreaming={streamingConversationId === tab.conversationId}
            />
          );
        })}
      </div>

      {/* New tab button */}
      <Tooltip title={t("chat.newConversation")} mouseEnterDelay={0.4}>
        <button
          type="button"
          onClick={handleNewConversation}
          className="ax-tabbar-new"
          aria-label={t("chat.newConversation")}
          style={{ color: token.colorTextSecondary }}
        >
          <MessageSquarePlus size={14} />
        </button>
      </Tooltip>

      {/* Help button */}
      <div className="ax-tabbar-right">
        <Tooltip title={t("help.title")}>
          <button
            type="button"
            onClick={toggleHelp}
            className="ax-tabbar-new"
            aria-label={t("help.title")}
            style={{ color: token.colorTextQuaternary }}
          >
            <HelpCircle size={14} />
          </button>
        </Tooltip>
      </div>
    </div>
  );
}
