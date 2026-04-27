import type { TraceMetrics } from "@/types/tracer";
import { Card, Progress } from "antd";

interface CostChartProps {
  metrics: TraceMetrics;
}

function formatTokens(tokens: number): string {
  if (tokens < 1000) return `${tokens}`;
  if (tokens < 1000000) return `${(tokens / 1000).toFixed(1)}K`;
  return `${(tokens / 1000000).toFixed(1)}M`;
}

export function CostChart({ metrics }: CostChartProps) {
  const { cost } = metrics;
  const total = cost.total_tokens || 1;

  return (
    <div className="p-4">
      <Card title="Token 使用分布">
        <div className="grid grid-cols-2 gap-4 mb-4">
          <div>
            <div className="text-gray-500 text-sm mb-1">Input Tokens</div>
            <div className="text-2xl font-bold">{formatTokens(cost.input_tokens)}</div>
            <Progress
              percent={(cost.input_tokens / total) * 100}
              showInfo={false}
              strokeColor="#1890ff"
            />
          </div>
          <div>
            <div className="text-gray-500 text-sm mb-1">Output Tokens</div>
            <div className="text-2xl font-bold">{formatTokens(cost.output_tokens)}</div>
            <Progress
              percent={(cost.output_tokens / total) * 100}
              showInfo={false}
              strokeColor="#52c41a"
            />
          </div>
          <div>
            <div className="text-gray-500 text-sm mb-1">Cache Creation</div>
            <div className="text-2xl font-bold">{formatTokens(cost.cache_creation_tokens)}</div>
            <Progress
              percent={(cost.cache_creation_tokens / total) * 100}
              showInfo={false}
              strokeColor="#faad14"
            />
          </div>
          <div>
            <div className="text-gray-500 text-sm mb-1">Cache Read</div>
            <div className="text-2xl font-bold">{formatTokens(cost.cache_read_tokens)}</div>
            <Progress
              percent={(cost.cache_read_tokens / total) * 100}
              showInfo={false}
              strokeColor="#f5222d"
            />
          </div>
        </div>
      </Card>

      <Card title="成本概览" className="mt-4">
        <div className="flex justify-around">
          <div className="text-center">
            <div className="text-gray-500 text-sm mb-1">Total Tokens</div>
            <div className="text-3xl font-bold">{formatTokens(cost.total_tokens)}</div>
          </div>
          <div className="text-center">
            <div className="text-gray-500 text-sm mb-1">Total Cost</div>
            <div className="text-3xl font-bold text-green-600">
              ${cost.total_cost_usd.toFixed(4)}
            </div>
          </div>
          <div className="text-center">
            <div className="text-gray-500 text-sm mb-1">Model</div>
            <div className="text-lg font-bold">{cost.model}</div>
          </div>
        </div>
      </Card>
    </div>
  );
}
