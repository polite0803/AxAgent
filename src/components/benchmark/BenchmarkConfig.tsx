import { useEvaluatorStore } from "@/stores/devtools/evaluatorStore";
import { Select, Slider, Switch } from "antd";
import { useTranslation } from "react-i18next";

export function BenchmarkConfig() {
  const { config, setConfig } = useEvaluatorStore();
  const { t } = useTranslation();

  return (
    <div className="grid grid-cols-2 gap-4">
      <div>
        <label className="block text-sm text-gray-600 mb-2">
          {t("benchmark.maxConcurrency", { value: config.max_concurrency })}
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
          {t("benchmark.timeoutLabel", { seconds: config.timeout_ms / 1000 })}
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
        <label className="block text-sm text-gray-600 mb-2">{t("benchmark.maxDifficulty")}</label>
        <Select
          className="w-full"
          placeholder={t("benchmark.noLimit")}
          value={config.max_difficulty}
          onChange={(value) => setConfig({ max_difficulty: value })}
          allowClear
          options={[
            { value: "easy", label: t("benchmark.difficultyEasy") },
            { value: "medium", label: t("benchmark.difficultyMedium") },
            { value: "hard", label: t("benchmark.difficultyHard") },
            { value: "expert", label: t("benchmark.difficultyExpert") },
          ]}
        />
      </div>

      <div>
        <label className="block text-sm text-gray-600 mb-2">
          {t("benchmark.includeTraces")}
        </label>
        <Switch
          checked={config.include_traces}
          onChange={(checked) => setConfig({ include_traces: checked })}
        />
      </div>
    </div>
  );
}
