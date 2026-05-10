/**
 * 统一智能体执行状态 Store
 * 整合：agentStore 执行态 + trajectoryStore + ExecutionPhase 状态机
 */
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

// ── 执行阶段状态机 ──

export type ExecutionPhase =
  | "idle"
  | "planning"
  | "executing"
  | "waiting_permission"
  | "completed"
  | "failed"
  | "cancelled";

const PHASE_TRANSITIONS: Record<ExecutionPhase, ExecutionPhase[]> = {
  idle: ["planning", "executing", "completed", "failed", "cancelled"],
  planning: ["executing", "failed", "cancelled"],
  executing: ["executing", "waiting_permission", "completed", "failed", "cancelled"],
  waiting_permission: ["executing", "cancelled"],
  completed: ["idle", "executing"],
  failed: ["idle", "executing"],
  cancelled: ["idle", "executing"],
};

export const ACTIVE_PHASES: Set<ExecutionPhase> = new Set([
  "planning",
  "executing",
  "waiting_permission",
]);

export const TERMINAL_PHASES: Set<ExecutionPhase> = new Set([
  "completed",
  "failed",
  "cancelled",
]);

// ── 工具调用追踪 ──

export interface CurrentToolCall {
  toolName: string;
  toolUseId: string;
  conversationId: string;
  startedAt: number;
}

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
  fetchTrajectoryDetail: (trajectoryId: string) => Promise<TrajectoryDetail | null>;

  // === 清理 ===
  clearConversation: (conversationId: string) => void;
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
            const allowed = PHASE_TRANSITIONS[from] || [];
            if (!allowed.includes(to)) {
              console.warn(
                `[executionStore] 非法阶段转换: ${from} → ${to} (conv: ${conversationId})`,
              );
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
        return Object.entries(get().phases)
          .filter(([, p]) => ACTIVE_PHASES.has(p))
          .map(([id]) => id);
      },

      // ── 进度 ──

      setCurrentTool: (tc) => {
        set({ currentToolCall: tc }, false, { type: "set-current-tool", tc });
      },

      setAgentStatus: (conversationId, message) => {
        set(
          (s) => ({ agentStatus: { ...s.agentStatus, [conversationId]: message } }),
          false,
          { type: "agent-status", conversationId },
        );
      },

      clearAgentStatus: (conversationId) => {
        set(
          (s) => {
            const { [conversationId]: _, ...rest } = s.agentStatus;
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
            return { agentPool: { ...s.agentPool, [item.conversationId]: pool } };
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
        if (total === 0) { return { total: 0, completed: 0, running: 0, pending: 0, failed: 0, pctComplete: 0 }; }
        const completed = pool.filter((i) => i.status === "completed").length;
        const running = pool.filter((i) => i.status === "running").length;
        const pending = pool.filter((i) => i.status === "pending").length;
        const failed = pool.filter((i) => i.status === "failed").length;
        return { total, completed, running, pending, failed, pctComplete: Math.round((completed / total) * 100) };
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
            const updates: Record<string, ToolCallState> = { [event.toolUseId]: tc };
            const idMap = { ...s.sdkIdToExecId };
            if (event.executionId) {
              updates[event.executionId] = { ...tc, toolUseId: event.executionId };
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
          { type: "tool-use", toolName: event.toolName, conversationId: event.conversationId },
        );
        // 自动进入 executing 阶段（避免重复转换造成 warn）
        const current = get().phases[event.conversationId];
        if (current !== "executing") {
          get().transition(event.conversationId, "executing");
        }
      },

      handleToolStart: (event) => {
        set(
          (s) => {
            const existing = s.toolCalls[event.toolUseId];
            const updates: Record<string, ToolCallState> = {
              [event.toolUseId]: {
                ...(existing
                  || {
                    toolUseId: event.toolUseId,
                    toolName: event.toolName,
                    input: event.input ?? {},
                    assistantMessageId: event.assistantMessageId || "",
                  }),
                executionStatus: "running",
              },
            };
            return { toolCalls: { ...s.toolCalls, ...updates } };
          },
          false,
          { type: "tool-start", toolUseId: event.toolUseId },
        );
      },

      handleToolResult: (event) => {
        set(
          (s) => {
            const existing = s.toolCalls[event.toolUseId];
            if (!existing) { return {}; }
            const updates: Record<string, ToolCallState> = {
              [event.toolUseId]: {
                ...existing,
                executionStatus: event.isError ? "failed" : "success",
                output: event.content,
                isError: event.isError,
              },
            };
            const currentToolCall = s.currentToolCall?.toolUseId === event.toolUseId ? null : s.currentToolCall;
            return { toolCalls: { ...s.toolCalls, ...updates }, currentToolCall };
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
          messageType: (event.messageType || "progress") as WorkerMessage["messageType"],
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
            const newStatus = (event.status || statusMap[event.messageType] || "running") as AgentPoolItem["status"];
            if (idx >= 0) {
              const existing = pool[idx];
              pool[idx] = {
                ...existing,
                status: newStatus,
                summary: event.messageType === "progress" ? event.content : existing.summary,
                error: event.messageType === "error" ? event.content : existing.error,
                messages: [...(existing.messages || []), msg],
                duration: existing.startedAt ? Date.now() - existing.startedAt : undefined,
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
            return { agentPool: { ...s.agentPool, [event.conversationId]: pool } };
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
              status: event.status === "running" ? "running" : event.status === "failed" ? "failed" : "completed",
              agentType: event.agentType,
              childConversationId: event.childConversationId,
              childSessionId: event.childSessionId,
              isFork: event.isFork,
              summary: event.description,
              startedAt: Date.now(),
              messageId: _latestMessageIdByConv[event.conversationId],
            };
            if (idx >= 0) { pool[idx] = { ...pool[idx], ...item }; }
            else { pool.push(item); }
            return { agentPool: { ...s.agentPool, [event.conversationId]: pool } };
          },
          false,
          { type: "sub-agent-card", conversationId: event.conversationId },
        );
      },

      handleDone: (event) => {
        get().transition(event.conversationId, "completed");
        set(
          (s) => ({
            currentToolCall: s.currentToolCall?.conversationId === event.conversationId ? null : s.currentToolCall,
            agentStatus: { ...s.agentStatus, [event.conversationId]: "" },
          }),
          false,
          { type: "agent-done", conversationId: event.conversationId },
        );
      },

      handleError: (event) => {
        get().transition(event.conversationId, "failed");
        set(
          (s) => ({
            currentToolCall: s.currentToolCall?.conversationId === event.conversationId ? null : s.currentToolCall,
            agentStatus: { ...s.agentStatus, [event.conversationId]: event.message || "Unknown error" },
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
        if (get().trajectoriesByConversation[conversationId]) { return; }
        set({ loadingTrajectories: true }, false, { type: "fetch-trajectory-list/start", conversationId });
        try {
          const result = await invoke<TrajectorySummary[]>("trajectory_list", {
            sessionId: conversationId,
            limit: 20,
          });
          set(
            (s) => ({ trajectoriesByConversation: { ...s.trajectoriesByConversation, [conversationId]: result } }),
            false,
            { type: "fetch-trajectory-list/done", conversationId, count: result.length },
          );
        } catch {
          // 轨迹服务可能未初始化
        } finally {
          set({ loadingTrajectories: false }, false, { type: "fetch-trajectory-list/end", conversationId });
        }
      },

      fetchTrajectoryDetail: async (trajectoryId: string) => {
        if (get().trajectoryDetails[trajectoryId] !== undefined) {
          return get().trajectoryDetails[trajectoryId];
        }
        set(
          (s) => ({ loadingTrajectoryDetail: { ...s.loadingTrajectoryDetail, [trajectoryId]: true } }),
          false,
          { type: "fetch-trajectory-detail/start", trajectoryId },
        );
        try {
          const result = await invoke<TrajectoryDetail>("get_trajectory_detail", { trajectoryId });
          set(
            (s) => ({ trajectoryDetails: { ...s.trajectoryDetails, [trajectoryId]: result } }),
            false,
            { type: "fetch-trajectory-detail/done", trajectoryId },
          );
          return result;
        } catch {
          set(
            (s) => ({ trajectoryDetails: { ...s.trajectoryDetails, [trajectoryId]: null } }),
            false,
            { type: "fetch-trajectory-detail/error", trajectoryId },
          );
          return null;
        } finally {
          set(
            (s) => ({ loadingTrajectoryDetail: { ...s.loadingTrajectoryDetail, [trajectoryId]: false } }),
            false,
            { type: "fetch-trajectory-detail/end", trajectoryId },
          );
        }
      },

      // ── 清理 ──

      clearConversation: (conversationId) => {
        set(
          (s) => {
            const {
              [conversationId]: _p,
              ...restPhases
            } = s.phases;
            const { [conversationId]: _a, ...restStatus } = s.agentStatus;
            const { [conversationId]: _pool, ...restPool } = s.agentPool;
            const { [conversationId]: _traj, ...restTraj } = s.trajectoriesByConversation;
            // 清理模块级 messageId 追踪，防止内存泄漏
            delete _latestMessageIdByConv[conversationId];
            return {
              phases: restPhases,
              agentStatus: restStatus,
              agentPool: restPool,
              trajectoriesByConversation: restTraj,
              currentToolCall: s.currentToolCall?.conversationId === conversationId ? null : s.currentToolCall,
            };
          },
          false,
          { type: "clear-conversation", conversationId },
        );
      },
    }),
    { name: "executionStore" },
  ),
);

// ── 事件监听器注册 ──

let _listenersSetup = false;

export function setupExecutionEventListeners(): () => void {
  if (_listenersSetup) { return () => {}; }
  _listenersSetup = true;

  const unlisteners: Promise<UnlistenFn>[] = [];
  const store = useExecutionStore.getState();

  unlisteners.push(listen<ToolUseEvent>("agent-tool-use", (e) => store.handleToolUse(e.payload)));
  unlisteners.push(listen<ToolStartEvent>("agent-tool-start", (e) => store.handleToolStart(e.payload)));
  unlisteners.push(listen<ToolResultEvent>("agent-tool-result", (e) => store.handleToolResult(e.payload)));
  unlisteners.push(
    listen<AgentStatusEvent>("agent-status", (e) => store.setAgentStatus(e.payload.conversationId, e.payload.message)),
  );
  unlisteners.push(listen<AgentDoneEvent>("agent-done", (e) => {
    store.clearAgentStatus(e.payload.conversationId);
    store.handleDone(e.payload);
  }));
  unlisteners.push(listen<AgentErrorEvent>("agent-error", (e) => store.handleError(e.payload)));
  unlisteners.push(listen<AgentCancelledEvent>("agent-cancelled", (e) => store.handleCancelled(e.payload)));
  unlisteners.push(listen<SubAgentCardEvent>("agent-subagent-card", (e) => store.handleSubAgentCard(e.payload)));

  // Worker 事件
  const workerPayload = {} as {
    conversationId: string;
    workerId: string;
    taskId: string;
    messageType: string;
    content: string;
    status?: string;
  };
  unlisteners.push(
    listen<typeof workerPayload>(
      "worker-created",
      (e) => store.handleWorkerEvent({ ...e.payload, messageType: "progress", content: "Worker created" }),
    ),
  );
  unlisteners.push(listen<typeof workerPayload>("worker-progress", (e) => store.handleWorkerEvent(e.payload)));
  unlisteners.push(
    listen<typeof workerPayload>(
      "worker-completed",
      (e) => store.handleWorkerEvent({ ...e.payload, messageType: "completion", status: "completed" }),
    ),
  );
  unlisteners.push(
    listen<typeof workerPayload>(
      "worker-failed",
      (e) => store.handleWorkerEvent({ ...e.payload, messageType: "error", status: "failed" }),
    ),
  );

  // Workflow 步骤事件
  unlisteners.push(
    listen<{ conversationId: string; stepId: string; stepGoal: string; agentRole: string }>(
      "workflow-step-start",
      (e) => {
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
      },
    ),
  );
  unlisteners.push(
    listen<{ conversationId: string; stepId: string; stepGoal: string; result: string }>(
      "workflow-step-complete",
      (e) => {
        store.upsertPoolItem({
          id: e.payload.stepId,
          conversationId: e.payload.conversationId,
          type: "workflow_step",
          name: e.payload.stepGoal,
          status: "completed",
          summary: e.payload.result,
        });
      },
    ),
  );
  unlisteners.push(listen<{ conversationId: string; stepId: string; error: string }>("workflow-step-error", (e) => {
    store.upsertPoolItem({
      id: e.payload.stepId,
      conversationId: e.payload.conversationId,
      type: "workflow_step",
      name: e.payload.stepId,
      status: "failed",
      error: e.payload.error,
    });
  }));

  return () => {
    _listenersSetup = false;
    unlisteners.forEach((u) => u.then((f) => f()));
  };
}
