import { Card, Badge, Typography, Descriptions } from "antd";
import type { TaskResult as TaskResultType } from "@/types/evaluator";
import { formatScore, formatDuration, getDifficultyLabel } from "@/types/evaluator";

interface TaskResultCardProps {
  result: TaskResultType;
}

export function TaskResultCard({ result }: TaskResultCardProps) {
  return (
    <Card className="mt-3">
      <div className="flex items-start justify-between mb-4">
        <div>
          <Typography.Title level={5} className="mb-1">
            {result.task_name}
          </Typography.Title>
          <div className="flex gap-2">
            <Badge color={result.success ? "green" : "red"}>
              {result.success ? "通过" : "失败"}
            </Badge>
            <Badge color="blue">{getDifficultyLabel(result.difficulty)}</Badge>
          </div>
        </div>
        <div className="text-right">
          <div className="text-2xl font-bold">{formatScore(result.overall_score)}</div>
          <div className="text-xs text-gray-500">{formatDuration(result.duration_ms)}</div>
        </div>
      </div>

      <Descriptions size="small" className="mb-4">
        <Descriptions.Item label="任务ID">{result.task_id}</Descriptions.Item>
        {result.trace_id && (
          <Descriptions.Item label="追踪ID">{result.trace_id}</Descriptions.Item>
        )}
      </Descriptions>

      {result.error && (
        <div className="mb-4 p-3 bg-red-50 border border-red-200 rounded">
          <Typography.Text type="danger" className="font-medium">
            错误: {result.error}
          </Typography.Text>
        </div>
      )}

      <div className="mb-4">
        <Typography.Text strong className="mb-2 block">
          评分详情
        </Typography.Text>
        <div className="space-y-2">
          {result.scores.map((score) => (
            <div
              key={score.criteria_name}
              className="flex items-center justify-between p-2 bg-gray-50 rounded"
            >
              <span className="text-sm">{score.criteria_name}</span>
              <div className="flex items-center gap-2">
                <span className="text-sm font-medium">
                  {formatScore(score.raw_score)}
                </span>
                <Badge color={score.passed ? "green" : "red"} />
              </div>
            </div>
          ))}
        </div>
      </div>

      {result.response && (
        <div>
          <Typography.Text strong className="mb-2 block">
            响应
          </Typography.Text>
          <pre className="block whitespace-pre-wrap bg-gray-50 p-2 rounded overflow-auto">{result.response}</pre>
        </div>
      )}
    </Card>
  );
}
