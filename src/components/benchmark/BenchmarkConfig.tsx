import { useEvaluatorStore } from "@/stores/devtools/evaluatorStore";
import { Select, Slider, Switch } from "antd";

export function BenchmarkConfig() {
  const { config, setConfig } = useEvaluatorStore();

  return (
    <div className="grid grid-cols-2 gap-4">
      <div>
        <label className="block text-sm text-gray-600 mb-2">
          最大并发数: {config.max_concurrency}
        </label>
        <Slider
          min={1}
          max={10}
          value={config.max_concurrency}
          onChange={(value) => setConfig({ max_concurrency: value })}
          marks={{ 1: "1", 5: "5", 10: "10" }}
        />
      </div>

      <div>
        <label className="block text-sm text-gray-600 mb-2">
          超时时间: {config.timeout_ms / 1000}s
        </label>
        <Slider
          min={5000}
          max={120000}
          step={5000}
          value={config.timeout_ms}
          onChange={(value) => setConfig({ timeout_ms: value })}
          marks={{ 5000: "5s", 60000: "60s", 120000: "120s" }}
        />
      </div>

      <div>
        <label className="block text-sm text-gray-600 mb-2">最大难度</label>
        <Select
          className="w-full"
          placeholder="不限制"
          value={config.max_difficulty}
          onChange={(value) => setConfig({ max_difficulty: value })}
          allowClear
          options={[
            { value: "easy", label: "简单" },
            { value: "medium", label: "中等" },
            { value: "hard", label: "困难" },
            { value: "expert", label: "专家" },
          ]}
        />
      </div>

      <div>
        <label className="block text-sm text-gray-600 mb-2">
          包含追踪记录
        </label>
        <Switch
          checked={config.include_traces}
          onChange={(checked) => setConfig({ include_traces: checked })}
        />
      </div>
    </div>
  );
}
