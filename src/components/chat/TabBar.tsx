import { useCallback, useRef, memo } from 'react';
import { theme, Tooltip, Dropdown } from 'antd';
import { X, MessageSquarePlus } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useTabStore, type TabItem, useConversationStore, useStreamStore } from '@/stores';
import { ModelIcon } from '@lobehub/icons';
import { getConvIcon } from '@/lib/convIcon';
import { Avatar } from 'antd';
import { Bot } from 'lucide-react';

/** Max visible title length before truncation */
const MAX_TITLE_LEN = 24;

function truncateTitle(title: string): string {
  if (title.length <= MAX_TITLE_LEN) return title;
  return title.slice(0, MAX_TITLE_LEN - 1) + '…';
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
    (e: React.MouseEvent) => {
      e.stopPropagation();
      onClose(tab.id);
    },
    [onClose, tab.id],
  );

  const customIcon = getConvIcon(tab.conversationId);

  const contextMenuItems = [
    { key: 'closeOthers', label: t('chat.tabCloseOthers') },
    { key: 'closeRight', label: t('chat.tabCloseRight') },
  ];

  const handleContextMenuClick = useCallback(
    ({ key }: { key: string }) => {
      if (key === 'closeOthers') onCloseOthers(tab.id);
      else if (key === 'closeRight') onCloseRight(tab.id);
    },
    [onCloseOthers, onCloseRight, tab.id],
  );

  return (
    <Dropdown
      menu={{ items: contextMenuItems, onClick: handleContextMenuClick }}
      trigger={['contextMenu']}
    >
      <div
        onClick={() => onSelect(tab.id)}
        className="chat-tab-item group"
        style={{
          display: 'inline-flex',
          alignItems: 'center',
          gap: 6,
          padding: '4px 8px 4px 10px',
          height: '100%',
          maxWidth: 200,
          minWidth: 80,
          cursor: 'pointer',
          userSelect: 'none',
          fontSize: 12,
          lineHeight: '18px',
          borderRadius: token.borderRadiusSM,
          backgroundColor: isActive ? token.colorBgElevated : 'transparent',
          color: isActive ? token.colorText : token.colorTextSecondary,
          borderRight: `1px solid ${token.colorBorderSecondary}`,
          position: 'relative',
          flexShrink: 0,
          transition: 'background-color 0.15s, color 0.15s',
          overflow: 'hidden',
        }}
        onMouseEnter={(e) => {
          if (!isActive) {
            e.currentTarget.style.backgroundColor = token.colorFillQuaternary;
          }
        }}
        onMouseLeave={(e) => {
          if (!isActive) {
            e.currentTarget.style.backgroundColor = 'transparent';
          }
        }}
      >
        {/* Icon */}
        <span style={{ display: 'flex', alignItems: 'center', flexShrink: 0 }}>
          {customIcon ? (
            customIcon.type === 'emoji' ? (
              <span style={{ fontSize: 12 }}>{customIcon.value}</span>
            ) : (
              <img
                src={customIcon.value}
                alt=""
                style={{ width: 14, height: 14, borderRadius: 2, objectFit: 'cover' }}
              />
            )
          ) : model_id ? (
            <ModelIcon model={model_id} size={14} type="avatar" />
          ) : (
            <Avatar
              size={14}
              icon={<Bot size={9} />}
              style={{ backgroundColor: token.colorPrimaryBg, color: token.colorPrimary }}
            />
          )}
        </span>

        {/* Title */}
        <span className="truncate" style={{ flex: 1, minWidth: 0 }}>
          {truncateTitle(tab.title)}
        </span>

        {/* Streaming indicator */}
        {isStreaming && (
          <span
            style={{
              width: 6,
              height: 6,
              borderRadius: '50%',
              backgroundColor: token.colorPrimary,
              flexShrink: 0,
              animation: 'axagent-tab-pulse 1.5s ease-in-out infinite',
            }}
          />
        )}

        {/* Close button */}
        <span
          onClick={handleClose}
          className="chat-tab-close"
          style={{
            display: 'inline-flex',
            alignItems: 'center',
            justifyContent: 'center',
            width: 16,
            height: 16,
            borderRadius: token.borderRadiusSM,
            flexShrink: 0,
            opacity: isActive ? 0.6 : 0,
            transition: 'opacity 0.15s, background-color 0.15s',
          }}
          onMouseEnter={(e) => {
            e.currentTarget.style.opacity = '1';
            e.currentTarget.style.backgroundColor = token.colorFillSecondary;
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.opacity = isActive ? '0.6' : '0';
            e.currentTarget.style.backgroundColor = 'transparent';
          }}
        >
          <X size={10} />
        </span>
      </div>
    </Dropdown>
  );
});

interface TabBarProps {
  onNewConversation?: () => void;
}

export function TabBar({ onNewConversation }: TabBarProps) {
  const { token } = theme.useToken();
  const { t } = useTranslation();
  const tabs = useTabStore((s) => s.tabs);
  const activeTabId = useTabStore((s) => s.activeTabId);
  const setActiveTab = useTabStore((s) => s.setActiveTab);
  const closeTab = useTabStore((s) => s.closeTab);
  const closeOtherTabs = useTabStore((s) => s.closeOtherTabs);
  const closeTabsToRight = useTabStore((s) => s.closeTabsToRight);

  const conversations = useConversationStore((s) => s.conversations);
  const streamingConversationId = useStreamStore((s) => s.streamingConversationId);

  const scrollRef = useRef<HTMLDivElement>(null);

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

  if (tabs.length === 0) return null;

  return (
    <div
      className="chat-tab-bar"
      style={{
        display: 'flex',
        alignItems: 'stretch',
        height: 34,
        minHeight: 34,
        borderBottom: `1px solid ${token.colorBorderSecondary}`,
        backgroundColor: token.colorBgContainer,
        overflow: 'hidden',
        position: 'relative',
      }}
    >
      {/* Scrollable tab strip */}
      <div
        ref={scrollRef}
        style={{
          display: 'flex',
          alignItems: 'stretch',
          flex: 1,
          overflowX: 'auto',
          overflowY: 'hidden',
          scrollbarWidth: 'none',
        }}
      >
        <style>{`
          .chat-tab-bar::-webkit-scrollbar { display: none; }
          .chat-tab-bar > div::-webkit-scrollbar { display: none; }
          .chat-tab-item:hover .chat-tab-close { opacity: 0.6 !important; }
          @keyframes axagent-tab-pulse {
            0%, 100% { opacity: 1; }
            50% { opacity: 0.3; }
          }
        `}</style>
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
      {onNewConversation && (
        <Tooltip title={t('chat.newConversation')} mouseEnterDelay={0.4}>
          <div
            onClick={onNewConversation}
            style={{
              display: 'inline-flex',
              alignItems: 'center',
              justifyContent: 'center',
              width: 34,
              height: '100%',
              cursor: 'pointer',
              color: token.colorTextSecondary,
              flexShrink: 0,
              borderLeft: `1px solid ${token.colorBorderSecondary}`,
              transition: 'color 0.15s, background-color 0.15s',
            }}
            onMouseEnter={(e) => {
              e.currentTarget.style.color = token.colorText;
              e.currentTarget.style.backgroundColor = token.colorFillQuaternary;
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.color = token.colorTextSecondary;
              e.currentTarget.style.backgroundColor = 'transparent';
            }}
          >
            <MessageSquarePlus size={14} />
          </div>
        </Tooltip>
      )}
    </div>
  );
}
