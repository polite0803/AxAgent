import { Table, Tag, Button } from "antd";
import type { TaskResult, BenchmarkTask } from "@/types/evaluator";
import { formatScore, formatDuration, getDifficultyLabel } from "@/types/evaluator";
import { TaskResultCard } from "./TaskResult";

interface TaskListProps {
  tasks: BenchmarkTask[];
  results: TaskResult[];
  onRetry?: (taskId: string) => void;
}

export function TaskList({ tasks, results, onRetry }: TaskListProps) {
  const columns = [
    {
      title: "任务",
      dataIndex: "name",
      key: "name",
      render: (_name: string, record: BenchmarkTask) => {
        return (
          <div>
            <div className="font-medium">{_name}</div>
            <div className="text-xs text-gray-500">{record.description}</div>
          </div>
        );
      },
    },
    {
      title: "难度",
      dataIndex: "difficulty",
      key: "difficulty",
      width: 80,
      render: (difficulty: BenchmarkTask["difficulty"]) => (
        <Tag color={getDifficultyColor(difficulty)}>{getDifficultyLabel(difficulty)}</Tag>
      ),
    },
    {
      title: "状态",
      dataIndex: "task_id",
      key: "status",
      width: 100,
      render: (_: string, record: BenchmarkTask) => {
        const result = results.find((r) => r.task_id === record.id);
        if (!result) return <Tag>等待</Tag>;
        return (
          <Tag color={result.success ? "green" : "red"}>
            {result.success ? "通过" : "失败"}
          </Tag>
        );
      },
    },
    {
      title: "得分",
      dataIndex: "task_id",
      key: "score",
      width: 80,
      render: (_: string, record: BenchmarkTask) => {
        const result = results.find((r) => r.task_id === record.id);
        if (!result) return "-";
        return formatScore(result.overall_score);
      },
    },
    {
      title: "耗时",
      dataIndex: "task_id",
      key: "duration",
      width: 100,
      render: (_: string, record: BenchmarkTask) => {
        const result = results.find((r) => r.task_id === record.id);
        if (!result) return "-";
        return formatDuration(result.duration_ms);
      },
    },
    {
      title: "操作",
      dataIndex: "task_id",
      key: "action",
      width: 100,
      render: (taskId: string) => (
        <Button size="small" onClick={() => onRetry?.(taskId)}>
          重试
        </Button>
      ),
    },
  ];

  return (
    <div className="task-list">
      <Table
        dataSource={tasks}
        columns={columns}
        rowKey="id"
        size="small"
        pagination={false}
        expandable={{
          expandedRowRender: (record) => {
            const result = results.find((r) => r.task_id === record.id);
            if (!result) return <div className="p-4 text-gray-500">暂无结果</div>;
            return <TaskResultCard result={result} />;
          },
        }}
      />
    </div>
  );
}

function getDifficultyColor(difficulty: BenchmarkTask["difficulty"]): string {
  switch (difficulty) {
    case "easy":
      return "green";
    case "medium":
      return "blue";
    case "hard":
      return "orange";
    case "expert":
      return "red";
    default:
      return "default";
  }
}
