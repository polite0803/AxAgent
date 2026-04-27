import { Button, Card, Collapse, Progress, Tag, Tooltip, Typography } from "antd";
import {
  CheckCircle,
  Circle,
  Clock,
  Loader2,
  XCircle,
  GitBranch,
  Play,
  Pause,
  RotateCcw,
  Bug,
  Search,
  Lightbulb,
  CheckCircle2,
} from "lucide-react";
import { useCallback, useEffect, useState } from "react";

const { Text } = Typography;

interface TaskNode {
  id: string;
  description: string;
  task_type: "tool_call" | "reasoning" | "query" | "validation";
  dependencies: string[];
  status: "pending" | "running" | "completed" | "failed" | "skipped";
  result?: unknown;
  error?: string;
  created_at: string;
  started_at?: string;
  completed_at?: string;
}

interface TaskGraph {
  tasks: TaskNode[];
  parallel_groups: string[][];
}

interface ExecutionProgress {
  total_tasks: number;
  completed_tasks: number;
  failed_tasks: number;
  current_tasks: string[];
  percentage: number;
}

interface TaskDecompositionPanelProps {
  graph: TaskGraph | null;
  progress: ExecutionProgress | null;
  isExecuting: boolean;
  onExecute?: () => void;
  onReset?: () => void;
}

const taskTypeIcons: Record<string, React.ReactNode> = {
  tool_call: <Bug size={14} />,
  reasoning: <Lightbulb size={14} />,
  query: <Search size={14} />,
  validation: <CheckCircle2 size={14} />,
};

const taskTypeColors: Record<string, string> = {
  tool_call: "orange",
  reasoning: "purple",
  query: "blue",
  validation: "green",
};

const statusIcons: Record<string, React.ReactNode> = {
  pending: <Circle size={14} />,
  running: <Loader2 size={14} className="animate-spin" />,
  completed: <CheckCircle size={14} className="text-green-500" />,
  failed: <XCircle size={14} className="text-red-500" />,
  skipped: <Pause size={14} className="text-gray-400" />,
};

function formatTimestamp(ts: string): string {
  try {
    const date = new Date(ts);
    return date.toLocaleTimeString();
  } catch {
    return ts;
  }
}

function TaskCard({
  task,
  isActive,
  allTasks,
}: {
  task: TaskNode;
  isActive: boolean;
  allTasks: TaskNode[];
}) {
  const dependencyNames = task.dependencies
    .map((depId) => allTasks.find((t) => t.id === depId)?.description || depId)
    .slice(0, 3);

  return (
    <Card
      size="small"
      className={`task-card ${isActive ? "active" : ""}`}
      style={{
        borderColor: isActive ? "#1890ff" : undefined,
        backgroundColor: isActive ? "#f0f7ff" : undefined,
      }}
    >
      <div className="flex items-start gap-2 mb-2">
        <Tag
          color={taskTypeColors[task.task_type]}
          icon={taskTypeIcons[task.task_type]}
        >
          {task.task_type.replace("_", " ")}
        </Tag>
        {statusIcons[task.status]}
        <Text type="secondary" className="text-xs">
          {formatTimestamp(task.created_at)}
        </Text>
      </div>

      <div className="mb-2">
        <Text className="text-sm">{task.description}</Text>
      </div>

      {task.dependencies.length > 0 && (
        <div className="mb-2">
          <Text type="secondary" className="text-xs">
            依赖:
          </Text>
          <div className="flex flex-wrap gap-1 mt-1">
            {dependencyNames.map((dep, idx) => (
              <Tag key={idx} className="text-xs">
                {dep.slice(0, 20)}
                {dep.length > 20 ? "..." : ""}
              </Tag>
            ))}
            {task.dependencies.length > 3 && (
              <Tag className="text-xs">+{task.dependencies.length - 3}</Tag>
            )}
          </div>
        </div>
      )}

      {task.result !== undefined && (
        <div className="p-2 bg-gray-50 rounded">
          <Text strong className="text-xs text-gray-500">
            结果:
          </Text>
          <p className="text-sm mt-1 mb-0 whitespace-pre-wrap">
            {JSON.stringify(task.result, null, 2).slice(0, 200)}
          </p>
        </div>
      )}

      {task.error && (
        <div className="p-2 bg-red-50 rounded border border-red-200">
          <Text strong className="text-xs text-red-500">
            错误:
          </Text>
          <p className="text-sm mt-1 mb-0 text-red-600">{task.error}</p>
        </div>
      )}

      {task.started_at && (
        <div className="mt-2 flex items-center gap-2 text-xs text-gray-400">
          <Clock size={12} />
          <span>开始: {formatTimestamp(task.started_at)}</span>
          {task.completed_at && (
            <span>
              → 完成: {formatTimestamp(task.completed_at)}
            </span>
          )}
        </div>
      )}
    </Card>
  );
}

export function TaskDecompositionPanel({
  graph,
  progress,
  isExecuting,
  onExecute,
  onReset,
}: TaskDecompositionPanelProps) {
  const [expandedKeys, setExpandedKeys] = useState<string[]>([]);

  useEffect(() => {
    if (graph && graph.tasks.length > 0) {
      setExpandedKeys(graph.tasks.map((_, i) => String(i)));
    }
  }, [graph]);

  const handleCollapseChange = useCallback((keys: string[]) => {
    setExpandedKeys(keys);
  }, []);

  if (!graph) {
    return (
      <Card size="small" className="task-decomposition-panel">
        <div className="flex items-center justify-center h-32 text-gray-400">
          <GitBranch size={24} className="mr-2" />
          <Text type="secondary">暂无任务分解</Text>
        </div>
      </Card>
    );
  }

  const items = graph.tasks.map((task, index) => {
    const isActive = task.status === "running";
    const isComplete = task.status === "completed";

    return {
      key: index,
      label: (
        <div className="flex items-center justify-between w-full pr-2">
          <div className="flex items-center gap-2">
            <Tag color={taskTypeColors[task.task_type]}>
              {task.task_type.replace("_", " ")}
            </Tag>
            <Text className="text-sm">{task.description.slice(0, 40)}</Text>
            {task.description.length > 40 && <Text>...</Text>}
          </div>
          <div className="flex items-center gap-2">
            {statusIcons[task.status]}
            {isComplete && (
              <Tooltip title="完成">
                <CheckCircle size={14} className="text-green-500" />
              </Tooltip>
            )}
          </div>
        </div>
      ),
      children: <TaskCard task={task} isActive={isActive} allTasks={graph.tasks} />,
    };
  });

  return (
    <Card
      size="small"
      className="task-decomposition-panel"
      title={
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <GitBranch size={16} />
            <span>任务分解</span>
            <Tag>{graph.tasks.length} 任务</Tag>
            {graph.parallel_groups.length > 0 && (
              <Tag>{graph.parallel_groups.length} 组</Tag>
            )}
          </div>
          {isExecuting && (
            <Tag color="blue" icon={<Loader2 size={12} className="animate-spin" />}>
              执行中
            </Tag>
          )}
        </div>
      }
      extra={
        <div className="flex items-center gap-2">
          {onReset && (
            <Button
              type="text"
              size="small"
              icon={<RotateCcw size={14} />}
              onClick={onReset}
              disabled={isExecuting}
            />
          )}
          {onExecute && (
            <Button
              type="primary"
              size="small"
              icon={isExecuting ? <Pause size={14} /> : <Play size={14} />}
              onClick={onExecute}
              disabled={isExecuting || graph.tasks.length === 0}
            >
              {isExecuting ? "暂停" : "执行"}
            </Button>
          )}
        </div>
      }
    >
      {progress && (
        <div className="mb-4">
          <div className="flex items-center justify-between mb-2">
            <Text type="secondary" className="text-sm">
              执行进度
            </Text>
            <Text className="text-sm font-medium">
              {progress.completed_tasks}/{progress.total_tasks}
              {progress.failed_tasks > 0 && (
                <span className="text-red-500 ml-2">
                  ({progress.failed_tasks} 失败)
                </span>
              )}
            </Text>
          </div>
          <Progress
            percent={Math.round(progress.percentage)}
            status={
              progress.failed_tasks > 0
                ? "exception"
                : progress.percentage === 100
                  ? "success"
                  : "active"
            }
            strokeColor={
              progress.failed_tasks > 0 ? "#ff4d4f" : undefined
            }
          />
          {progress.current_tasks.length > 0 && (
            <div className="flex items-center gap-2 mt-2">
              <Loader2 size={12} className="animate-spin text-blue-500" />
              <Text type="secondary" className="text-xs">
                当前: {progress.current_tasks.join(", ")}
              </Text>
            </div>
          )}
        </div>
      )}

      <div className="mb-4">
        <Text type="secondary" className="text-sm">
          并行分组
        </Text>
        <div className="flex flex-wrap gap-2 mt-2">
          {graph.parallel_groups.map((group, idx) => (
            <Tooltip
              key={idx}
              title={`组 ${idx + 1}: ${group.join(", ")}`}
            >
              <Tag
                className="cursor-pointer"
                onClick={() => {
                  const startIdx = graph.tasks.findIndex(
                    (t) => t.id === group[0]
                  );
                  if (startIdx >= 0) {
                    const key = String(startIdx);
                    setExpandedKeys((prev) => {
                      if (prev.includes(key)) {
                        return prev.filter((k) => k !== key);
                      }
                      return [...prev, key];
                    });
                  }
                }}
              >
                {idx + 1}: {group.length} 任务
              </Tag>
            </Tooltip>
          ))}
        </div>
      </div>

      <Collapse
        activeKey={expandedKeys}
        onChange={handleCollapseChange}
        items={items}
        bordered={false}
      />
    </Card>
  );
}

export function useTaskDecomposition() {
  const [graph, setGraph] = useState<TaskGraph | null>(null);
  const [progress, setProgress] = useState<ExecutionProgress | null>(null);
  const [isExecuting, setIsExecuting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const prepare = useCallback(async (userInput: string) => {
    setError(null);
    try {
      const response = await fetch("/api/task/decompose", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ input: userInput }),
      });

      if (!response.ok) {
        throw new Error("Failed to decompose task");
      }

      const result = await response.json();
      setGraph(result);
      return result;
    } catch (e) {
      setError(String(e));
      throw e;
    }
  }, []);

  const execute = useCallback(async () => {
    if (!graph) return;

    setIsExecuting(true);
    setError(null);

    try {
      const response = await fetch("/api/task/execute", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ graph }),
      });

      if (!response.ok) {
        throw new Error("Failed to execute tasks");
      }

      const result = await response.json();
      setGraph(result);
      setProgress({
        total_tasks: result.tasks.length,
        completed_tasks: result.tasks.filter(
          (t: TaskNode) => t.status === "completed"
        ).length,
        failed_tasks: result.tasks.filter(
          (t: TaskNode) => t.status === "failed"
        ).length,
        current_tasks: [],
        percentage: 100,
      });
    } catch (e) {
      setError(String(e));
    } finally {
      setIsExecuting(false);
    }
  }, [graph]);

  const reset = useCallback(() => {
    setGraph(null);
    setProgress(null);
    setIsExecuting(false);
    setError(null);
  }, []);

  return {
    graph,
    progress,
    isExecuting,
    error,
    prepare,
    execute,
    reset,
  };
}
