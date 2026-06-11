// SPDX-License-Identifier: AGPL-3.0-only

import { invoke, logIpcError } from "@/lib/invoke";
import {
  Background,
  type Edge,
  Handle,
  type Node,
  type NodeProps,
  Position,
  ReactFlow,
  ReactFlowProvider,
  useReactFlow,
} from "@xyflow/react";
import { Button, message, Spin, theme } from "antd";
import type { GlobalToken } from "antd/es/theme/interface";
import {
  AlertTriangle,
  CheckCircle,
  ChevronDown,
  ChevronRight,
  Clock,
  GitBranch,
  Loader2,
  SkipForward,
  StopCircle,
  XCircle,
} from "lucide-react";
import React, { memo, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import "@xyflow/react/dist/style.css";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

// WorkflowNode JSON shape（来自后端 serde 序列化）
interface WorkflowNodeJson {
  type?: string;
  id?: string;
  base?: { id: string; title: string; description?: string; enabled: boolean };
  title?: string;
  description?: string;
  [key: string]: unknown;
}

// NodeRuntimeState JSON shape
interface NodeRuntimeState {
  status: "pending" | "ready" | "running" | "completed" | "failed" | "skipped";
  attempts: number;
  error: string | null;
  started_at: number | null;
  completed_at: number | null;
}

interface WorkflowEdgeJson {
  id: string;
  source: string;
  target: string;
  edge_type: string;
}

interface WorkflowData {
  id: string;
  name: string;
  status:
    | "created"
    | "running"
    | "completed"
    | "partially_completed"
    | "failed"
    | "cancelled";
  nodes: WorkflowNodeJson[];
  edges: WorkflowEdgeJson[];
  node_states: Record<string, NodeRuntimeState>;
  results: Record<string, unknown>;
  /** 工作流最终输出（经 output_schema 过滤或 EndNode 聚合） */
  output?: unknown;
  created_at?: number;
  completed_at?: number;
}

// 从 edges 推导每个节点的 needs 列表
function computeNeeds(edges: WorkflowEdgeJson[]): Record<string, string[]> {
  const needs: Record<string, string[]> = {};
  for (const e of edges) {
    if (!needs[e.target]) { needs[e.target] = []; }
    needs[e.target].push(e.source);
  }
  return needs;
}

// 将 WorkflowNode JSON 转为可视化用的 StepLike 视图
interface StepLike {
  id: string;
  goal: string;
  agent_role: string;
  needs: string[];
  status: "pending" | "running" | "completed" | "failed" | "skipped";
  result: string | null;
  error: string | null;
  attempts: number;
  max_retries: number;
  on_failure: "abort" | "skip";
}

function toStepLike(
  wf: WorkflowData,
): StepLike[] {
  const needs = computeNeeds(wf.edges);
  return wf.nodes.map((n) => {
    const nodeId = n.base?.id ?? n.id ?? "";
    const state = wf.node_states[nodeId];
    return {
      id: nodeId,
      goal: n.base?.title ?? n.title ?? n.description ?? nodeId,
      agent_role: (n as Record<string, unknown>).config
        ? ((n as Record<string, unknown>).config as Record<string, unknown>).role as string ?? "executor"
        : "executor",
      needs: needs[nodeId] ?? [],
      status: (state?.status ?? "pending") as StepLike["status"],
      result: wf.results[nodeId] ? JSON.stringify(wf.results[nodeId]) : null,
      error: state?.error ?? null,
      attempts: state?.attempts ?? 0,
      max_retries: 2,
      on_failure: "abort",
    };
  });
}

interface WorkflowProgressPanelProps {
  conversationId: string;
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const TERMINAL_STATUSES = new Set<WorkflowData["status"]>([
  "completed",
  "partially_completed",
  "failed",
  "cancelled",
]);

const POLL_INTERVAL_MS = 2000;

const NODE_WIDTH = 160;
const NODE_HEIGHT = 56;
const H_GAP = 36;
const V_GAP = 28;
const DAG_HEIGHT = 220;

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

/// 解析错误消息中的 error code，返回 i18n 翻译文本
function translateError(errorStr: string | null, t: (key: string) => string): string {
  if (!errorStr) { return ""; }
  try {
    const parsed = JSON.parse(errorStr);
    if (parsed.code) {
      const i18nKey = `chat.workflow.errorDetail.${parsed.code}`;
      return t(i18nKey);
    }
  } catch {
    // Not JSON, return as-is
  }
  return errorStr;
}

function truncate(str: string | null, maxLen: number): string {
  if (!str) {
    return "";
  }
  if (str.length <= maxLen) {
    return str;
  }
  return str.slice(0, maxLen) + "...";
}

function getStorageKey(conversationId: string): string {
  return `axagent:workflow-id:${conversationId}`;
}

function getWorkflowIdFromStorage(conversationId: string): string | null {
  try {
    return localStorage.getItem(getStorageKey(conversationId));
  } catch {
    return null;
  }
}

// ---------------------------------------------------------------------------
// Status utilities
// ---------------------------------------------------------------------------

const getStatusColor = (status: string, token: GlobalToken) => {
  switch (status) {
    case "pending":
    case "created":
      return token.colorTextTertiary;
    case "running":
      return token.colorPrimary;
    case "completed":
      return token.colorSuccess;
    case "failed":
      return token.colorError;
    case "skipped":
    case "partially_completed":
      return token.colorWarning;
    case "cancelled":
      return token.colorTextTertiary;
    default:
      return token.colorTextTertiary;
  }
};

function isDone(status: StepLike["status"]): boolean {
  return status === "completed" || status === "failed" || status === "skipped";
}

// ---------------------------------------------------------------------------
// DAG Layout
// ---------------------------------------------------------------------------

interface WorkflowDagNodeData extends Record<string, unknown> {
  stepId: string;
  goal: string;
  agentRole: string;
  status: StepLike["status"];
}

function computeDagLayout(steps: StepLike[], token: GlobalToken): {
  nodes: Node[];
  edges: Edge[];
} {
  if (steps.length === 0) {
    return { nodes: [], edges: [] };
  }

  const stepMap = new Map(steps.map((s) => [s.id, s]));
  const layerCache = new Map<string, number>();

  function getLayer(id: string): number {
    if (layerCache.has(id)) {
      return layerCache.get(id)!;
    }
    const step = stepMap.get(id);
    if (!step || step.needs.length === 0) {
      layerCache.set(id, 0);
      return 0;
    }
    const maxDep = Math.max(...step.needs.map((depId) => getLayer(depId)));
    const layer = maxDep + 1;
    layerCache.set(id, layer);
    return layer;
  }

  for (const step of steps) {
    getLayer(step.id);
  }

  const layerGroups = new Map<number, StepLike[]>();
  for (const step of steps) {
    const layer = layerCache.get(step.id)!;
    if (!layerGroups.has(layer)) {
      layerGroups.set(layer, []);
    }
    layerGroups.get(layer)!.push(step);
  }

  const sortedLayers = [...layerGroups.keys()].toSorted((a, b) => a - b);
  const nodes: Node[] = [];

  for (const layer of sortedLayers) {
    const group = layerGroups.get(layer)!;
    const totalWidth = group.length * NODE_WIDTH + (group.length - 1) * H_GAP;
    const startX = -totalWidth / 2;

    for (let i = 0; i < group.length; i++) {
      nodes.push({
        id: group[i].id,
        type: "workflowStep",
        position: {
          x: startX + i * (NODE_WIDTH + H_GAP),
          y: layer * (NODE_HEIGHT + V_GAP),
        },
        data: {
          stepId: group[i].id,
          goal: group[i].goal,
          agentRole: group[i].agent_role,
          status: group[i].status,
        },
        sourcePosition: Position.Bottom,
        targetPosition: Position.Top,
      });
    }
  }

  const nodeIds = new Set(steps.map((s) => s.id));
  const edges: Edge[] = [];

  for (const step of steps) {
    for (const dep of step.needs) {
      if (nodeIds.has(dep)) {
        edges.push({
          id: `${dep}->${step.id}`,
          source: dep,
          target: step.id,
          animated: step.status === "running",
          style: {
            stroke: step.status === "failed" ? token.colorError : token.colorBorder,
            strokeWidth: 1.5,
          },
        });
      }
    }
  }

  return { nodes, edges };
}

// ---------------------------------------------------------------------------
// WorkflowDagNode
// ---------------------------------------------------------------------------

const WorkflowDagNode: React.FC<NodeProps> = memo(
  ({ data, selected }) => {
    const dagData = data as unknown as WorkflowDagNodeData;
    const { token } = theme.useToken();
    const color = getStatusColor(dagData.status, token);
    const isRunning = dagData.status === "running";
    const isFailed = dagData.status === "failed";

    return (
      <div
        className="rounded-md border px-2 py-1.5 bg-white dark:bg-zinc-800 shadow-sm"
        style={{
          width: NODE_WIDTH,
          borderColor: selected ? token.colorPrimary : color,
          borderWidth: selected ? 2 : 1,
          opacity: dagData.status === "skipped" ? 0.65 : 1,
        }}
      >
        <Handle
          type="target"
          position={Position.Top}
          style={{ visibility: "hidden" }}
        />
        <div className="flex items-center gap-1">
          {isRunning
            ? (
              <Loader2
                size={10}
                className="animate-spin shrink-0"
                style={{ color }}
              />
            )
            : isFailed
            ? <XCircle size={10} className="shrink-0" style={{ color }} />
            : <CheckCircle size={10} className="shrink-0" style={{ color }} />}
          <span
            className="text-[10px] font-mono font-medium truncate"
            style={{ color }}
          >
            {dagData.stepId}
          </span>
        </div>
        <div className="text-[10px] text-zinc-500 dark:text-zinc-400 truncate mt-0.5 leading-tight">
          {truncate(dagData.goal, 28)}
        </div>
        <div className="text-[9px] text-zinc-400 dark:text-zinc-500 truncate">
          {dagData.agentRole}
        </div>
        <Handle
          type="source"
          position={Position.Bottom}
          style={{ visibility: "hidden" }}
        />
      </div>
    );
  },
);
WorkflowDagNode.displayName = "WorkflowDagNode";

const nodeTypes = { workflowStep: WorkflowDagNode };

// ---------------------------------------------------------------------------
// WorkflowDagView (inner - needs ReactFlowProvider)
// ---------------------------------------------------------------------------

const WorkflowDagView: React.FC<{ steps: StepLike[] }> = memo(
  ({ steps }) => {
    const { token } = theme.useToken();
    const { fitView } = useReactFlow();
    const { nodes, edges } = useMemo(() => computeDagLayout(steps, token), [steps, token]);

    useEffect(() => {
      if (nodes.length > 0) {
        const timer = setTimeout(() => {
          fitView({ padding: 0.3, duration: 200 });
        }, 60);
        return () => clearTimeout(timer);
      }
    }, [nodes, fitView]);

    if (nodes.length === 0) {
      return null;
    }

    return (
      <ReactFlow
        nodes={nodes}
        edges={edges}
        nodeTypes={nodeTypes}
        nodesDraggable={false}
        nodesConnectable={false}
        nodesFocusable={false}
        elementsSelectable={false}
        panOnDrag={false}
        zoomOnScroll={false}
        zoomOnDoubleClick={false}
        preventScrolling={false}
        fitView
        proOptions={{ hideAttribution: true }}
        style={{ background: "transparent" }}
      >
        <Background color={token.colorBorderSecondary} gap={16} size={0.5} />
      </ReactFlow>
    );
  },
);
WorkflowDagView.displayName = "WorkflowDagView";

// ---------------------------------------------------------------------------
// StepRow (memoized)
// ---------------------------------------------------------------------------

interface StepRowProps {
  step: StepLike;
  expanded: boolean;
  onToggle: () => void;
}

const StepRow = memo(function StepRow({
  step,
  expanded,
  onToggle,
}: StepRowProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const color = getStatusColor(step.status, token);

  const StatusIcon = useMemo(() => {
    switch (step.status) {
      case "pending":
        return Clock;
      case "running":
        return Loader2;
      case "completed":
        return CheckCircle;
      case "failed":
        return XCircle;
      case "skipped":
        return SkipForward;
      default:
        return Clock;
    }
  }, [step.status]);

  const iconClass = step.status === "running" ? "animate-spin" : "";

  return (
    <div className="border-b border-zinc-100 dark:border-zinc-800 last:border-b-0">
      <div
        className="flex items-center gap-2 px-3 py-2 cursor-pointer hover:bg-zinc-50 dark:hover:bg-zinc-800/50"
        role="button"
        tabIndex={0}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            onToggle();
          }
        }}
        onClick={onToggle}
      >
        <span
          style={{
            color,
            display: "flex",
            alignItems: "center",
            flexShrink: 0,
          }}
        >
          <StatusIcon size={14} className={iconClass} />
        </span>
        <span
          className="text-xs font-mono font-medium shrink-0"
          style={{ color }}
        >
          {step.id}
        </span>
        <span className="text-xs text-zinc-500 dark:text-zinc-400 truncate flex-1">
          {step.goal}
        </span>
        <span className="text-xs text-zinc-400 dark:text-zinc-500 shrink-0">
          {step.agent_role}
        </span>
        {step.attempts > 1 && (
          <span
            className="text-xs text-orange-500 shrink-0"
            title={t("chat.workflow.attempts", { count: step.attempts })}
          >
            <AlertTriangle size={12} />
          </span>
        )}
        <span style={{ display: "flex", alignItems: "center", flexShrink: 0 }}>
          {expanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
        </span>
      </div>
      {expanded && (
        <div className="px-3 pb-2 text-xs space-y-1">
          <div className="flex gap-4">
            <span className="text-zinc-500">
              {t("chat.workflow.stepStatus")}
            </span>
            <span style={{ color }}>
              {t(`chat.workflow.status.${step.status}`)}
            </span>
          </div>
          {step.needs.length > 0 && (
            <div className="flex gap-4">
              <span className="text-zinc-500">
                {t("chat.workflow.dependsOn")}
              </span>
              <span>{step.needs.join(", ")}</span>
            </div>
          )}
          <div className="flex gap-4">
            <span className="text-zinc-500">{t("chat.workflow.retries")}</span>
            <span>
              {step.attempts}/{step.max_retries + 1}
            </span>
          </div>
          {step.result && (
            <div>
              <span className="text-zinc-500">{t("chat.workflow.result")}</span>
              <pre className="mt-1 p-2 bg-zinc-100 dark:bg-zinc-800 rounded text-xs max-h-32 overflow-auto whitespace-pre-wrap">
                {truncate(step.result, 500)}
              </pre>
            </div>
          )}
          {step.error && (
            <div>
              <span className="text-red-500">{t("chat.workflow.error")}</span>
              <pre className="mt-1 p-2 bg-red-50 dark:bg-red-900/20 rounded text-xs max-h-32 overflow-auto whitespace-pre-wrap text-red-600 dark:text-red-400">
                {translateError(step.error, t) || truncate(step.error, 500)}
              </pre>
            </div>
          )}
        </div>
      )}
    </div>
  );
});

// ---------------------------------------------------------------------------
// Main Component
// ---------------------------------------------------------------------------

export const WorkflowProgressPanel: React.FC<WorkflowProgressPanelProps> = ({
  conversationId,
}) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();

  const [workflow, setWorkflow] = useState<WorkflowData | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [cancelling, setCancelling] = useState(false);
  const [expandedSteps, setExpandedSteps] = useState<Set<string>>(new Set());
  const [showDag, setShowDag] = useState(true);
  const [dagCollapsed, setDagCollapsed] = useState(false);

  const [workflowId, setWorkflowId] = useState<string | null>(() => getWorkflowIdFromStorage(conversationId));

  const fetchIdRef = useRef(0);
  const pollTimerRef = useRef<ReturnType<typeof setInterval> | null>(null);

  // --- Sync: custom event for same-tab changes (from ChatViewToolbar) ---
  useEffect(() => {
    const handler = (e: Event) => {
      const { conversationId: cid, workflowId: wid } = (
        e as CustomEvent<{
          conversationId: string;
          workflowId: string | null;
        }>
      ).detail;
      if (cid === conversationId) {
        setWorkflowId(wid);
      }
    };
    window.addEventListener("axagent:workflow-changed", handler);
    return () => window.removeEventListener("axagent:workflow-changed", handler);
  }, [conversationId]);

  // --- Sync: cross-tab storage event ---
  useEffect(() => {
    const handleStorageChange = (e: StorageEvent) => {
      if (e.key === getStorageKey(conversationId)) {
        setWorkflowId(e.newValue);
      }
    };
    window.addEventListener("storage", handleStorageChange);
    return () => window.removeEventListener("storage", handleStorageChange);
  }, [conversationId]);

  // --- Re-read storage when conversationId changes ---
  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    setWorkflowId(getWorkflowIdFromStorage(conversationId));
  }, [conversationId]);

  // --- Reset internal state on workflowId change ---
  /* eslint-disable react-hooks/set-state-in-effect */
  useEffect(() => {
    setWorkflow(null);
    setLoading(false);
    setError(null);
    setExpandedSteps(new Set());
    setShowDag(true);
    setDagCollapsed(false);
  }, [workflowId]);
  /* eslint-enable react-hooks/set-state-in-effect */

  // --- Poll workflow status with race-condition protection & terminal stop ---
  const workflowRef = useRef(workflow);
  // eslint-disable-next-line react-hooks/refs
  workflowRef.current = workflow;

  /* eslint-disable react-hooks/set-state-in-effect */
  useEffect(() => {
    if (pollTimerRef.current) {
      clearInterval(pollTimerRef.current);
      pollTimerRef.current = null;
    }

    if (!workflowId) {
      setWorkflow(null);
      return;
    }

    let stoppedByTerminal = false;

    const poll = async () => {
      if (stoppedByTerminal) {
        return;
      }
      const requestId = ++fetchIdRef.current;

      if (requestId === 1) {
        setLoading(true);
      }

      try {
        const data = await invoke<WorkflowData>("workflow_get_status", {
          workflow_id: workflowId,
        });
        if (fetchIdRef.current !== requestId) {
          return;
        }
        setWorkflow(data);
        setError(null);

        if (TERMINAL_STATUSES.has(data.status)) {
          stoppedByTerminal = true;
          if (pollTimerRef.current) {
            clearInterval(pollTimerRef.current);
            pollTimerRef.current = null;
          }
        }
      } catch (e) {
        if (fetchIdRef.current !== requestId) {
          return;
        }
        const msg = e instanceof Error ? e.message : String(e);
        setError(msg);
        if (workflowRef.current) {
          message.warning(msg);
        }
        logIpcError("WorkflowProgressPanel.fetchStatus")(e);
      } finally {
        if (fetchIdRef.current === requestId) {
          setLoading(false);
        }
      }
    };

    poll();
    pollTimerRef.current = setInterval(poll, POLL_INTERVAL_MS);

    return () => {
      stoppedByTerminal = true;
      if (pollTimerRef.current) {
        clearInterval(pollTimerRef.current);
        pollTimerRef.current = null;
      }
    };
  }, [workflowId]);
  /* eslint-enable react-hooks/set-state-in-effect */

  // --- Step toggle ---
  const toggleStep = useCallback((stepId: string) => {
    setExpandedSteps((prev) => {
      const next = new Set(prev);
      if (next.has(stepId)) {
        next.delete(stepId);
      } else {
        next.add(stepId);
      }
      return next;
    });
  }, []);

  // --- Cancel ---
  const handleCancel = useCallback(async () => {
    if (!workflowId) {
      return;
    }
    setCancelling(true);
    try {
      await invoke("workflow_cancel", { workflow_id: workflowId });
      message.success(t("chat.workflow.cancelled"));
      const data = await invoke<WorkflowData>("workflow_get_status", {
        workflow_id: workflowId,
      });
      setWorkflow(data);
      setError(null);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      message.error(t("chat.workflow.cancelFailed") + ": " + msg);
    } finally {
      setCancelling(false);
    }
  }, [workflowId, t]);

  // --- Render ---

  // 必须在所有条件 return 之前调用，保持 hooks 数量恒定
  const steps = useMemo(() => workflow ? toStepLike(workflow) : [], [workflow]);

  if (!workflowId) {
    return null;
  }

  if (loading && !workflow) {
    return (
      <div className="mx-3 my-1.5 border border-purple-200 dark:border-purple-800 rounded-lg bg-purple-50/50 dark:bg-purple-900/10 p-4">
        <div className="flex items-center gap-2">
          <Spin size="small" />
          <span className="text-sm text-zinc-500">
            {t("chat.workflow.loading")}
          </span>
        </div>
      </div>
    );
  }

  if (error && !workflow) {
    return (
      <div className="mx-3 my-1.5 border border-red-200 dark:border-red-800 rounded-lg bg-red-50/50 dark:bg-red-900/10 p-4">
        <div className="flex items-center gap-2">
          <XCircle size={16} className="text-red-500" />
          <span className="text-sm text-red-600">{error}</span>
        </div>
      </div>
    );
  }

  if (!workflow) {
    return null;
  }

  const workflowColor = getStatusColor(workflow.status, token);
  const doneCount = steps.filter((s) => isDone(s.status)).length;
  const totalCount = steps.length;
  const progressPct = totalCount > 0 ? (doneCount / totalCount) * 100 : 0;
  const canCancel = workflow.status === "running" && !cancelling;

  return (
    <div className="mx-3 my-1.5 border border-purple-200 dark:border-purple-800 rounded-lg bg-purple-50/50 dark:bg-purple-900/10 overflow-hidden">
      {/* Header */}
      <div className="flex items-center gap-2 px-3 py-1.5 border-b border-purple-200 dark:border-purple-800">
        <GitBranch size={14} style={{ color: workflowColor }} />
        <span className="text-xs font-medium" style={{ color: workflowColor }}>
          {workflow.name}
        </span>
        <span className="text-xs text-zinc-500 dark:text-zinc-400">
          {t(`chat.workflow.workflowStatus.${workflow.status}`)}
        </span>

        {/* Progress bar */}
        <div className="flex-1 h-1.5 bg-zinc-200 dark:bg-zinc-700 rounded-full overflow-hidden ml-2">
          <div
            className="h-full rounded-full transition-all duration-300"
            style={{ width: `${progressPct}%`, backgroundColor: workflowColor }}
          />
        </div>
        <span className="text-xs text-zinc-500 dark:text-zinc-400 tabular-nums">
          {doneCount}/{totalCount}
        </span>

        {/* Polling indicator */}
        {loading && <Spin size="small" />}

        <Button
          type="text"
          size="small"
          onClick={() => setShowDag(!showDag)}
          className="text-xs px-1.5 py-0.5 rounded border border-purple-300 dark:border-purple-700 hover:bg-purple-100 dark:hover:bg-purple-800/30 transition-colors"
        >
          {showDag ? t("chat.workflow.listView") : t("chat.workflow.dagView")}
        </Button>
        {canCancel && (
          <Button
            type="text"
            size="small"
            icon={<StopCircle size={12} />}
            loading={cancelling}
            onClick={handleCancel}
            className="text-xs px-1.5 py-0.5 text-red-500 hover:text-red-600"
            danger
          >
            {t("chat.workflow.cancel")}
          </Button>
        )}

        {/* 工作流最终输出（经 output_schema 过滤或 EndNode 聚合） */}
        {(workflow.status === "completed" || workflow.status === "partially_completed")
          && workflow.output != null && (
          <Button
            type="text"
            size="small"
            onClick={() => {
              const outputStr = typeof workflow.output === "string"
                ? workflow.output
                : JSON.stringify(workflow.output, null, 2);
              navigator.clipboard.writeText(outputStr);
              message.success(t("chat.workflow.outputCopied"));
            }}
            className="text-xs px-1.5 py-0.5"
          >
            {t("chat.workflow.viewOutput")}
          </Button>
        )}
      </div>

      {/* DAG View */}
      {showDag && (
        <div className="border-b border-purple-200 dark:border-purple-800">
          <button
            onClick={() => setDagCollapsed(!dagCollapsed)}
            className="flex items-center gap-1 w-full px-3 py-1 text-xs text-zinc-500 hover:bg-zinc-50 dark:hover:bg-zinc-800/50"
          >
            {dagCollapsed ? <ChevronRight size={12} /> : <ChevronDown size={12} />}
            {t("chat.workflow.dagVisualization")}
          </button>
          {!dagCollapsed
            && (steps.length === 0
              ? (
                <div className="px-3 py-4 text-xs text-zinc-400 text-center">
                  {t("chat.workflow.noSteps")}
                </div>
              )
              : (
                <div className="px-3 pb-2">
                  <div
                    className="border border-zinc-200 dark:border-zinc-700 rounded overflow-hidden"
                    style={{ height: DAG_HEIGHT }}
                  >
                    <ReactFlowProvider>
                      <WorkflowDagView steps={steps} />
                    </ReactFlowProvider>
                  </div>
                </div>
              ))}
        </div>
      )}

      {/* Step List View */}
      {!showDag
        && (steps.length === 0
          ? (
            <div className="px-3 py-4 text-xs text-zinc-400 text-center">
              {t("chat.workflow.noSteps")}
            </div>
          )
          : (
            <div className="max-h-64 overflow-auto">
              {steps.map((step) => (
                <StepRow
                  key={step.id}
                  step={step}
                  expanded={expandedSteps.has(step.id)}
                  onToggle={() => toggleStep(step.id)}
                />
              ))}
            </div>
          ))}
    </div>
  );
};
