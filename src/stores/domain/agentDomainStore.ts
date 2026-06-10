import { invoke, listen, logIpcError, type UnlistenFn } from "@/lib/invoke";
import type {
  AgentDoneEvent,
  AgentPoolItem,
  AgentPoolSummary,
  AgentSession,
  AskUserEvent,
  PermissionRequestEvent,
  Plan,
  SubAgentCardData,
  SubAgentCardEvent,
  ToolCallState,
  ToolResultEvent,
  ToolStartEvent,
  ToolUseEvent,
} from "@/types";
import { create } from "zustand";
import { persist } from "zustand/middleware";

import { useAgentStore } from "../feature/agentStore";
import { useExecutionStore } from "../feature/executionStore";
import { usePlanStore } from "../feature/planStore";

// ── Derived execution state (lightweight snapshot per conversation) ──

export interface ExecutionState {
  phase:
    | "idle"
    | "planning"
    | "executing"
    | "waiting_permission"
    | "completed"
    | "failed"
    | "cancelled";
  currentToolCallToolName: string | null;
  currentToolCallToolUseId: string | null;
  statusMessage: string | null;
}

// ── Plan state snapshot ──

export interface PlanState {
  activePlan: Plan | null;
  planHistoryCount: number;
  loading: boolean;
  error: string | null;
}

// ── Conversation aggregated view ──

export interface ConversationSummary {
  conversationId: string;
  session: AgentSession | null;
  executionState: ExecutionState;
  planState: PlanState;
  pendingPermissionsCount: number;
  pendingAskUserCount: number;
  isPaused: boolean;
  totalToolCalls: number;
  activeToolCalls: number;
}

// ── Store interface ──

interface AgentDomainStore {
  // ── Core state ──

  agentSession: Record<string, AgentSession>;
  executionState: Record<string, ExecutionState>;
  planState: Record<string, PlanState>;
  toolCalls: Record<string, ToolCallState>;
  permissions: Record<string, PermissionRequestEvent>;
  askUser: Record<string, AskUserEvent>;
  pausedConversations: Set<string>;
  agentPool: Record<string, AgentPoolItem[]>;
  subAgentCards: Record<string, SubAgentCardData>;
  sdkIdToExecId: Record<string, string>;

  // ── Session actions ──

  updateSession: (conversationId: string, session: AgentSession) => void;
  fetchSession: (conversationId: string) => Promise<AgentSession | null>;
  updateCwd: (conversationId: string, cwd: string) => Promise<void>;
  updatePermissionMode: (conversationId: string, mode: string) => Promise<void>;

  // ── Execution actions ──

  updateExecutionState: (
    conversationId: string,
    state: Partial<ExecutionState>,
  ) => void;

  // ── Plan actions ──

  updatePlanState: (conversationId: string, state: Partial<PlanState>) => void;

  // ── Tool call actions ──

  upsertToolCall: (toolUseId: string, toolCall: ToolCallState) => void;
  updateToolCallStatus: (
    toolUseId: string,
    status: ToolCallState["executionStatus"],
  ) => void;

  // ── Permission actions ──

  setPermission: (requestId: string, event: PermissionRequestEvent) => void;
  removePermission: (requestId: string) => void;
  expirePermissionsForConversation: (conversationId: string) => void;

  // ── AskUser actions ──

  setAskUser: (askId: string, event: AskUserEvent) => void;
  removeAskUser: (askId: string) => void;

  // ── Pause / Resume actions ──

  pauseAgent: (conversationId: string) => Promise<void>;
  resumeAgent: (conversationId: string) => Promise<void>;
  isPaused: (conversationId: string) => boolean;

  // ── AgentPool actions ──

  upsertPoolItem: (item: AgentPoolItem) => void;
  removePoolItem: (conversationId: string, itemId: string) => void;
  getPoolSummary: (conversationId: string) => AgentPoolSummary;

  // ── SubAgentCard actions ──

  upsertSubAgentCard: (cardId: string, card: SubAgentCardData) => void;

  // ── Event handlers ──

  handleToolUse: (event: ToolUseEvent) => void;
  handleToolStart: (event: ToolStartEvent) => void;
  handleToolResult: (event: ToolResultEvent) => void;
  handlePermissionRequest: (event: PermissionRequestEvent) => void;
  handlePermissionResolved: (toolUseId: string, decision: string) => void;
  handleAskUser: (event: AskUserEvent) => void;
  handleAskUserResolved: (askId: string) => void;
  handleSubAgentCard: (event: SubAgentCardEvent) => void;
  handleDone: (event: AgentDoneEvent) => void;

  // ── Cleanup ──

  clearConversation: (conversationId: string) => void;

  // ── Bulk sync from legacy stores ──

  syncFromAgentStore: (data: {
    sessions: Record<string, AgentSession>;
    toolCalls: Record<string, ToolCallState>;
    pendingPermissions: Record<string, PermissionRequestEvent>;
    pendingAskUser: Record<string, AskUserEvent>;
    pausedConversations: Set<string>;
    agentPool: Record<string, AgentPoolItem[]>;
    subAgentCards: Record<string, SubAgentCardData>;
    currentToolCall: {
      toolName: string;
      toolUseId: string;
      conversationId: string;
      startedAt: number;
    } | null;
    isExecuting: Record<string, boolean>;
    executingConversationIds: string[];
  }) => void;

  // ── Selectors (derived / computed state) ──

  getActiveToolCall: (conversationId: string) => ToolCallState | null;
  getConversationSummary: (conversationId: string) => ConversationSummary;
  getPendingActions: (conversationId: string) => {
    permissions: PermissionRequestEvent[];
    askUser: AskUserEvent[];
  };
  isConversationExecuting: (conversationId: string) => boolean;
  getExecutingConversationIds: () => string[];
}

// ── Helper: derive ExecutionState from executionStore ──

function deriveExecutionStateFromExecutionStore(
  conversationId: string,
): ExecutionState {
  const execStore = useExecutionStore.getState();
  const phase = execStore.phases[conversationId] || "idle";
  const currentToolCall = execStore.currentToolCall;
  const statusMessage = execStore.agentStatus[conversationId] || null;

  return {
    phase,
    currentToolCallToolName: currentToolCall?.conversationId === conversationId
      ? currentToolCall.toolName
      : null,
    currentToolCallToolUseId: currentToolCall?.conversationId === conversationId
      ? currentToolCall.toolUseId
      : null,
    statusMessage,
  };
}

// ── Helper: derive PlanState from planStore ──

function derivePlanStateFromPlanStore(conversationId: string): PlanState {
  const planStore = usePlanStore.getState();
  const activePlan = planStore.activePlans[conversationId] || null;
  const history = planStore.planHistory[conversationId] || [];
  const loading = planStore.loading[conversationId] || false;
  const error = planStore.errors[conversationId] || null;

  return {
    activePlan,
    planHistoryCount: history.length,
    loading,
    error,
  };
}

// ── Store implementation ──

/**
 * Safely extract a Set from a potentially corrupted persisted state.
 * Zustand persist serializes Set<string> to {}, so on restore it's a plain object.
 */
function toPausedSet(v: unknown): Set<string> {
  if (v instanceof Set) {
    return v;
  }
  if (Array.isArray(v)) {
    return new Set(v);
  }
  if (v && typeof v === "object") {
    return new Set(Object.keys(v));
  }
  return new Set<string>();
}

export const useAgentDomainStore = create<AgentDomainStore>()(
  persist(
    (set, get) => ({
      // ── Initial state ──

      agentSession: {},
      executionState: {},
      planState: {},
      toolCalls: {},
      permissions: {},
      askUser: {},
      pausedConversations: new Set<string>(),
      agentPool: {},
      subAgentCards: {},
      sdkIdToExecId: {},

      // ── Session actions ──

      updateSession: (conversationId, session) => {
        set((s) => ({
          agentSession: { ...s.agentSession, [conversationId]: session },
        }));
      },

      fetchSession: async (conversationId) => {
        try {
          const session = await invoke<AgentSession | null>(
            "agent_get_session",
            { conversationId },
          );
          if (session) {
            get().updateSession(conversationId, session);
          }
          return session;
        } catch (e) {
          logIpcError("agentDomainStore: fetchSession failed")(e);
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
          get().updateSession(conversationId, session);
        } catch (e) {
          logIpcError("agentDomainStore: updateCwd failed")(e);
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
          get().updateSession(conversationId, session);
        } catch (e) {
          logIpcError("agentDomainStore: updatePermissionMode failed")(e);
        }
      },

      // ── Execution actions ──

      updateExecutionState: (conversationId, state) => {
        set((s) => {
          const existing = s.executionState[conversationId] || {
            phase: "idle" as const,
            currentToolCallToolName: null,
            currentToolCallToolUseId: null,
            statusMessage: null,
          };
          return {
            executionState: {
              ...s.executionState,
              [conversationId]: { ...existing, ...state },
            },
          };
        });
      },

      // ── Plan actions ──

      updatePlanState: (conversationId, state) => {
        set((s) => {
          const existing = s.planState[conversationId] || {
            activePlan: null,
            planHistoryCount: 0,
            loading: false,
            error: null,
          };
          return {
            planState: {
              ...s.planState,
              [conversationId]: { ...existing, ...state },
            },
          };
        });
      },

      // ── Tool call actions ──

      upsertToolCall: (toolUseId, toolCall) => {
        set((s) => ({
          toolCalls: { ...s.toolCalls, [toolUseId]: toolCall },
        }));
      },

      updateToolCallStatus: (toolUseId, status) => {
        set((s) => {
          const existing = s.toolCalls[toolUseId];
          if (!existing) {
            return {};
          }
          return {
            toolCalls: {
              ...s.toolCalls,
              [toolUseId]: { ...existing, executionStatus: status },
            },
          };
        });
      },

      // ── Permission actions ──

      setPermission: (requestId, event) => {
        set((s) => ({
          permissions: { ...s.permissions, [requestId]: event },
        }));
      },

      removePermission: (requestId) => {
        set((s) => {
          const rest = { ...s.permissions };
          delete rest[requestId];
          return { permissions: rest };
        });
      },

      expirePermissionsForConversation: (conversationId) => {
        set((s) => {
          const expiredKeys = new Set<string>();
          for (const [id, pr] of Object.entries(s.permissions)) {
            if (pr.conversationId === conversationId) {
              expiredKeys.add(id);
            }
          }
          if (expiredKeys.size === 0) {
            return {};
          }

          const permissions: Record<string, PermissionRequestEvent> = {};
          for (const [id, pr] of Object.entries(s.permissions)) {
            if (!expiredKeys.has(id)) {
              permissions[id] = pr;
            }
          }

          const toolCalls: Record<string, ToolCallState> = { ...s.toolCalls };
          for (const id of expiredKeys) {
            if (toolCalls[id]) {
              toolCalls[id] = {
                ...toolCalls[id],
                approvalStatus: "denied" as const,
              };
            }
          }

          return { permissions, toolCalls };
        });
      },

      // ── AskUser actions ──

      setAskUser: (askId, event) => {
        set((s) => ({
          askUser: { ...s.askUser, [askId]: event },
        }));
      },

      removeAskUser: (askId) => {
        set((s) => {
          const rest = { ...s.askUser };
          delete rest[askId];
          return { askUser: rest };
        });
      },

      // ── Pause / Resume actions ──

      pauseAgent: async (conversationId) => {
        try {
          await invoke("agent_pause", { conversationId });
          set((s) => {
            const pausedConversations = toPausedSet(s.pausedConversations);
            pausedConversations.add(conversationId);
            return { pausedConversations };
          });
        } catch (err) {
          logIpcError("agentDomainStore: pauseAgent failed")(err);
        }
      },

      resumeAgent: async (conversationId) => {
        try {
          await invoke("agent_resume", { conversationId });
          set((s) => {
            const pausedConversations = toPausedSet(s.pausedConversations);
            pausedConversations.delete(conversationId);
            return { pausedConversations };
          });
        } catch (err) {
          logIpcError("agentDomainStore: resumeAgent failed")(err);
        }
      },

      isPaused: (conversationId) => {
        return toPausedSet(get().pausedConversations).has(conversationId);
      },

      // ── AgentPool actions ──

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
          const pool = [...(s.agentPool[conversationId] || [])];
          const idx = pool.findIndex((p) => p.id === itemId);
          if (idx >= 0) {
            pool.splice(idx, 1);
            return { agentPool: { ...s.agentPool, [conversationId]: pool } };
          }
          return {};
        });
      },

      getPoolSummary: (conversationId) => {
        const pool = get().agentPool[conversationId] || [];
        const total = pool.length;
        if (total === 0) {
          return {
            total: 0,
            completed: 0,
            running: 0,
            pending: 0,
            failed: 0,
            pctComplete: 0,
          };
        }
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
          pctComplete: Math.round((completed / total) * 100),
        };
      },

      // ── SubAgentCard actions ──

      upsertSubAgentCard: (cardId, card) => {
        set((s) => ({
          subAgentCards: { ...s.subAgentCards, [cardId]: card },
        }));
      },

      // ── Event handlers ──

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
            updates[event.executionId] = {
              ...toolCall,
              toolUseId: event.executionId,
            };
            idMap[event.toolUseId] = event.executionId;
          }

          const execState: ExecutionState = {
            phase: "executing",
            currentToolCallToolName: event.toolName,
            currentToolCallToolUseId: event.toolUseId,
            statusMessage: null,
          };

          return {
            toolCalls: { ...s.toolCalls, ...updates },
            sdkIdToExecId: idMap,
            executionState: {
              ...s.executionState,
              [event.conversationId]: execState,
            },
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
          if (!existing) {
            return {};
          }

          const newStatus: ToolCallState["executionStatus"] = event.isError
            ? "failed"
            : "success";
          const updated: ToolCallState = {
            ...existing,
            executionStatus: newStatus,
            output: event.content,
            isError: event.isError,
            toolName: event.toolName || existing.toolName,
          };

          const updates: Record<string, ToolCallState> = {
            [event.toolUseId]: updated,
          };

          const execId = s.sdkIdToExecId[event.toolUseId];
          if (execId) {
            updates[execId] = { ...updated, toolUseId: execId };
          }

          const isCurrentlyExecuting = s.executionState[event.conversationId]?.currentToolCallToolUseId
            === event.toolUseId;

          let executionState = s.executionState;
          if (isCurrentlyExecuting) {
            executionState = {
              ...s.executionState,
              [event.conversationId]: {
                ...s.executionState[event.conversationId],
                currentToolCallToolName: null,
                currentToolCallToolUseId: null,
              },
            };
          }

          return { toolCalls: { ...s.toolCalls, ...updates }, executionState };
        });
      },

      handlePermissionRequest: (event) => {
        const key = event.requestId || event.toolUseId;
        get().setPermission(key, event);
      },

      handlePermissionResolved: (toolUseId, decision) => {
        get().removePermission(toolUseId);
        set((s) => {
          const existing = s.toolCalls[toolUseId];
          if (!existing) {
            return {};
          }
          return {
            toolCalls: {
              ...s.toolCalls,
              [toolUseId]: {
                ...existing,
                approvalStatus: decision === "deny"
                  ? ("denied" as const)
                  : ("approved" as const),
              },
            },
          };
        });
      },

      handleAskUser: (event) => {
        get().setAskUser(event.askId, event);
      },

      handleAskUserResolved: (askId) => {
        get().removeAskUser(askId);
      },

      handleSubAgentCard: (event) => {
        const cardId = event.childConversationId || `card-${Date.now()}`;
        const card: SubAgentCardData = {
          id: cardId,
          conversationId: event.conversationId,
          agentType: event.agentType,
          agentName: event.agentName,
          description: event.description,
          status: event.status,
          childConversationId: event.childConversationId,
          childSessionId: event.childSessionId,
          isFork: event.isFork,
        };

        const poolItem: AgentPoolItem = {
          id: cardId,
          conversationId: event.conversationId,
          type: "sub_agent",
          name: event.agentName || event.agentType,
          status: event.status === "failed"
            ? "failed"
            : event.status === "completed"
            ? "completed"
            : "running",
          agentType: event.agentType,
          childConversationId: event.childConversationId,
          childSessionId: event.childSessionId,
          isFork: event.isFork,
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

      handleDone: (event) => {
        get().expirePermissionsForConversation(event.conversationId);
        set((s) => ({
          executionState: {
            ...s.executionState,
            [event.conversationId]: {
              ...(s.executionState[event.conversationId] || {
                phase: "idle",
                currentToolCallToolName: null,
                currentToolCallToolUseId: null,
                statusMessage: null,
              }),
              phase: "completed",
              currentToolCallToolName: null,
              currentToolCallToolUseId: null,
            },
          },
        }));
      },

      // ── Cleanup ──

      clearConversation: (conversationId) => {
        set((s) => {
          const agentSession = { ...s.agentSession };
          delete agentSession[conversationId];
          const executionState = { ...s.executionState };
          delete executionState[conversationId];
          const planState = { ...s.planState };
          delete planState[conversationId];
          const agentPool = { ...s.agentPool };
          delete agentPool[conversationId];

          const permissions: Record<string, PermissionRequestEvent> = {};
          for (const [id, pr] of Object.entries(s.permissions)) {
            if (pr.conversationId !== conversationId) {
              permissions[id] = pr;
            }
          }

          const askUser: Record<string, AskUserEvent> = {};
          for (const [id, ask] of Object.entries(s.askUser)) {
            if (ask.conversationId !== conversationId) {
              askUser[id] = ask;
            }
          }

          const removedPermToolUseIds = new Set<string>();
          for (const [id, pr] of Object.entries(s.permissions)) {
            if (pr.conversationId === conversationId) {
              removedPermToolUseIds.add(pr.toolUseId);
              removedPermToolUseIds.add(id);
            }
          }

          const toolCalls: Record<string, ToolCallState> = {};
          for (const [id, tc] of Object.entries(s.toolCalls)) {
            if (
              !removedPermToolUseIds.has(id)
              && !removedPermToolUseIds.has(tc.toolUseId)
            ) {
              toolCalls[id] = tc;
            }
          }

          const sdkIdToExecId: Record<string, string> = {};
          for (const [sdkId, execId] of Object.entries(s.sdkIdToExecId)) {
            if (
              !removedPermToolUseIds.has(sdkId)
              && !removedPermToolUseIds.has(execId)
            ) {
              sdkIdToExecId[sdkId] = execId;
            }
          }

          const subAgentCards: Record<string, SubAgentCardData> = {};
          for (const [id, card] of Object.entries(s.subAgentCards)) {
            if (card.conversationId !== conversationId) {
              subAgentCards[id] = card;
            }
          }

          const pausedConversations = toPausedSet(s.pausedConversations);
          pausedConversations.delete(conversationId);

          return {
            agentSession,
            executionState,
            planState,
            agentPool,
            permissions,
            askUser,
            toolCalls,
            sdkIdToExecId,
            subAgentCards,
            pausedConversations,
          };
        });
      },

      // ── Bulk sync from legacy stores ──

      syncFromAgentStore: (data) => {
        set((s) => {
          const executionState = { ...s.executionState };
          const executingIds = data.executingConversationIds ?? [];
          const pausedConvs = data.pausedConversations ?? new Set<string>();

          for (const convId of executingIds) {
            const existing = executionState[convId] || {
              phase: "idle" as const,
              currentToolCallToolName: null,
              currentToolCallToolUseId: null,
              statusMessage: null,
            };
            executionState[convId] = {
              ...existing,
              phase: data.isExecuting?.[convId] ? "executing" : existing.phase,
              currentToolCallToolName: data.currentToolCall?.conversationId === convId
                ? data.currentToolCall.toolName
                : existing.currentToolCallToolName,
              currentToolCallToolUseId: data.currentToolCall?.conversationId === convId
                ? data.currentToolCall.toolUseId
                : existing.currentToolCallToolUseId,
            };
          }

          return {
            agentSession: { ...s.agentSession, ...data.sessions },
            toolCalls: { ...s.toolCalls, ...data.toolCalls },
            permissions: { ...s.permissions, ...data.pendingPermissions },
            askUser: { ...s.askUser, ...data.pendingAskUser },
            pausedConversations: new Set([
              ...toPausedSet(s.pausedConversations),
              ...pausedConvs,
            ]),
            agentPool: { ...s.agentPool, ...data.agentPool },
            subAgentCards: { ...s.subAgentCards, ...data.subAgentCards },
            executionState,
          };
        });
      },

      // ── Selectors ──

      getActiveToolCall: (conversationId) => {
        const state = get();
        const currentToolUseId = state.executionState[conversationId]?.currentToolCallToolUseId;
        if (!currentToolUseId) {
          return null;
        }
        return state.toolCalls[currentToolUseId] || null;
      },

      getConversationSummary: (conversationId) => {
        const state = get();
        const session = state.agentSession[conversationId] || null;
        const execState = state.executionState[conversationId]
          || deriveExecutionStateFromExecutionStore(conversationId);
        const planSt = state.planState[conversationId]
          || derivePlanStateFromPlanStore(conversationId);

        const permissionCount = Object.values(state.permissions).filter(
          (p) => p.conversationId === conversationId,
        ).length;

        const askUserCount = Object.values(state.askUser).filter(
          (a) => a.conversationId === conversationId,
        ).length;

        const toolCallEntries = Object.values(state.toolCalls);
        const relatedToolCalls = toolCallEntries.filter((tc) => {
          const execId = state.sdkIdToExecId[tc.toolUseId];
          return tc.assistantMessageId === session?.id || execId !== undefined;
        });

        const activeToolCalls = relatedToolCalls.filter(
          (tc) => tc.executionStatus === "running" || tc.executionStatus === "queued",
        ).length;

        return {
          conversationId,
          session,
          executionState: execState,
          planState: planSt,
          pendingPermissionsCount: permissionCount,
          pendingAskUserCount: askUserCount,
          isPaused: toPausedSet(state.pausedConversations).has(conversationId),
          totalToolCalls: relatedToolCalls.length,
          activeToolCalls,
        };
      },

      getPendingActions: (conversationId) => {
        const state = get();
        const permissions = Object.values(state.permissions).filter(
          (p) => p.conversationId === conversationId,
        );
        const askUser = Object.values(state.askUser).filter(
          (a) => a.conversationId === conversationId,
        );
        return { permissions, askUser };
      },

      isConversationExecuting: (conversationId) => {
        const state = get();
        const execState = state.executionState[conversationId];
        if (
          execState
          && (execState.phase === "executing"
            || execState.phase === "waiting_permission")
        ) {
          return true;
        }
        return Object.values(state.toolCalls).some(
          (tc) => tc.executionStatus === "running" || tc.executionStatus === "queued",
        );
      },

      getExecutingConversationIds: () => {
        const state = get();
        const executingIds = new Set<string>();

        for (const [convId, execSt] of Object.entries(state.executionState)) {
          if (
            execSt.phase === "executing"
            || execSt.phase === "waiting_permission"
          ) {
            executingIds.add(convId);
          }
        }

        for (const [, tc] of Object.entries(state.toolCalls)) {
          if (
            tc.executionStatus === "running"
            || tc.executionStatus === "queued"
          ) {
            const execId = state.sdkIdToExecId[tc.toolUseId];
            if (execId) {
              for (
                const [convId, execSt] of Object.entries(
                  state.executionState,
                )
              ) {
                if (
                  execSt.currentToolCallToolUseId === tc.toolUseId
                  || execSt.currentToolCallToolUseId === execId
                ) {
                  executingIds.add(convId);
                }
              }
            }
          }
        }

        return Array.from(executingIds);
      },
    }),
    {
      name: "axagent:agent-domain-store",
      partialize: (state) => ({
        agentSession: state.agentSession,
        sdkIdToExecId: state.sdkIdToExecId,
      }),
    },
  ),
);

// ── Bridge helpers for backward compatibility ──

export function syncAgentStoreToDomainStore(): void {
  const agentStore = useAgentStore.getState();

  const sessionUpdates: Record<string, AgentSession> = {};
  for (const [convId, session] of Object.entries(agentStore.sessions)) {
    sessionUpdates[convId] = session;
  }
  if (Object.keys(sessionUpdates).length > 0) {
    useAgentDomainStore.setState({ agentSession: sessionUpdates });
  }
}

export function syncExecutionStoreToDomainStore(): void {
  const execStore = useExecutionStore.getState();

  const executionStates: Record<string, ExecutionState> = {};
  for (const [convId, phase] of Object.entries(execStore.phases)) {
    const statusMessage = execStore.agentStatus[convId] || null;
    const currentToolCall = execStore.currentToolCall;
    executionStates[convId] = {
      phase,
      currentToolCallToolName: currentToolCall?.conversationId === convId
        ? currentToolCall.toolName
        : null,
      currentToolCallToolUseId: currentToolCall?.conversationId === convId
        ? currentToolCall.toolUseId
        : null,
      statusMessage,
    };
  }

  if (Object.keys(executionStates).length > 0) {
    useAgentDomainStore.setState({ executionState: executionStates });
  }
}

export function syncPlanStoreToDomainStore(): void {
  const planStore = usePlanStore.getState();
  const planStates: Record<string, PlanState> = {};

  for (const [convId, plan] of Object.entries(planStore.activePlans)) {
    const history = planStore.planHistory[convId] || [];
    const loading = planStore.loading[convId] || false;
    const error = planStore.errors[convId] || null;
    planStates[convId] = {
      activePlan: plan,
      planHistoryCount: history.length,
      loading,
      error,
    };
  }

  if (Object.keys(planStates).length > 0) {
    useAgentDomainStore.setState({ planState: planStates });
  }
}

export function syncAllStoresToDomain(): void {
  const agentState = useAgentStore.getState();
  const domainState = useAgentDomainStore.getState();

  domainState.syncFromAgentStore({
    sessions: agentState.sessions,
    toolCalls: agentState.toolCalls,
    pendingPermissions: agentState.pendingPermissions,
    pendingAskUser: agentState.pendingAskUser,
    pausedConversations: agentState.pausedConversations,
    agentPool: agentState.agentPool,
    subAgentCards: agentState.subAgentCards,
    currentToolCall: agentState.currentToolCall,
    isExecuting: agentState.isExecuting,
    executingConversationIds: agentState.executingConversationIds,
  });

  syncExecutionStoreToDomainStore();
  syncPlanStoreToDomainStore();
}

// ── Domain store to legacy store bridge (for reading from domain store) ──

export function getDomainStoreAsAgentStoreSnapshot() {
  const domain = useAgentDomainStore.getState();

  return {
    sessions: domain.agentSession,
    pendingPermissions: domain.permissions,
    pendingAskUser: domain.askUser,
    toolCalls: domain.toolCalls,
    pausedConversations: domain.pausedConversations,
    agentPool: domain.agentPool,
    subAgentCards: domain.subAgentCards,
  };
}

export function getDomainStoreAsExecutionStoreSnapshot() {
  const domain = useAgentDomainStore.getState();

  const phases: Record<
    string,
    | "idle"
    | "planning"
    | "executing"
    | "waiting_permission"
    | "completed"
    | "failed"
    | "cancelled"
  > = {};
  const agentStatus: Record<string, string> = {};
  const currentToolCall = domain.toolCalls
    ? Object.values(domain.toolCalls).find(
      (tc) => tc.executionStatus === "running" || tc.executionStatus === "queued",
    )
    : null;

  for (const [convId, execSt] of Object.entries(domain.executionState)) {
    phases[convId] = execSt.phase;
    if (execSt.statusMessage) {
      agentStatus[convId] = execSt.statusMessage;
    }
  }

  return {
    phases,
    agentStatus,
    toolCalls: domain.toolCalls,
    sdkIdToExecId: domain.sdkIdToExecId,
    agentPool: domain.agentPool,
    currentToolCall: currentToolCall
      ? {
        toolName: currentToolCall.toolName,
        toolUseId: currentToolCall.toolUseId,
        conversationId: "",
        startedAt: 0,
      }
      : null,
  };
}

export function getDomainStoreAsPlanStoreSnapshot() {
  const domain = useAgentDomainStore.getState();

  const activePlans: Record<string, Plan> = {};
  const planHistory: Record<string, Plan[]> = {};
  const loading: Record<string, boolean> = {};
  const errors: Record<string, string | null> = {};

  for (const [convId, planSt] of Object.entries(domain.planState)) {
    if (planSt.activePlan) {
      activePlans[convId] = planSt.activePlan;
    }
    loading[convId] = planSt.loading;
    errors[convId] = planSt.error;
  }

  return { activePlans, planHistory, loading, errors };
}

// ── Unified event listener setup ──

let _domainListenersSetup = false;

export function setupAgentDomainEventListeners(): () => void {
  if (_domainListenersSetup) {
    return () => {};
  }
  _domainListenersSetup = true;

  const unlisteners: Promise<UnlistenFn>[] = [];
  const store = useAgentDomainStore.getState();

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
    listen<SubAgentCardEvent>("agent-subagent-card", (event) => {
      store.handleSubAgentCard(event.payload);
    }),
  );

  unlisteners.push(
    listen<AgentDoneEvent>("agent-done", (event) => {
      store.handleDone(event.payload);
    }),
  );

  unlisteners.push(
    listen<{ conversationId: string }>("agent-paused", (event) => {
      useAgentDomainStore.setState((s) => {
        const pausedConversations = toPausedSet(s.pausedConversations);
        pausedConversations.add(event.payload.conversationId);
        return { pausedConversations };
      });
    }),
  );

  unlisteners.push(
    listen<{ conversationId: string }>("agent-resumed", (event) => {
      useAgentDomainStore.setState((s) => {
        const pausedConversations = toPausedSet(s.pausedConversations);
        pausedConversations.delete(event.payload.conversationId);
        return { pausedConversations };
      });
    }),
  );

  return () => {
    _domainListenersSetup = false;
    for (const p of unlisteners) {
      p.then((u) => u());
    }
  };
}
