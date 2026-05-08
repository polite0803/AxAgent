import { BenchmarkConfig } from "@/components/benchmark/BenchmarkConfig";
import { BenchmarkReportView } from "@/components/benchmark/BenchmarkReportView";
import { BenchmarkSelector } from "@/components/benchmark/BenchmarkSelector";
import { useEvaluatorStore } from "@/stores/devtools/evaluatorStore";
import { Button, Card, message, Spin } from "antd";
import { useEffect } from "react";
import { useTranslation } from "react-i18next";

export function BenchmarkRunner() {
  const { t } = useTranslation();
  const {
    selectedBenchmark,
    currentResult,
    currentReport,
    isRunning,
    isLoading,
    config,
    loadBenchmarks,
    runBenchmark,
    generateReport,
  } = useEvaluatorStore();

  useEffect(() => {
    loadBenchmarks();
  }, [loadBenchmarks]);

  useEffect(() => {
    if (currentResult && !currentReport) {
      generateReport();
    }
  }, [currentResult, currentReport, generateReport]);

  const handleRunBenchmark = async () => {
    if (!selectedBenchmark) {
      message.warning(t("benchmark.selectFirst", "请先选择一个基准测试"));
      return;
    }

    try {
      await runBenchmark(selectedBenchmark.id, config);
      message.success(t("benchmark.completed", "基准测试完成"));
    } catch (error) {
      message.error(`${t("benchmark.failed", "运行失败")}: ${error}`);
    }
  };

  return (
    <div className="h-full flex flex-col p-6 overflow-auto">
      <div className="flex justify-between items-center mb-6">
        <h2 className="text-xl font-bold">{t("benchmark.title", "基准测试运行器")}</h2>
        <Button
          type="primary"
          onClick={handleRunBenchmark}
          loading={isRunning}
          disabled={!selectedBenchmark}
          size="large"
        >
          {isRunning ? t("benchmark.running", "运行中...") : t("benchmark.run", "运行基准测试")}
        </Button>
      </div>

      <div className="grid grid-cols-4 gap-4 mb-6">
        <Card size="small" title={t("benchmark.selectTitle", "基准测试选择")}>
          <BenchmarkSelector />
        </Card>
        <Card size="small" title={t("benchmark.configTitle", "运行配置")} className="col-span-3">
          <BenchmarkConfig />
        </Card>
      </div>

      {isLoading && !currentResult && (
        <div className="flex items-center justify-center py-20">
          <Spin size="large" tip={t("common.loading", "加载中...")} />
        </div>
      )}

      {currentResult && currentReport && <BenchmarkReportView report={currentReport} />}

      {!currentResult && !isLoading && (
        <div className="text-center text-gray-400 py-20">
          {t("benchmark.empty", "选择基准测试并点击运行...")}
        </div>
      )}
    </div>
  );
}
