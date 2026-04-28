import { Button, Card, Progress, Space, Typography } from "antd";
import {
  CheckCircle,
  ChevronDown,
  ChevronRight,
  Clock,
  Loader2,
  Play,
  SkipForward,
  XCircle,
} from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { ProgressDashboard } from "./ProgressDashboard";
import { TaskDependencyGraph, TaskNode } from "./TaskDependencyGraph";

const { Text } = Typography;

interface PlannedTask {
  id: string;
  description: string;
  action_type: string;
  parameters: Record<string, unknown>;
  dependencies: string[];
  status: "pending" | "in_progress" | "completed" | "failed" | "skipped";
  result?: string;
  retry_count: number;
  max_retries: number;
}

interface Phase {
  id: string;
  name: string;
  description: string;
  tasks: PlannedTask[];
  dependencies: string[];
  status: "pending" | "in_progress" | "completed" | "failed" | "skipped";
}

interface Plan {
  id: string;
  goal: string;
  phases: Phase[];
  status: "draft" | "executing" | "paused" | "completed" | "failed";
  created_at: number;
  updated_at: number;
}

interface AutonomousPlanViewProps {
  plan: Plan;
  onStartPlan?: (planId: string) => void;
  onPausePlan?: (planId: string) => void;
  onCancelPlan?: (planId: string) => void;
  onRetryTask?: (planId: string, taskId: string) => void;
  readonly?: boolean;
}

function taskToTaskNode(task: PlannedTask, phaseName: string): TaskNode {
  return {
    id: task.id,
    description: task.description,
    status: task.status === "in_progress" ? "running" : task.status,
    dependencies: task.dependencies,
    result: task.result,
    retry_count: task.retry_count,
    max_retries: task.max_retries,
    phase: phaseName,
  };
}

function AutonomousPlanView({
  plan,
  onStartPlan,
  onPausePlan,
  onCancelPlan,
  onRetryTask,
  readonly = false,
}: AutonomousPlanViewProps) {
  const { t } = useTranslation();
  const [expandedPhases, setExpandedPhases] = useState<Set<string>>(new Set(plan.phases.map((p) => p.id)));
  const [showGraph, setShowGraph] = useState(false);

  const allTasks: TaskNode[] = plan.phases.flatMap((phase) =>
    phase.tasks.map((task) => taskToTaskNode(task, phase.name))
  );

  const togglePhase = (phaseId: string) => {
    setExpandedPhases((prev) => {
      const next = new Set(prev);
      if (next.has(phaseId)) {
        next.delete(phaseId);
      } else {
        next.add(phaseId);
      }
      return next;
    });
  };

  const getPhaseIcon = (status: Phase["status"]) => {
    switch (status) {
      case "completed":
        return <CheckCircle size={16} className="text-green-500" />;
      case "in_progress":
        return <Loader2 size={16} className="text-blue-500 animate-spin" />;
      case "failed":
        return <XCircle size={16} className="text-red-500" />;
      case "skipped":
        return <SkipForward size={16} className="text-yellow-500" />;
      default:
        return <Clock size={16} className="text-gray-400" />;
    }
  };

  const getTaskIcon = (status: PlannedTask["status"]) => {
    switch (status) {
      case "completed":
        return <CheckCircle size={12} className="text-green-500" />;
      case "in_progress":
        return <Loader2 size={12} className="text-blue-500 animate-spin" />;
      case "failed":
        return <XCircle size={12} className="text-red-500" />;
      case "skipped":
        return <SkipForward size={12} className="text-yellow-500" />;
      default:
        return <Clock size={12} className="text-gray-400" />;
    }
  };

  return (
    <div className="autonomous-plan-view space-y-3">
      <ProgressDashboard
        plan={{
          id: plan.id,
          goal: plan.goal,
          phases: plan.phases.map((p) => ({
            id: p.id,
            name: p.name,
            status: p.status,
            tasks: p.tasks.map((t) => ({
              id: t.id,
              description: t.description,
              status: t.status === "in_progress" ? ("running" as const) : t.status,
              result: t.result,
              retry_count: t.retry_count,
              max_retries: t.max_retries,
            })),
          })),
          status: plan.status,
          created_at: plan.created_at,
          updated_at: plan.updated_at,
        }}
      />

      <div className="flex items-center gap-2">
        {!readonly && plan.status === "draft" && onStartPlan && (
          <Button type="primary" size="small" icon={<Play size={14} />} onClick={() => onStartPlan(plan.id)}>
            {t("chat.planner.startPlan")}
          </Button>
        )}
        {!readonly && plan.status === "executing" && onPausePlan && (
          <Button size="small" onClick={() => onPausePlan(plan.id)}>
            {t("chat.planner.pausePlan")}
          </Button>
        )}
        {!readonly && (plan.status === "executing" || plan.status === "paused") && onCancelPlan && (
          <Button size="small" danger onClick={() => onCancelPlan(plan.id)}>
            {t("chat.planner.cancelPlan")}
          </Button>
        )}
        <Button size="small" type="link" onClick={() => setShowGraph(!showGraph)}>
          {showGraph ? t("chat.planner.hideGraph") : t("chat.planner.showGraph")}
        </Button>
      </div>

      {showGraph && allTasks.length > 0 && (
        <TaskDependencyGraph tasks={allTasks} showMermaid showDetails title={t("chat.planner.taskGraph")} />
      )}

      <div className="phases-list space-y-2">
        {plan.phases.map((phase) => {
          const completed = phase.tasks.filter((t) => t.status === "completed").length;
          const total = phase.tasks.length;
          const percent = total > 0 ? Math.round((completed / total) * 100) : 0;
          const isExpanded = expandedPhases.has(phase.id);

          return (
            <Card key={phase.id} size="small" className="phase-card">
              <div
                className="flex items-center justify-between cursor-pointer"
                onClick={() => togglePhase(phase.id)}
              >
                <Space>
                  {getPhaseIcon(phase.status)}
                  <Text strong>{phase.name}</Text>
                  {phase.description && (
                    <Text type="secondary" className="text-xs">
                      {phase.description}
                    </Text>
                  )}
                </Space>
                <Space size="small">
                  <Progress
                    type="circle"
                    percent={percent}
                    size={28}
                    strokeColor={
                      phase.status === "completed"
                        ? "#52c41a"
                        : phase.status === "failed"
                          ? "#ff4d4f"
                          : "#1890ff"
                    }
                  />
                  {isExpanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
                </Space>
              </div>

              {isExpanded && (
                <div className="mt-2 pl-4 border-l-2 border-gray-200 dark:border-gray-700 space-y-1">
                  {phase.tasks.length === 0 && (
                    <Text type="secondary" className="text-xs">
                      {t("chat.planner.noTasks")}
                    </Text>
                  )}
                  {phase.tasks.map((task) => (
                    <div key={task.id} className="flex items-center justify-between py-1">
                      <Space size="small">
                        {getTaskIcon(task.status)}
                        <Text className="text-sm" ellipsis={{ tooltip: task.description }}>
                          {task.description}
                        </Text>
                        <Text type="secondary" className="text-xs font-mono">
                          {task.action_type}
                        </Text>
                      </Space>
                      <Space size="small">
                        {task.retry_count > 0 && (
                          <Text type="warning" className="text-xs">
                            {t("chat.planner.retry", { count: task.retry_count })}
                          </Text>
                        )}
                        {!readonly && task.status === "failed" && onRetryTask && (
                          <Button size="small" type="link" onClick={() => onRetryTask(plan.id, task.id)}>
                            {t("chat.planner.retryTask")}
                          </Button>
                        )}
                      </Space>
                    </div>
                  ))}
                </div>
              )}
            </Card>
          );
        })}
      </div>
    </div>
  );
}

export default AutonomousPlanView;
