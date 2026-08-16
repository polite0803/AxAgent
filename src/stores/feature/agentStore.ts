// SPDX-License-Identifier: AGPL-3.0-only

import { pushNotification } from "@/components/layout/NotificationBell";
import i18n from "@/i18n";
import { invoke, listen, logIpcError, type UnlistenFn } from "@/lib/invoke";
import { message } from "@/lib/toast";
import { useConversationStore } from "@/stores/domain/conversationStore";
import { useStreamStore } from "@/stores/domain/streamStore";
import { deriveLegacyStreamFields, getStreamingMessageId } from "@/stores/domain/streamStore";
import type {
  AgentCancelledEvent,
  AgentDoneEvent,
  AgentErrorEvent,
  AgentPoolItem,
  AgentProfile,
  AgentRateLimitEvent,
  AgentSession,
  AskUserEvent,
  CreateAgentProfileInput,
  PermissionRequestEvent,
  PlanApprovalEvent,
  SubAgentCardData,
  SubAgentCardEvent,
  ToolCallState,
  ToolResultEvent,
  ToolStartEvent,
  ToolUseEvent,
  UpdateAgentProfileInput,
} from "@/types";
import type { ToolExecution } from "@/types";
import { create } from "zustand";
import { setupExecutionEventListeners, useExecutionStore } from "./executionStore";
import type { CurrentToolCall } from "./executionToolCallUtils";

export type { CurrentToolCall };

/** 计划确认闸门开关的 localStorage 持久化键（P0-2） */
const PLAN_APPROVAL_ENABLED_KEY = "axagent:agent:planApprovalEnabled";

/** 从 localStorage 读取开关状态；不可用时回退 false */
function loadPlanApprovalEnabled(): boolean {
  try {
    return localStorage.getItem(PLAN_APPROVAL_ENABLED_KEY) === "true";
  } catch {
    return false;
  }
}

/** 持久化开关状态到 localStorage */
function persistPlanApprovalEnabled(enabled: boolean): void {
  try {
    localStorage.setItem(PLAN_APPROVAL_ENABLED_KEY, String(enabled));
  } catch {
    // localStorage 不可用，忽略
  }
}

interface QueryStats {
  numTurns?: number;
  inputTokens?: number;
  outputTokens?: number;
  costUsd?: number;
}

/** 当前正在执行（或最近执行）的工具调用追踪（类型复用自 executionToolCallUtils） */

interface AgentStore {
  // Session cache (truth lives in backend DB)
  sessions: Record<string, AgentSession>;

  // Runtime state (unique to agentStore — execution state delegated to executionStore)
  pendingPlan: PlanApprovalEvent | null; // 当前待确认的计划草稿（P0-2）
  pendingPermissions: Record<string, PermissionRequestEvent>; // toolUseId → request
  pendingAskUser: Record<string, AskUserEvent>; // askId → request
  queryStats: Record<string, QueryStats>; // assistantMessageId → cost stats
  rateLimitInfo: Record<string, AgentRateLimitEvent>; // conversationId → rate limit event
  pausedConversations: Set<string>; // conversationIds that are paused
  planApprovalEnabled: boolean; // 计划确认闸门总开关（P0-2），默认关闭
  subAgentCards: Record<string, SubAgentCardData>; // cardId → card data

  // 执行进度追踪（仅 agentStore 独有的标志——agentPool/agentStatus/sdkIdToExecId 由 executionStore 管理）
  isExecuting: Record<string, boolean>; // conversationId → 是否正在执行工具
  executingConversationIds: string[]; // 当前有工具在执行的对话 ID 列表（有序）
  currentToolCall: CurrentToolCall | null;
  sdkIdToExecId: Record<string, string>;
  toolCalls: Record<string, ToolCallState>;
  agentStatus: Record<string, string>;
  agentPool: Record<string, AgentPoolItem[]>;

  // 队友管理
  addTeammateMessage: (
    conversationId: string,
    agentId: string,
    message: string,
  ) => void;
  updateTeammateTask: (
    conversationId: string,
    agentId: string,
    task: string,
  ) => void;

  // Actions
  fetchSession: (conversationId: string) => Promise<AgentSession | null>;
  updateCwd: (conversationId: string, cwd: string) => Promise<void>;
  updatePermissionMode: (conversationId: string, mode: string) => Promise<void>;
  approveToolUse: (
    conversationId: string,
    toolUseId: string,
    decision: string,
    toolName?: string,
  ) => Promise<void>;
  setPlanApprovalEnabled: (enabled: boolean) => void;
  approvePlan: (conversationId: string, decision: string) => Promise<void>;
  handlePlanApproval: (event: PlanApprovalEvent) => void;

  // Event handlers
  handleToolUse: (event: ToolUseEvent) => void;
  handleToolStart: (event: ToolStartEvent) => void;
  handleToolResult: (event: ToolResultEvent) => void;
  handlePermissionRequest: (event: PermissionRequestEvent) => void;
  handlePermissionResolved: (toolUseId: string, decision: string) => void;
  handlePermissionTimeout: (event: { conversationId: string; requestId: string; toolName: string }) => void;
  handleAskUser: (event: AskUserEvent) => void;
  handleAskUserResolved: (askId: string) => void;
  respondAskUser: (askId: string, answer: string) => Promise<void>;
  handleStatus: (conversationId: string, message: string) => void;
  clearStatus: (conversationId: string) => void;
  handleDone: (event: AgentDoneEvent) => void;
  handleError: (event: AgentErrorEvent) => void;
  handleCancelled: (event: AgentCancelledEvent) => void;
  handleRateLimit: (event: AgentRateLimitEvent) => void;
  handleSubAgentCard: (event: SubAgentCardEvent) => void;

  // Expire unresolved permissions for a conversation
  expirePendingPermissions: (conversationId: string) => void;

  // History
  loadToolHistory: (conversationId: string) => Promise<void>;

  // Cleanup
  clearConversation: (conversationId: string) => void;
  clearConversationUI: (conversationId: string) => void;

  // Pause / Resume
  pauseAgent: (conversationId: string) => Promise<void>;
  resumeAgent: (conversationId: string) => Promise<void>;
  isAgentPaused: (conversationId: string) => boolean;

  // --- Agent Profile management ---
  profiles: AgentProfile[];
  loaded: boolean;

  loadProfiles(): Promise<void>;
  getAllProfiles(): AgentProfile[];
  getProfileById(id: string): AgentProfile | undefined;

  importFromAgency(): Promise<{ count: number; errors: string[] }>;

  createCustomProfile(input: CreateAgentProfileInput): Promise<AgentProfile>;
  updateCustomProfile(
    id: string,
    input: UpdateAgentProfileInput,
  ): Promise<AgentProfile>;
  deleteCustomProfile(id: string): Promise<void>;
}

export const useAgentStore = create<AgentStore>((set, get) => ({
  sessions: {},
  pendingPlan: null,
  pendingPermissions: {},
  pendingAskUser: {},
  queryStats: {},
  rateLimitInfo: {},
  pausedConversations: new Set<string>(),
  planApprovalEnabled: loadPlanApprovalEnabled(),
  subAgentCards: {},
  isExecuting: {},
  executingConversationIds: [],
  currentToolCall: null,
  sdkIdToExecId: {},
  toolCalls: {},
  agentStatus: {},
  agentPool: {},

  // --- Agent Profile state ---
  profiles: [],
  loaded: false,
  // --- AgentPool actions delegated to executionStore ---

  upsertPoolItem: () => {/* 由 executionStore 管理 */},
  removePoolItem: () => {/* 由 executionStore 管理 */},
  getPoolSummary: () => ({
    totalItems: 0,
    subAgents: 0,
    workers: 0,
    workflowSteps: 0,
    completedItems: 0,
    failedItems: 0,
    runningItems: 0,
  }),
  handleWorkerEvent: () => {/* 由 executionStore 管理 */},

  async loadProfiles(): Promise<void> {
    try {
      const rows: AgentProfile[] = await invoke("list_agent_profiles");
      set({ profiles: Array.isArray(rows) ? rows : [], loaded: true });
    } catch {
      set({ loaded: true });
    }
  },

  getAllProfiles(): AgentProfile[] {
    return get().profiles;
  },

  getProfileById(id: string): AgentProfile | undefined {
    return get().profiles.find((p) => p.id === id);
  },

  async importFromAgency(): Promise<{ count: number; errors: string[] }> {
    const result: { count: number; errors: string[] } = await invoke(
      "import_agent_profiles_from_agency",
    );
    await get().loadProfiles();
    return result;
  },

  async createCustomProfile(
    input: CreateAgentProfileInput,
  ): Promise<AgentProfile> {
    const profile: AgentProfile = await invoke("create_agent_profile", {
      input,
    });
    set((s) => ({ profiles: [...s.profiles, profile] }));
    return profile;
  },

  async updateCustomProfile(
    id: string,
    input: UpdateAgentProfileInput,
  ): Promise<AgentProfile> {
    const profile: AgentProfile = await invoke("update_agent_profile", {
      id,
      input,
    });
    set((s) => ({
      profiles: s.profiles.map((p) => (p.id === id ? profile : p)),
    }));
    return profile;
  },

  async deleteCustomProfile(id: string): Promise<void> {
    await invoke("delete_agent_profile", { id });
    set((s) => ({ profiles: s.profiles.filter((p) => p.id !== id) }));
  },

  clearConversationUI: (_conversationId) => {
    // 已无会话级 UI 残留字段，保留空实现以维持接口
  },

  // ── 队友管理 ──

  addTeammateMessage: () => {
    // 由 executionStore 管理
  },

  updateTeammateTask: () => {
    // 由 executionStore 管理
  },

  fetchSession: async (conversationId) => {
    try {
      const session = await invoke<AgentSession | null>("agent_get_session", {
        request: {
          conversationId,
        },
      });
      if (session) {
        set((s) => ({
          sessions: { ...s.sessions, [conversationId]: session },
        }));
      }
      return session;
    } catch (e) {
      logIpcError("agentStore.fetchSession")(e);
      return null;
    }
  },

  updateCwd: async (conversationId, cwd) => {
    try {
      const session = await invoke<AgentSession>("agent_update_session", {
        request: {
          conversationId,
          cwd,
        },
      });
      set((s) => ({
        sessions: { ...s.sessions, [conversationId]: session },
      }));
    } catch (e) {
      logIpcError("agentStore.updateCwd")(e);
    }
  },

  updatePermissionMode: async (conversationId, mode) => {
    try {
      const session = await invoke<AgentSession>("agent_update_session", {
        request: {
          conversationId,
          permissionMode: mode,
        },
      });
      set((s) => ({
        sessions: { ...s.sessions, [conversationId]: session },
      }));
    } catch (e) {
      logIpcError("agentStore.updatePermissionMode")(e);
    }
  },

  approveToolUse: async (conversationId, toolUseId, decision, toolName) => {
    try {
      await invoke("agent_approve", {
        request: {
          conversationId,
          toolUseId,
          decision,
          toolName,
        },
      });
      get().handlePermissionResolved(toolUseId, decision);
    } catch (e) {
      logIpcError("agentStore.approveToolUse")(e);
    }
  },

  setPlanApprovalEnabled: (enabled) => {
    persistPlanApprovalEnabled(enabled);
    set({ planApprovalEnabled: enabled });
  },

  approvePlan: async (conversationId, decision) => {
    try {
      await invoke("agent_approve_plan", {
        request: { conversationId, decision },
      });
      set({ pendingPlan: null });
    } catch (e) {
      logIpcError("agentStore.approvePlan")(e);
    }
  },

  handlePlanApproval: (event) => {
    set({ pendingPlan: event });
  },

  handleToolUse: (event) => {
    set((s) => {
      // Create optimistic sub-agent card when task tool is called
      const cardUpdates: Record<string, SubAgentCardData> = {};
      if (event.toolName === "task" && event.conversationId) {
        const cardId = `task-${event.toolUseId}`;
        cardUpdates[cardId] = {
          id: cardId,
          conversationId: event.conversationId,
          agentType: (event.input.agent_type as string) || "general",
          agentName: (event.input.agent_type as string) || "general",
          description: (event.input.description as string) || "Untitled task",
          status: "running",
        };
      }
      // 追踪当前执行状态（仅 agentStore 独有的标志）
      const isExecuting = { ...s.isExecuting, [event.conversationId]: true };
      const executingIds = s.executingConversationIds.includes(
          event.conversationId,
        )
        ? s.executingConversationIds
        : [...s.executingConversationIds, event.conversationId];
      return {
        subAgentCards: { ...s.subAgentCards, ...cardUpdates },
        isExecuting,
        executingConversationIds: executingIds,
      };
    });
  },

  handleToolStart: (_event) => {
    // 委托给 executionStore——agentStore 不维护 toolCalls
  },

  handleToolResult: (event) => {
    set((s) => {
      // 仅处理 agentStore 独有的执行标志；toolCalls/agentPool 由 executionStore 管理
      const isExecuting = { ...s.isExecuting };
      if (event.conversationId) {
        delete isExecuting[event.conversationId];
      }
      const executingIds = s.executingConversationIds.filter(
        (id) => id !== event.conversationId,
      );
      return {
        isExecuting,
        executingConversationIds: executingIds,
      };
    });
  },

  handlePermissionRequest: (event) => {
    // Use requestId as the key (this is what agent_approve needs to deliver the decision)
    const key = event.requestId || event.toolUseId;
    set((s) => ({
      pendingPermissions: { ...s.pendingPermissions, [key]: event },
    }));
  },

  handlePermissionResolved: (toolUseId, decision) => {
    set((s) => {
      const rest = { ...s.pendingPermissions };
      delete rest[toolUseId];
      const existing = s.toolCalls[toolUseId];
      const updatedToolCalls = existing
        ? {
          ...s.toolCalls,
          [toolUseId]: {
            ...existing,
            approvalStatus: decision === "deny"
              ? ("denied" as const)
              : ("approved" as const),
          },
        }
        : s.toolCalls;
      return {
        pendingPermissions: rest,
        toolCalls: updatedToolCalls,
      };
    });
  },

  handlePermissionTimeout: (event) => {
    // 权限请求超时（5 分钟无响应），清理挂起的权限 UI
    // 使用 requestId 作为 key（与 handlePermissionRequest 一致）
    set((s) => {
      const rest = { ...s.pendingPermissions };
      delete rest[event.requestId];
      const existing = s.toolCalls[event.requestId];
      const updatedToolCalls = existing
        ? {
          ...s.toolCalls,
          [event.requestId]: {
            ...existing,
            approvalStatus: "denied" as const,
            approvalMessage: `Permission for "${event.toolName}" timed out`,
          },
        }
        : s.toolCalls;
      return { pendingPermissions: rest, toolCalls: updatedToolCalls };
    });
  },

  handleAskUser: (event) => {
    set((s) => ({
      pendingAskUser: { ...s.pendingAskUser, [event.askId]: event },
    }));
  },

  handleAskUserResolved: (askId) => {
    set((s) => {
      const rest = { ...s.pendingAskUser };
      delete rest[askId];
      return { pendingAskUser: rest };
    });
  },

  respondAskUser: async (askId, answer) => {
    try {
      await invoke("agent_respond_ask", { request: { askId, answer } });
      // Brief delay so user sees the loading/submitted feedback
      await new Promise((r) => setTimeout(r, 500));
      get().handleAskUserResolved(askId);
    } catch (e) {
      logIpcError("agentStore.respondAskUser")(e);
    }
  },

  handleStatus: () => {
    // 由 executionStore 管理
  },

  clearStatus: () => {
    // 由 executionStore 管理
  },

  handleDone: (event) => {
    const stats: QueryStats = {};
    if (event.numTurns != null) {
      stats.numTurns = event.numTurns;
    }
    if (event.usage) {
      stats.inputTokens = event.usage.inputTokens;
      stats.outputTokens = event.usage.outputTokens;
    }
    if (event.costUsd != null) {
      stats.costUsd = event.costUsd;
    }
    if (event.assistantMessageId && Object.keys(stats).length > 0) {
      set((s) => ({
        queryStats: { ...s.queryStats, [event.assistantMessageId]: stats },
      }));
    }
    // Clear streaming state and expire unresolved permissions
    get().expirePendingPermissions(event.conversationId);
    get().clearStatus(event.conversationId);
    set((s) => {
      const isExecuting = { ...s.isExecuting };
      delete isExecuting[event.conversationId];
      return {
        isExecuting,
        executingConversationIds: s.executingConversationIds.filter(
          (id) => id !== event.conversationId,
        ),
      };
    });
    // Agent 完成通知
    const turns = event.numTurns ?? 0;
    const cost = event.costUsd != null
      ? ` (${event.costUsd < 0.01 ? "<$0.01" : `$${event.costUsd.toFixed(2)}`})`
      : "";
    const doneText = i18n.t("agentStore.executionComplete", { turns, cost });
    message.success(doneText);
    pushNotification("success", doneText);
  },

  handleError: (event) => {
    logIpcError("agentStore.handleError")(event);
    if (event.conversationId) {
      get().clearStatus(event.conversationId);
      get().expirePendingPermissions(event.conversationId);
      set((s) => {
        const isExecuting = { ...s.isExecuting };
        delete isExecuting[event.conversationId];
        return {
          isExecuting,
          executingConversationIds: s.executingConversationIds.filter(
            (id) => id !== event.conversationId,
          ),
        };
      });
    }
    // Fallback: update message content if per-invocation listener missed it.
    const { activeStreams } = useStreamStore.getState();
    const streamMsgId = getStreamingMessageId(
      activeStreams,
      event.conversationId,
    );
    if (streamMsgId) {
      const targetId = streamMsgId;
      // Detect stream interruption errors that may have partial content
      const isStreamInterrupt = event.message?.toLowerCase().includes("stream")
        && (event.message?.toLowerCase().includes("interrupt")
          || event.message?.toLowerCase().includes("timeout")
          || event.message?.toLowerCase().includes("connection")
          || event.message?.toLowerCase().includes("network"));
      const errorPrefix = isStreamInterrupt
        ? "⚠️ Stream interrupted — partial response may be lost. "
        : "";
      useStreamStore.setState((s) => {
        const restStreams = { ...s.activeStreams };
        delete restStreams[event.conversationId];
        const restCount = Object.keys(restStreams).length;
        return {
          activeStreams: restStreams,
          ...(restCount > 0
            ? deriveLegacyStreamFields(restStreams)
            : {
              streaming: false,
              streamingMessageId: null,
              streamingConversationId: null,
            }),
          streamingStartTimestamps: (() => {
            const t = { ...s.streamingStartTimestamps };
            delete t[event.conversationId];
            return t;
          })(),
          thinkingActiveMessageIds: (() => {
            const current = s.thinkingActiveMessageIds;
            const next = new Set(current);
            if (targetId) {
              next.delete(targetId);
            }
            return next;
          })(),
        };
      });
      useConversationStore.setState((s) => ({
        messages: s.messages.map((m) =>
          m.id === targetId
            ? {
              ...m,
              content: errorPrefix + event.message,
              status: "error" as const,
            }
            : m
        ),
      }));
    }
    // Agent 错误通知
    const errMsg = event.message?.slice(0, 100) || i18n.t("agentStore.unknownError");
    const errorText = i18n.t("agentStore.executionFailed", { errMsg });
    message.error(errorText);
    pushNotification("error", errorText);
  },

  handleCancelled: (event) => {
    console.info("[agentStore] Agent cancelled:", event.reason);
    const reason = event.reason || i18n.t("agentStore.userInterrupt");
    message.warning(i18n.t("agentStore.executionCancelled", { reason }));
    // Clear status and expire unresolved permissions for the conversation
    if (event.conversationId) {
      get().clearStatus(event.conversationId);
      get().expirePendingPermissions(event.conversationId);
      // 清除当前对话的执行状态
      set((s) => {
        const isExecuting = { ...s.isExecuting };
        delete isExecuting[event.conversationId];
        return {
          isExecuting,
          executingConversationIds: s.executingConversationIds.filter(
            (id) => id !== event.conversationId,
          ),
        };
      });
    }
  },

  handleRateLimit: (event) => {
    set((s) => ({
      rateLimitInfo: { ...s.rateLimitInfo, [event.conversationId]: event },
    }));
    // Auto-clear after the retry duration
    const clearAfter = event.retryAfterMs > 0 ? event.retryAfterMs : 5000;
    // Clear any existing timer for this conversation
    if (_rateLimitTimers[event.conversationId]) {
      clearTimeout(_rateLimitTimers[event.conversationId]);
    }
    _rateLimitTimers[event.conversationId] = setTimeout(() => {
      delete _rateLimitTimers[event.conversationId];
      set((s) => {
        const rest = { ...s.rateLimitInfo };
        delete rest[event.conversationId];
        return { rateLimitInfo: rest };
      });
    }, clearAfter);
  },

  handleSubAgentCard: (event) => {
    const cardId = event.childConversationId ?? `card-${Date.now()}`;
    const card: SubAgentCardData = {
      id: cardId,
      conversationId: event.conversationId,
      agentType: event.agentType,
      agentName: event.agentName,
      description: event.description,
      status: event.status,
      childConversationId: event.childConversationId,
      childSessionId: event.childSessionId,
    };
    // 同时写入 agentPool — subAgentCards 已通过 card 写入
    set((s) => ({
      subAgentCards: { ...s.subAgentCards, [cardId]: card },
    }));
  },

  expirePendingPermissions: (conversationId) => {
    set((s) => {
      // Find all pending permission keys for this conversation
      const expiredKeys = new Set<string>();
      for (const [id, pr] of Object.entries(s.pendingPermissions)) {
        if (pr.conversationId === conversationId) {
          expiredKeys.add(id);
        }
      }
      if (expiredKeys.size === 0) {
        return s;
      }

      // Remove from pendingPermissions and mark toolCalls as expired
      const pendingPermissions: Record<string, PermissionRequestEvent> = {};
      for (const [id, pr] of Object.entries(s.pendingPermissions)) {
        if (!expiredKeys.has(id)) {
          pendingPermissions[id] = pr;
        }
      }
      const toolCalls: Record<string, ToolCallState> = {};
      for (const [id, tc] of Object.entries(s.toolCalls)) {
        if (expiredKeys.has(id)) {
          toolCalls[id] = { ...tc, approvalStatus: "denied" as const };
        } else {
          toolCalls[id] = tc;
        }
      }
      return { pendingPermissions, toolCalls };
    });
  },

  loadToolHistory: async (conversationId) => {
    try {
      const executions = await invoke<ToolExecution[]>("list_tool_executions", {
        conversationId,
      });
      const agentExecs = executions.filter(
        (e) => e.serverId === "__agent_sdk__",
      );

      const toolCalls: Record<string, ToolCallState> = {};
      for (const exec of agentExecs) {
        let executionStatus: ToolCallState["executionStatus"] = "queued";
        if (exec.status === "running") {
          executionStatus = "running";
        } else if (exec.status === "success") {
          executionStatus = "success";
        } else if (exec.status === "failed") {
          executionStatus = "failed";
        } else if (exec.status === "cancelled") {
          executionStatus = "cancelled";
        }

        // Historical records still showing pending/running means the agent
        // was interrupted or a duplicate record was left behind.
        // Treat them as cancelled to differentiate from actually completed executions.
        if (executionStatus === "queued" || executionStatus === "running") {
          executionStatus = "cancelled";
        }

        let approvalStatus: ToolCallState["approvalStatus"] | undefined;
        if (exec.approvalStatus === "approved") {
          approvalStatus = "approved";
        } else if (exec.approvalStatus === "denied") {
          approvalStatus = "denied";
        } else if (exec.approvalStatus === "pending") {
          approvalStatus = "pending";
        }

        let input: Record<string, unknown> = {};
        if (exec.inputPreview) {
          try {
            input = JSON.parse(exec.inputPreview);
          } catch {
            /* leave empty */
          }
        }

        toolCalls[exec.id] = {
          toolUseId: exec.id,
          toolName: exec.toolName,
          input,
          assistantMessageId: exec.messageId ?? "",
          executionStatus,
          approvalStatus,
          output: exec.outputPreview ?? exec.errorMessage,
          isError: exec.status === "failed",
        };
      }

      set((s) => ({
        toolCalls: { ...toolCalls, ...s.toolCalls },
      }));
    } catch (e) {
      logIpcError("agentStore.loadToolHistory")(e);
    }
  },

  clearConversation: (conversationId) => {
    // Delegate toolCalls/agentPool/agentStatus cleanup to executionStore
    useExecutionStore.getState().clearConversation(conversationId);

    // Clear rate-limit timer for this conversation
    if (_rateLimitTimers[conversationId]) {
      clearTimeout(_rateLimitTimers[conversationId]);
      delete _rateLimitTimers[conversationId];
    }

    set((s) => {
      const sessions = { ...s.sessions };
      delete sessions[conversationId];

      const pendingPermissions: Record<string, PermissionRequestEvent> = {};
      for (const [id, pr] of Object.entries(s.pendingPermissions)) {
        if (pr.conversationId !== conversationId) {
          pendingPermissions[id] = pr;
        }
      }

      const pendingAskUser: Record<string, AskUserEvent> = {};
      for (const [id, ask] of Object.entries(s.pendingAskUser)) {
        if (ask.conversationId !== conversationId) {
          pendingAskUser[id] = ask;
        }
      }

      // Clean up sdkIdToExecId mappings (toolCalls/agentPool/agentStatus handled by executionStore)
      const removedPermKeys = new Set<string>();
      for (const [id, pr] of Object.entries(s.pendingPermissions)) {
        if (pr.conversationId === conversationId) {
          removedPermKeys.add(id);
          removedPermKeys.add(pr.toolUseId);
        }
      }
      const sdkIdToExecId: Record<string, string> = {};
      for (const [sdkId, execId] of Object.entries(s.sdkIdToExecId)) {
        if (!removedPermKeys.has(sdkId) && !removedPermKeys.has(execId)) {
          sdkIdToExecId[sdkId] = execId;
        }
      }

      const rateLimitInfo = { ...s.rateLimitInfo };
      delete rateLimitInfo[conversationId];
      const pausedConversations = new Set(s.pausedConversations);
      pausedConversations.delete(conversationId);
      const isExecuting = { ...s.isExecuting };
      delete isExecuting[conversationId];
      const executingConversationIds = s.executingConversationIds.filter(
        (id) => id !== conversationId,
      );
      const queryStats = { ...s.queryStats };
      delete queryStats[conversationId];
      const subAgentCards = { ...s.subAgentCards };
      delete subAgentCards[conversationId];
      return {
        sessions,
        pendingPermissions,
        pendingAskUser,
        sdkIdToExecId,
        rateLimitInfo,
        pausedConversations,
        isExecuting,
        executingConversationIds,
        currentToolCall: s.currentToolCall?.conversationId === conversationId
          ? null
          : s.currentToolCall,
        queryStats,
        subAgentCards,
      };
    });
  },

  pauseAgent: async (conversationId) => {
    try {
      await invoke("agent_pause", { conversationId });
      set((s) => {
        const pausedConversations = new Set(s.pausedConversations);
        pausedConversations.add(conversationId);
        return { pausedConversations };
      });
    } catch (err) {
      logIpcError("agentStore.pauseAgent")(err);
    }
  },

  resumeAgent: async (conversationId) => {
    try {
      await invoke("agent_resume", { conversationId });
      set((s) => {
        const pausedConversations = new Set(s.pausedConversations);
        pausedConversations.delete(conversationId);
        return { pausedConversations };
      });
    } catch (err) {
      logIpcError("agentStore.resumeAgent")(err);
    }
  },

  isAgentPaused: (conversationId) => {
    return get().pausedConversations.has(conversationId);
  },
}));

// Rate-limit timer tracking for cleanup
const _rateLimitTimers: Record<string, ReturnType<typeof setTimeout>> = {};

let _agentListenerRefCount = 0;

/**
 * 注册 agentStore 独有的 Tauri 事件监听器。
 * 执行相关事件（tool-use/worker/status 等）统一
 * 委托给 setupExecutionEventListeners 处理，避免重复。
 */
export function setupAgentEventListeners(): () => void {
  _agentListenerRefCount++;
  if (_agentListenerRefCount > 1) {
    return () => {
      _agentListenerRefCount--;
    };
  }

  // 执行事件由 executionStore 统一接管
  const execCleanup = setupExecutionEventListeners();

  const unlisteners: Promise<UnlistenFn>[] = [];
  const store = useAgentStore.getState();

  // ── agentStore 独有的事件 ──

  unlisteners.push(
    listen<PermissionRequestEvent>("agent-permission-request", (event) => {
      store.handlePermissionRequest(event.payload);
    }),
  );

  unlisteners.push(
    listen<PlanApprovalEvent>("agent-plan-ready-for-approval", (event) => {
      store.handlePlanApproval(event.payload);
    }),
  );

  unlisteners.push(
    listen<{ conversationId: string; requestId: string; toolName: string }>(
      "agent-permission-timeout",
      (event) => {
        store.handlePermissionTimeout(event.payload);
      },
    ),
  );

  unlisteners.push(
    listen<AskUserEvent>("agent-ask-user", (event) => {
      store.handleAskUser(event.payload);
    }),
  );

  unlisteners.push(
    listen<AgentRateLimitEvent>("agent-rate-limit", (event) => {
      store.handleRateLimit(event.payload);
    }),
  );

  // ── Agent 生命周期事件（清理 agentStatus、isExecuting、currentToolCall 等） ─

  unlisteners.push(
    listen<AgentDoneEvent>("agent-done", (event) => {
      store.handleDone(event.payload);
    }),
  );

  unlisteners.push(
    listen<AgentErrorEvent>("agent-error", (event) => {
      store.handleError(event.payload);
    }),
  );

  unlisteners.push(
    listen<AgentCancelledEvent>("agent-cancelled", (event) => {
      store.handleCancelled(event.payload);
    }),
  );

  unlisteners.push(
    listen<{ conversationId: string }>("agent-paused", (event) => {
      useAgentStore.setState((s) => {
        const pausedConversations = new Set(s.pausedConversations);
        pausedConversations.add(event.payload.conversationId);
        return { pausedConversations };
      });
    }),
  );

  unlisteners.push(
    listen<{ conversationId: string }>("agent-resumed", (event) => {
      useAgentStore.setState((s) => {
        const pausedConversations = new Set(s.pausedConversations);
        pausedConversations.delete(event.payload.conversationId);
        return { pausedConversations };
      });
    }),
  );

  return () => {
    _agentListenerRefCount--;
    if (_agentListenerRefCount <= 0) {
      _agentListenerRefCount = 0;
      execCleanup();
      for (const p of unlisteners) {
        p.then((u) => u());
      }
    }
  };
}

// P0-2 兜底：在模块顶层注册 plan 事件监听，避免依赖 setupAgentEventListeners
// 的 useEffect 时机（ChatView mount 与 Playwright click send 之间可能存在
// 竞态，导致事件丢失 → pendingPlan 未设置 → Modal 不渲染）。
listen<PlanApprovalEvent>("agent-plan-ready-for-approval", (event) => {
  useAgentStore.getState().handlePlanApproval(event.payload);
});
