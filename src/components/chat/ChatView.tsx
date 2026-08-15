// SPDX-License-Identifier: AGPL-3.0-only

import { listen } from "@/lib/invoke";
import { App, Button, Input, Modal, Spin, Switch, theme, Typography } from "antd";
import DOMPurify from "dompurify";
import { ChevronDown } from "lucide-react";
import NodeRenderer from "markstream-react";
import React, { type ReactNode, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import { ModuleErrorBoundary } from "@/components/layout/ModuleErrorBoundary";
import { useResolvedDarkMode } from "@/hooks/useResolvedDarkMode";
import { logIpcError } from "@/lib/invoke";
import {
  setupAgentEventListeners,
  setupDreamEventListeners,
  setupPlanEventListeners,
  useAgentStore,
  useCacheStore,
  useCompressStore,
  useConversationStore,
  usePlanStore,
  useProviderStore,
  useSettingsStore,
  useStreamStore,
} from "@/stores";
import { useTopicGroupStore } from "@/stores/feature/topicGroupStore";

import { registerHighlight } from "stream-markdown";

import { PrefetchIndicator } from "../proactive/PrefetchIndicator";
import { AgentProgressBar } from "./AgentProgressBar";
import { AgentStatsPanel } from "./AgentStatsPanel";
import { BreadcrumbBar } from "./BreadcrumbBar";
import { CacheIndicator } from "./CacheIndicator";
import {
  type CodeBlockPreviewPayload,
  getChatCodeThemes,
  setCodeBlockPreviewHandler,
  setMermaidOpenModalHandler,
} from "./ChatMarkdownNodes";
import { ChatMinimap, MinimapScrollProvider } from "./ChatMinimap";
import { ChatScrollIndicator } from "./ChatScrollIndicator";
import { ClarifyCard } from "./ClarifyCard";
import { CodeBlockPreviewModal } from "./CodeBlockPreviewModal";
import { ContextBar, estimateConversationTokens } from "./ContextBar";
import { ContextGraphPanel } from "./ContextGraphPanel";
import { ExtractMemoriesModal } from "./ExtractMemoriesModal";
import { InputArea } from "./InputArea";
import { PermissionModal } from "./PermissionModal";
import { PlanApprovalModal } from "./PlanApprovalModal";
import { PlanCard } from "./PlanCard";
// QuickCommandBar removed: /clear, /compact, /model are covered by bottom toolbar & header ModelSelector
import { WorkflowEndMarker } from "./WorkflowEndMarker";
import { WorkflowProgressPanel } from "./WorkflowProgressPanel";

import { useChatViewMessages } from "./ChatViewMessages";
import { StreamingStyles } from "./ChatViewStreaming";

// Memoized to ensure style tags inject only once
const MemoizedStreamingStyles = React.memo(StreamingStyles);

/** IntersectionObserver-based lazy bubble wrapper for long message lists */
const LAZY_BUBBLE_ROOT_MARGIN = "300px";
const LAZY_BUBBLE_MIN_HEIGHT = 60;

// 共享单例 IntersectionObserver + 回调注册表，避免每条消息各建一个 observer
const lazyBubbleObserver:
  | { observer: IntersectionObserver; callbacks: WeakMap<Element, () => void> }
  | null = typeof IntersectionObserver === "undefined"
    ? null
    : (() => {
      const callbacks = new WeakMap<Element, () => void>();
      const observer = new IntersectionObserver(
        (entries) => {
          for (const entry of entries) {
            if (entry.isIntersecting) {
              const cb = callbacks.get(entry.target);
              callbacks.delete(entry.target);
              observer.unobserve(entry.target);
              cb?.();
            }
          }
        },
        { rootMargin: LAZY_BUBBLE_ROOT_MARGIN },
      );
      return { observer, callbacks };
    })();

const LazyBubble = React.memo(function LazyBubble({
  children,
}: {
  children: ReactNode;
}) {
  const [visible, setVisible] = useState(false);
  const ref = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const el = ref.current;
    if (!el || !lazyBubbleObserver) {
      setVisible(true);
      return;
    }
    const { observer, callbacks } = lazyBubbleObserver;
    callbacks.set(el, () => setVisible(true));
    observer.observe(el);
    return () => {
      callbacks.delete(el);
      observer.unobserve(el);
    };
  }, []);

  return (
    <div ref={ref} style={{ minHeight: visible ? undefined : LAZY_BUBBLE_MIN_HEIGHT }}>
      {visible ? children : null}
    </div>
  );
});
import { ChatViewToolbar } from "./ChatViewToolbar";
import { ChatViewWelcome } from "./ChatViewWelcome";
import { FilePermissionDialog } from "./FilePermissionDialog";
import type { FilePermissionRequest } from "./FilePermissionDialog";
import { useChatViewActions } from "./useChatViewActions";
import { useChatViewScroll } from "./useChatViewScroll";

function ChatViewInner({
  onScrollToReady,
}: {
  onScrollToReady?: (api: {
    scrollTo: (messageId: string) => void;
    scrollBoxRef: React.RefObject<HTMLElement | null>;
  }) => void;
}) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const { message: messageApi } = App.useApp();
  const planApprovalEnabled = useAgentStore((s) => s.planApprovalEnabled);
  const setPlanApprovalEnabled = useAgentStore((s) => s.setPlanApprovalEnabled);

  const conversations = useConversationStore((s) => s.conversations);
  const activeConversationId = useConversationStore(
    (s) => s.activeConversationId,
  );
  const setActiveConversation = useConversationStore(
    (s) => s.setActiveConversation,
  );
  const messages = useConversationStore((s) => s.messages);
  const loading = useConversationStore((s) => s.loading);
  const loadingOlder = useConversationStore((s) => s.loadingOlder);
  const hasOlderMessages = useConversationStore((s) => s.hasOlderMessages);
  const activeStreams = useStreamStore((s) => s.activeStreams);
  const streaming = activeConversationId
    ? activeConversationId in activeStreams
    : false;
  const compressing = useCompressStore((s) => s.compressing);
  const settings = useSettingsStore((s) => s.settings);
  const bubbleStyle = settings.bubble_style;
  const providers = useProviderStore((s) => s.providers);
  const isDarkMode = useResolvedDarkMode(settings.theme_mode);
  const storeError = useConversationStore((s) => s.error);
  const toggleArchive = useConversationStore((s) => s.toggleArchive);
  const loadOlderMessages = useConversationStore((s) => s.loadOlderMessages);
  const streamingMessageId = useStreamStore((s) => s.streamingMessageId);
  const cacheValid = useCacheStore((s) => s.cacheValid);
  const hasPendingChanges = useCacheStore((s) => s.hasPendingChanges);
  const tokensSaved = useCacheStore((s) => s.tokensSaved);
  const cacheHits = useCacheStore((s) => s.cacheHits);
  const fetchCacheState = useCacheStore((s) => s.fetchCacheState);

  const activeConversation = conversations.find(
    (c) => c.id === activeConversationId,
  );

  // 合并 preview 模态框状态，避免级联 setState
  const [previewState, setPreviewState] = useState<{
    payload: CodeBlockPreviewPayload | null;
    open: boolean;
  }>({
    payload: null,
    open: false,
  });
  // 合并 mermaid 预览模态框状态，避免级联 setState
  const [mermaidState, setMermaidState] = useState<{
    svg: string | null;
    open: boolean;
  }>({
    svg: null,
    open: false,
  });

  const [filePermDialogOpen, setFilePermDialogOpen] = useState(false);
  const [filePermRequest, setFilePermRequest] = useState<FilePermissionRequest | null>(null);

  const {
    darkTheme: codeBlockDarkTheme,
    lightTheme: codeBlockLightTheme,
    themes: codeBlockThemes,
  } = useMemo(
    () => getChatCodeThemes(settings.code_theme, settings.code_theme_light),
    [settings.code_theme, settings.code_theme_light],
  );

  const bubbleListThemeKey = `bubble-list:${isDarkMode ? "dark" : "light"}:${settings.code_theme ?? ""}:${
    settings.code_theme_light ?? ""
  }`;

  useEffect(() => {
    if (codeBlockThemes.length > 0) {
      registerHighlight({
        themes: codeBlockThemes as import("@shikijs/types").ThemeInput[],
      }).catch(logIpcError("preload_highlight_themes"));
    }
  }, [codeBlockThemes, codeBlockDarkTheme, codeBlockLightTheme, isDarkMode]);

  useEffect(() => {
    setCodeBlockPreviewHandler((payload: CodeBlockPreviewPayload) => {
      setPreviewState({ payload, open: true });
    });
    return () => {
      setCodeBlockPreviewHandler(null);
    };
  }, []);

  useEffect(() => {
    setMermaidOpenModalHandler((svgString: string | null) => {
      setMermaidState({ svg: svgString, open: true });
    });
    return () => {
      setMermaidOpenModalHandler(null);
    };
  }, []);

  useEffect(() => {
    fetchCacheState();
  }, [fetchCacheState, activeConversationId]);

  useEffect(() => {
    if (!activeConversationId) {
      return;
    }
    const conversation = conversations.find(
      (c) => c.id === activeConversationId,
    );
    if (conversation?.mode === "agent") {
      const { activePlans, loadActivePlan } = usePlanStore.getState();
      if (!activePlans[activeConversationId]) {
        void loadActivePlan(activeConversationId);
      }
    }
  }, [activeConversationId, conversations]);

  useEffect(() => {
    if (activeConversation?.mode === "agent" && activeConversationId) {
      useAgentStore.getState().loadToolHistory(activeConversationId);
    }
  }, [activeConversationId, activeConversation?.mode]);

  useEffect(() => {
    if (storeError) {
      messageApi.error(storeError);
      useConversationStore.setState({ error: null });
    }
  }, [storeError, messageApi]);

  useEffect(() => {
    const cleanupAgent = setupAgentEventListeners();
    const cleanupPlan = setupPlanEventListeners();
    const cleanupDream = setupDreamEventListeners();
    return () => {
      cleanupAgent();
      cleanupPlan();
      cleanupDream();
    };
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen<FilePermissionRequest>("file-permission-request", (event) => {
      setFilePermRequest(event.payload);
      setFilePermDialogOpen(true);
    }).then((fn) => {
      unlisten = fn;
    });
    return () => {
      unlisten?.();
    };
  }, []);

  const currentAgentStatus = useAgentStore((s) =>
    activeConversationId ? s.agentStatus[activeConversationId] : undefined
  );

  const bubbleListRef = useRef<HTMLDivElement & { scrollBoxNativeElement?: HTMLElement | null } | null>(null);
  const messageAreaRef = useRef<HTMLDivElement | null>(null);

  const actions = useChatViewActions({
    activeConversationId,
    activeConversation,
    messages,
    bubbleListRef,
    messageAreaRef,
    loadOlderMessages,
  });

  const contextBarModel = useMemo(() => {
    if (!activeConversation) {
      return null;
    }
    const provider = providers.find(
      (p) => p.id === activeConversation.provider_id,
    );
    const model = provider?.models.find(
      (m) => m.model_id === activeConversation.model_id,
    );
    return {
      name: model?.name ?? activeConversation.model_id,
      maxTokens: model?.max_tokens ?? activeConversation.max_tokens ?? undefined,
    };
  }, [activeConversation, providers]);

  const topicGroupEnabled = useTopicGroupStore((s) =>
    activeConversationId
      ? s.enabledByConversation[activeConversationId]
      : false
  );

  const msgState = useChatViewMessages({
    activeConversationId,
    activeConversation,
    messages,
    streaming,
    compressing,
    bubbleStyle,
    bubbleListRef,
    handleEditMessage: actions.handleEditMessage,
  });

  const scroll = useChatViewScroll({
    bubbleListRef,
    activeConversationId,
    bubbleListThemeKey,
    messageCount: messages.length,
    streaming,
    hasOlderMessages,
    loading,
    loadingOlder,
    loadOlderMessages,
    allBubbleItems: msgState.allBubbleItems,
    lastBubbleKey: msgState.lastBubbleKey,
  });

  const onScrollToReadyRef = useRef(onScrollToReady);

  useEffect(() => {
    onScrollToReadyRef.current = onScrollToReady;
  }, [onScrollToReady]);

  useEffect(() => {
    onScrollToReadyRef.current?.({
      scrollTo: scroll.minimapScrollTo,
      scrollBoxRef: scroll.scrollBoxRef,
    });
  }, [scroll.minimapScrollTo, scroll.scrollBoxRef]);

  const activeMessages = useMemo(
    () => messages.filter((msg) => msg.is_active !== false),
    [messages],
  );

  // FE-I8 修复：tokenUsed 用 useMemo 缓存，避免流式每 50ms 重渲染时全量重算。
  const tokenUsed = useMemo(
    () =>
      activeMessages.length > 0
        ? estimateConversationTokens(
          activeMessages.map((m) => ({ role: m.role, content: m.content })),
        )
        : 0,
    [activeMessages],
  );

  return (
    <div className="ax-cyber-grid flex flex-col h-full min-h-0">
      <MemoizedStreamingStyles />
      {/* BubbleStyleOverrides removed — using native CSS */}

      <ChatViewToolbar
        activeConversation={activeConversation}
        activeConversationId={activeConversationId}
        editingTitle={actions.editingTitle}
        titleDraft={actions.titleDraft}
        setTitleDraft={actions.setTitleDraft}
        titleInputRef={actions.titleInputRef}
        handleTitleClick={actions.handleTitleClick}
        handleTitleSave={actions.handleTitleSave}
        handleRegenerateTitle={actions.handleRegenerateTitle}
        isTitleGenerating={actions.isTitleGenerating}
        renderConvIconForChat={msgState.renderConvIconForChat}
        topicGroupEnabled={topicGroupEnabled}
        handleTopicGroupToggle={actions.handleTopicGroupToggle}
        statsOpen={actions.statsOpen}
        stats={actions.stats}
        handleStatsOpenChange={actions.handleStatsOpenChange}
        exportMenuItems={actions.exportMenuItems}
        setExtractMemoriesOpen={actions.setExtractMemoriesOpen}
        streamingMessageId={streamingMessageId}
        token={token}
      />

      <BreadcrumbBar
        conversations={conversations}
        activeConversationId={activeConversationId}
        setActiveConversation={setActiveConversation}
      />

      {contextBarModel && (
        <ContextBar
          modelName={contextBarModel.name}
          searchEnabled={activeConversation?.search_enabled ?? false}
          toolCount={actions.toolCount}
          knowledgeCount={activeConversation?.enabled_knowledge_base_ids?.length ?? 0}
          memoryEnabled={(activeConversation?.enabled_memory_namespace_ids?.length ?? 0) > 0}
          tokenUsed={tokenUsed > 0 ? tokenUsed : undefined}
          tokenMax={contextBarModel.maxTokens}
          mode={activeConversation?.mode}
        />
      )}

      <AgentStatsPanel />

      <CacheIndicator
        cacheValid={cacheValid}
        hasPendingChanges={hasPendingChanges}
        tokensSaved={tokensSaved}
        cacheHits={cacheHits}
      />

      {activeConversationId && messages.length > 0
        && (() => {
          const ctxProvider = providers.find(
            (p) => p.id === activeConversation?.provider_id,
          );
          const ctxModel = ctxProvider?.models.find(
            (m) => m.model_id === activeConversation?.model_id,
          );
          return (
            <div style={{ padding: "0 16px", flexShrink: 0 }}>
              <ContextGraphPanel
                conversationTitle={activeConversation?.title}
                conversationId={activeConversationId}
                modelName={ctxModel?.name ?? activeConversation?.model_id}
                providerName={ctxProvider?.name}
                knowledgeBaseIds={activeConversation?.enabled_knowledge_base_ids ?? []}
                memoryNamespaceIds={activeConversation?.enabled_memory_namespace_ids ?? []}
                mcpServerIds={activeConversation?.enabled_mcp_server_ids ?? []}
                searchEnabled={activeConversation?.search_enabled ?? false}
                enabledSkillIds={activeConversation?.enabled_skill_ids ?? []}
              />
            </div>
          );
        })()}

      <div
        ref={messageAreaRef}
        data-message-area
        data-message-count={messages.length}
        className={`flex-1 min-h-0 overflow-hidden relative bubble-${bubbleStyle || "modern"}`}
        role="log"
        aria-live="polite"
        aria-atomic="false"
        aria-relevant="additions"
        aria-label={t("chat.messageArea")}
        style={{ display: "flex", flexDirection: "column" }}
      >
        {messages.length === 0
          ? (
            <ChatViewWelcome
              loading={loading}
              activeConversationId={activeConversationId}
            />
          )
          : (
            <>
              {msgState.hiddenEarlierCount > 0
                && msgState.hiddenEarlierCount
                  === msgState.allBubbleItems.length
                && (
                  <div style={{ textAlign: "center", padding: "8px 0", flexShrink: 0 }}>
                    <Button
                      size="small"
                      type="link"
                      loading={loadingOlder}
                      onClick={() => {
                        const scrollBox = scroll.scrollBoxRef.current;
                        if (scrollBox) {
                          scrollBox.scrollTo({ top: 0, behavior: "smooth" });
                        }
                      }}
                    >
                      {t("chat.showAllMessages", {
                        count: msgState.allBubbleItems.length,
                      })}
                    </Button>
                  </div>
                )}
              <div
                ref={bubbleListRef}
                className="msg-list-scroll-box"
                onScroll={scroll.handleBubbleListScroll}
                style={{
                  flex: "1 1 0%",
                  minHeight: 0,
                  padding: settings.chat_minimap_enabled
                      && settings.chat_minimap_style === "sticky"
                    ? "40px 16px 8px 16px"
                    : "8px 16px",
                  overflowX: "hidden",
                  overflowY: "auto",
                  display: "flex",
                  flexDirection: "column-reverse",
                  gap: "4px",
                }}
              >
                {msgState.visibleBubbleItems.map((item) => {
                  const roleFn = msgState.roles[item.role as keyof typeof msgState.roles];
                  if (!roleFn) { return null; }
                  const rendered = roleFn(item);
                  const variantClass = rendered.variant ? `bubble-${rendered.variant}` : "";
                  const bubbleNode = (
                    <div
                      key={item.key}
                      className={rendered.className ?? `msg-row ${rendered.placement === "end" ? "user" : "assistant"}`}
                      style={rendered.style}
                    >
                      {rendered.avatar && <div className="msg-avatar">{rendered.avatar}</div>}
                      <div className="msg-body">
                        {rendered.header && <div className="msg-header">{rendered.header}</div>}
                        <div className={`msg-content ${variantClass}`}>
                          {rendered.loading
                            ? <Spin />
                            : rendered.contentRender
                            ? rendered.contentRender(item.content as ReactNode, item)
                            : item.content as ReactNode}
                        </div>
                        {rendered.footer && <div className="msg-footer">{rendered.footer}</div>}
                      </div>
                    </div>
                  );
                  // Only use lazy rendering for large lists (>20 items)
                  if (messages.length > 20) {
                    return (
                      <LazyBubble key={item.key}>
                        {bubbleNode}
                      </LazyBubble>
                    );
                  }
                  return bubbleNode;
                })}
                {activeConversation?.session_type === "workflow"
                  && activeConversation?.workflow_status === "completed" && (
                  <WorkflowEndMarker
                    workflowName={activeConversation.workflow_template_id
                      ?? t("chat.workflowLabel")}
                    stepCount={0}
                    completedCount={0}
                    durationSeconds={0}
                    onArchive={() => {
                      void toggleArchive(activeConversation.id);
                    }}
                  />
                )}
                <ClarifyCard />
              </div>
              <ChatScrollIndicator />
              <MinimapScrollProvider
                scrollTo={scroll.minimapScrollTo}
                scrollBoxRef={scroll.scrollBoxRef}
              >
                <ChatMinimap />
              </MinimapScrollProvider>
            </>
          )}
      </div>

      {currentAgentStatus && (
        <div
          data-testid="agent-status"
          style={{
            display: "flex",
            alignItems: "center",
            gap: 8,
            padding: "6px 24px",
            fontSize: 13,
            color: token.colorTextSecondary,
          }}
        >
          <Spin size="small" /> {currentAgentStatus}
        </div>
      )}
      {activeConversation?.mode === "agent" && activeConversationId && (
        <div className="flex flex-col" style={{ gap: 2 }}>
          <AgentProgressBar conversationId={activeConversationId} />
          <WorkflowProgressPanel conversationId={activeConversationId} />
          <PlanCardWrapper conversationId={activeConversationId} />
        </div>
      )}

      {/* QuickCommandBar removed */}

      <div className="relative">
        {scroll.showScrollToBottom && (
          <Button
            size="small"
            shape="round"
            icon={<ChevronDown size={14} />}
            onClick={scroll.handleScrollToBottom}
            aria-label={t("chat.scrollToBottom")}
            style={{
              position: "absolute",
              left: "50%",
              top: -28,
              zIndex: 2,
              transform: "translateX(-50%)",
              boxShadow: token.boxShadowSecondary,
            }}
          >
            {t("chat.scrollToBottom")}
          </Button>
        )}
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: 8,
            padding: "4px 12px",
          }}
        >
          <Switch
            size="small"
            data-testid="plan-approval-toggle"
            checked={planApprovalEnabled}
            onChange={(v) => setPlanApprovalEnabled(v)}
          />
          <Typography.Text type="secondary" style={{ fontSize: 12 }}>
            {t("planApproval.toggleLabel")}
          </Typography.Text>
        </div>
        <InputArea />
      </div>

      <PermissionModal />
      <PlanApprovalModal />
      {filePermRequest && (
        <FilePermissionDialog
          open={filePermDialogOpen}
          onClose={() => {
            setFilePermDialogOpen(false);
            setFilePermRequest(null);
          }}
          path={filePermRequest.path}
          reason={filePermRequest.reason}
        />
      )}
      <Modal
        title={t("chat.compressionSummary")}
        open={msgState.summaryModalOpen}
        onCancel={() => msgState.setSummaryModalOpen(false)}
        footer={null}
        width={640}
      >
        <div style={{ maxHeight: 480, overflow: "auto", padding: "8px 0" }}>
          <NodeRenderer
            content={msgState.summaryModalText}
            isDark={isDarkMode}
            customId="summary"
            final
            themes={codeBlockThemes}
            codeBlockLightTheme={codeBlockLightTheme}
            codeBlockDarkTheme={codeBlockDarkTheme}
          />
        </div>
      </Modal>
      <Modal
        title={t("chat.editMessage")}
        open={!!actions.editingMessageId}
        onCancel={() => {
          actions.resetEditing();
        }}
        footer={[
          <Button
            key="cancel"
            onClick={() => {
              actions.resetEditing();
            }}
          >
            {t("common.cancel")}
          </Button>,
          <Button
            key="save"
            onClick={actions.handleEditSaveOnly}
            loading={actions.editSaving}
          >
            {t("chat.saveOnly")}
          </Button>,
          ...(actions.editingMessageRole === "assistant"
            ? []
            : [
              <Button
                key="saveResend"
                type="primary"
                onClick={actions.handleEditSaveAndResend}
                loading={actions.editSaving}
              >
                {t("chat.saveAndResend")}
              </Button>,
            ]),
        ]}
        width={640}
      >
        <Input.TextArea
          id="chat-view-input-textarea-8"
          value={actions.editingContent}
          onChange={(e) => actions.setEditingContent(e.target.value)}
          autoSize={{ minRows: 3, maxRows: 12 }}
          style={{ marginTop: 8 }}
        />
      </Modal>
      <CodeBlockPreviewModal
        payload={previewState.payload}
        open={previewState.open}
        onClose={() => setPreviewState({ payload: null, open: false })}
      />
      <Modal
        title={`Mermaid ${t("common.preview")}`}
        open={mermaidState.open}
        onCancel={() => setMermaidState({ svg: null, open: false })}
        footer={null}
        width="80vw"
        style={{ top: 32 }}
        styles={{
          body: { height: "calc(80vh - 55px)", overflow: "auto", padding: 16 },
        }}
        destroyOnHidden
      >
        {mermaidState.svg && (
          <div
            style={{ width: "100%", display: "flex", justifyContent: "center" }}
            dangerouslySetInnerHTML={{
              __html: DOMPurify.sanitize(mermaidState.svg),
            }}
          />
        )}
      </Modal>
      <ExtractMemoriesModal
        open={actions.extractMemoriesOpen}
        onClose={() => actions.setExtractMemoriesOpen(false)}
        conversationId={activeConversationId ?? ""}
      />
      <PrefetchIndicator />
    </div>
  );
}

export interface ChatViewScrollApi {
  scrollTo: (messageId: string) => void;
  scrollBoxRef: React.RefObject<HTMLElement | null>;
}

export function ChatView({
  onScrollToReady,
}: {
  onScrollToReady?: (api: ChatViewScrollApi) => void;
}) {
  return (
    <ModuleErrorBoundary
      moduleName="ChatView"
      showDetails={import.meta.env.DEV}
    >
      <ChatViewInner onScrollToReady={onScrollToReady} />
    </ModuleErrorBoundary>
  );
}

function PlanCardWrapper({ conversationId }: { conversationId: string }) {
  const plan = usePlanStore((s) => s.activePlans[conversationId]);
  if (!plan) {
    return null;
  }
  return (
    <div style={{ padding: "8px 16px" }}>
      <PlanCard plan={plan} conversationId={conversationId} />
    </div>
  );
}
