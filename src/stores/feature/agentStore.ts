import { invoke, listen, type UnlistenFn } from "@/lib/invoke";
import { useConversationStore, useStreamStore } from "@/stores";
import { deriveLegacyStreamFields, getStreamingMessageId } from "@/stores/domain/streamStore";
import type {
  AgentCancelledEvent,
  AgentDoneEvent,
  AgentErrorEvent,
  AgentPoolItem,
  AgentPoolSummary,
  AgentRateLimitEvent,
  AgentSession,
  AgentStatusEvent,
  AskUserEvent,
  PermissionRequestEvent,
  SubAgentCardData,
  SubAgentCardEvent,
  ToolCallState,
  ToolResultEvent,
  ToolStartEvent,
  ToolUseEvent,
  WorkerMessage,
} from "@/types/agent";
import type { ToolExecution } from "@/types/mcp";
import { create } from "zustand";

interface QueryStats {
  numTurns?: number;
  inputTokens?: number;
  outputTokens?: number;
  costUsd?: number;
}

interface AgentStore {
  // Session cache (truth lives in backend DB)
  sessions: Record<string, AgentSession>;

  // Runtime state
  agentStatus: Record<string, string>; // conversationId → status message
  pendingPermissions: Record<string, PermissionRequestEvent>; // toolUseId → request
  pendingAskUser: Record<string, AskUserEvent>; // askId → request
  toolCalls: Record<string, ToolCallState>; // toolUseId or execId → state
  sdkIdToExecId: Record<string, string>; // SDK toolUseId → DB execution ID mapping
  queryStats: Record<string, QueryStats>; // assistantMessageId → cost stats
  rateLimitInfo: Record<string, AgentRateLimitEvent>; // conversationId → rate limit event
  pausedConversations: Set<string>; // conversationIds that are paused
  subAgentCards: Record<string, SubAgentCardData>; // cardId → card data

  // Unified Agent Pool — 子Agent + 工作者 + 工作流步骤
  agentPool: Record<string, AgentPoolItem[]>; // conversationId → pool items

  // Pool actions
  upsertPoolItem: (item: AgentPoolItem) => void;
  removePoolItem: (conversationId: string, itemId: string) => void;
  getPoolSummary: (conversationId: string) => AgentPoolSummary;
  handleWorkerEvent: (event: {
    conversationId: string;
    workerId: string;
    taskId: string;
    messageType: string;
    content: string;
    status?: string;
  }) => void;

  // Actions
  fetchSession: (conversationId: string) => Promise<AgentSession | null>;
  updateCwd: (conversationId: string, cwd: string) => Promise<void>;
  updatePermissionMode: (conversationId: string, mode: string) => Promise<void>;
  approveToolUse: (conversationId: string, toolUseId: string, decision: string, toolName?: string) => Promise<void>;

  // Event handlers
  handleToolUse: (event: ToolUseEvent) => void;
  handleToolStart: (event: ToolStartEvent) => void;
  handleToolResult: (event: ToolResultEvent) => void;
  handlePermissionRequest: (event: PermissionRequestEvent) => void;
  handlePermissionResolved: (toolUseId: string, decision: string) => void;
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

  // Pause / Resume
  pauseAgent: (conversationId: string) => Promise<void>;
  resumeAgent: (conversationId: string) => Promise<void>;
  isAgentPaused: (conversationId: string) => boolean;
}

export const useAgentStore = create<AgentStore>((set, get) => ({
  sessions: {},
  agentStatus: {},
  pendingPermissions: {},
  pendingAskUser: {},
  toolCalls: {},
  sdkIdToExecId: {},
  queryStats: {},
  rateLimitInfo: {},
  pausedConversations: new Set<string>(),
  subAgentCards: {},
  agentPool: {},

  // --- AgentPool actions ---

  upsertPoolItem: (item) => {
    set((s) => {
      const pool = [...(s.agentPool[item.conversationId] || [])];
      const idx = pool.findIndex((p) => p.id === item.id);
      if (idx >= 0) {
        pool[idx] = { ...pool[idx], ...item };
      } else {
        pool.push(item);
      }
      return { agentPool: { ...s.agentPool, [item.conversationId]: pool } };
    });
  },

  removePoolItem: (conversationId, itemId) => {
    set((s) => {
      const pool = (s.agentPool[conversationId] || []).filter(
        (p) => p.id !== itemId,
      );
      return { agentPool: { ...s.agentPool, [conversationId]: pool } };
    });
  },

  getPoolSummary: (conversationId) => {
    const pool = get().agentPool[conversationId] || [];
    const total = pool.length;
    const completed = pool.filter((p) => p.status === "completed").length;
    const running = pool.filter((p) => p.status === "running").length;
    const pending = pool.filter((p) => p.status === "pending").length;
    const failed = pool.filter((p) => p.status === "failed").length;
    return {
      total,
      completed,
      running,
      pending,
      failed,
      pctComplete: total > 0 ? Math.round((completed / total) * 100) : 0,
    };
  },

  handleWorkerEvent: (event) => {
    const poolId = `worker-${event.workerId}`;
    const msg: WorkerMessage = {
      workerId: event.workerId,
      taskId: event.taskId,
      messageType: (event.messageType || "progress") as WorkerMessage["messageType"],
      content: event.content,
      timestamp: Date.now(),
    };

    set((s) => {
      const pool = [...(s.agentPool[event.conversationId] || [])];
      const idx = pool.findIndex((p) => p.id === poolId);

      const statusMap: Record<string, AgentPoolItem["status"]> = {
        progress: "running",
        result: "completed",
        completion: "completed",
        error: "failed",
      };

      const newStatus = (event.status ||
        statusMap[event.messageType] ||
        "running") as AgentPoolItem["status"];

      if (idx >= 0) {
        const existing = pool[idx];
        pool[idx] = {
          ...existing,
          status: newStatus,
          summary:
            event.messageType === "progress"
              ? event.content
              : existing.summary,
          error: event.messageType === "error" ? event.content : existing.error,
          messages: [...(existing.messages || []), msg],
          duration: existing.startedAt
            ? Date.now() - existing.startedAt
            : undefined,
        };
      } else {
        pool.push({
          id: poolId,
          conversationId: event.conversationId,
          type: "worker",
          name: event.workerId,
          status: "running",
          taskDescription: event.taskId,
          messages: [msg],
          startedAt: Date.now(),
        });
      }

      return { agentPool: { ...s.agentPool, [event.conversationId]: pool } };
    });
  },

  fetchSession: async (conversationId) => {
    try {
      const session = await invoke<AgentSession | null>("agent_get_session", {
        conversation_id: conversationId,
      });
      if (session) {
        set((s) => ({
          sessions: { ...s.sessions, [conversationId]: session },
        }));
      }
      return session;
    } catch (e) {
      console.error("[agentStore] fetchSession failed:", e);
      return null;
    }
  },

  updateCwd: async (conversationId, cwd) => {
    try {
      const session = await invoke<AgentSession>("agent_update_session", {
        conversation_id: conversationId,
        cwd,
      });
      set((s) => ({
        sessions: { ...s.sessions, [conversationId]: session },
      }));
    } catch (e) {
      console.error("[agentStore] updateCwd failed:", e);
    }
  },

  updatePermissionMode: async (conversationId, mode) => {
    try {
      const session = await invoke<AgentSession>("agent_update_session", {
        conversation_id: conversationId,
        permission_mode: mode,
      });
      set((s) => ({
        sessions: { ...s.sessions, [conversationId]: session },
      }));
    } catch (e) {
      console.error("[agentStore] updatePermissionMode failed:", e);
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
      console.error("[agentStore] approveToolUse failed:", e);
    }
  },

  handleToolUse: (event) => {
    set((s) => {
      const toolCall: ToolCallState = {
        toolUseId: event.toolUseId,
        toolName: event.toolName,
        input: event.input,
        assistantMessageId: event.assistantMessageId,
        executionStatus: "queued",
      };
      const updates: Record<string, ToolCallState> = {
        [event.toolUseId]: toolCall,
      };
      const idMap = { ...s.sdkIdToExecId };
      if (event.executionId) {
        updates[event.executionId] = { ...toolCall, toolUseId: event.executionId };
        idMap[event.toolUseId] = event.executionId;
      }
      // Create optimistic sub-agent card when task tool is called
      let cardUpdates: Record<string, SubAgentCardData> = {};
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
      return {
        toolCalls: { ...s.toolCalls, ...updates },
        sdkIdToExecId: idMap,
        subAgentCards: { ...s.subAgentCards, ...cardUpdates },
      };
    });
  },

  handleToolStart: (event) => {
    set((s) => {
      const existing = s.toolCalls[event.toolUseId];
      const updated: ToolCallState = {
        toolUseId: event.toolUseId,
        toolName: event.toolName,
        input: event.input,
        assistantMessageId: event.assistantMessageId,
        executionStatus: "running",
        approvalStatus: existing?.approvalStatus,
      };
      const updates: Record<string, ToolCallState> = {
        [event.toolUseId]: updated,
      };
      const execId = s.sdkIdToExecId[event.toolUseId];
      if (execId) {
        updates[execId] = { ...updated, toolUseId: execId };
      }
      return { toolCalls: { ...s.toolCalls, ...updates } };
    });
  },

  handleToolResult: (event) => {
    set((s) => {
      const existing = s.toolCalls[event.toolUseId];
      const newStatus = event.isError ? "failed" : "success";
      const updated: ToolCallState = {
        toolUseId: event.toolUseId,
        toolName: event.toolName || existing?.toolName || "",
        input: existing?.input ?? {},
        assistantMessageId: event.assistantMessageId,
        executionStatus: newStatus,
        approvalStatus: existing?.approvalStatus,
        output: event.content,
        isError: event.isError,
      };
      const updates: Record<string, ToolCallState> = {
        [event.toolUseId]: updated,
      };
      const execId = s.sdkIdToExecId[event.toolUseId];
      if (execId) {
        updates[execId] = { ...updated, toolUseId: execId };
      }
      return { toolCalls: { ...s.toolCalls, ...updates } };
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
      const { [toolUseId]: _removed, ...rest } = s.pendingPermissions;
      const existing = s.toolCalls[toolUseId];
      const updatedToolCalls = existing
        ? {
          ...s.toolCalls,
          [toolUseId]: {
            ...existing,
            approvalStatus: decision === "deny" ? ("denied" as const) : ("approved" as const),
          },
        }
        : s.toolCalls;
      return {
        pendingPermissions: rest,
        toolCalls: updatedToolCalls,
      };
    });
  },

  handleAskUser: (event) => {
    set((s) => ({
      pendingAskUser: { ...s.pendingAskUser, [event.askId]: event },
    }));
  },

  handleAskUserResolved: (askId) => {
    set((s) => {
      const { [askId]: _removed, ...rest } = s.pendingAskUser;
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
      console.error("[agentStore] respondAskUser failed:", e);
    }
  },

  handleStatus: (conversationId, message) => {
    set((s) => ({
      agentStatus: { ...s.agentStatus, [conversationId]: message },
    }));
  },

  clearStatus: (conversationId) => {
    set((s) => {
      const { [conversationId]: _removed, ...rest } = s.agentStatus;
      return { agentStatus: rest };
    });
  },

  handleDone: (event) => {
    const stats: QueryStats = {};
    if (event.numTurns != null) { stats.numTurns = event.numTurns; }
    if (event.usage) {
      stats.inputTokens = event.usage.input_tokens;
      stats.outputTokens = event.usage.output_tokens;
    }
    if (event.costUsd != null) { stats.costUsd = event.costUsd; }
    if (event.assistantMessageId && Object.keys(stats).length > 0) {
      set((s) => ({
        queryStats: { ...s.queryStats, [event.assistantMessageId]: stats },
      }));
    }
    // Clear streaming state and expire unresolved permissions
    get().expirePendingPermissions(event.conversationId);
  },

  handleError: (event) => {
    console.error("[agentStore] Agent error:", event);
    // Clear status and expire unresolved permissions for the conversation
    if (event.conversationId) {
      get().clearStatus(event.conversationId);
      get().expirePendingPermissions(event.conversationId);
    }
    // Fallback: update message content if per-invocation listener missed it.
    const { activeStreams } = useStreamStore.getState();
    const streamMsgId = getStreamingMessageId(activeStreams, event.conversationId);
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
        const { [event.conversationId]: _removed, ...restStreams } = s.activeStreams;
        const restCount = Object.keys(restStreams).length;
        return {
          activeStreams: restStreams,
          ...(restCount > 0
            ? deriveLegacyStreamFields(restStreams)
            : { streaming: false, streamingMessageId: null, streamingConversationId: null }),
          streamingStartTimestamps: (() => {
            const t = { ...s.streamingStartTimestamps };
            delete t[event.conversationId];
            return t;
          })(),
          thinkingActiveMessageIds: (() => {
            const current = s.thinkingActiveMessageIds;
            const next = new Set(current);
            if (targetId) { next.delete(targetId); }
            return next;
          })(),
        };
      });
      useConversationStore.setState((s) => ({
        messages: s.messages.map((m) =>
          m.id === targetId
            ? { ...m, content: errorPrefix + event.message, status: "error" as const }
            : m
        ),
      }));
    }
  },

  handleCancelled: (event) => {
    console.info("[agentStore] Agent cancelled:", event.reason);
    // Clear status and expire unresolved permissions for the conversation
    if (event.conversationId) {
      get().clearStatus(event.conversationId);
      get().expirePendingPermissions(event.conversationId);
    }
  },

  handleRateLimit: (event) => {
    console.warn("[agentStore] Rate limited:", event.message);
    set((s) => ({
      rateLimitInfo: { ...s.rateLimitInfo, [event.conversationId]: event },
    }));
    // Auto-clear after the retry duration
    const clearAfter = event.retryAfterMs > 0 ? event.retryAfterMs : 5000;
    setTimeout(() => {
      set((s) => {
        const { [event.conversationId]: _removed, ...rest } = s.rateLimitInfo;
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
    // 同时写入 agentPool
    const poolItem: AgentPoolItem = {
      id: cardId,
      conversationId: event.conversationId,
      type: "sub_agent",
      name: event.agentName || event.agentType,
      status: event.status === "failed" ? "failed" : event.status === "completed" ? "completed" : "running",
      agentType: event.agentType,
      childConversationId: event.childConversationId,
      childSessionId: event.childSessionId,
      summary: event.description,
      startedAt: Date.now(),
    };
    set((s) => {
      const pool = [...(s.agentPool[event.conversationId] || [])];
      const idx = pool.findIndex((p) => p.id === cardId);
      if (idx >= 0) {
        pool[idx] = { ...pool[idx], ...poolItem };
      } else {
        pool.push(poolItem);
      }
      return {
        subAgentCards: { ...s.subAgentCards, [cardId]: card },
        agentPool: { ...s.agentPool, [event.conversationId]: pool },
      };
    });
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
      if (expiredKeys.size === 0) { return s; }

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
      const agentExecs = executions.filter((e) => e.serverId === "__agent_sdk__");

      const toolCalls: Record<string, ToolCallState> = {};
      for (const exec of agentExecs) {
        let executionStatus: ToolCallState["executionStatus"] = "queued";
        if (exec.status === "running") { executionStatus = "running"; }
        else if (exec.status === "success") { executionStatus = "success"; }
        else if (exec.status === "failed") { executionStatus = "failed"; }
        else if (exec.status === "cancelled") { executionStatus = "cancelled"; }

        // Historical records still showing pending/running means the agent
        // was interrupted or a duplicate record was left behind.
        // Treat them as success to avoid perpetual loading spinners.
        if (executionStatus === "queued" || executionStatus === "running") {
          executionStatus = "success";
        }

        let approvalStatus: ToolCallState["approvalStatus"] | undefined;
        if (exec.approvalStatus === "approved") { approvalStatus = "approved"; }
        else if (exec.approvalStatus === "denied") { approvalStatus = "denied"; }
        else if (exec.approvalStatus === "pending") { approvalStatus = "pending"; }

        let input: Record<string, unknown> = {};
        if (exec.inputPreview) {
          try {
            input = JSON.parse(exec.inputPreview);
          } catch { /* leave empty */ }
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
      console.error("[agentStore] loadToolHistory failed:", e);
    }
  },

  clearConversation: (conversationId) => {
    set((s) => {
      const { [conversationId]: _session, ...sessions } = s.sessions;
      const { [conversationId]: _status, ...agentStatus } = s.agentStatus;

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

      // ToolCallState doesn't carry conversationId directly, but we can identify
      // related tool calls via the pendingPermissions that were already filtered above.
      // Collect toolUseIds from the removed permissions, then remove those from toolCalls.
      const removedPermKeys = new Set<string>();
      for (const [id, pr] of Object.entries(s.pendingPermissions)) {
        if (pr.conversationId === conversationId) {
          removedPermKeys.add(id);
          removedPermKeys.add(pr.toolUseId);
        }
      }
      const toolCalls: Record<string, ToolCallState> = {};
      for (const [id, tc] of Object.entries(s.toolCalls)) {
        if (!removedPermKeys.has(id) && !removedPermKeys.has(tc.toolUseId)) {
          toolCalls[id] = tc;
        }
      }

      // Also clean up sdkIdToExecId mappings for removed tool calls
      const sdkIdToExecId: Record<string, string> = {};
      for (const [sdkId, execId] of Object.entries(s.sdkIdToExecId)) {
        if (!removedPermKeys.has(sdkId) && !removedPermKeys.has(execId)) {
          sdkIdToExecId[sdkId] = execId;
        }
      }

      const { [conversationId]: _rateLimit, ...rateLimitInfo } = s.rateLimitInfo;
      const pausedConversations = new Set(s.pausedConversations);
      pausedConversations.delete(conversationId);
      return {
        sessions,
        agentStatus,
        pendingPermissions,
        pendingAskUser,
        toolCalls,
        sdkIdToExecId,
        rateLimitInfo,
        pausedConversations,
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
      console.error("[agentStore] pauseAgent failed:", err);
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
      console.error("[agentStore] resumeAgent failed:", err);
    }
  },

  isAgentPaused: (conversationId) => {
    return get().pausedConversations.has(conversationId);
  },
}));

// ── Event listener setup ─────────────────────────────────────────────────

let _listenersSetup = false;

export function setupAgentEventListeners(): () => void {
  // Guard against double registration (e.g., React 18 Strict Mode)
  if (_listenersSetup) {
    return () => {};
  }
  _listenersSetup = true;

  const unlisteners: Promise<UnlistenFn>[] = [];
  const store = useAgentStore.getState();

  unlisteners.push(
    listen<ToolUseEvent>("agent-tool-use", (event) => {
      store.handleToolUse(event.payload);
    }),
  );

  unlisteners.push(
    listen<ToolStartEvent>("agent-tool-start", (event) => {
      store.handleToolStart(event.payload);
    }),
  );

  unlisteners.push(
    listen<ToolResultEvent>("agent-tool-result", (event) => {
      store.handleToolResult(event.payload);
    }),
  );

  unlisteners.push(
    listen<PermissionRequestEvent>("agent-permission-request", (event) => {
      store.handlePermissionRequest(event.payload);
    }),
  );

  unlisteners.push(
    listen<AskUserEvent>("agent-ask-user", (event) => {
      store.handleAskUser(event.payload);
    }),
  );

  unlisteners.push(
    listen<AgentStatusEvent>("agent-status", (event) => {
      store.handleStatus(event.payload.conversationId, event.payload.message);
    }),
  );

  unlisteners.push(
    listen<AgentDoneEvent>("agent-done", (event) => {
      store.clearStatus(event.payload.conversationId);
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
    listen<AgentRateLimitEvent>("agent-rate-limit", (event) => {
      store.handleRateLimit(event.payload);
    }),
  );

  unlisteners.push(
    listen<SubAgentCardEvent>("agent-subagent-card", (event) => {
      store.handleSubAgentCard(event.payload);
    }),
  );

  // Worker events
  unlisteners.push(
    listen<{
      conversationId: string;
      workerId: string;
      taskId: string;
      messageType: string;
      content: string;
      status?: string;
    }>("worker-created", (event) => {
      store.handleWorkerEvent({ ...event.payload, messageType: "progress", content: "Worker created" });
    }),
  );

  unlisteners.push(
    listen<{
      conversationId: string;
      workerId: string;
      taskId: string;
      messageType: string;
      content: string;
      status?: string;
    }>("worker-progress", (event) => {
      store.handleWorkerEvent(event.payload);
    }),
  );

  unlisteners.push(
    listen<{
      conversationId: string;
      workerId: string;
      taskId: string;
      messageType: string;
      content: string;
      status?: string;
    }>("worker-completed", (event) => {
      store.handleWorkerEvent({ ...event.payload, messageType: "completion", status: "completed" });
    }),
  );

  unlisteners.push(
    listen<{
      conversationId: string;
      workerId: string;
      taskId: string;
      messageType: string;
      content: string;
      status?: string;
    }>("worker-failed", (event) => {
      store.handleWorkerEvent({ ...event.payload, messageType: "error", status: "failed" });
    }),
  );

  // Workflow step events → sync to agentPool
  unlisteners.push(
    listen<{
      conversationId: string;
      stepId: string;
      stepGoal: string;
      agentRole: string;
    }>("workflow-step-start", (event) => {
      const item: AgentPoolItem = {
        id: event.payload.stepId,
        conversationId: event.payload.conversationId,
        type: "workflow_step",
        name: event.payload.stepGoal,
        status: "running",
        agentRole: event.payload.agentRole,
        startedAt: Date.now(),
      };
      store.upsertPoolItem(item);
    }),
  );

  unlisteners.push(
    listen<{
      conversationId: string;
      stepId: string;
      stepGoal: string;
      result: string;
    }>("workflow-step-complete", (event) => {
      store.upsertPoolItem({
        id: event.payload.stepId,
        conversationId: event.payload.conversationId,
        type: "workflow_step",
        name: event.payload.stepGoal,
        status: "completed",
        summary: event.payload.result,
      });
    }),
  );

  unlisteners.push(
    listen<{
      conversationId: string;
      stepId: string;
      error: string;
    }>("workflow-step-error", (event) => {
      store.upsertPoolItem({
        id: event.payload.stepId,
        conversationId: event.payload.conversationId,
        type: "workflow_step",
        name: event.payload.stepId,
        status: "failed",
        error: event.payload.error,
      });
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
    _listenersSetup = false;
    for (const p of unlisteners) {
      p.then((u) => u());
    }
  };
}
