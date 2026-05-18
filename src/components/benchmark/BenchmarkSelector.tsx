import { useEvaluatorStore } from "@/stores/devtools/evaluatorStore";
import type { Benchmark } from "@/types";
import { getCategoryKey } from "@/types";
import { Select, Typography } from "antd";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

interface BenchmarkOption {
  value: string;
  label: string;
  benchmark: Benchmark;
}

export function BenchmarkSelector() {
  const { benchmarks, selectedBenchmark, selectBenchmark } = useEvaluatorStore();
  const { t } = useTranslation();

  const options: BenchmarkOption[] = benchmarks.map((b) => ({
    value: b.id,
    label: `${b.name} (${t(getCategoryKey(b.category))})`,
    benchmark: b,
  }));

  const handleChange = (value: string) => {
    selectBenchmark(value);
  };

  return (
    <div>
      <Select
        className="w-full"
        placeholder={t("benchmark.selectBenchmark")}
        value={selectedBenchmark?.id}
        onChange={handleChange}
        options={options}
        optionRender={(option) => (
          <div>
            <div>{option.data.label}</div>
            <Text type="secondary" className="text-xs">
              {t("benchmark.taskCount", { count: option.data.benchmark.tasks.length })}
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
              {t("benchmark.tags", { tags: selectedBenchmark.metadata.tags.join(", ") || t("benchmark.none") })}
            </Text>
          </div>
        </div>
      )}
    </div>
  );
}
