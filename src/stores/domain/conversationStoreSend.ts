// SPDX-License-Identifier: AGPL-3.0-only

// S-20: Send method factory extracted from conversationStore

/**
 * 会话级禁用工具列表的 localStorage key（与 contextLimit 的 localStorage 先例一致）。
 * 由 ConversationSettingsModal 读写、sendAgentMessage 读取后经 options.disabledTools 传给后端。
 */
export const DISABLED_TOOLS_KEY = (conversationId: string) => `axagent_disabled_tools_${conversationId}`;

/** 读取会话级禁用工具列表（无则返回空数组） */
export function getConversationDisabledTools(conversationId: string): string[] {
  try {
    const raw = localStorage.getItem(DISABLED_TOOLS_KEY(conversationId));
    if (!raw) {
      return [];
    }
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed.filter((x): x is string => typeof x === "string") : [];
  } catch {
    return [];
  }
}

import i18n from "@/i18n";
import { translateBackendError } from "@/lib/errorI18n";
import { invoke, isTauri, listen, logIpcError, type UnlistenFn } from "@/lib/invoke";
import { message } from "@/lib/toast";
import type {
  AgentDoneEvent,
  AgentErrorEvent,
  AgentStreamTextEvent,
  AgentStreamThinkingEvent,
  AttachmentInput,
  CognitiveCandidateSummary,
  CognitiveQueryResponse,
  Message,
  WorkflowCompleteEvent,
  WorkflowEvent,
} from "@/types";
import { useAgentStore } from "../feature/agentStore";
import { useCognitiveRouteStore } from "../feature/cognitiveRouteStore";
import { useExecutionStore } from "../feature/executionStore";
import { useAgentPanelStore } from "../shared/agentPanelStore";
import { useMultiModelStore } from "./multiModelStore";
import { getEffectiveThinkingBudget, usePreferenceStore } from "./preferenceStore";
import {
  _streamUiFlushTimer,
  getStreamingMessageId,
  isConversationStreaming as isConvStreaming,
  markStreamActivity,
  setPendingUiChunk,
  setStreamUiFlushTimer,
  startConversationStream,
  stopConversationStream,
  STREAM_UI_FLUSH_INTERVAL_MS,
  useStreamStore,
} from "./streamStore";

import { tempId } from "./conversationStore";

import type { ConversationState } from "./conversationStore";

/** 用户意图提示（显式覆盖执行模式，缺省 auto 由认知编排器自动决策） */
export type SendModeHint = "auto" | "ask" | "plan" | "act";

export interface SendMethods {
  /** 统一消息入口：所有会话（chat / plan / agent）统一走 cognitive_query 认知编排器。
   *  modeHint 仅在用户显式选择时传入，缺省 auto 由路由自动决策执行模式。 */
  sendMessage: (
    content: string,
    attachments?: AttachmentInput[],
    searchProviderId?: string | null,
    quotedMessageId?: string | null,
    modeHint?: SendModeHint,
    disabledTools?: string[],
    resumeClarify?: {
      /** Clarify 二次执行：强制路由到该能力（跳过三层路由） */
      capabilityId: string;
      /** 澄清时乐观创建的用户消息 ID（复用现有用户消息，避免重复插入） */
      userMessageId: string;
    },
  ) => Promise<void>;
  /** Clarify 二次执行：用户选中候选后携带 capabilityId 重新调用 cognitive_query。
   *  复用澄清时的用户消息与当前会话，避免重复插入用户消息。 */
  executeClarify: (capabilityId: string) => Promise<void>;
  regenerateMessage: (targetMessageId?: string) => Promise<void>;
  regenerateWithModel: (
    targetMessageId: string,
    providerId: string,
    modelId: string,
  ) => Promise<void>;
  sendMultiModelMessage: (
    content: string,
    companionModels: Array<{ providerId: string; modelId: string }>,
    attachments?: AttachmentInput[],
    searchProviderId?: string | null,
  ) => Promise<void>;
}

export function createSendMethods(
  set: (
    partial:
      | Partial<ConversationState>
      | ((s: ConversationState) => Partial<ConversationState>),
  ) => void,
  get: () => ConversationState,
): SendMethods {
  return {
    sendMessage: async (
      content: string,
      attachments: AttachmentInput[] = [],
      searchProviderId: string | null = null,
      quotedMessageId: string | null = null,
      modeHint: SendModeHint = "auto",
      disabledTools?: string[],
      resumeClarify?: {
        capabilityId: string;
        userMessageId: string;
      },
    ) => {
      const conversationId = get().activeConversationId;
      if (!conversationId) {
        throw new Error("No active conversation");
      }

      let conversation = get().conversations.find(
        (c) => c.id === conversationId,
      );
      if (!conversation) {
        throw new Error("Conversation not found");
      }

      // Clarify 二次执行：复用澄清时乐观创建的用户消息，不重复插入用户消息
      const isResumeClarify = !!resumeClarify;
      let resumeUserMessage: Message | null = null;
      if (isResumeClarify) {
        resumeUserMessage = get().messages.find((m) => m.id === resumeClarify!.userMessageId)
          ?? null;
        if (!resumeUserMessage) {
          throw new Error("Clarify user message not found");
        }
      }

      // Guard: prevent duplicate sends while a stream is already active for this conversation
      if (
        isConvStreaming(useStreamStore.getState().activeStreams, conversationId)
      ) {
        return;
      }

      // 注：浏览器 mock 模式（npm run dev / E2E）下 sendMessage 不再短路——
      // 统一走 cognitive_query，由 browserMock 模拟后端（含 P0-2 计划确认闸门）。
      // isTauri 专属的 Agent 分支守卫已由 cognitive_query 路由内部的
      // executionMode 判断承担，这里移除 2e9af1be 引入的误提短路。

      const providerId = conversation.providerId;
      const modelId = conversation.modelId;

      // Optimistic user message（Clarify 二次执行时复用现有用户消息，不新建）
      const optimisticUserMsg: Message = isResumeClarify
        ? resumeUserMessage!
        : {
          id: tempId("temp-user-"),
          conversationId: conversationId,
          role: "user",
          content,
          providerId: null,
          modelId: null,
          tokenCount: null,
          attachments: attachments.map((a) => ({
            id: tempId("temp-att-"),
            fileName: a.fileName,
            fileType: a.fileType,
            filePath: "",
            fileSize: a.fileSize,
            data: a.data,
          })),
          thinking: null,
          toolCallsJson: null,
          toolCallId: null,
          createdAt: Date.now(),
          parentMessageId: null,
          versionIndex: 0,
          isActive: true,
          status: "complete",
          // 引用回复：乐观更新时携带 quotedMessageId，确保 UI 立即显示引用块
          quotedMessageId: quotedMessageId,
        };

      // Placeholder assistant message
      let currentMsgId = `temp-agent-${Date.now()}`;
      const placeholderAssistant: Message = {
        id: currentMsgId,
        conversationId: conversationId,
        role: "assistant",
        content: i18n.t("agentMode.thinking"),
        providerId: providerId,
        modelId: modelId,
        tokenCount: null,
        attachments: [],
        thinking: null,
        toolCallsJson: null,
        toolCallId: null,
        createdAt: Date.now(),
        parentMessageId: optimisticUserMsg.id,
        versionIndex: 0,
        isActive: true,
        status: "partial",
      };

      set((s) => ({
        messages: isResumeClarify
          // Clarify 二次执行：用户消息已在消息流中，只追加占位 assistant 消息
          ? [...s.messages, placeholderAssistant]
          : [...s.messages, optimisticUserMsg, placeholderAssistant],
      }));
      useStreamStore.setState((s) => ({
        ...startConversationStream(
          s.activeStreams,
          conversationId,
          currentMsgId,
        ),
        streamingStartTimestamps: {
          ...s.streamingStartTimestamps,
          [conversationId]: Date.now(),
        },
      }));

      let unlistenDone: UnlistenFn | null = null;
      let unlistenError: UnlistenFn | null = null;
      let unlistenStreamText: UnlistenFn | null = null;
      let unlistenStreamThinking: UnlistenFn | null = null;
      let unlistenMessageId: UnlistenFn | null = null;
      let unlistenWorkflowComplete: UnlistenFn | null = null;
      let unlistenStatus: UnlistenFn | null = null;

      const AGENT_TIMEOUT_MS = 10 * 60 * 1000;
      let _agentReject: ((reason: Error) => void) | null = null;

      const onAgentTimeout = (messageKey: "agentMode.timeout" | "agentMode.timeoutShort") => {
        if (
          !isConvStreaming(
            useStreamStore.getState().activeStreams,
            conversationId,
          )
        ) {
          return;
        }
        cleanup();
        set((s) => ({
          messages: s.messages.map((m) =>
            m.id === currentMsgId
              ? {
                ...m,
                content: i18n.t(messageKey),
                status: "error" as const,
              }
              : m
          ),
        }));
        useStreamStore.setState((s) => ({
          ...stopConversationStream(s.activeStreams, conversationId),
          streamingStartTimestamps: (() => {
            const t = { ...s.streamingStartTimestamps };
            delete t[conversationId];
            return t;
          })(),
        }));
        if (_agentReject) {
          _agentReject(new Error(i18n.t(messageKey)));
        }
      };

      const resetAgentTimeout = () => {
        if (timeoutId !== null) {
          clearTimeout(timeoutId);
        }
        timeoutId = setTimeout(() => onAgentTimeout("agentMode.timeout"), AGENT_TIMEOUT_MS);
      };

      let timeoutId: ReturnType<typeof setTimeout> | null = setTimeout(
        () => onAgentTimeout("agentMode.timeout"),
        AGENT_TIMEOUT_MS,
      );

      // ── Agent stream buffering (same pattern as Q&A _pendingUiChunk) ──
      let _agentPendingText = "";
      let _agentPendingThinking = "";
      // ── Agent stream buffer & flush (priority-tiered) ──
      //
      // Agent events produce text, thinking, and workflow updates concurrently.
      // Rendering everything at the same 50ms cadence creates unnecessary re-renders
      // for low-urgency content (thinking, workflow steps). We split into two timers:
      //
      //   P1 (text):     50ms flush — user-visible text must feel responsive
      //   P2 (thinking): 200ms flush — thinking is background context, low urgency
      //   P3 (workflow): piggybacks on text flush — no independent timer
      //
      // Tool-call events (P0) are handled by agentStore.ts separately; they trigger
      // immediate UI updates without buffering.

      const AGENT_THINKING_FLUSH_MS = 200;

      let _agentFlushTimer: ReturnType<typeof setTimeout> | null = null;
      let _agentThinkingFlushTimer: ReturnType<typeof setTimeout> | null = null;

      const flushAgentTextChunks = () => {
        if (_agentFlushTimer !== null) {
          clearTimeout(_agentFlushTimer);
          _agentFlushTimer = null;
        }
        const textChunk = _agentPendingText;
        _agentPendingText = "";
        if (!textChunk) {
          return;
        }

        // Guard: don't update messages if user switched to a different conversation
        if (get().activeConversationId !== conversationId) {
          return;
        }

        set((s) => {
          const wasThinking = useStreamStore
            .getState()
            .thinkingActiveMessageIds.has(currentMsgId);
          let nextThinkingIds = useStreamStore.getState().thinkingActiveMessageIds;

          const updatedMessages = s.messages.map((m) => {
            if (m.id !== currentMsgId) {
              return m;
            }
            let content = m.content || "";

            // Close thinking block if we were in thinking mode
            if (wasThinking) {
              content += "\n</think>\n\n";
              const n = new Set(nextThinkingIds);
              n.delete(currentMsgId);
              nextThinkingIds = n;
            }
            content += textChunk;
            return { ...m, content };
          });

          useStreamStore.setState({
            thinkingActiveMessageIds: nextThinkingIds,
          });
          return { messages: updatedMessages };
        });
      };

      const flushAgentThinkingChunks = () => {
        if (_agentThinkingFlushTimer !== null) {
          clearTimeout(_agentThinkingFlushTimer);
          _agentThinkingFlushTimer = null;
        }
        const thinkingChunk = _agentPendingThinking;
        _agentPendingThinking = "";
        if (!thinkingChunk) {
          return;
        }

        // Guard: don't update messages if user switched to a different conversation
        if (get().activeConversationId !== conversationId) {
          return;
        }

        set((s) => {
          const wasThinking = useStreamStore
            .getState()
            .thinkingActiveMessageIds.has(currentMsgId);
          let nextThinkingIds = useStreamStore.getState().thinkingActiveMessageIds;

          const updatedMessages = s.messages.map((m) => {
            if (m.id !== currentMsgId) {
              return m;
            }
            let content = m.content || "";
            let thinking = m.thinking || "";

            if (!wasThinking) {
              content += '<think data-axagent="1">\n';
            }
            content += thinkingChunk;
            thinking += thinkingChunk;
            nextThinkingIds = new Set([...nextThinkingIds, currentMsgId]);

            return { ...m, content, thinking };
          });

          useStreamStore.setState({
            thinkingActiveMessageIds: nextThinkingIds,
          });
          return { messages: updatedMessages };
        });
      };

      const scheduleAgentFlush = () => {
        if (_agentFlushTimer === null) {
          _agentFlushTimer = setTimeout(
            flushAgentTextChunks,
            STREAM_UI_FLUSH_INTERVAL_MS,
          );
        }
      };

      const scheduleAgentThinkingFlush = () => {
        if (_agentThinkingFlushTimer === null) {
          _agentThinkingFlushTimer = setTimeout(
            flushAgentThinkingChunks,
            AGENT_THINKING_FLUSH_MS,
          );
        }
      };

      const handleWorkflowEvent = (event: WorkflowEvent) => {
        const text = formatWorkflowEventAsText(event);
        if (text) {
          _agentPendingText += text;
          // P3: Workflow events are lazy — they piggyback on the next text/thinking flush.
          // 对话驱动工作流模式下没有 TextDelta 事件，必须主动调度 flush 才能实时渲染步骤。
          scheduleAgentFlush();
        }
      };

      const formatWorkflowEventAsText = (event: WorkflowEvent): string => {
        switch (event.type) {
          case "workflow_start":
            return `\n[Workflow Started: ${event.workflowId}]\n`;
          case "workflow_step_start":
            return `\n[Step Start] ${event.agentRole}: ${event.stepGoal}\n`;
          case "workflow_step_complete":
            return `[Step Complete] ${event.stepGoal}: ${event.result}\n`;
          case "workflow_step_error":
            return `[Step Error] ${event.stepId}: ${event.error}\n`;
          default:
            return "";
        }
      };

      const clearAgentStreamBuffer = () => {
        if (_agentFlushTimer !== null) {
          clearTimeout(_agentFlushTimer);
          _agentFlushTimer = null;
        }
        if (_agentThinkingFlushTimer !== null) {
          clearTimeout(_agentThinkingFlushTimer);
          _agentThinkingFlushTimer = null;
        }
        _agentPendingText = "";
        _agentPendingThinking = "";
      };

      const cleanup = () => {
        clearAgentStreamBuffer();
        if (timeoutId !== null) {
          clearTimeout(timeoutId);
          timeoutId = null;
        }
        unlistenStreamText?.();
        unlistenStreamThinking?.();
        unlistenDone?.();
        unlistenError?.();
        unlistenMessageId?.();
        unlistenWorkflowComplete?.();
        unlistenStatus?.();
        unlistenStreamText = null;
        unlistenStreamThinking = null;
        unlistenDone = null;
        unlistenError = null;
        unlistenMessageId = null;
        unlistenWorkflowComplete = null;
        unlistenStatus = null;
      };

      try {
        const eventPromise = new Promise<void>((resolve, reject) => {
          _agentReject = reject;
          // Listen for the real assistant message ID from the backend
          // This replaces the temp ID so tool call events can be matched
          listen<{ conversationId: string; assistantMessageId: string }>(
            "agent-message-id",
            (event) => {
              if (event.payload.conversationId !== conversationId) {
                return;
              }
              markStreamActivity(conversationId);
              resetAgentTimeout();
              flushAgentTextChunks();
              flushAgentThinkingChunks();
              const realId = event.payload.assistantMessageId;
              const oldId = currentMsgId;
              currentMsgId = realId;
              useStreamStore.setState((s) => ({
                ...startConversationStream(
                  s.activeStreams,
                  conversationId,
                  realId,
                ),
                streamingMessageId: realId,
              }));
              set((s) => ({
                messages: s.messages.map((m) => m.id === oldId ? { ...m, id: realId } : m),
              }));
            },
          ).then((fn) => {
            unlistenMessageId = fn;
          });

          // Listen for incremental text chunks — buffer and flush periodically
          listen<AgentStreamTextEvent | WorkflowEvent>(
            "agent-stream-text",
            (event) => {
              if (event.payload.conversationId !== conversationId) {
                return;
              }
              markStreamActivity(conversationId);
              resetAgentTimeout();

              if ("type" in event.payload) {
                handleWorkflowEvent(event.payload as WorkflowEvent);
                return;
              }

              // Regular text event
              _agentPendingText += event.payload.text;
              scheduleAgentFlush();
            },
          ).then((fn) => {
            unlistenStreamText = fn;
          });

          // Listen for incremental thinking chunks — buffer and flush periodically
          listen<AgentStreamThinkingEvent>("agent-stream-thinking", (event) => {
            if (event.payload.conversationId !== conversationId) {
              return;
            }
            markStreamActivity(conversationId);
            resetAgentTimeout();
            _agentPendingThinking += event.payload.thinking;
            scheduleAgentThinkingFlush();
          }).then((fn) => {
            unlistenStreamThinking = fn;
          });

          // Listen for agent-done — correction overwrite with final content
          listen<AgentDoneEvent>("agent-done", (event) => {
            if (event.payload.conversationId !== conversationId) {
              return;
            }
            markStreamActivity(conversationId);
            // 对话驱动工作流：保留已流式的步骤事件（不做覆盖），先把缓冲落盘，
            // 再在尾部追加结果；普通 agent 会话维持原有覆盖行为。
            if (isWorkflowDriven) {
              flushAgentTextChunks();
              flushAgentThinkingChunks();
            } else {
              // Clear pending buffer (done event overwrites with final content)
              clearAgentStreamBuffer();
            }
            // Skip if streaming was already cancelled (avoid stale fetchMessages re-render)
            const isStillStreaming = isConvStreaming(
              useStreamStore.getState().activeStreams,
              conversationId,
            );
            if (!isStillStreaming) {
              cleanup();
              resolve();
              return;
            }

            useStreamStore.setState((s) => ({
              ...stopConversationStream(s.activeStreams, conversationId),
              streamingStartTimestamps: (() => {
                const t = { ...s.streamingStartTimestamps };
                delete t[conversationId];
                return t;
              })(),
              thinkingActiveMessageIds: (() => {
                const next = new Set(s.thinkingActiveMessageIds);
                next.delete(currentMsgId);
                return next;
              })(),
            }));
            set((s) => ({
              messages: s.messages.map((m) => {
                if (m.id === currentMsgId) {
                  // workflow 会话：保留步骤事件，结果追加在尾部
                  // 普通会话：用最终内容重建（thinking 包装为 <think> 块，与流式格式一致）
                  let finalContent = isWorkflowDriven ? (m.content || "") : "";
                  const thinkingText = event.payload.thinking;
                  if (!isWorkflowDriven && thinkingText) {
                    finalContent = `<think data-axagent="1">\n${thinkingText}\n</think>\n\n`;
                  }
                  finalContent += event.payload.text;

                  return {
                    ...m,
                    id: event.payload.assistantMessageId || m.id,
                    content: finalContent,
                    thinking: thinkingText || m.thinking,
                    status: "complete" as const,
                    prompt_tokens: event.payload.usage?.inputTokens ?? null,
                    completion_tokens: event.payload.usage?.outputTokens ?? null,
                    blocks: event.payload.blocks ?? m.blocks,
                  } as Message;
                }
                return m;
              }),
            }));

            cleanup();
            // Fetch messages to fully sync with backend (real user message ID, etc.)
            get().fetchMessages(conversationId);
            resolve();
          }).then((fn) => {
            unlistenDone = fn;
          });

          // Listen for workflow-complete
          listen<WorkflowCompleteEvent>("workflow-complete", (event) => {
            if (event.payload.conversationId !== conversationId) {
              return;
            }
            const text = event.payload.success
              ? `\n[Workflow Complete: ${event.payload.workflowId}]\n`
              : `\n[Workflow Failed: ${event.payload.workflowId}]\n`;
            _agentPendingText += text;
            // P3: Lazy — piggybacks on next text flush, no independent timer
          }).then((fn) => {
            unlistenWorkflowComplete = fn;
          });

          // Listen for agent-error
          listen<AgentErrorEvent>("agent-error", (event) => {
            if (event.payload.conversationId !== conversationId) {
              return;
            }
            // Clear pending buffer (error event overwrites content)
            clearAgentStreamBuffer();
            // Skip if streaming was already cancelled
            const isStillStreaming = isConvStreaming(
              useStreamStore.getState().activeStreams,
              conversationId,
            );
            if (!isStillStreaming) {
              cleanup();
              resolve();
              return;
            }

            useStreamStore.setState((s) => ({
              ...stopConversationStream(s.activeStreams, conversationId),
              streamingStartTimestamps: (() => {
                const t = { ...s.streamingStartTimestamps };
                delete t[conversationId];
                return t;
              })(),
              thinkingActiveMessageIds: (() => {
                const next = new Set(s.thinkingActiveMessageIds);
                next.delete(currentMsgId);
                return next;
              })(),
            }));
            set((s) => ({
              messages: s.messages.map((m) => {
                if (m.id === currentMsgId) {
                  return {
                    ...m,
                    content: event.payload.message,
                    status: "error" as const,
                  } as Message;
                }
                return m;
              }),
            }));

            // Sync messages from DB so temp- prefixed user messages get replaced
            // with real backend IDs, enabling regenerate after an agent error.
            // Preserve the optimistic user message — if agent_query failed before
            // persisting it, fetchMessages would otherwise drop the user's input.
            get().fetchMessages(conversationId, [optimisticUserMsg.id]);
            cleanup();
            reject(new Error(event.payload.message));
          }).then((fn) => {
            unlistenError = fn;
          });
        });

        // Listen for agent status updates — update placeholder message to show progress
        listen<{ conversationId: string; phase: string; message: string }>(
          "agent-status",
          (event) => {
            if (event.payload.conversationId !== conversationId) {
              return;
            }
            markStreamActivity(conversationId);
            resetAgentTimeout();
            set((s) => ({
              messages: s.messages.map((m) =>
                m.id === currentMsgId
                  ? { ...m, thinking: `🔄 ${event.payload.message}` }
                  : m
              ),
            }));
          },
        ).then((fn) => {
          unlistenStatus = fn;
        });

        // Invoke the backend command (this creates the real user message in DB)
        // agent_query can run for a very long time (10+ minutes for complex tasks).
        // We must NOT use the default 5-minute invoke timeout — the backend continues
        // running and we rely on agent-done/agent-error events for completion.
        // Setting timeoutMs=0 disables the invoke-level timeout entirely.

        // 读取当前页面上下文注入到请求中
        const agentContext = useAgentPanelStore.getState().agentContext;
        const agentContextPayload = agentContext
          ? {
            page: agentContext.page,
            url: agentContext.url,
            quick_actions: agentContext.quickActions?.map((a) => ({
              id: a.id,
              description: a.description,
              require_confirmation: a.requireConfirmation ?? false,
            })) ?? [],
            data: agentContext.data ?? null,
          }
          : undefined;

        // 会话级禁用工具列表（ConversationSettingsModal 配置，localStorage 持久化）。
        // 传入后后端不会将这些工具交给 LLM，也不会执行。
        const effectiveDisabledTools = disabledTools
          ?? getConversationDisabledTools(conversationId);

        // 统一消息入口：所有会话（chat / plan / agent）统一走 cognitive_query 认知编排器。
        // 先完成三层路由决策，再按 executionMode 分发执行；后端已同步调用对应执行器：
        // - Workflow / ParameterExtract → WorkEngine 已执行命中的工作流模板
        // - Delegate / Ask / Act → agent_query 已执行（前端监听 agent-done 呈现）
        // - Plan → plan_generate 已触发（planStore 监听 plan-generated 渲染 PlanCard）
        // - Clarify → 返回 Top2 候选，前端渲染候选卡片供用户选择后二次执行
        let isWorkflowDriven = false;

        const cognitiveResult = await invoke<CognitiveQueryResponse>(
          "cognitive_query",
          {
            request: {
              input: content,
              conversationId,
              providerId,
              modelId,
              // Clarify 二次执行：强制路由到用户选中的能力
              forcedCapabilityId: isResumeClarify
                ? resumeClarify!.capabilityId
                : undefined,
              agentProfileId: conversation.agentProfileId ?? undefined,
              systemPrompt: conversation.systemPrompt ?? undefined,
              searchProviderId: searchProviderId ?? undefined,
              agentContext: agentContextPayload,
              options: {
                disabledTools: effectiveDisabledTools.length > 0
                  ? effectiveDisabledTools
                  : undefined,
                // P0-2 计划确认闸门：开关开启时要求后端对复杂任务先出计划草稿等待批准
                requirePlanApproval: useAgentStore.getState().planApprovalEnabled
                  || undefined,
              },
              // 用户意图提示：仅在显式选择时传入，缺省 auto 由路由自动决策
              modeHint: modeHint !== "auto" ? modeHint : undefined,
            },
          },
          0,
        );

        // ── 认知编排执行分支分发 ──
        const execKind = cognitiveResult?.execution?.kind;
        // 记录路由观测
        if (cognitiveResult) {
          useCognitiveRouteStore.getState().recordObservation(
            conversationId,
            cognitiveResult,
          );
        }
        if (execKind === "workflow") {
          // 后端已执行 WorkEngine，前端只需按工作流事件流呈现（保留步骤事件，不做覆盖重建）。
          isWorkflowDriven = true;
        } else if (execKind === "clarify") {
          // 澄清分支：模糊命中（0.60 ≤ 置信度 ≤ 0.90），停止占位流并把候选交 UI 呈现，
          // 用户选择候选后携带 capability_id 二次执行。
          const candidates = (cognitiveResult?.execution as {
            candidates?: CognitiveCandidateSummary[];
          })?.candidates ?? [];
          useStreamStore.setState((s) => ({
            ...stopConversationStream(s.activeStreams, conversationId),
            streamingStartTimestamps: (() => {
              const t = { ...s.streamingStartTimestamps };
              delete t[conversationId];
              return t;
            })(),
          }));
          set((s) => ({
            messages: s.messages.map((m) =>
              m.id === currentMsgId
                ? {
                  ...m,
                  content: i18n.t("cognitive.clarifyPrompt"),
                  status: "complete" as const,
                }
                : m
            ),
          }));
          get().setPendingClarification({
            candidates,
            originalInput: content,
            conversationId,
            userMessageId: optimisticUserMsg.id,
          });
          cleanup();
          window.setTimeout(() => {
            void get().fetchMessages(conversationId, [optimisticUserMsg.id]);
          }, 120);
          return;
        } else if (execKind === "plan") {
          // Plan 分支：后端已触发 plan_generate，planStore 监听 plan-generated 渲染 PlanCard。
          // 停止占位流并更新为"计划已生成"提示，等待 PlanCard 接管。
          useStreamStore.setState((s) => ({
            ...stopConversationStream(s.activeStreams, conversationId),
            streamingStartTimestamps: (() => {
              const t = { ...s.streamingStartTimestamps };
              delete t[conversationId];
              return t;
            })(),
          }));
          set((s) => ({
            messages: s.messages.map((m) =>
              m.id === currentMsgId
                ? {
                  ...m,
                  content: i18n.t("agentMode.planGenerated"),
                  status: "complete" as const,
                }
                : m
            ),
          }));
          cleanup();
          window.setTimeout(() => {
            void get().fetchMessages(conversationId, [optimisticUserMsg.id]);
          }, 120);
          return;
        }

        // 认知路由命中的 Workflow 分支由后端执行，前端无需重复触发。

        // 计划确认被用户拒绝（P0-2）：后端直接返回 rejected，不会发 agent-done/agent-error。
        // cognitive_query 的 Agent 执行分支透传 agent_query 的 status。
        const isRejected = cognitiveResult?.execution?.kind === "agent"
          && cognitiveResult.execution.status === "rejected";
        if (isRejected) {
          set((s) => ({
            messages: s.messages.filter((m) => m.id !== currentMsgId),
          }));
          cleanup();
          message.info(i18n.t("planApproval.rejectedToast"));
          return;
        }
        // Wait for agent-done or agent-error event
        await eventPromise;
      } catch (e) {
        // Safeguard: ensure listeners are always cleaned up, even if cleanup() itself throws
        try {
          cleanup();
        } catch {
          /* ignore cleanup errors */
        }
        const errMsg = translateBackendError(e);
        logIpcError("sendMessage")(errMsg);

        // Stale guard: user switched conversations while agent was running
        if (get().activeConversationId !== conversationId) {
          return;
        }

        // Only set error state if the message doesn't already have an error state
        // (agent-error event listener may have already set it with the backend message)
        const currentMsgs = get().messages;
        const msgAlreadyHasError = currentMsgs.some(
          (m) => m.id === currentMsgId && m.status === "error",
        );
        if (msgAlreadyHasError) {
          // agent-error event already handled the failure — no duplicate needed
          return;
        }

        // If streaming is still true, the error came from invoke itself (not an event)
        if (
          isConvStreaming(
            useStreamStore.getState().activeStreams,
            conversationId,
          )
        ) {
          useStreamStore.setState((s) => ({
            ...stopConversationStream(s.activeStreams, conversationId),
            streamingStartTimestamps: (() => {
              const t = { ...s.streamingStartTimestamps };
              delete t[conversationId];
              return t;
            })(),
          }));
          set((s) => ({
            messages: s.messages.map((m) =>
              m.id === currentMsgId
                ? { ...m, content: errMsg, status: "error" as const }
                : m
            ),
          }));
        }
        // Clean up agent/execution state for this conversation since the send failed.
        // The conversation itself is not being deleted — just the execution attempt failed.
        useAgentStore.getState().clearStatus(conversationId);
        useExecutionStore.getState().clearConversation(conversationId);

        // Sync messages from DB so temp- prefixed user messages get replaced
        // with real backend IDs, enabling regenerate after an agent send failure.
        // Preserve the optimistic user message to prevent it from being dropped
        // when agent_query failed before persisting the user message.
        window.setTimeout(() => {
          void get().fetchMessages(conversationId, [optimisticUserMsg.id]);
        }, 120);
      }
    },

    executeClarify: async (capabilityId: string) => {
      const pending = get().pendingClarification;
      if (!pending) {
        return;
      }
      if (pending.conversationId !== get().activeConversationId) {
        // 会话已切换：清空过期的澄清状态，避免误执行
        get().setPendingClarification(null);
        return;
      }
      // 清空澄清状态，交由 sendMessage 二次执行
      get().setPendingClarification(null);
      await get().sendMessage(
        pending.originalInput,
        [],
        null,
        null,
        "auto",
        undefined,
        { capabilityId, userMessageId: pending.userMessageId },
      );
    },

    regenerateMessage: async (targetMessageId?: string) => {
      const conversationId = get().activeConversationId;
      if (!conversationId) {
        throw new Error("No active conversation");
      }

      // Guard: prevent duplicate sends while a stream is already active for this conversation
      if (
        isConvStreaming(useStreamStore.getState().activeStreams, conversationId)
      ) {
        return;
      }

      const msgs = get().messages;
      // Find the user message (either specific or last one)
      let userMsg: Message | undefined;
      if (targetMessageId) {
        // Find the AI message, then its parent user message
        const aiMsg = msgs.find((m) => m.id === targetMessageId);
        if (aiMsg?.parentMessageId) {
          userMsg = msgs.find((m) => m.id === aiMsg.parentMessageId);
        }
      }
      if (!userMsg) {
        for (let i = msgs.length - 1; i >= 0; i--) {
          if (msgs[i].role === "user") {
            userMsg = msgs[i];
            break;
          }
        }
      }
      if (!userMsg) {
        throw new Error("No user message found");
      }

      // Guard: reject temp IDs that haven't been persisted to the backend yet
      if (userMsg.id.startsWith("temp-")) {
        throw new Error(
          "Message is still being sent. Please wait and try again.",
        );
      }

      // Create placeholder for new version, preserving original created_at for position
      const tempAssistantId = tempId("temp-assistant-");
      const parentId = userMsg.id;

      // Find the original active AI message to preserve its created_at
      const originalAiMsg = msgs.find(
        (m) => m.parentMessageId === parentId && m.isActive,
      );
      const placeholderAssistant: Message = {
        id: tempAssistantId,
        conversationId: conversationId,
        role: "assistant",
        content: "",
        providerId: originalAiMsg?.providerId ?? null,
        modelId: originalAiMsg?.modelId ?? null,
        tokenCount: null,
        attachments: [],
        thinking: null,
        toolCallsJson: null,
        toolCallId: null,
        createdAt: originalAiMsg?.createdAt ?? Date.now(),
        parentMessageId: userMsg.id,
        versionIndex: 0,
        isActive: true,
        status: "partial",
      };

      // Replace the active AI message in-place with placeholder (preserve position)
      set((s) => {
        let inserted = false;
        const updated: Message[] = [];
        for (const m of s.messages) {
          if (m.parentMessageId === parentId && m.isActive) {
            updated.push({ ...m, isActive: false });
            if (!inserted) {
              updated.push(placeholderAssistant);
              inserted = true;
            }
          } else {
            updated.push(m);
          }
        }
        if (!inserted) {
          updated.push(placeholderAssistant);
        }
        return {
          messages: updated,
        };
      });
      useStreamStore.setState((s) => ({
        ...startConversationStream(
          s.activeStreams,
          conversationId,
          tempAssistantId,
        ),
        streamingStartTimestamps: {
          ...s.streamingStartTimestamps,
          [conversationId]: Date.now(),
        },
        thinkingActiveMessageIds: new Set<string>(),
      }));
      setPendingUiChunk(null);
      if (_streamUiFlushTimer !== null) {
        clearTimeout(_streamUiFlushTimer);
        setStreamUiFlushTimer(null);
      }

      try {
        const rMcpIds = usePreferenceStore.getState().enabledMcpServerIds;
        const rThinkingBudget = getEffectiveThinkingBudget(conversationId);
        const rKbIds = usePreferenceStore.getState().enabledKnowledgeBaseIds;
        const rMemNsId = usePreferenceStore.getState().activeMemoryNamespaceId;
        const rMemIds = rMemNsId ? [rMemNsId] : [];
        const rWikiIds = usePreferenceStore.getState().enabledWikiIds;
        await invoke("regenerate_message", {
          params: {
            conversationId,
            userMessageId: userMsg.id,
            options: {
              enabledMcpServerIds: rMcpIds.length > 0 ? rMcpIds : undefined,
              thinkingBudget: rThinkingBudget,
              enabledKnowledgeBaseIds: rKbIds.length > 0 ? rKbIds : undefined,
              enabledMemoryNamespaceIds: rMemIds.length > 0 ? rMemIds : undefined,
              enabledWikiIds: rWikiIds.length > 0 ? rWikiIds : undefined,
            },
          },
        });

        // In browser mode, simulate brief loading then fetch the mock AI response
        if (!isTauri()) {
          await new Promise((r) => setTimeout(r, 600));
          useStreamStore.setState((s) => ({
            ...stopConversationStream(s.activeStreams, conversationId),
            streamingStartTimestamps: (() => {
              const t = { ...s.streamingStartTimestamps };
              delete t[conversationId];
              return t;
            })(),
            thinkingActiveMessageIds: new Set<string>(),
          }));
          get().fetchMessages(conversationId);
        }
      } catch (e) {
        logIpcError("regenerateMessage", { notify: true })(e);
        const errMsg = translateBackendError(e);
        const currentStreamingMessageId = getStreamingMessageId(
          useStreamStore.getState().activeStreams,
          conversationId,
        );
        useStreamStore.setState((s) => ({
          ...stopConversationStream(s.activeStreams, conversationId),
          streamingStartTimestamps: (() => {
            const t = { ...s.streamingStartTimestamps };
            delete t[conversationId];
            return t;
          })(),
          thinkingActiveMessageIds: new Set<string>(),
        }));
        set((s) => ({
          messages: currentStreamingMessageId
            ? s.messages.map((m) =>
              m.id === currentStreamingMessageId
                ? { ...m, content: errMsg, status: "error" as const }
                : m
            )
            : s.messages,
        }));
      }
    },

    regenerateWithModel: async (
      targetMessageId: string,
      providerId: string,
      modelId: string,
    ) => {
      const conversationId = get().activeConversationId;
      if (!conversationId) {
        throw new Error("No active conversation");
      }

      const msgs = get().messages;
      // Find the AI message, then its parent user message
      const aiMsg = msgs.find((m) => m.id === targetMessageId);
      if (!aiMsg?.parentMessageId) {
        throw new Error("Cannot find parent user message");
      }
      const userMsg = msgs.find((m) => m.id === aiMsg.parentMessageId);
      if (!userMsg) {
        throw new Error("User message not found");
      }

      const parentId = userMsg.id;
      const originalAiMsg = msgs.find(
        (m) => m.parentMessageId === parentId && m.isActive,
      );

      // Create placeholder with the target model info
      const tempAssistantId = tempId("temp-assistant-");
      const placeholderAssistant: Message = {
        id: tempAssistantId,
        conversationId: conversationId,
        role: "assistant",
        content: "",
        providerId: providerId,
        modelId: modelId,
        tokenCount: null,
        attachments: [],
        thinking: null,
        toolCallsJson: null,
        toolCallId: null,
        createdAt: originalAiMsg?.createdAt ?? Date.now(),
        parentMessageId: userMsg.id,
        versionIndex: 0,
        isActive: true,
        status: "partial",
      };

      // Replace the active AI message in-place with placeholder
      set((s) => {
        let inserted = false;
        const updated: Message[] = [];
        for (const m of s.messages) {
          if (m.parentMessageId === parentId && m.isActive) {
            updated.push({ ...m, isActive: false });
            if (!inserted) {
              updated.push(placeholderAssistant);
              inserted = true;
            }
          } else {
            updated.push(m);
          }
        }
        if (!inserted) {
          updated.push(placeholderAssistant);
        }
        return {
          messages: updated,
        };
      });
      useStreamStore.setState((s) => ({
        ...startConversationStream(
          s.activeStreams,
          conversationId,
          tempAssistantId,
        ),
        streamingStartTimestamps: {
          ...s.streamingStartTimestamps,
          [conversationId]: Date.now(),
        },
        thinkingActiveMessageIds: new Set<string>(),
      }));
      setPendingUiChunk(null);
      if (_streamUiFlushTimer !== null) {
        clearTimeout(_streamUiFlushTimer);
        setStreamUiFlushTimer(null);
      }

      try {
        const rMcpIds = usePreferenceStore.getState().enabledMcpServerIds;
        const rThinkingBudget = getEffectiveThinkingBudget(conversationId);
        const rKbIds = usePreferenceStore.getState().enabledKnowledgeBaseIds;
        const rMemNsId2 = usePreferenceStore.getState().activeMemoryNamespaceId;
        const rMemIds = rMemNsId2 ? [rMemNsId2] : [];
        const rWikiIds = usePreferenceStore.getState().enabledWikiIds;
        await invoke("regenerate_with_model", {
          params: {
            conversationId,
            userMessageId: userMsg.id,
            targetProviderId: providerId,
            targetModelId: modelId,
            options: {
              enabledMcpServerIds: rMcpIds.length > 0 ? rMcpIds : undefined,
              thinkingBudget: rThinkingBudget,
              enabledKnowledgeBaseIds: rKbIds.length > 0 ? rKbIds : undefined,
              enabledMemoryNamespaceIds: rMemIds.length > 0 ? rMemIds : undefined,
              enabledWikiIds: rWikiIds.length > 0 ? rWikiIds : undefined,
            },
          },
        });

        if (!isTauri()) {
          await new Promise((r) => setTimeout(r, 600));
          useStreamStore.setState((s) => ({
            ...stopConversationStream(s.activeStreams, conversationId),
            streamingStartTimestamps: (() => {
              const t = { ...s.streamingStartTimestamps };
              delete t[conversationId];
              return t;
            })(),
            thinkingActiveMessageIds: new Set<string>(),
          }));
          get().fetchMessages(conversationId);
        }
      } catch (e) {
        logIpcError("regenerateWithModel", { notify: true })(e);
        const errMsg = translateBackendError(e);
        const currentStreamingMessageId = getStreamingMessageId(
          useStreamStore.getState().activeStreams,
          conversationId,
        );
        useStreamStore.setState((s) => ({
          ...stopConversationStream(s.activeStreams, conversationId),
          streamingStartTimestamps: (() => {
            const t = { ...s.streamingStartTimestamps };
            delete t[conversationId];
            return t;
          })(),
          thinkingActiveMessageIds: new Set<string>(),
        }));
        set((s) => ({
          messages: currentStreamingMessageId
            ? s.messages.map((m) =>
              m.id === currentStreamingMessageId
                ? { ...m, content: errMsg, status: "error" as const }
                : m
            )
            : s.messages,
        }));
      }
    },

    sendMultiModelMessage: (
      content: string,
      companionModels: Array<{ providerId: string; modelId: string }>,
      attachments?: AttachmentInput[],
      searchProviderId?: string | null,
    ) => {
      // 委托给 multiModelStore 实现
      return useMultiModelStore
        .getState()
        .sendMultiModelMessage(
          content,
          companionModels,
          attachments,
          searchProviderId,
        );
    },
  };
}
