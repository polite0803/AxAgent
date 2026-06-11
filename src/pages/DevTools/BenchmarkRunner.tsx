// SPDX-License-Identifier: AGPL-3.0-only

import { BenchmarkConfig } from "@/components/benchmark/BenchmarkConfig";
import { BenchmarkReportView } from "@/components/benchmark/BenchmarkReportView";
import { BenchmarkSelector } from "@/components/benchmark/BenchmarkSelector";
import { TaskList } from "@/components/benchmark/TaskList";
import { useEvaluatorStore } from "@/stores/devtools/evaluatorStore";
import type { BenchmarkTask, TaskResult } from "@/types";
import { Button, Card, message, Spin, Tabs, theme } from "antd";
import { useEffect } from "react";
import { useTranslation } from "react-i18next";

export function BenchmarkRunner() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
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
      message.warning(t("benchmark.selectFirst"));
      return;
    }

    try {
      await runBenchmark(selectedBenchmark.id, config);
      message.success(t("benchmark.completed"));
    } catch (error) {
      message.error(`${t("benchmark.failed")}: ${error}`);
    }
  };

  return (
    <div className="h-full flex flex-col p-6 overflow-auto">
      <div className="flex justify-between items-center mb-6">
        <h2 className="text-xl font-semibold">{t("benchmark.title")}</h2>
        <Button
          type="primary"
          onClick={handleRunBenchmark}
          loading={isRunning}
          disabled={!selectedBenchmark}
          size="large"
        >
          {isRunning ? t("benchmark.running") : t("benchmark.run")}
        </Button>
      </div>

      <div className="grid grid-cols-4 gap-4 mb-6">
        <Card size="small" title={t("benchmark.selectTitle")}>
          <BenchmarkSelector />
        </Card>
        <Card
          size="small"
          title={t("benchmark.configTitle")}
          className="col-span-3"
        >
          <BenchmarkConfig />
        </Card>
      </div>

      {isLoading && !currentResult && (
        <div className="flex items-center justify-center py-20">
          <Spin size="large" tip={t("common.loading")} />
        </div>
      )}

      {currentResult && currentReport && (
        <Tabs
          items={[
            { key: "report", label: t("benchmark.report"), children: <BenchmarkReportView report={currentReport} /> },
            {
              key: "tasks",
              label: t("benchmark.tasks"),
              children: (
                <TaskList
                  tasks={(currentReport as unknown as { tasks: BenchmarkTask[] }).tasks ?? []}
                  results={(currentReport as unknown as { results: TaskResult[] }).results ?? []}
                />
              ),
            },
          ]}
        />
      )}

      {!currentResult && !isLoading && (
        <div className="text-center py-20" style={{ color: token.colorTextTertiary }}>
          {t("benchmark.empty")}
        </div>
      )}
    </div>
  );
}
