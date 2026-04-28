import { Badge, Button, Card, Progress, Space, Tag, Typography } from "antd";
import {
  BarChart3,
  CheckCircle,
  Clock,
  Loader2,
  Play,
  Terminal,
  XCircle,
  Zap,
} from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text, Title } = Typography;

interface BenchTaskResult {
  task_id: string;
  status: "pending" | "running" | "success" | "failed" | "timeout" | "skipped";
  score: number;
  steps_taken: number;
  output: string | null;
  error: string | null;
}

interface BenchSummary {
  total_tasks: number;
  passed: number;
  failed: number;
  skipped: number;
  timed_out: number;
  pass_rate: number;
  avg_score: number;
  avg_steps: number;
}

interface BenchResult {
  run_id: string;
  benchmark_id: string;
  started_at: number;
  completed_at: number | null;
  task_results: BenchTaskResult[];
  summary: BenchSummary;
}

interface BenchSuiteInfo {
  name: string;
  benchmarks: Array<{
    id: string;
    name: string;
    description: string;
    category: string;
    task_count: number;
  }>;
}

function BenchmarkPanel() {
  const { t } = useTranslation();
  const [suites, setSuites] = useState<BenchSuiteInfo[]>([]);
  const [results, setResults] = useState<BenchResult[]>([]);
  const [running, setRunning] = useState(false);
  const [expandedResult, setExpandedResult] = useState<string | null>(null);

  const fetchSuites = async () => {
    try {
      const { invoke } = await import("@/lib/invoke");
      const data = await invoke<BenchSuiteInfo[]>(
        "benchmark_list_suites"
      ).catch(() => []);
      setSuites(data);
    } catch {
      // ignore
    }
  };

  const runBenchmark = async (benchmarkId: string) => {
    setRunning(true);
    try {
      const { invoke } = await import("@/lib/invoke");
      const result = await invoke<BenchResult>("benchmark_run", {
        benchmarkId,
      });
      setResults((prev) => [result, ...prev]);
      setExpandedResult(result.run_id);
    } catch {
      // ignore
    } finally {
      setRunning(false);
    }
  };

  useEffect(() => {
    fetchSuites();
  }, []);

  const getCategoryIcon = (category: string) => {
    switch (category) {
      case "TerminalOperations":
        return <Terminal size={14} />;
      case "CodeRepair":
        return <Zap size={14} />;
      case "CodeGeneration":
        return <BarChart3 size={14} />;
      default:
        return <BarChart3 size={14} />;
    }
  };

  return (
    <Card size="small" className="benchmark-panel">
      <div className="flex items-center justify-between mb-3">
        <Space>
          <BarChart3 size={16} className="text-orange-500" />
          <Title level={5} className="mb-0">
            {t("chat.benchmarks.title")}
          </Title>
        </Space>
        <Button size="small" onClick={fetchSuites}>
          {t("chat.benchmarks.refresh")}
        </Button>
      </div>

      {/* Benchmark suites */}
      {suites.map((suite) => (
        <div key={suite.name} className="mb-4">
          <Text strong className="text-sm block mb-2">
            {suite.name}
          </Text>
          <div className="space-y-2">
            {suite.benchmarks.map((bench) => (
              <Card key={bench.id} size="small" className="bench-card">
                <div className="flex items-center justify-between">
                  <Space>
                    {getCategoryIcon(bench.category)}
                    <div>
                      <Text strong className="text-sm block">
                        {bench.name}
                      </Text>
                      <Text type="secondary" className="text-xs">
                        {bench.description}
                      </Text>
                    </div>
                  </Space>
                  <Space>
                    <Tag color="blue" className="text-xs">
                      {bench.category}
                    </Tag>
                    <Text type="secondary" className="text-xs">
                      {bench.task_count} {t("chat.benchmarks.tasks")}
                    </Text>
                    <Button
                      size="small"
                      type="primary"
                      icon={running ? <Loader2 size={12} className="animate-spin" /> : <Play size={12} />}
                      loading={running}
                      onClick={() => runBenchmark(bench.id)}
                    >
                      {t("chat.benchmarks.run")}
                    </Button>
                  </Space>
                </div>
              </Card>
            ))}
          </div>
        </div>
      ))}

      {/* Results */}
      {results.length > 0 && (
        <div className="mt-4">
          <Text strong className="text-sm block mb-2">
            {t("chat.benchmarks.results")}
          </Text>
          {results.map((result) => (
            <Card
              key={result.run_id}
              size="small"
              className={expandedResult === result.run_id ? "border-blue-200" : ""}
            >
              <div
                className="flex items-center justify-between cursor-pointer"
                onClick={() =>
                  setExpandedResult(
                    expandedResult === result.run_id ? null : result.run_id
                  )
                }
              >
                <Space>
                  <BarChart3 size={14} className="text-blue-500" />
                  <Text strong className="text-sm">
                    {result.benchmark_id}
                  </Text>
                  <Tag color={result.summary.pass_rate > 0.8 ? "success" : "warning"}>
                    {Math.round(result.summary.pass_rate * 100)}%
                  </Tag>
                </Space>
                <Space size="small">
                  <Badge status="success" text={<Text type="secondary" className="text-xs">{result.summary.passed}</Text>} />
                  <Badge status="error" text={<Text type="secondary" className="text-xs">{result.summary.failed}</Text>} />
                </Space>
              </div>

              {expandedResult === result.run_id && (
                <div className="mt-2 pt-2 border-t border-gray-100 dark:border-gray-800">
                  <div className="grid grid-cols-2 gap-2 mb-2">
                    <Card size="small" className="bg-green-50 dark:bg-green-900/10 text-center">
                      <Text className="text-lg font-bold text-green-600 block">
                        {result.summary.passed}
                      </Text>
                      <Text type="secondary" className="text-xs">{t("chat.benchmarks.passed")}</Text>
                    </Card>
                    <Card size="small" className="bg-red-50 dark:bg-red-900/10 text-center">
                      <Text className="text-lg font-bold text-red-600 block">
                        {result.summary.failed}
                      </Text>
                      <Text type="secondary" className="text-xs">{t("chat.benchmarks.failed")}</Text>
                    </Card>
                  </div>

                  <Progress
                    percent={Math.round(result.summary.pass_rate * 100)}
                    strokeColor="#52c41a"
                    size="small"
                  />

                  <div className="mt-2 space-y-1 max-h-48 overflow-auto">
                    {result.task_results.map((task) => (
                      <div
                        key={task.task_id}
                        className="flex items-center gap-2 px-2 py-1 rounded text-xs"
                      >
                        {task.status === "success" ? (
                          <CheckCircle size={10} className="text-green-500 shrink-0" />
                        ) : task.status === "failed" || task.status === "timeout" ? (
                          <XCircle size={10} className="text-red-500 shrink-0" />
                        ) : (
                          <Clock size={10} className="text-gray-400 shrink-0" />
                        )}
                        <Text className="flex-1 truncate">{task.task_id}</Text>
                        <Text type="secondary">
                          {task.steps_taken}s
                        </Text>
                        <Tag
                          color={
                            task.score > 0.8
                              ? "success"
                              : task.score > 0.4
                                ? "warning"
                                : "error"
                          }
                          className="text-xs"
                        >
                          {Math.round(task.score * 100)}%
                        </Tag>
                      </div>
                    ))}
                  </div>
                </div>
              )}
            </Card>
          ))}
        </div>
      )}
    </Card>
  );
}

export default BenchmarkPanel;
