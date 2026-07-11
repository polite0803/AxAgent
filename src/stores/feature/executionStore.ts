// SPDX-License-Identifier: AGPL-3.0-only

/**
 * 统一智能体执行状态 Store
 * 整合：agentStore 执行态 + trajectoryStore + ExecutionPhase 状态机
 */
import i18n from "@/i18n";
import { invoke, listen, type UnlistenFn } from "@/lib/invoke";
import type {
  AgentCancelledEvent,
  AgentDoneEvent,
  AgentErrorEvent,
  AgentPoolItem,
  AgentPoolSummary,
  AgentStatusEvent,
  SubAgentCardEvent,
  ToolCallState,
  ToolResultEvent,
  ToolStartEvent,
  ToolUseEvent,
  TrajectoryDetail,
  TrajectorySummary,
  WorkerMessage,
} from "@/types";
import { create } from "zustand";
import { devtools } from "zustand/middleware";

import { ACTIVE_PHASES, type ExecutionPhase, PHASE_TRANSITIONS, TERMINAL_PHASES } from "./executionPhaseMachine";

import type { CurrentToolCall } from "./executionToolCallUtils";
import { shouldClearToolCall } from "./executionToolCallUtils";

// ── Store 接口 ──

interface ExecutionStore {
  // === 阶段状态机 (per conversationId) ===
  phases: Record<string, ExecutionPhase>;

  // === 执行进度追踪 ===
  currentToolCall: CurrentToolCall | null;
  agentStatus: Record<string, string>;

  // === 工具调用 ===
  toolCalls: Record<string, ToolCallState>;
  sdkIdToExecId: Record<string, string>;

  // === Agent 池 ===
  agentPool: Record<string, AgentPoolItem[]>;

  // === 轨迹 ===
  trajectoriesByConversation: Record<string, TrajectorySummary[]>;
  trajectoryDetails: Record<string, TrajectoryDetail | null>;
  loadingTrajectories: boolean;
  loadingTrajectoryDetail: Record<string, boolean>;

  // === 阶段机 Actions ===
  transition: (conversationId: string, to: ExecutionPhase) => void;
  isActive: (conversationId: string) => boolean;
  isTerminal: (conversationId: string) => boolean;
  getActiveConversations: () => string[];

  // === 进度 Actions ===
  setCurrentTool: (tc: CurrentToolCall | null) => void;
  setAgentStatus: (conversationId: string, message: string) => void;
  clearAgentStatus: (conversationId: string) => void;

  // === 池 Actions ===
  upsertPoolItem: (item: AgentPoolItem) => void;
  removePoolItem: (conversationId: string, itemId: string) => void;
  getPoolSummary: (conversationId: string) => AgentPoolSummary;

  // === 工具调用 Actions ===
  handleToolUse: (event: ToolUseEvent) => void;
  handleToolStart: (event: ToolStartEvent) => void;
  handleToolResult: (event: ToolResultEvent) => void;

  // === Worker Actions ===
  handleWorkerEvent: (event: {
    conversationId: string;
    workerId: string;
    taskId: string;
    messageType: string;
    content: string;
    status?: string;
  }) => void;

  // === 生命周期 Actions ===
  handleSubAgentCard: (event: SubAgentCardEvent) => void;
  handleDone: (event: AgentDoneEvent) => void;
  handleError: (event: AgentErrorEvent) => void;
  handleCancelled: (event: AgentCancelledEvent) => void;

  // === 轨迹 Actions ===
  fetchTrajectoryList: (conversationId: string) => Promise<void>;
  fetchTrajectoryDetail: (
    trajectoryId: string,
  ) => Promise<TrajectoryDetail | null>;

  // === 清理 ===
  clearConversation: (conversationId: string) => void;
  clearConversationUI: (conversationId: string) => void;
}

// ── 模块级追踪：最近一次的 assistantMessageId ──

const _latestMessageIdByConv: Record<string, string> = {};

// ── 初始状态 ──

const initialState = {
  phases: {} as Record<string, ExecutionPhase>,
  currentToolCall: null as CurrentToolCall | null,
  agentStatus: {} as Record<string, string>,
  toolCalls: {} as Record<string, ToolCallState>,
  sdkIdToExecId: {} as Record<string, string>,
  agentPool: {} as Record<string, AgentPoolItem[]>,
  trajectoriesByConversation: {} as Record<string, TrajectorySummary[]>,
  trajectoryDetails: {} as Record<string, TrajectoryDetail | null>,
  loadingTrajectories: false,
  loadingTrajectoryDetail: {} as Record<string, boolean>,
};

export const useExecutionStore = create<ExecutionStore>()(
  devtools(
    (set, get) => ({
      ...initialState,

      // ── 阶段机 ──

      transition: (conversationId, to) => {
        set(
          (s) => {
            const from = s.phases[conversationId] || "idle";
            if (from === to) {
              return {};
            }
            const allowed = PHASE_TRANSITIONS[from] || [];
            if (!allowed.includes(to)) {
              return {};
            }
            return { phases: { ...s.phases, [conversationId]: to } };
          },
          false,
          { type: "phase-transition", conversationId, to },
        );
      },

      isActive: (conversationId) => {
        const phase = get().phases[conversationId];
        return phase ? ACTIVE_PHASES.has(phase) : false;
      },

      isTerminal: (conversationId) => {
        const phase = get().phases[conversationId];
        return phase ? TERMINAL_PHASES.has(phase) : false;
      },

      getActiveConversations: () => {
        return Object.entries(get().phases).flatMap(([id, p]) => ACTIVE_PHASES.has(p) ? [id] : []);
      },

      // ── 进度 ──

      setCurrentTool: (tc) => {
        set({ currentToolCall: tc }, false, { type: "set-current-tool", tc });
      },

      setAgentStatus: (conversationId, message) => {
        set(
          (s) => ({
            agentStatus: { ...s.agentStatus, [conversationId]: message },
          }),
          false,
          { type: "agent-status", conversationId },
        );
      },

      clearAgentStatus: (conversationId) => {
        set(
          (s) => {
            const rest = { ...s.agentStatus };
            delete rest[conversationId];
            return { agentStatus: rest };
          },
          false,
          { type: "clear-agent-status", conversationId },
        );
      },

      // ── 池 ──

      upsertPoolItem: (item) => {
        set(
          (s) => {
            const pool = [...(s.agentPool[item.conversationId] || [])];
            const idx = pool.findIndex((p) => p.id === item.id);
            if (idx >= 0) {
              pool[idx] = { ...pool[idx], ...item };
            } else {
              pool.push(item);
            }
            return {
              agentPool: { ...s.agentPool, [item.conversationId]: pool },
            };
          },
          false,
          { type: "upsert-pool-item", item },
        );
      },

      removePoolItem: (conversationId, itemId) => {
        set(
          (s) => {
            const pool = [...(s.agentPool[conversationId] || [])];
            const idx = pool.findIndex((p) => p.id === itemId);
            if (idx >= 0) {
              pool.splice(idx, 1);
              return { agentPool: { ...s.agentPool, [conversationId]: pool } };
            }
            return {};
          },
          false,
          { type: "remove-pool-item", conversationId, itemId },
        );
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
        const completed = pool.filter((i) => i.status === "completed").length;
        const running = pool.filter((i) => i.status === "running").length;
        const pending = pool.filter((i) => i.status === "pending").length;
        const failed = pool.filter((i) => i.status === "failed").length;
        return {
          total,
          completed,
          running,
          pending,
          failed,
          pctComplete: Math.round((completed / total) * 100),
        };
      },

      // ── 工具调用 ──

      handleToolUse: (event) => {
        if (event.assistantMessageId && event.conversationId) {
          _latestMessageIdByConv[event.conversationId] = event.assistantMessageId;
        }
        set(
          (s) => {
            const tc: ToolCallState = {
              toolUseId: event.toolUseId,
              toolName: event.toolName,
              input: event.input,
              assistantMessageId: event.assistantMessageId,
              executionStatus: "queued",
            };
            const updates: Record<string, ToolCallState> = {
              [event.toolUseId]: tc,
            };
            const idMap = { ...s.sdkIdToExecId };
            if (event.executionId) {
              updates[event.executionId] = {
                ...tc,
                toolUseId: event.executionId,
              };
              idMap[event.toolUseId] = event.executionId;
            }
            const currentToolCall: CurrentToolCall = {
              toolName: event.toolName,
              toolUseId: event.toolUseId,
              conversationId: event.conversationId,
              startedAt: Date.now(),
            };
            return {
              toolCalls: { ...s.toolCalls, ...updates },
              sdkIdToExecId: idMap,
              currentToolCall,
            };
          },
          false,
          {
            type: "tool-use",
            toolName: event.toolName,
            conversationId: event.conversationId,
          },
        );
        // 自动进入 executing 阶段（避免重复转换造成 warn）
        const current = get().phases[event.conversationId];
        if (current !== "executing") {
          get().transition(event.conversationId, "executing");
        }
      },

      handleToolStart: (event) => {
        const poolId = `tool-${event.toolUseId}`;
        set(
          (s) => {
            const existing = s.toolCalls[event.toolUseId];
            const updates: Record<string, ToolCallState> = {
              [event.toolUseId]: {
                ...(existing || {
                  toolUseId: event.toolUseId,
                  toolName: event.toolName,
                  input: event.input ?? {},
                  assistantMessageId: event.assistantMessageId || "",
                }),
                executionStatus: "running",
                startedAt: existing?.startedAt ?? Date.now(),
              },
            };
            // Also add to agentPool so AgentPoolPanel / ExecutionTimeline
            // show tool executions during the main agent path.
            const pool = [...(s.agentPool[event.conversationId] || [])];
            const idx = pool.findIndex((p) => p.id === poolId);
            const poolItem: AgentPoolItem = {
              id: poolId,
              conversationId: event.conversationId,
              type: "worker",
              name: event.toolName,
              status: "running",
              currentTask: event.toolName,
              startedAt: Date.now(),
              messageId: _latestMessageIdByConv[event.conversationId],
            };
            if (idx >= 0) { pool[idx] = { ...pool[idx], ...poolItem }; }
            else { pool.push(poolItem); }
            return {
              toolCalls: { ...s.toolCalls, ...updates },
              agentPool: { ...s.agentPool, [event.conversationId]: pool },
            };
          },
          false,
          { type: "tool-start", toolUseId: event.toolUseId },
        );
      },

      handleToolResult: (event) => {
        const poolId = `tool-${event.toolUseId}`;
        set(
          (s) => {
            const existing = s.toolCalls[event.toolUseId];
            if (!existing) {
              console.warn(
                `[executionStore] Tool result for unknown toolUseId: ${event.toolUseId}`,
              );
              const fallback: ToolCallState = {
                toolUseId: event.toolUseId,
                toolName: event.toolName || "unknown",
                input: event.input ?? {},
                assistantMessageId: event.assistantMessageId || "",
                executionStatus: event.isError ? "failed" : "success",
                output: event.content,
                isError: event.isError,
              };
              // 2.1: Sync fallback to agentPool so clearConversation can clean it up
              const pool = [...(s.agentPool[event.conversationId] || [])];
              const poolIdx = pool.findIndex((p) => p.id === poolId);
              const poolItem: AgentPoolItem = {
                id: poolId,
                conversationId: event.conversationId,
                type: "worker",
                name: event.toolName || "unknown",
                status: event.isError ? "failed" : "completed",
                summary: event.content?.slice(0, 200),
                error: event.isError ? (event.content ?? "Tool failed") : undefined,
                messageId: event.assistantMessageId || _latestMessageIdByConv[event.conversationId],
              };
              if (poolIdx >= 0) { pool[poolIdx] = { ...pool[poolIdx], ...poolItem }; }
              else { pool.push(poolItem); }
              return {
                toolCalls: { ...s.toolCalls, [event.toolUseId]: fallback },
                agentPool: { ...s.agentPool, [event.conversationId]: pool },
              };
            }
            const updates: Record<string, ToolCallState> = {
              [event.toolUseId]: {
                ...existing,
                executionStatus: event.isError ? "failed" : "success",
                output: event.content,
                isError: event.isError,
              },
            };
            // 2.3: Sync executionId key via sdkIdToExecId map
            const execId = s.sdkIdToExecId[event.toolUseId];
            if (execId && s.toolCalls[execId]) {
              updates[execId] = {
                ...s.toolCalls[execId],
                executionStatus: event.isError ? "failed" : "success",
                output: event.content,
                isError: event.isError,
              };
            }
            const currentToolCall = s.currentToolCall?.toolUseId === event.toolUseId
              ? null
              : s.currentToolCall;
            // Update pool item for AgentPoolPanel / ExecutionTimeline
            const pool = [...(s.agentPool[event.conversationId] || [])];
            const idx = pool.findIndex((p) => p.id === poolId);
            if (idx >= 0) {
              pool[idx] = {
                ...pool[idx],
                status: event.isError ? "failed" : "completed",
                summary: event.content?.slice(0, 200),
                error: event.isError ? (event.content ?? "Tool failed") : undefined,
                duration: pool[idx].startedAt != null
                  ? Date.now() - pool[idx].startedAt
                  : undefined,
              };
            }
            return {
              toolCalls: { ...s.toolCalls, ...updates },
              currentToolCall,
              agentPool: { ...s.agentPool, [event.conversationId]: pool },
            };
          },
          false,
          { type: "tool-result", toolUseId: event.toolUseId },
        );
      },

      // ── Worker ──

      handleWorkerEvent: (event) => {
        const poolId = `worker-${event.workerId}`;
        const msg: WorkerMessage = {
          workerId: event.workerId,
          taskId: event.taskId,
          messageType: (event.messageType
            || "progress") as WorkerMessage["messageType"],
          content: event.content,
          timestamp: Date.now(),
        };
        set(
          (s) => {
            const pool = [...(s.agentPool[event.conversationId] || [])];
            const idx = pool.findIndex((p) => p.id === poolId);
            const statusMap: Record<string, AgentPoolItem["status"]> = {
              progress: "running",
              result: "completed",
              completion: "completed",
              error: "failed",
            };
            const newStatus = (event.status
              || statusMap[event.messageType]
              || "running") as AgentPoolItem["status"];
            if (idx >= 0) {
              const existing = pool[idx];
              pool[idx] = {
                ...existing,
                status: newStatus,
                summary: event.messageType === "progress"
                  ? event.content
                  : existing.summary,
                error: event.messageType === "error"
                  ? event.content
                  : existing.error,
                messages: [...(existing.messages || []), msg],
                duration: existing.startedAt != null
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
                messageId: _latestMessageIdByConv[event.conversationId],
              });
            }
            return {
              agentPool: { ...s.agentPool, [event.conversationId]: pool },
            };
          },
          false,
          { type: "worker-event", workerId: event.workerId },
        );
      },

      // ── 生命周期 ──

      handleSubAgentCard: (event) => {
        const cardId = event.childConversationId || event.agentName;
        const poolId = `sub-${cardId}`;
        set(
          (s) => {
            const pool = [...(s.agentPool[event.conversationId] || [])];
            const idx = pool.findIndex((p) => p.id === poolId);
            const item: AgentPoolItem = {
              id: poolId,
              conversationId: event.conversationId,
              type: "sub_agent",
              name: event.agentName || event.agentType,
              status: event.status === "running"
                ? "running"
                : event.status === "failed"
                ? "failed"
                : "completed",
              agentType: event.agentType,
              childConversationId: event.childConversationId,
              childSessionId: event.childSessionId,
              isFork: event.isFork,
              summary: event.description,
              startedAt: Date.now(),
              messageId: _latestMessageIdByConv[event.conversationId],
            };
            if (idx >= 0) {
              pool[idx] = { ...pool[idx], ...item };
            } else {
              pool.push(item);
            }
            return {
              agentPool: { ...s.agentPool, [event.conversationId]: pool },
            };
          },
          false,
          { type: "sub-agent-card", conversationId: event.conversationId },
        );
      },

      handleDone: (event) => {
        const current = get().phases[event.conversationId];
        if (current && TERMINAL_PHASES.has(current as ExecutionPhase)) {
          return;
        }
        get().transition(event.conversationId, "completed");
        set(
          (s) => ({
            currentToolCall: shouldClearToolCall(s.currentToolCall, s.phases, event.conversationId)
              ? null
              : s.currentToolCall,
            agentStatus: { ...s.agentStatus, [event.conversationId]: "" },
          }),
          false,
          { type: "agent-done", conversationId: event.conversationId },
        );
      },

      handleError: (event) => {
        const current = get().phases[event.conversationId];
        if (current && TERMINAL_PHASES.has(current as ExecutionPhase)) {
          return;
        }
        get().transition(event.conversationId, "failed");
        set(
          (s) => ({
            currentToolCall: shouldClearToolCall(s.currentToolCall, s.phases, event.conversationId)
              ? null
              : s.currentToolCall,
            agentStatus: {
              ...s.agentStatus,
              [event.conversationId]: event.message || "Unknown error",
            },
          }),
          false,
          { type: "agent-error", conversationId: event.conversationId },
        );
      },

      handleCancelled: (event) => {
        get().transition(event.conversationId, "cancelled");
        set(
          (s) => ({
            currentToolCall: null,
            agentStatus: { ...s.agentStatus, [event.conversationId]: "" },
          }),
          false,
          { type: "agent-cancelled", conversationId: event.conversationId },
        );
      },

      // ── 轨迹 ──

      fetchTrajectoryList: async (conversationId: string) => {
        if (get().trajectoriesByConversation[conversationId]) {
          return;
        }
        set({ loadingTrajectories: true }, false, {
          type: "fetch-trajectory-list/start",
          conversationId,
        });
        try {
          const result = await invoke<TrajectorySummary[]>("trajectory_list", {
            sessionId: conversationId,
            limit: 20,
          });
          set(
            (s) => ({
              trajectoriesByConversation: {
                ...s.trajectoriesByConversation,
                [conversationId]: result,
              },
            }),
            false,
            {
              type: "fetch-trajectory-list/done",
              conversationId,
              count: result.length,
            },
          );
        } catch {
          // 轨迹服务可能未初始化
        } finally {
          set({ loadingTrajectories: false }, false, {
            type: "fetch-trajectory-list/end",
            conversationId,
          });
        }
      },

      fetchTrajectoryDetail: async (trajectoryId: string) => {
        if (get().trajectoryDetails[trajectoryId] !== undefined) {
          return get().trajectoryDetails[trajectoryId];
        }
        set(
          (s) => ({
            loadingTrajectoryDetail: {
              ...s.loadingTrajectoryDetail,
              [trajectoryId]: true,
            },
          }),
          false,
          { type: "fetch-trajectory-detail/start", trajectoryId },
        );
        try {
          const result = await invoke<TrajectoryDetail>(
            "get_trajectory_detail",
            { trajectoryId },
          );
          set(
            (s) => ({
              trajectoryDetails: {
                ...s.trajectoryDetails,
                [trajectoryId]: result,
              },
            }),
            false,
            { type: "fetch-trajectory-detail/done", trajectoryId },
          );
          return result;
        } catch {
          set(
            (s) => ({
              trajectoryDetails: {
                ...s.trajectoryDetails,
                [trajectoryId]: null,
              },
            }),
            false,
            { type: "fetch-trajectory-detail/error", trajectoryId },
          );
          return null;
        } finally {
          set(
            (s) => ({
              loadingTrajectoryDetail: {
                ...s.loadingTrajectoryDetail,
                [trajectoryId]: false,
              },
            }),
            false,
            { type: "fetch-trajectory-detail/end", trajectoryId },
          );
        }
      },

      // ── 清理 ──

      clearConversation: (conversationId) => {
        set(
          (s) => {
            const restPhases = { ...s.phases };
            delete restPhases[conversationId];
            const restStatus = { ...s.agentStatus };
            delete restStatus[conversationId];
            const restPool = { ...s.agentPool };
            delete restPool[conversationId];
            const restTraj = { ...s.trajectoriesByConversation };
            delete restTraj[conversationId];
            delete _latestMessageIdByConv[conversationId];

            // Identify tool call IDs belonging to this conversation via agentPool
            // 2.2: Track both tool- and worker- prefixed pool items
            const removedToolUseIds = new Set<string>();
            for (const item of (s.agentPool[conversationId] || [])) {
              if (item.id.startsWith("tool-")) {
                removedToolUseIds.add(item.id.replace("tool-", ""));
              } else if (item.id.startsWith("worker-")) {
                removedToolUseIds.add(item.id.replace("worker-", ""));
              }
            }
            // 2.2: Scan toolCalls for orphan entries whose assistantMessageId
            // matches _latestMessageIdByConv (was already deleted above), or
            // whose ids match removed pool items
            const poolMessageIds = new Set(
              (s.agentPool[conversationId] || [])
                .map((item) => item.messageId)
                .filter(Boolean) as string[],
            );
            for (const [id, tc] of Object.entries(s.toolCalls)) {
              if (
                tc.assistantMessageId && poolMessageIds.has(tc.assistantMessageId)
              ) {
                removedToolUseIds.add(id);
                removedToolUseIds.add(tc.toolUseId);
              }
            }
            const restToolCalls: Record<string, ToolCallState> = {};
            for (const [id, tc] of Object.entries(s.toolCalls)) {
              if (!removedToolUseIds.has(id) && !removedToolUseIds.has(tc.toolUseId)) {
                restToolCalls[id] = tc;
              }
            }
            const restSdkIdToExecId: Record<string, string> = {};
            for (const [sdkId, execId] of Object.entries(s.sdkIdToExecId)) {
              if (!removedToolUseIds.has(sdkId) && !removedToolUseIds.has(execId)) {
                restSdkIdToExecId[sdkId] = execId;
              }
            }

            return {
              phases: restPhases,
              agentStatus: restStatus,
              agentPool: restPool,
              trajectoriesByConversation: restTraj,
              currentToolCall: s.currentToolCall?.conversationId === conversationId
                ? null
                : s.currentToolCall,
              toolCalls: restToolCalls,
              sdkIdToExecId: restSdkIdToExecId,
            };
          },
          false,
          { type: "clear-conversation", conversationId },
        );
      },

      clearConversationUI: (conversationId) => {
        set(
          (s) => ({
            currentToolCall: s.currentToolCall?.conversationId === conversationId
              ? null
              : s.currentToolCall,
          }),
          false,
          { type: "clear-conversation-ui", conversationId },
        );
      },
    }),
    { name: "executionStore" },
  ),
);

// ── 事件监听器注册 ──

let _listenerRefCount = 0;

export function setupExecutionEventListeners(): () => void {
  _listenerRefCount++;
  if (_listenerRefCount > 1) {
    return () => {
      _listenerRefCount--;
    };
  }

  const unlisteners: Promise<UnlistenFn>[] = [];
  const store = useExecutionStore.getState();

  unlisteners.push(
    listen<ToolUseEvent>("agent-tool-use", (e) => store.handleToolUse(e.payload)),
  );
  unlisteners.push(
    listen<ToolStartEvent>("agent-tool-start", (e) => store.handleToolStart(e.payload)),
  );
  unlisteners.push(
    listen<ToolResultEvent>("agent-tool-result", (e) => store.handleToolResult(e.payload)),
  );
  unlisteners.push(
    listen<AgentStatusEvent>("agent-status", (e) => store.setAgentStatus(e.payload.conversationId, e.payload.message)),
  );
  unlisteners.push(
    listen<AgentDoneEvent>("agent-done", (e) => {
      store.clearAgentStatus(e.payload.conversationId);
      store.handleDone(e.payload);
    }),
  );
  unlisteners.push(
    listen<AgentErrorEvent>("agent-error", (e) => store.handleError(e.payload)),
  );
  unlisteners.push(
    listen<AgentCancelledEvent>("agent-cancelled", (e) => store.handleCancelled(e.payload)),
  );
  unlisteners.push(
    listen<SubAgentCardEvent>("agent-subagent-card", (e) => store.handleSubAgentCard(e.payload)),
  );

  // Worker 事件
  type WorkerPayload = {
    conversationId: string;
    workerId: string;
    taskId: string;
    messageType: string;
    content: string;
    status?: string;
  };
  unlisteners.push(
    listen<WorkerPayload>("worker-created", (e) =>
      store.handleWorkerEvent({
        ...e.payload,
        messageType: "progress",
        content: i18n.t("executionStore.workerCreated"),
      })),
  );
  unlisteners.push(
    listen<WorkerPayload>("worker-progress", (e) => store.handleWorkerEvent(e.payload)),
  );
  unlisteners.push(
    listen<WorkerPayload>("worker-completed", (e) =>
      store.handleWorkerEvent({
        ...e.payload,
        messageType: "completion",
        status: "completed",
      })),
  );
  unlisteners.push(
    listen<WorkerPayload>("worker-failed", (e) =>
      store.handleWorkerEvent({
        ...e.payload,
        messageType: "error",
        status: "failed",
      })),
  );

  // Workflow 步骤事件
  unlisteners.push(
    listen<{
      conversationId: string;
      stepId: string;
      stepGoal: string;
      agentRole: string;
    }>("workflow-step-start", (e) => {
      store.upsertPoolItem({
        id: e.payload.stepId,
        conversationId: e.payload.conversationId,
        type: "workflow_step",
        name: e.payload.stepGoal,
        status: "running",
        agentRole: e.payload.agentRole,
        startedAt: Date.now(),
        messageId: _latestMessageIdByConv[e.payload.conversationId],
      });
    }),
  );
  unlisteners.push(
    listen<{
      conversationId: string;
      stepId: string;
      stepGoal: string;
      result: string;
    }>("workflow-step-complete", (e) => {
      store.upsertPoolItem({
        id: e.payload.stepId,
        conversationId: e.payload.conversationId,
        type: "workflow_step",
        name: e.payload.stepGoal,
        status: "completed",
        summary: e.payload.result,
      });
    }),
  );
  unlisteners.push(
    listen<{ conversationId: string; stepId: string; error: string }>(
      "workflow-step-error",
      (e) => {
        store.upsertPoolItem({
          id: e.payload.stepId,
          conversationId: e.payload.conversationId,
          type: "workflow_step",
          name: e.payload.stepId,
          status: "failed",
          error: e.payload.error,
        });
      },
    ),
  );

  return () => {
    _listenerRefCount--;
    if (_listenerRefCount <= 0) {
      _listenerRefCount = 0;
      unlisteners.forEach((u) => u.then((f) => f()));
    }
  };
}
