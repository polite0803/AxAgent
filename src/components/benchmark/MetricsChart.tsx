import { useMemo } from "react";
import { Card, Typography } from "antd";
import type { TaskResult, BenchmarkReport } from "@/types/evaluator";

interface MetricsChartProps {
  results: TaskResult[];
  report?: BenchmarkReport;
}

export function MetricsChart({ results, report }: MetricsChartProps) {
  const chartData = useMemo(() => {
    return results.map((result, index) => ({
      name: result.task_name.length > 20
        ? result.task_name.substring(0, 20) + "..."
        : result.task_name,
      score: Math.round(result.overall_score * 100),
      duration: Math.round(result.duration_ms / 100) / 10,
      success: result.success,
      index,
    }));
  }, [results]);

  const maxScore = Math.max(...chartData.map((d) => d.score), 100);

  if (chartData.length === 0) {
    return (
      <Card className="h-full flex items-center justify-center">
        <Typography.Text type="secondary">暂无数据</Typography.Text>
      </Card>
    );
  }

  return (
    <Card className="h-full">
      <Typography.Title level={5} className="mb-4">
        任务得分对比
      </Typography.Title>

      <div className="space-y-3">
        {chartData.map((item) => (
          <div key={item.index} className="relative">
            <div className="flex items-center justify-between mb-1">
              <Typography.Text
                className="text-sm truncate flex-1"
                type={item.success ? undefined : "danger"}
              >
                {item.name}
              </Typography.Text>
              <span className="text-xs text-gray-500 ml-2">
                {item.score}% | {item.duration}s
              </span>
            </div>
            <div className="h-2 bg-gray-100 rounded-full overflow-hidden">
              <div
                className={`h-full rounded-full transition-all ${
                  item.success ? "bg-green-500" : "bg-red-500"
                }`}
                style={{ width: `${(item.score / maxScore) * 100}%` }}
              />
            </div>
          </div>
        ))}
      </div>

      {report && (
        <div className="mt-6 border-t pt-4">
          <Typography.Title level={5} className="mb-3">
            评分分布
          </Typography.Title>
          <div className="space-y-2">
            {Object.entries(report.category_scores).map(([category, score]) => (
              <div key={category} className="flex justify-between text-sm">
                <span className="capitalize">{category}</span>
                <span className="font-medium">{Math.round(score * 100)}%</span>
              </div>
            ))}
          </div>
        </div>
      )}
    </Card>
  );
}
