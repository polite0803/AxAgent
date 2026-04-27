import { Progress, Typography } from "antd";
import type { AggregateMetrics } from "@/types/evaluator";

interface ScoreCardProps {
  metrics: AggregateMetrics;
  title?: string;
}

export function ScoreCard({ metrics, title }: ScoreCardProps) {
  const passRatePercent = Math.round(metrics.pass_rate * 100);

  return (
    <div className="bg-white rounded-lg border p-4">
      {title && (
        <Typography.Title level={5} className="mb-4">
          {title}
        </Typography.Title>
      )}

      <div className="space-y-4">
        <div>
          <div className="flex justify-between text-sm mb-1">
            <span>通过率</span>
            <span className="font-medium">{passRatePercent}%</span>
          </div>
          <Progress percent={passRatePercent} showInfo={false} />
        </div>

        <div className="grid grid-cols-3 gap-4">
          <div className="text-center">
            <div className="text-2xl font-bold text-green-600">
              {metrics.passed_tasks}
            </div>
            <div className="text-xs text-gray-500">通过</div>
          </div>
          <div className="text-center">
            <div className="text-2xl font-bold text-red-600">
              {metrics.failed_tasks}
            </div>
            <div className="text-xs text-gray-500">失败</div>
          </div>
          <div className="text-center">
            <div className="text-2xl font-bold text-blue-600">
              {metrics.total_tasks}
            </div>
            <div className="text-xs text-gray-500">总计</div>
          </div>
        </div>

        <div className="border-t pt-4">
          <div className="flex justify-between text-sm mb-1">
            <span>平均得分</span>
            <span className="font-medium">
              {Math.round(metrics.avg_score * 100)}%
            </span>
          </div>
          <Progress
            percent={Math.round(metrics.avg_score * 100)}
            showInfo={false}
            strokeColor="#3b82f6"
          />
        </div>

        <div className="border-t pt-4">
          <Typography.Text type="secondary" className="text-xs">
            平均任务耗时
          </Typography.Text>
          <div className="text-lg font-semibold">
            {formatDuration(metrics.avg_duration_ms)}
          </div>
        </div>
      </div>
    </div>
  );
}

function formatDuration(ms: number): string {
  if (ms < 1000) return `${Math.round(ms)}ms`;
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
  return `${(ms / 60000).toFixed(1)}m`;
}
