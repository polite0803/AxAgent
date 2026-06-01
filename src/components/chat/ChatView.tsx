import { listen } from "@tauri-apps/api/event";
import { App, Button, Input, Modal, Spin, theme } from "antd";
import DOMPurify from "dompurify";
import { ChevronDown } from "lucide-react";
import NodeRenderer from "markstream-react";
import React, { type ReactNode, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import { ModuleErrorBoundary } from "@/components/layout/ModuleErrorBoundary";
import { useResolvedDarkMode } from "@/hooks/useResolvedDarkMode";
import { invoke, logIpcError } from "@/lib/invoke";
import { estimateTokens } from "@/lib/tokenEstimator";
import {
  setupAgentEventListeners,
  setupDreamEventListeners,
  setupPlanEventListeners,
  syncAllStoresToDomain,
  useAgentStore,
  useCacheStore,
  useCompressStore,
  useConversationStore,
  useExpertStore,
  usePlanStore,
  useProviderStore,
  useSettingsStore,
  useStreamStore,
} from "@/stores";
import { useAppConfigStore } from "@/stores/feature/appConfigStore";
import { useProactiveStore } from "@/stores/feature/proactiveStore";
import { useTopicGroupStore } from "@/stores/feature/topicGroupStore";

import { registerHighlight } from "stream-markdown";

import { ContextPredictionPanel } from "../proactive/ContextPredictionPanel";
import { PrefetchIndicator } from "../proactive/PrefetchIndicator";
import { ProactiveSuggestionBar } from "../proactive/ProactiveSuggestionBar";
import { ReminderList } from "../proactive/ReminderList";
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
import { CodeBlockPreviewModal } from "./CodeBlockPreviewModal";
import { ContextBar, estimateConversationTokens } from "./ContextBar";
import { ContextClassificationBar } from "./ContextClassificationBar";
import type { ContextSegment } from "./ContextClassificationBar";
import { ContextGraphPanel } from "./ContextGraphPanel";
import { ExpertSelector } from "./ExpertSelector";
import { ExtractMemoriesModal } from "./ExtractMemoriesModal";
import { InputArea } from "./InputArea";
import { PermissionModal } from "./PermissionModal";
import { PlanCard } from "./PlanCard";
// QuickCommandBar removed: /clear, /compact, /model are covered by bottom toolbar & header ModelSelector
import { SteerInput } from "./SteerInput";
import { WorkflowEndMarker } from "./WorkflowEndMarker";
import { WorkflowProgressPanel } from "./WorkflowProgressPanel";
import { WorkflowSuggestionCard } from "./WorkflowSuggestionCard";

import { useChatViewMessages } from "./ChatViewMessages";
import { StreamingStyles } from "./ChatViewStreaming";
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
  const updateConversation = useConversationStore((s) => s.updateConversation);
  const fetchConversation = useConversationStore((s) => s.fetchConversations);
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
    syncAllStoresToDomain();
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

  const prevMessageCount = useRef(messages.length);

  const buildChatContext = (): Record<string, unknown> => {
    const now = new Date();
    const lastUserMsg = [...messages].reverse().find((m) => m.role === "user");
    const content = lastUserMsg?.content || "";

    const fileRegex = /[\w/\\-]+\.(tsx?|jsx?|py|rs|go|java|rb|php|html|css|json|yml|yaml|md|sql|sh)/gi;
    const fileMatches = content.match(fileRegex) || [];

    const langMap: Record<string, string> = {
      ts: "typescript",
      tsx: "typescript",
      js: "javascript",
      jsx: "javascript",
      py: "python",
      rs: "rust",
      go: "go",
      java: "java",
      rb: "ruby",
      php: "php",
      html: "html",
      css: "css",
      json: "json",
      yml: "yaml",
      yaml: "yaml",
      md: "markdown",
      sql: "sql",
      sh: "shell",
    };

    const current_file = fileMatches.length > 0 ? fileMatches[0] : null;
    let current_language = null;
    if (current_file) {
      const ext = current_file.split(".").pop()?.toLowerCase() || "";
      current_language = langMap[ext] || null;
    }

    const recent_actions: string[] = [];
    if (messages.length > 0) {
      recent_actions.push("UserMessaged");
    }
    const errorKeywords = ["error", "Error", "bug", "修复", "报错"];
    const refactorKeywords = ["refactor", "优化", "重构", "improve"];
    const testKeywords = ["test", "测试", "spec"];
    const docKeywords = ["document", "文档", "readme", "doc"];

    if (errorKeywords.some((kw) => content.includes(kw))) {
      recent_actions.push("ErrorDetected");
    }
    if (refactorKeywords.some((kw) => content.includes(kw))) {
      recent_actions.push("RefactorKeyword");
    }
    if (testKeywords.some((kw) => content.includes(kw))) {
      recent_actions.push("TestKeyword");
    }
    if (docKeywords.some((kw) => content.includes(kw))) {
      recent_actions.push("DocKeyword");
    }
    if (fileMatches.length > 0) {
      recent_actions.push("FileOpened");
    }

    const detected_errors = content.toLowerCase().includes("error") || content.includes("报错")
      ? ["error_detected_in_context"]
      : [];
    const detected_patterns = fileMatches.map((f) => ({
      pattern: `file_reference_${f}`,
      match_type: "file_reference",
    }));

    const activity = messages.length > 0
        && now.getTime() / 1000 - (lastUserMsg?.created_at || 0) < 60
      ? ("high" as const)
      : ("medium" as const);

    const projectTypeMap: Record<string, string> = {
      ts: "typescript",
      tsx: "typescript",
      js: "javascript",
      jsx: "javascript",
      py: "python",
      rs: "rust",
      go: "go",
      java: "java",
      rb: "ruby",
      php: "php",
    };
    let project_type: string | null = null;
    if (fileMatches.length > 0) {
      const exts = fileMatches.flatMap((f) => {
        const r = f.split(".").pop()?.toLowerCase();
        return r ? [r] : [];
      });
      const counts = new Map<string, number>();
      for (const ext of exts) {
        counts.set(ext, (counts.get(ext) || 0) + 1);
      }
      let dominantExt = "";
      let dominantCount = 0;
      for (const [ext, cnt] of counts) {
        if (cnt > dominantCount) {
          dominantCount = cnt;
          dominantExt = ext;
        }
      }
      if (dominantExt && projectTypeMap[dominantExt]) {
        project_type = projectTypeMap[dominantExt];
      }
    }

    return {
      current_file,
      current_language,
      recent_actions,
      time_of_day: now.getHours(),
      day_of_week: now
        .toLocaleDateString("en-US", { weekday: "long" })
        .toLowerCase(),
      project_type,
      user_activity_level: activity,
      detected_errors,
      detected_patterns,
    };
  };

  useEffect(() => {
    if (
      useAppConfigStore.getState().features.proactiveMode
      && messages.length > prevMessageCount.current
      && activeConversationId
    ) {
      const { refreshSuggestions } = useProactiveStore.getState();
      refreshSuggestions(buildChatContext());
    }
    prevMessageCount.current = messages.length;
  }, [messages.length, activeConversationId]);

  const currentAgentStatus = useAgentStore((s) =>
    activeConversationId ? s.agentStatus[activeConversationId] : undefined
  );
  const workflowMatchSuggestion = useAgentStore((s) => s.workflowMatchSuggestion);

  const bubbleListRef = useRef<any>(null);
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

  useEffect(() => {
    onScrollToReady?.({
      scrollTo: scroll.minimapScrollTo,
      scrollBoxRef: scroll.scrollBoxRef,
    });
  }, [scroll.minimapScrollTo]);

  const activeMessages = useMemo(
    () => messages.filter((msg) => msg.is_active !== false),
    [messages],
  );

  const tokenUsed = activeMessages.length > 0
    ? estimateConversationTokens(
      activeMessages.map((m) => ({ role: m.role, content: m.content })),
    )
    : 0;

  const [showTokenDetail, setShowTokenDetail] = useState(false);

  const classificationSegments = useMemo<ContextSegment[]>(() => {
    const segments: ContextSegment[] = [
      {
        key: "messages",
        labelKey: "chat.context.messages",
        tokens: tokenUsed,
        color: token.colorPrimary,
      },
    ];

    const systemPrompt = activeConversation?.system_prompt;
    if (systemPrompt) {
      segments.push({
        key: "system_prompt",
        labelKey: "chat.context.systemPrompt",
        tokens: estimateTokens(systemPrompt),
        color: token.colorSuccess,
      });
    }

    const knowledgeCount = activeConversation?.enabled_knowledge_base_ids?.length ?? 0;
    if (knowledgeCount > 0) {
      segments.push({
        key: "knowledge",
        labelKey: "chat.context.knowledge",
        tokens: knowledgeCount * 500,
        color: "var(--orange, #fa8c16)",
      });
    }

    const memoryCount = activeConversation?.enabled_memory_namespace_ids?.length ?? 0;
    if (memoryCount > 0) {
      segments.push({
        key: "memory",
        labelKey: "chat.context.memory",
        tokens: memoryCount * 200,
        color: "var(--magenta, #eb2f96)",
      });
    }

    if (actions.toolCount > 0) {
      segments.push({
        key: "tools",
        labelKey: "chat.context.tools",
        tokens: actions.toolCount * 200,
        color: "var(--purple, #722ed1)",
      });
    }

    const skillCount = activeConversation?.enabled_skill_ids?.length ?? 0;
    if (skillCount > 0) {
      segments.push({
        key: "skills",
        labelKey: "chat.context.skills",
        tokens: skillCount * 300,
        color: "var(--cyan, #13c2c2)",
      });
    }

    return segments;
  }, [tokenUsed, activeConversation, actions.toolCount, token.colorPrimary, token.colorSuccess]);

  return (
    <div className="ax-cyber-grid flex flex-col h-full min-h-0">
      <StreamingStyles />
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
        setExpertOpen={actions.setExpertOpen}
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
          onTokenClick={() => setShowTokenDetail((v) => !v)}
        />
      )}

      {showTokenDetail && classificationSegments.length > 0 && (
        <ContextClassificationBar
          segments={classificationSegments}
          maxTokens={contextBarModel?.maxTokens}
        />
      )}

      <AgentStatsPanel />

      <CacheIndicator
        cacheValid={cacheValid}
        hasPendingChanges={hasPendingChanges}
        tokensSaved={tokensSaved}
        cacheHits={cacheHits}
      />

      <div
        ref={messageAreaRef}
        data-message-area
        data-message-count={messages.length}
        className={`flex-1 min-h-0 overflow-hidden relative bubble-${bubbleStyle || "modern"}`}
        role="log"
        aria-live="polite"
        aria-atomic="false"
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
              {activeConversationId
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
                    ? "50px 24px 16px 24px"
                    : "16px 24px",
                  overflowX: "hidden",
                  overflowY: "auto",
                  display: "flex",
                  flexDirection: "column-reverse",
                  gap: 10,
                }}
              >
                {msgState.visibleBubbleItems.map((item) => {
                  const roleFn = msgState.roles[item.role as keyof typeof msgState.roles];
                  if (!roleFn) { return null; }
                  const rendered = roleFn(item);
                  const variantClass = rendered.variant ? `bubble-${rendered.variant}` : "";
                  return (
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
                {workflowMatchSuggestion
                  && workflowMatchSuggestion.conversationId === activeConversation?.id
                  && activeConversation?.mode === "agent"
                  && (
                    <WorkflowSuggestionCard
                      match={{
                        templateId: workflowMatchSuggestion.templateId,
                        templateName: workflowMatchSuggestion.templateName,
                        similarity: workflowMatchSuggestion.similarity,
                      }}
                      onSwitch={(templateId) => {
                        void updateConversation(activeConversation.id, {
                          session_type: "workflow",
                          workflow_template_id: templateId,
                        });
                        fetchConversation();
                        useAgentStore.getState().setWorkflowMatchSuggestion(null);
                      }}
                      onDismiss={() => {
                        useAgentStore.getState().setWorkflowMatchSuggestion(null);
                      }}
                    />
                  )}
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
      <ProactiveSuggestionBar />
      <ProactivePanelsSection context={buildChatContext()} />

      {activeConversation?.mode === "agent" && activeConversationId && (
        <div className="flex flex-col" style={{ gap: 2 }}>
          <AgentProgressBar conversationId={activeConversationId} />
          <WorkflowProgressPanel conversationId={activeConversationId} />
          <PlanCardWrapper conversationId={activeConversationId} />
          {streaming && <SteerInput conversationId={activeConversationId} />}
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
        <InputArea />
      </div>

      <PermissionModal />
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
      <ExpertSelector
        open={actions.expertOpen}
        onClose={() => actions.setExpertOpen(false)}
        selectedRoleId={activeConversation?.agent_profile_id ?? null}
        onSelect={(roleId) => {
          if (!activeConversationId) {
            return;
          }
          const expertStore = useExpertStore.getState();
          const role = expertStore.getRoleById(roleId);
          if (!role) {
            return;
          }

          // 确保 AgentProfile 在 DB 中存在
          invoke("ensure_agent_profile", {
            id: roleId,
            name: role.name,
            expertId: role.source === "agency" ? roleId : (role.expertId ?? null),
            agentRole: role.agentRole ?? null,
          }).catch(() => {/* profile 可能已存在，忽略错误 */});

          updateConversation(activeConversationId, {
            agent_profile_id: roleId,
            session_type: "conversation",
            workflow_template_id: null,
          });
          expertStore.recordSwitch(activeConversationId, roleId);

          if (role.suggestedProviderId && role.suggestedModelId) {
            updateConversation(activeConversationId, {
              provider_id: role.suggestedProviderId,
              model_id: role.suggestedModelId,
            });
          }
          if (role.suggestedTemperature != null) {
            updateConversation(activeConversationId, {
              temperature: role.suggestedTemperature,
            });
          }

          if (role.recommendPermissionMode) {
            const { updatePermissionMode } = useAgentStore.getState();
            updatePermissionMode(
              activeConversationId,
              role.recommendPermissionMode,
            );
          }

          actions.setExpertOpen(false);
        }}
      />
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

function ProactivePanelsSection({
  context,
}: {
  context: Record<string, unknown>;
}) {
  const { t } = useTranslation();
  const proactiveMode = useAppConfigStore((s) => s.features.proactiveMode);

  if (!proactiveMode) {
    return null;
  }

  return (
    <div className="border-b border-border px-4 py-2">
      <details className="group">
        <summary className="cursor-pointer text-xs text-muted-foreground hover:text-foreground select-none">
          {t("chat.proactiveInsights")}
        </summary>
        <div className="mt-2 space-y-2">
          <ContextPredictionPanel context={context} />
          <ReminderList />
        </div>
      </details>
    </div>
  );
}
