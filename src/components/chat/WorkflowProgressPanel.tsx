import { invoke } from "@/lib/invoke";
import { useConversationStore } from "@/stores";
import {
  AlertTriangle,
  CheckCircle,
  ChevronDown,
  ChevronRight,
  Clock,
  GitBranch,
  Loader2,
  SkipForward,
  XCircle,
} from "lucide-react";
import React, { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

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
}

// ---------------------------------------------------------------------------
// Status helpers (i18n-aware)
// ---------------------------------------------------------------------------

const getStatusConfig = (
  t: (key: string) => string,
): Record<string, { icon: React.ReactNode; color: string; label: string }> => ({
  pending: { icon: <Clock size={14} />, color: "#8c8c8c", label: t("chat.workflow.status.pending") },
  running: {
    icon: <Loader2 size={14} className="animate-spin" />,
    color: "#1890ff",
    label: t("chat.workflow.status.running"),
  },
  completed: { icon: <CheckCircle size={14} />, color: "#52c41a", label: t("chat.workflow.status.completed") },
  failed: { icon: <XCircle size={14} />, color: "#ff4d4f", label: t("chat.workflow.status.failed") },
  skipped: { icon: <SkipForward size={14} />, color: "#faad14", label: t("chat.workflow.status.skipped") },
});

const getWorkflowStatusConfig = (t: (key: string) => string): Record<string, { color: string; label: string }> => ({
  created: { color: "#8c8c8c", label: t("chat.workflow.workflowStatus.created") },
  running: { color: "#1890ff", label: t("chat.workflow.workflowStatus.running") },
  completed: { color: "#52c41a", label: t("chat.workflow.workflowStatus.completed") },
  partially_completed: { color: "#faad14", label: t("chat.workflow.workflowStatus.partiallyCompleted") },
  failed: { color: "#ff4d4f", label: t("chat.workflow.workflowStatus.failed") },
  cancelled: { color: "#8c8c8c", label: t("chat.workflow.workflowStatus.cancelled") },
});

// ---------------------------------------------------------------------------
// Mermaid DAG generator
// ---------------------------------------------------------------------------

function generateMermaidDag(steps: WorkflowStep[]): string {
  const lines: string[] = ["graph TD"];
  const added = new Set<string>();

  for (const step of steps) {
    if (!added.has(step.id)) {
      lines.push(`  ${step.id}["${step.id}<br/>${step.goal.slice(0, 30)}${step.goal.length > 30 ? "..." : ""}"]`);
      added.add(step.id);
    }
    for (const dep of step.needs) {
      if (!added.has(dep)) {
        const depStep = steps.find(s => s.id === dep);
        const depGoal = depStep ? depStep.goal.slice(0, 30) : dep;
        lines.push(`  ${dep}["${dep}<br/>${depGoal}${depStep && depStep.goal.length > 30 ? "..." : ""}"]`);
        added.add(dep);
      }
      lines.push(`  ${dep} --> ${step.id}`);
    }
  }

  // Add style classes for each status
  const byStatus: Record<string, string[]> = {};
  for (const step of steps) {
    (byStatus[step.status] ??= []).push(step.id);
  }
  if (byStatus.running?.length) { lines.push(`  style ${byStatus.running.join(",")} fill:#e6f7ff,stroke:#1890ff`); }
  if (byStatus.completed?.length) { lines.push(`  style ${byStatus.completed.join(",")} fill:#f6ffed,stroke:#52c41a`); }
  if (byStatus.failed?.length) { lines.push(`  style ${byStatus.failed.join(",")} fill:#fff2f0,stroke:#ff4d4f`); }
  if (byStatus.skipped?.length) { lines.push(`  style ${byStatus.skipped.join(",")} fill:#fffbe6,stroke:#faad14`); }

  return lines.join("\n");
}

// ---------------------------------------------------------------------------
// Step detail row
// ---------------------------------------------------------------------------

function StepRow({ step, expanded, onToggle, statusConfig, t }: {
  step: WorkflowStep;
  expanded: boolean;
  onToggle: () => void;
  statusConfig: Record<string, { icon: React.ReactNode; color: string; label: string }>;
  t: (key: string, options?: Record<string, unknown>) => string;
}) {
  const cfg = statusConfig[step.status] ?? statusConfig.pending;

  return (
    <div className="border-b border-gray-100 dark:border-gray-800 last:border-b-0">
      <div
        className="flex items-center gap-2 px-3 py-2 cursor-pointer hover:bg-gray-50 dark:hover:bg-gray-800/50"
        onClick={onToggle}
      >
        <span style={{ color: cfg.color, display: "flex", alignItems: "center", flexShrink: 0 }}>
          {cfg.icon}
        </span>
        <span className="text-xs font-mono font-medium shrink-0" style={{ color: cfg.color }}>
          {step.id}
        </span>
        <span className="text-xs text-gray-500 dark:text-gray-400 truncate flex-1">
          {step.goal}
        </span>
        <span className="text-xs text-gray-400 dark:text-gray-500 shrink-0">
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
        <span
          style={{ display: "flex", alignItems: "center", flexShrink: 0, color: "var(--color-text-secondary, #999)" }}
        >
          {expanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
        </span>
      </div>
      {expanded && (
        <div className="px-3 pb-2 text-xs space-y-1">
          <div className="flex gap-4">
            <span className="text-gray-500">{t("chat.workflow.stepStatus")}</span>
            <span style={{ color: cfg.color }}>{cfg.label}</span>
          </div>
          {step.needs.length > 0 && (
            <div className="flex gap-4">
              <span className="text-gray-500">{t("chat.workflow.dependsOn")}</span>
              <span>{step.needs.join(", ")}</span>
            </div>
          )}
          <div className="flex gap-4">
            <span className="text-gray-500">{t("chat.workflow.retries")}</span>
            <span>{step.attempts}/{step.max_retries + 1}</span>
          </div>
          {step.result && (
            <div>
              <span className="text-gray-500">{t("chat.workflow.result")}</span>
              <pre className="mt-1 p-2 bg-gray-100 dark:bg-gray-800 rounded text-xs max-h-32 overflow-auto whitespace-pre-wrap">
                {step.result.length > 500 ? step.result.slice(0, 500) + '...' : step.result}
              </pre>
            </div>
          )}
          {step.error && (
            <div>
              <span className="text-red-500">{t("chat.workflow.error")}</span>
              <pre className="mt-1 p-2 bg-red-50 dark:bg-red-900/20 rounded text-xs max-h-32 overflow-auto whitespace-pre-wrap text-red-600 dark:text-red-400">
                {step.error}
              </pre>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main component
// ---------------------------------------------------------------------------

const WorkflowProgressPanel: React.FC = () => {
  const { t } = useTranslation();
  const activeConversationId = useConversationStore((s) => s.activeConversationId);

  const [workflow, setWorkflow] = useState<WorkflowData | null>(null);
  const [expandedSteps, setExpandedSteps] = useState<Set<string>>(new Set());
  const [showDag, setShowDag] = useState(true);
  const [dagCollapsed, setDagCollapsed] = useState(false);

  const statusConfig = useMemo(() => getStatusConfig(t), [t]);
  const workflowStatusConfig = useMemo(() => getWorkflowStatusConfig(t), [t]);

  // Read workflow ID from localStorage (set by WorkflowTemplateSelector)
  const workflowId = useMemo(() => {
    if (!activeConversationId) { return null; }
    return localStorage.getItem(`axagent:workflow-id:${activeConversationId}`);
  }, [activeConversationId]);

  // Poll workflow status
  useEffect(() => {
    if (!workflowId) {
      setWorkflow(null);
      return;
    }

    const fetchStatus = async () => {
      try {
        const data = await invoke<WorkflowData>("workflow_get_status", { workflowId });
        setWorkflow(data);
      } catch {
        // Workflow may not exist yet
      }
    };

    fetchStatus();
    const interval = setInterval(fetchStatus, 2000);
    return () => clearInterval(interval);
  }, [workflowId]);

  const toggleStep = useCallback((stepId: string) => {
    setExpandedSteps(prev => {
      const next = new Set(prev);
      if (next.has(stepId)) { next.delete(stepId); }
      else { next.add(stepId); }
      return next;
    });
  }, []);

  // Don't render if no workflow
  if (!workflowId || !workflow) { return null; }

  const wsCfg = workflowStatusConfig[workflow.status] ?? workflowStatusConfig.created;
  const completedCount = workflow.steps.filter(s => s.status === "completed").length;
  const totalCount = workflow.steps.length;
  const progressPct = totalCount > 0 ? (completedCount / totalCount) * 100 : 0;

  const mermaidCode = useMemo(() => generateMermaidDag(workflow.steps), [workflow.steps]);

  return (
    <div className="mx-3 my-1.5 border border-purple-200 dark:border-purple-800 rounded-lg bg-purple-50/50 dark:bg-purple-900/10 overflow-hidden">
      {/* Header */}
      <div className="flex items-center gap-2 px-3 py-1.5 border-b border-purple-200 dark:border-purple-800">
        <GitBranch size={14} style={{ color: wsCfg.color }} />
        <span className="text-xs font-medium" style={{ color: wsCfg.color }}>
          {workflow.name}
        </span>
        <span className="text-xs text-gray-500 dark:text-gray-400">
          {wsCfg.label}
        </span>
        {/* Progress bar */}
        <div className="flex-1 h-1.5 bg-gray-200 dark:bg-gray-700 rounded-full overflow-hidden ml-2">
          <div
            className="h-full rounded-full transition-all duration-300"
            style={{
              width: `${progressPct}%`,
              backgroundColor: wsCfg.color,
            }}
          />
        </div>
        <span className="text-xs text-gray-500 dark:text-gray-400 tabular-nums">
          {completedCount}/{totalCount}
        </span>
        <button
          onClick={() => setShowDag(!showDag)}
          className="text-xs px-1.5 py-0.5 rounded border border-purple-300 dark:border-purple-700 hover:bg-purple-100 dark:hover:bg-purple-800/30 transition-colors"
        >
          {showDag ? t("chat.workflow.listView") : t("chat.workflow.dagView")}
        </button>
      </div>

      {/* DAG view (Mermaid) */}
      {showDag && (
        <div className="border-b border-purple-200 dark:border-purple-800">
          <button
            onClick={() => setDagCollapsed(!dagCollapsed)}
            className="flex items-center gap-1 w-full px-3 py-1 text-xs text-gray-500 hover:bg-gray-50 dark:hover:bg-gray-800/50"
          >
            {dagCollapsed ? <ChevronRight size={12} /> : <ChevronDown size={12} />}
            {t("chat.workflow.dagVisualization")}
          </button>
          {!dagCollapsed && (
            <div className="px-3 pb-2">
              <pre className="text-xs bg-white dark:bg-gray-900 border border-gray-200 dark:border-gray-700 rounded p-2 overflow-auto max-h-48">
                {mermaidCode}
              </pre>
              <p className="text-xs text-gray-400 mt-1">
                {t("chat.workflow.dagHint")}
              </p>
            </div>
          )}
        </div>
      )}

      {/* Step list */}
      {!showDag && (
        <div className="max-h-64 overflow-auto">
          {workflow.steps.map(step => (
            <StepRow
              key={step.id}
              step={step}
              expanded={expandedSteps.has(step.id)}
              onToggle={() => toggleStep(step.id)}
              statusConfig={statusConfig}
              t={t}
            />
          ))}
        </div>
      )}
    </div>
  );
};

export default WorkflowProgressPanel;
