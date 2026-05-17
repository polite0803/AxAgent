import { App, Button, Input, Modal, Spin, theme } from "antd";
import DOMPurify from "dompurify";
import { ChevronDown } from "lucide-react";
import NodeRenderer from "markstream-react";
import React, { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import { ModuleErrorBoundary } from "@/components/layout/ModuleErrorBoundary";
import { useResolvedDarkMode } from "@/hooks/useResolvedDarkMode";
import { logIpcError } from "@/lib/invoke";
import {
  setupAgentEventListeners,
  setupDreamEventListeners,
  setupPlanEventListeners,
  syncAllStoresToDomain,
  useAgentStore,
  useCompressStore,
  useConversationStore,
  useExpertStore,
  usePlanStore,
  useProviderStore,
  useSettingsStore,
  useStreamStore,
} from "@/stores";
import { useTopicGroupStore } from "@/stores/feature/topicGroupStore";

import { registerHighlight } from "stream-markdown";

import Bubble from "@ant-design/x/es/bubble";
import { ProactiveSuggestionBar } from "../proactive/ProactiveSuggestionBar";
import { AgentProgressBar } from "./AgentProgressBar";
import { BreadcrumbBar } from "./BreadcrumbBar";
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
import { ContextGraphPanel } from "./ContextGraphPanel";
import { ExpertSelector } from "./ExpertSelector";
import { ExtractMemoriesModal } from "./ExtractMemoriesModal";
import { InputArea } from "./InputArea";
import { PermissionModal } from "./PermissionModal";
import { PlanCard } from "./PlanCard";
import { QuickCommandBar } from "./QuickCommandBar";
import { WorkflowEndMarker } from "./WorkflowEndMarker";
import { WorkflowSuggestionCard } from "./WorkflowSuggestionCard";

import { useChatViewMessages } from "./ChatViewMessages";
import { BubbleStyleOverrides, StreamingStyles } from "./ChatViewStreaming";
import { ChatViewToolbar } from "./ChatViewToolbar";
import { ChatViewWelcome } from "./ChatViewWelcome";
import { useChatViewActions } from "./useChatViewActions";
import { useChatViewScroll } from "./useChatViewScroll";

function ChatViewInner({ onScrollToReady }: {
  onScrollToReady?: (
    api: { scrollTo: (messageId: string) => void; scrollBoxRef: React.RefObject<HTMLElement | null> },
  ) => void;
}) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const { message: messageApi } = App.useApp();

  const conversations = useConversationStore((s) => s.conversations);
  const activeConversationId = useConversationStore((s) => s.activeConversationId);
  const setActiveConversation = useConversationStore((s) => s.setActiveConversation);
  const messages = useConversationStore((s) => s.messages);
  const loading = useConversationStore((s) => s.loading);
  const loadingOlder = useConversationStore((s) => s.loadingOlder);
  const hasOlderMessages = useConversationStore((s) => s.hasOlderMessages);
  const activeStreams = useStreamStore((s) => s.activeStreams);
  const streaming = activeConversationId ? (activeConversationId in activeStreams) : false;
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

  const activeConversation = conversations.find((c) => c.id === activeConversationId);

  const [previewPayload, setPreviewPayload] = useState<CodeBlockPreviewPayload | null>(null);
  const [previewModalOpen, setPreviewModalOpen] = useState(false);
  const [mermaidPreviewSvg, setMermaidPreviewSvg] = useState<string | null>(null);
  const [mermaidPreviewOpen, setMermaidPreviewOpen] = useState(false);

  const { darkTheme: codeBlockDarkTheme, lightTheme: codeBlockLightTheme, themes: codeBlockThemes } = useMemo(
    () => getChatCodeThemes(settings.code_theme, settings.code_theme_light),
    [settings.code_theme, settings.code_theme_light],
  );

  const bubbleListThemeKey = `bubble-list:${isDarkMode ? "dark" : "light"}:${settings.code_theme ?? ""}:${
    settings.code_theme_light ?? ""
  }`;

  useEffect(() => {
    if (codeBlockThemes.length > 0) {
      registerHighlight({ themes: codeBlockThemes as import("@shikijs/types").ThemeInput[] }).catch(
        logIpcError("preload_highlight_themes"),
      );
    }
  }, [codeBlockThemes, codeBlockDarkTheme, codeBlockLightTheme, isDarkMode]);

  useEffect(() => {
    setCodeBlockPreviewHandler((payload: CodeBlockPreviewPayload) => {
      setPreviewPayload(payload);
      setPreviewModalOpen(true);
    });
    return () => {
      setCodeBlockPreviewHandler(null);
    };
  }, []);

  useEffect(() => {
    setMermaidOpenModalHandler((svgString: string | null) => {
      setMermaidPreviewSvg(svgString);
      setMermaidPreviewOpen(true);
    });
    return () => {
      setMermaidOpenModalHandler(null);
    };
  }, []);

  useEffect(() => {
    if (!activeConversationId) { return; }
    const conversation = conversations.find((c) => c.id === activeConversationId);
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

  const currentAgentStatus = useAgentStore(
    (s) => (activeConversationId ? s.agentStatus[activeConversationId] : undefined),
  );

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

  const topicGroupEnabled = useTopicGroupStore((s) =>
    activeConversationId ? s.enabledByConversation[activeConversationId] : false
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
    onScrollToReady?.({ scrollTo: scroll.minimapScrollTo, scrollBoxRef: scroll.scrollBoxRef });
  }, [scroll.minimapScrollTo]);

  const activeMessages = useMemo(
    () => messages.filter((msg) => msg.is_active !== false),
    [messages],
  );

  return (
    <div className="ax-cyber-grid flex flex-col h-full min-h-0">
      <StreamingStyles />
      <BubbleStyleOverrides />

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

      {(() => {
        const contextBarModel = activeConversation
          ? (() => {
            const provider = providers.find((p) => p.id === activeConversation.provider_id);
            const model = provider?.models.find((m) => m.model_id === activeConversation.model_id);
            return {
              name: model?.name ?? activeConversation.model_id,
              maxTokens: model?.max_tokens ?? activeConversation.max_tokens ?? undefined,
            };
          })()
          : null;

        return (
          <ContextBar
            modelName={contextBarModel?.name}
            searchEnabled={activeConversation?.search_enabled ?? false}
            toolCount={actions.toolCount}
            knowledgeCount={activeConversation?.enabled_knowledge_base_ids?.length ?? 0}
            memoryEnabled={(activeConversation?.enabled_memory_namespace_ids?.length ?? 0) > 0}
            tokenUsed={activeMessages.length > 0
              ? estimateConversationTokens(activeMessages.map(m => ({ role: m.role, content: m.content })))
              : undefined}
            tokenMax={contextBarModel?.maxTokens}
          />
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
        aria-label={t("chat.messageArea")}
        style={{
          ...(messages.length > 50
            ? {
              contentVisibility: "auto",
              containIntrinsicSize: "auto 5000px",
            }
            : {}),
        }}
      >
        {messages.length === 0
          ? (
            <ChatViewWelcome
              loading={loading}
              activeConversationId={activeConversationId}
              onPromptClick={actions.handlePromptClick}
              token={token}
            />
          )
          : (
            <>
              {activeConversationId && (() => {
                const ctxProvider = providers.find((p) => p.id === activeConversation?.provider_id);
                const ctxModel = ctxProvider?.models.find((m) => m.model_id === activeConversation?.model_id);
                return (
                  <div style={{ padding: "0 16px" }}>
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
              {msgState.hiddenEarlierCount > 0 && msgState.hiddenEarlierCount === msgState.allBubbleItems.length && (
                <div style={{ textAlign: "center", padding: "8px 0" }}>
                  <Button
                    size="small"
                    type="link"
                    loading={loadingOlder}
                    onClick={() => {
                      msgState.virtualizer.scrollToIndex(0, { behavior: "smooth" });
                    }}
                  >
                    {t("chat.showAllMessages", { count: msgState.allBubbleItems.length })}
                  </Button>
                </div>
              )}
              <Bubble.List
                key={bubbleListThemeKey}
                ref={bubbleListRef}
                items={msgState.visibleBubbleItems}
                autoScroll={false}
                onScroll={scroll.handleBubbleListScroll}
                role={msgState.roles}
                style={{
                  height: "100%",
                  padding: settings.chat_minimap_enabled && settings.chat_minimap_style === "sticky"
                    ? "50px 24px 16px 24px"
                    : "16px 24px",
                  overflowX: "hidden",
                }}
              />
              {activeConversation?.session_type === "workflow"
                && activeConversation?.workflow_status === "completed"
                && (
                  <WorkflowEndMarker
                    workflowName={activeConversation.workflow_template_id ?? t("chat.workflowLabel")}
                    stepCount={0}
                    completedCount={0}
                    durationSeconds={0}
                    onArchive={() => {
                      void toggleArchive(activeConversation.id);
                    }}
                  />
                )}
              {(() => {
                const suggestion = useAgentStore.getState().workflowMatchSuggestion;
                if (
                  suggestion
                  && suggestion.conversationId === activeConversation?.id
                  && activeConversation?.mode === "agent"
                ) {
                  return (
                    <WorkflowSuggestionCard
                      match={{
                        templateId: suggestion.templateId,
                        templateName: suggestion.templateName,
                        similarity: suggestion.similarity,
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
                  );
                }
                return null;
              })()}
              <ChatScrollIndicator />
              <MinimapScrollProvider scrollTo={scroll.minimapScrollTo} scrollBoxRef={scroll.scrollBoxRef}>
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

      {activeConversation?.mode === "agent" && activeConversationId && (
        <AgentProgressBar conversationId={activeConversationId} />
      )}

      {activeConversation?.mode === "agent" && activeConversationId && (
        <PlanCardWrapper conversationId={activeConversationId} />
      )}

      {activeConversation?.mode === "agent" && <QuickCommandBar />}

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
      <ExpertSelector
        open={actions.expertOpen}
        onClose={() => actions.setExpertOpen(false)}
        selectedRoleId={activeConversation?.expert_role_id ?? null}
        onSelect={(roleId) => {
          if (!activeConversationId) { return; }
          const expertStore = useExpertStore.getState();
          const role = expertStore.getRoleById(roleId);
          if (!role) { return; }

          updateConversation(activeConversationId, {
            expert_role_id: roleId,
            system_prompt: role.systemPrompt || undefined,
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
            updatePermissionMode(activeConversationId, role.recommendPermissionMode);
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
          <Button key="save" onClick={actions.handleEditSaveOnly} loading={actions.editSaving}>
            {t("chat.saveOnly")}
          </Button>,
          ...(actions.editingMessageRole === "assistant" ? [] : [
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
        payload={previewPayload}
        open={previewModalOpen}
        onClose={() => setPreviewModalOpen(false)}
      />
      <Modal
        title={`Mermaid ${t("common.preview")}`}
        open={mermaidPreviewOpen}
        onCancel={() => {
          setMermaidPreviewOpen(false);
          setMermaidPreviewSvg(null);
        }}
        footer={null}
        width="80vw"
        style={{ top: 32 }}
        styles={{ body: { height: "calc(80vh - 55px)", overflow: "auto", padding: 16 } }}
        destroyOnHidden
      >
        {mermaidPreviewSvg && (
          <div
            style={{ width: "100%", display: "flex", justifyContent: "center" }}
            dangerouslySetInnerHTML={{ __html: DOMPurify.sanitize(mermaidPreviewSvg) }}
          />
        )}
      </Modal>
      <ExtractMemoriesModal
        open={actions.extractMemoriesOpen}
        onClose={() => actions.setExtractMemoriesOpen(false)}
        conversationId={activeConversationId ?? ""}
      />
    </div>
  );
}

export interface ChatViewScrollApi {
  scrollTo: (messageId: string) => void;
  scrollBoxRef: React.RefObject<HTMLElement | null>;
}

export function ChatView({ onScrollToReady }: {
  onScrollToReady?: (api: ChatViewScrollApi) => void;
}) {
  return (
    <ModuleErrorBoundary moduleName="ChatView" showDetails={import.meta.env.DEV}>
      <ChatViewInner onScrollToReady={onScrollToReady} />
    </ModuleErrorBoundary>
  );
}

function PlanCardWrapper({ conversationId }: { conversationId: string }) {
  const plan = usePlanStore((s) => s.activePlans[conversationId]);
  if (!plan) { return null; }
  return (
    <div style={{ padding: "8px 16px" }}>
      <PlanCard plan={plan} conversationId={conversationId} />
    </div>
  );
}
