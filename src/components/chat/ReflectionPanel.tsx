import { invoke } from "@/lib/invoke";
import { Alert, Badge, Button, Card, Progress, Tag, Tooltip, Typography } from "antd";
import { AlertTriangle, Brain, CheckCircle, Clock, Lightbulb, RefreshCw, Sparkles, TrendingUp } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

interface QualityMetricsData {
  task_success_score: number;
  tool_efficiency_score: number;
  iteration_efficiency_score: number;
  time_efficiency_score: number;
  error_recovery_score: number;
  goal_completion_score: number;
  overall_weighted_score: number;
}

interface ReflectionData {
  task_id: string;
  timestamp: string;
  quality_score: number;
  quality_analysis: string;
  efficiency_analysis: string;
  error_patterns: string[];
  reusable_patterns: string[];
  knowledge_suggestions: string[];
  improvement_suggestions: string[];
  overall_summary: string;
  quality_metrics: QualityMetricsData | null;
}

interface Insight {
  id: string;
  category: string;
  title: string;
  content: string;
  source_task_id: string;
  confidence: number;
  tags: string[];
  created_at: string;
  usage_count: number;
  last_used: string | null;
}

interface ReflectionPanelProps {
  taskId?: string;
  taskDescription?: string;
  onReflectionComplete?: (reflection: ReflectionData) => void;
  executionRecord?: {
    success: boolean;
    error?: string;
    toolsUsed: string[];
    iterations: number;
    durationMs: number;
  };
}

const categoryIcons: Record<string, React.ReactNode> = {
  error_pattern: <AlertTriangle size={14} className="text-red-500" />,
  success_pattern: <CheckCircle size={14} className="text-green-500" />,
  optimization: <TrendingUp size={14} className="text-blue-500" />,
  knowledge: <Lightbulb size={14} className="text-yellow-500" />,
  workflow: <RefreshCw size={14} className="text-purple-500" />,
  tool_usage: <Sparkles size={14} className="text-orange-500" />,
};

const categoryColors: Record<string, string> = {
  error_pattern: "red",
  success_pattern: "green",
  optimization: "blue",
  knowledge: "gold",
  workflow: "purple",
  tool_usage: "orange",
};

function QualityScore({ score, t }: { score: number; t: (key: string) => string }) {
  const color = score >= 7 ? "#52c41a" : score >= 4 ? "#faad14" : "#ff4d4f";
  const label = score >= 7
    ? t("reflection.excellent")
    : score >= 4
    ? t("reflection.good")
    : t("reflection.needsImprovement");

  return (
    <div className="flex items-center gap-3">
      <Progress type="circle" percent={score * 10} size={50} strokeColor={color} format={() => score} />
      <div>
        <Text strong style={{ fontSize: 16 }}>
          {t("reflection.qualityScore")}
        </Text>
        <div>
          <Tag color={score >= 7 ? "green" : score >= 4 ? "gold" : "red"}>{label}</Tag>
        </div>
      </div>
    </div>
  );
}

function AnalysisSection({
  title,
  icon,
  content,
  type,
}: {
  title: string;
  icon: React.ReactNode;
  content: string;
  type: "success" | "warning" | "info";
}) {
  return (
    <Alert
      type={type}
      message={
        <div className="flex items-center gap-2">
          {icon}
          <Text strong>{title}</Text>
        </div>
      }
      description={<Text className="text-sm whitespace-pre-line">{content}</Text>}
      className="mb-3"
    />
  );
}

function PatternList({
  patterns,
  type,
  t,
}: {
  patterns: string[];
  type: "error" | "success";
  t: (key: string) => string;
}) {
  if (patterns.length === 0) {
    return null;
  }

  return (
    <div className="mb-3">
      <Text strong className="mb-2 block">
        {type === "error" ? t("reflection.errorPatterns") : t("reflection.reusablePatterns")}
      </Text>
      <div className="flex flex-wrap gap-2">
        {patterns.map((pattern) => (
          <Tooltip key={pattern} title={pattern.length > 50 ? pattern : undefined}>
            <Tag
              color={type === "error" ? "red" : "green"}
              icon={type === "error" ? <AlertTriangle size={12} /> : <CheckCircle size={12} />}
            >
              {pattern.length > 50 ? pattern.substring(0, 50) + "..." : pattern}
            </Tag>
          </Tooltip>
        ))}
      </div>
    </div>
  );
}

function InsightCard({ insight }: { insight: Insight }) {
  const { t } = useTranslation();
  return (
    <Card size="small" className="insight-card">
      <div className="flex items-start justify-between">
        <div className="flex items-center gap-2">
          {categoryIcons[insight.category] || <Lightbulb size={14} />}
          <Text strong className="text-sm">
            {insight.title}
          </Text>
        </div>
        <Tag color={categoryColors[insight.category] || "default"}>{(insight.confidence * 100).toFixed(0)}%</Tag>
      </div>
      <Text type="secondary" className="text-xs block mt-1">
        {insight.content.length > 100 ? insight.content.substring(0, 100) + "..." : insight.content}
      </Text>
      <div className="flex items-center gap-2 mt-2">
        {insight.tags.slice(0, 3).map((tag) => (
          <Tag key={tag} className="text-xs">
            {tag}
          </Tag>
        ))}
        {insight.usage_count > 0 && (
          <Badge count={insight.usage_count} size="small" title={t("reflection.usageCount")} />
        )}
      </div>
    </Card>
  );
}

function QualityMetricsBreakdown({ metrics, t }: { metrics: QualityMetricsData; t: (key: string) => string }) {
  const dimensions = [
    { key: "taskSuccessScore", value: metrics.task_success_score, color: "#1890ff" },
    { key: "toolEfficiencyScore", value: metrics.tool_efficiency_score, color: "#52c41a" },
    { key: "iterationEfficiencyScore", value: metrics.iteration_efficiency_score, color: "#722ed1" },
    { key: "timeEfficiencyScore", value: metrics.time_efficiency_score, color: "#fa8c16" },
    { key: "errorRecoveryScore", value: metrics.error_recovery_score, color: "#eb2f96" },
    { key: "goalCompletionScore", value: metrics.goal_completion_score, color: "#13c2c2" },
  ];

  return (
    <div className="mb-4">
      <Text strong className="mb-2 block">
        {t("reflection.qualityMetrics")}
      </Text>
      <div className="grid grid-cols-1 gap-2">
        {dimensions.map((dim) => (
          <div key={dim.key} className="flex items-center gap-2">
            <Text className="text-xs w-24 flex-shrink-0">{t(`reflection.${dim.key}`)}</Text>
            <Progress
              percent={(dim.value / 10) * 100}
              size="small"
              strokeColor={dim.color}
              format={() => dim.value.toFixed(1)}
              className="flex-1"
            />
          </div>
        ))}
      </div>
    </div>
  );
}

export function ReflectionPanel({
  taskId,
  taskDescription,
  onReflectionComplete,
  executionRecord,
}: ReflectionPanelProps) {
  const { t } = useTranslation();
  const [isRefecting, setIsRefecting] = useState(false);
  const [reflection, setReflection] = useState<ReflectionData | null>(null);
  const [insights, setInsights] = useState<Insight[]>([]);
  const [error, setError] = useState<string | null>(null);
  const reflectedTaskId = useRef<string | null>(null);

  const performReflection = useCallback(async () => {
    if (!executionRecord || !taskId) {
      return;
    }

    if (reflectedTaskId.current === taskId) {
      return;
    }
    reflectedTaskId.current = taskId;

    setIsRefecting(true);
    setReflection(null);
    setError(null);

    try {
      const result = await invoke<ReflectionData>("reflect_on_task", {
        taskId,
        taskDescription: taskDescription || "",
        success: executionRecord.success,
        error: executionRecord.error || null,
        toolsUsed: executionRecord.toolsUsed,
        iterations: executionRecord.iterations,
        durationMs: executionRecord.durationMs,
      });

      setReflection(result);
      onReflectionComplete?.(result);

      try {
        const fetchedInsights = await invoke<Insight[]>("get_reflection_insights", { category: null });
        setInsights(fetchedInsights.slice(-10));
      } catch {
        // insights fetch is non-critical
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setIsRefecting(false);
    }
  }, [executionRecord, taskId, taskDescription, onReflectionComplete]);

  useEffect(() => {
    if (executionRecord && taskId) {
      performReflection();
    }
  }, [performReflection]);

  const handleStartReflection = () => {
    reflectedTaskId.current = null;
    performReflection();
  };

  const handleReset = () => {
    setIsRefecting(false);
    setReflection(null);
    setError(null);
    setInsights([]);
    reflectedTaskId.current = null;
  };

  if (error) {
    return (
      <Card size="small" className="reflection-panel">
        <Alert type="error" message={t("reflection.reflectError")} description={error} />
        <Button type="link" icon={<RefreshCw size={14} />} onClick={handleReset} className="mt-2">
          {t("reflection.retry")}
        </Button>
      </Card>
    );
  }

  if (!reflection && !isRefecting) {
    return (
      <Card size="small" className="reflection-panel">
        <div className="flex items-center justify-center h-32 text-gray-400">
          <Brain size={24} className="mr-2" />
          <Text type="secondary">{t("reflection.noReflection")}</Text>
        </div>
        {(taskDescription || taskId) && (
          <div className="mt-4">
            <Button type="primary" icon={<Brain size={14} />} onClick={handleStartReflection} block>
              {t("reflection.startReflection")}
            </Button>
          </div>
        )}
      </Card>
    );
  }

  if (isRefecting && !reflection) {
    return (
      <Card
        size="small"
        className="reflection-panel"
        title={
          <div className="flex items-center gap-2">
            <Brain size={16} className="text-blue-500 animate-pulse" />
            <span>{t("reflection.reflecting")}</span>
          </div>
        }
      >
        <div className="flex items-center justify-center h-40 flex-col gap-4">
          <Brain size={48} className="text-blue-400 animate-pulse" />
          <Text type="secondary">{t("reflection.analyzing")}</Text>
        </div>
      </Card>
    );
  }

  return (
    <Card
      size="small"
      className="reflection-panel"
      title={
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Brain size={16} className="text-purple-500" />
            <span>{t("reflection.title")}</span>
            <Tag color="purple">{reflection?.task_id || taskId || "unknown"}</Tag>
          </div>
          <Button type="text" size="small" icon={<RefreshCw size={14} />} onClick={handleReset} />
        </div>
      }
    >
      {reflection && (
        <>
          <div className="mb-4">
            <QualityScore score={reflection.quality_score} t={t} />
          </div>

          {reflection.quality_metrics && <QualityMetricsBreakdown metrics={reflection.quality_metrics} t={t} />}

          <div className="grid grid-cols-2 gap-4 mb-4">
            <div>
              <Text type="secondary" className="text-xs">
                {t("reflection.errorPatterns")}
              </Text>
              <div className="text-lg font-medium text-red-500">{reflection.error_patterns.length}</div>
            </div>
            <div>
              <Text type="secondary" className="text-xs">
                {t("reflection.reusablePatterns")}
              </Text>
              <div className="text-lg font-medium text-green-500">{reflection.reusable_patterns.length}</div>
            </div>
          </div>

          <AnalysisSection
            title={t("reflection.qualityAnalysis")}
            icon={<CheckCircle size={14} className="text-green-500" />}
            content={reflection.quality_analysis}
            type="success"
          />

          <AnalysisSection
            title={t("reflection.efficiencyAnalysis")}
            icon={<Clock size={14} className="text-blue-500" />}
            content={reflection.efficiency_analysis}
            type="info"
          />

          <PatternList patterns={reflection.error_patterns} type="error" t={t} />
          <PatternList patterns={reflection.reusable_patterns} type="success" t={t} />

          {reflection.knowledge_suggestions.length > 0 && (
            <div className="mb-3">
              <Text strong className="mb-2 block">
                {t("reflection.knowledgeSuggestions")}
              </Text>
              {reflection.knowledge_suggestions.map((suggestion) => (
                <Alert
                  key={suggestion}
                  type="info"
                  message={suggestion}
                  className="mb-2"
                  icon={<Lightbulb size={14} />}
                />
              ))}
            </div>
          )}

          {reflection.improvement_suggestions.length > 0 && (
            <div className="mb-3">
              <Text strong className="mb-2 block">
                {t("reflection.improvementSuggestions")}
              </Text>
              {reflection.improvement_suggestions.map((suggestion) => (
                <Alert key={suggestion} type="warning" message={suggestion} className="mb-2" />
              ))}
            </div>
          )}

          {insights.length > 0 && (
            <div className="mt-4">
              <div className="flex items-center justify-between mb-2">
                <Text strong>{t("reflection.generatedInsights")}</Text>
                <Badge count={insights.length} />
              </div>
              <div className="space-y-2">
                {insights.map((insight) => <InsightCard key={insight.id} insight={insight} />)}
              </div>
            </div>
          )}

          <Alert
            type="info"
            message={t("reflection.summary")}
            description={reflection.overall_summary}
            className="mt-4"
          />
        </>
      )}
    </Card>
  );
}

export function useReflection() {
  const [reflection, setReflection] = useState<ReflectionData | null>(null);
  const [isRefecting, setIsRefecting] = useState(false);
  const [insights, setInsights] = useState<Insight[]>([]);
  const [error, setError] = useState<string | null>(null);

  const startReflection = useCallback(
    async (params: {
      taskId: string;
      taskDescription: string;
      success: boolean;
      error?: string;
      toolsUsed: string[];
      iterations: number;
      durationMs: number;
    }) => {
      setIsRefecting(true);
      setReflection(null);
      setError(null);

      try {
        const result = await invoke<ReflectionData>("reflect_on_task", {
          task_id: params.taskId,
          task_description: params.taskDescription,
          success: params.success,
          error: params.error || null,
          tools_used: params.toolsUsed,
          iterations: params.iterations,
          duration_ms: params.durationMs,
        });

        setReflection(result);

        try {
          const fetchedInsights = await invoke<Insight[]>("get_reflection_insights", { category: null });
          setInsights(fetchedInsights.slice(-10));
        } catch {
          // non-critical
        }

        return result;
      } catch (e) {
        setError(String(e));
        throw e;
      } finally {
        setIsRefecting(false);
      }
    },
    [],
  );

  const reset = useCallback(() => {
    setReflection(null);
    setIsRefecting(false);
    setInsights([]);
    setError(null);
  }, []);

  return {
    reflection,
    isRefecting,
    insights,
    error,
    startReflection,
    reset,
  };
}
