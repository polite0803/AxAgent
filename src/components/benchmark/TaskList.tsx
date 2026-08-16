// SPDX-License-Identifier: AGPL-3.0-only

import type { BenchmarkTask, TaskResult } from "@/types";
import { formatDuration, formatScore, getDifficultyKey } from "@/types";
import { Button, Table, Tag } from "antd";
import { useTranslation } from "react-i18next";
import { TaskResultCard } from "./TaskResult";

interface TaskListProps {
  tasks: BenchmarkTask[];
  results: TaskResult[];
  onRetry?: (taskId: string) => void;
}

export function TaskList({ tasks, results, onRetry }: TaskListProps) {
  const { t } = useTranslation();
  const columns = [
    {
      title: t("benchmark.task"),
      dataIndex: "name",
      key: "name",
      render: (_name: string, record: BenchmarkTask) => {
        return (
          <div>
            <div className="font-medium">{_name}</div>
            <div className="text-xs text-zinc-500">{record.description}</div>
          </div>
        );
      },
    },
    {
      title: t("benchmark.difficulty"),
      dataIndex: "difficulty",
      key: "difficulty",
      width: 80,
      render: (difficulty: BenchmarkTask["difficulty"]) => (
        <Tag color={getDifficultyColor(difficulty)}>
          {t(getDifficultyKey(difficulty))}
        </Tag>
      ),
    },
    {
      title: t("benchmark.status"),
      dataIndex: "taskId",
      key: "status",
      width: 100,
      render: (_: string, record: BenchmarkTask) => {
        const result = results.find((r) => r.taskId === record.id);
        if (!result) {
          return <Tag>{t("benchmark.waiting")}</Tag>;
        }
        return (
          <Tag color={result.success ? "green" : "red"}>
            {result.success ? t("benchmark.passed") : t("benchmark.failed")}
          </Tag>
        );
      },
    },
    {
      title: t("benchmark.score"),
      dataIndex: "taskId",
      key: "score",
      width: 80,
      render: (_: string, record: BenchmarkTask) => {
        const result = results.find((r) => r.taskId === record.id);
        if (!result) {
          return "-";
        }
        return formatScore(result.overallScore);
      },
    },
    {
      title: t("benchmark.duration"),
      dataIndex: "taskId",
      key: "duration",
      width: 100,
      render: (_: string, record: BenchmarkTask) => {
        const result = results.find((r) => r.taskId === record.id);
        if (!result) {
          return "-";
        }
        return formatDuration(result.durationMs);
      },
    },
    {
      title: t("benchmark.action"),
      dataIndex: "taskId",
      key: "action",
      width: 100,
      render: (taskId: string) => (
        <Button size="small" onClick={() => onRetry?.(taskId)}>
          {t("benchmark.retry")}
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
            const result = results.find((r) => r.taskId === record.id);
            if (!result) {
              return (
                <div className="p-4 text-zinc-500">
                  {t("benchmark.noResult")}
                </div>
              );
            }
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
