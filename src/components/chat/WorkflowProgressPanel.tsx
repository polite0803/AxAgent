import { invoke } from "@/lib/invoke";
import { Button, message, Spin } from "antd";
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
import ReactFlow, {
  Background,
  type Edge,
  Handle,
  type Node,
  type NodeProps,
  Position,
  ReactFlowProvider,
  useReactFlow,
} from "reactflow";
import "reactflow/dist/style.css";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface WorkflowStep {
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

interface WorkflowData {
  id: string;
  name: string;
  status: "created" | "running" | "completed" | "partially_completed" | "failed" | "cancelled";
  steps: WorkflowStep[];
  max_concurrent: number;
  created_at?: number;
  completed_at?: number;
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

function truncate(str: string | null, maxLen: number): string {
  if (!str) { return ""; }
  if (str.length <= maxLen) { return str; }
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

const getStatusColor = (status: string) => {
  switch (status) {
    case "pending":
    case "created":
      return "#8c8c8c";
    case "running":
      return "#1890ff";
    case "completed":
      return "#52c41a";
    case "failed":
      return "#ff4d4f";
    case "skipped":
    case "partially_completed":
      return "#faad14";
    case "cancelled":
      return "#8c8c8c";
    default:
      return "#8c8c8c";
  }
};

function isDone(status: WorkflowStep["status"]): boolean {
  return status === "completed" || status === "failed" || status === "skipped";
}

// ---------------------------------------------------------------------------
// DAG Layout
// ---------------------------------------------------------------------------

interface WorkflowDagNodeData {
  stepId: string;
  goal: string;
  agentRole: string;
  status: WorkflowStep["status"];
}

function computeDagLayout(steps: WorkflowStep[]): { nodes: Node<WorkflowDagNodeData>[]; edges: Edge[] } {
  if (steps.length === 0) { return { nodes: [], edges: [] }; }

  const stepMap = new Map(steps.map((s) => [s.id, s]));
  const layerCache = new Map<string, number>();

  function getLayer(id: string): number {
    if (layerCache.has(id)) { return layerCache.get(id)!; }
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

  const layerGroups = new Map<number, WorkflowStep[]>();
  for (const step of steps) {
    const layer = layerCache.get(step.id)!;
    if (!layerGroups.has(layer)) { layerGroups.set(layer, []); }
    layerGroups.get(layer)!.push(step);
  }

  const sortedLayers = [...layerGroups.keys()].toSorted((a, b) => a - b);
  const nodes: Node<WorkflowDagNodeData>[] = [];

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
          style: { stroke: step.status === "failed" ? "#ff4d4f" : "#b1b1b7", strokeWidth: 1.5 },
        });
      }
    }
  }

  return { nodes, edges };
}

// ---------------------------------------------------------------------------
// WorkflowDagNode
// ---------------------------------------------------------------------------

const WorkflowDagNode: React.FC<NodeProps<WorkflowDagNodeData>> = memo(({ data, selected }) => {
  const color = getStatusColor(data.status);
  const isRunning = data.status === "running";
  const isFailed = data.status === "failed";

  return (
    <div
      className="rounded-md border px-2 py-1.5 bg-white dark:bg-zinc-800 shadow-sm"
      style={{
        width: NODE_WIDTH,
        borderColor: selected ? "#1890ff" : color,
        borderWidth: selected ? 2 : 1,
        opacity: data.status === "skipped" ? 0.65 : 1,
      }}
    >
      <Handle type="target" position={Position.Top} style={{ visibility: "hidden" }} />
      <div className="flex items-center gap-1">
        {isRunning
          ? <Loader2 size={10} className="animate-spin shrink-0" style={{ color }} />
          : isFailed
          ? <XCircle size={10} className="shrink-0" style={{ color }} />
          : <CheckCircle size={10} className="shrink-0" style={{ color }} />}
        <span className="text-[10px] font-mono font-medium truncate" style={{ color }}>
          {data.stepId}
        </span>
      </div>
      <div className="text-[10px] text-zinc-500 dark:text-zinc-400 truncate mt-0.5 leading-tight">
        {truncate(data.goal, 28)}
      </div>
      <div className="text-[9px] text-zinc-400 dark:text-zinc-500 truncate">
        {data.agentRole}
      </div>
      <Handle type="source" position={Position.Bottom} style={{ visibility: "hidden" }} />
    </div>
  );
});
WorkflowDagNode.displayName = "WorkflowDagNode";

const nodeTypes = { workflowStep: WorkflowDagNode };

// ---------------------------------------------------------------------------
// WorkflowDagView (inner - needs ReactFlowProvider)
// ---------------------------------------------------------------------------

const WorkflowDagView: React.FC<{ steps: WorkflowStep[] }> = memo(({ steps }) => {
  const { fitView } = useReactFlow();
  const { nodes, edges } = useMemo(() => computeDagLayout(steps), [steps]);

  useEffect(() => {
    if (nodes.length > 0) {
      const timer = setTimeout(() => {
        fitView({ padding: 0.3, duration: 200 });
      }, 60);
      return () => clearTimeout(timer);
    }
  }, [nodes, fitView]);

  if (nodes.length === 0) { return null; }

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
      <Background color="#e5e5e5" gap={16} size={0.5} />
    </ReactFlow>
  );
});
WorkflowDagView.displayName = "WorkflowDagView";

// ---------------------------------------------------------------------------
// StepRow (memoized)
// ---------------------------------------------------------------------------

interface StepRowProps {
  step: WorkflowStep;
  expanded: boolean;
  onToggle: () => void;
}

const StepRow = memo(function StepRow({ step, expanded, onToggle }: StepRowProps) {
  const { t } = useTranslation();
  const color = getStatusColor(step.status);

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
        <span style={{ color, display: "flex", alignItems: "center", flexShrink: 0 }}>
          <StatusIcon size={14} className={iconClass} />
        </span>
        <span className="text-xs font-mono font-medium shrink-0" style={{ color }}>
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
            <span className="text-zinc-500">{t("chat.workflow.stepStatus")}</span>
            <span style={{ color }}>{t(`chat.workflow.status.${step.status}`)}</span>
          </div>
          {step.needs.length > 0 && (
            <div className="flex gap-4">
              <span className="text-zinc-500">{t("chat.workflow.dependsOn")}</span>
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
                {truncate(step.error, 500)}
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

export const WorkflowProgressPanel: React.FC<WorkflowProgressPanelProps> = ({ conversationId }) => {
  const { t } = useTranslation();

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
      const { conversationId: cid, workflowId: wid } = (e as CustomEvent<{
        conversationId: string;
        workflowId: string | null;
      }>).detail;
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
    setWorkflowId(getWorkflowIdFromStorage(conversationId));
  }, [conversationId]);

  // --- Reset internal state on workflowId change ---
  useEffect(() => {
    setWorkflow(null);
    setLoading(false);
    setError(null);
    setExpandedSteps(new Set());
    setShowDag(true);
    setDagCollapsed(false);
  }, [workflowId]);

  // --- Poll workflow status with race-condition protection & terminal stop ---
  const workflowRef = useRef(workflow);
  workflowRef.current = workflow;

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
      if (stoppedByTerminal) { return; }
      const requestId = ++fetchIdRef.current;

      if (requestId === 1) {
        setLoading(true);
      }

      try {
        const data = await invoke<WorkflowData>("workflow_get_status", { workflowId });
        if (fetchIdRef.current !== requestId) { return; }
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
        if (fetchIdRef.current !== requestId) { return; }
        const msg = e instanceof Error ? e.message : String(e);
        setError(msg);
        if (workflowRef.current) {
          message.warning(msg);
        }
        console.error("[WorkflowProgressPanel] Failed to fetch workflow status:", e);
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
    if (!workflowId) { return; }
    setCancelling(true);
    try {
      await invoke("workflow_cancel", { workflowId });
      message.success(t("chat.workflow.cancelled"));
      const data = await invoke<WorkflowData>("workflow_get_status", { workflowId });
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

  if (!workflowId) {
    return null;
  }

  if (loading && !workflow) {
    return (
      <div className="mx-3 my-1.5 border border-purple-200 dark:border-purple-800 rounded-lg bg-purple-50/50 dark:bg-purple-900/10 p-4">
        <div className="flex items-center gap-2">
          <Spin size="small" />
          <span className="text-sm text-zinc-500">{t("chat.workflow.loading")}</span>
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

  const workflowColor = getStatusColor(workflow.status);
  const doneCount = workflow.steps.filter((s) => isDone(s.status)).length;
  const totalCount = workflow.steps.length;
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
            && (workflow.steps.length === 0
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
                      <WorkflowDagView steps={workflow.steps} />
                    </ReactFlowProvider>
                  </div>
                </div>
              ))}
        </div>
      )}

      {/* Step List View */}
      {!showDag
        && (workflow.steps.length === 0
          ? (
            <div className="px-3 py-4 text-xs text-zinc-400 text-center">
              {t("chat.workflow.noSteps")}
            </div>
          )
          : (
            <div className="max-h-64 overflow-auto">
              {workflow.steps.map((step) => (
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
