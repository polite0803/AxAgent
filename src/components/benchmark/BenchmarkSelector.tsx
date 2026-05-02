import { useEvaluatorStore } from "@/stores/devtools/evaluatorStore";
import type { Benchmark } from "@/types/evaluator";
import { getCategoryLabel } from "@/types/evaluator";
import { Select, Typography } from "antd";

const { Text } = Typography;

interface BenchmarkOption {
  value: string;
  label: string;
  benchmark: Benchmark;
}

export function BenchmarkSelector() {
  const { benchmarks, selectedBenchmark, selectBenchmark } = useEvaluatorStore();

  const options: BenchmarkOption[] = benchmarks.map((b) => ({
    value: b.id,
    label: `${b.name} (${getCategoryLabel(b.category)})`,
    benchmark: b,
  }));

  const handleChange = (value: string) => {
    selectBenchmark(value);
  };

  return (
    <div>
      <Select
        className="w-full"
        placeholder="选择基准测试"
        value={selectedBenchmark?.id}
        onChange={handleChange}
        options={options}
        optionRender={(option) => (
          <div>
            <div>{option.data.label}</div>
            <Text type="secondary" className="text-xs">
              {option.data.benchmark.tasks.length} 个任务
            </Text>
          </div>
        )}
      />

      {selectedBenchmark && (
        <div className="mt-3">
          <Text type="secondary" className="text-sm">
            {selectedBenchmark.description}
          </Text>
          <div className="mt-2">
            <Text className="text-xs">
              标签: {selectedBenchmark.metadata.tags.join(", ") || "无"}
            </Text>
          </div>
        </div>
      )}
    </div>
  );
}
