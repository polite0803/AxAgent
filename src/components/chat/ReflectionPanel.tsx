import { Alert, Badge, Button, Card, Progress, Tag, Typography } from "antd";
import { AlertTriangle, Brain, CheckCircle, Clock, Lightbulb, RefreshCw, Sparkles, TrendingUp } from "lucide-react";
import { useEffect, useState } from "react";

const { Text } = Typography;

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
}

interface Insight {
  id: string;
  category: string;
  title: string;
  content: string;
  confidence: number;
  tags: string[];
  usage_count: number;
  created_at: string;
}

interface ReflectionPanelProps {
  taskId?: string;
  taskDescription?: string;
  onReflectionComplete?: (reflection: ReflectionData) => void;
  initialReflection?: ReflectionData | null;
  isRefecting?: boolean;
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

function QualityScore({ score }: { score: number }) {
  const color = score >= 7 ? "#52c41a" : score >= 4 ? "#faad14" : "#ff4d4f";
  const label = score >= 7 ? "Excellent" : score >= 4 ? "Good" : "Needs Improvement";

  return (
    <div className="flex items-center gap-3">
      <Progress
        type="circle"
        percent={score * 10}
        size={50}
        strokeColor={color}
        format={() => score}
      />
      <div>
        <Text strong style={{ fontSize: 16 }}>
          Quality Score
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
      description={<Text className="text-sm">{content}</Text>}
      className="mb-3"
    />
  );
}

function PatternList({
  patterns,
  type,
}: {
  patterns: string[];
  type: "error" | "success";
}) {
  if (patterns.length === 0) { return null; }

  return (
    <div className="mb-3">
      <Text strong className="mb-2 block">
        {type === "error" ? "Error Patterns" : "Reusable Patterns"}
      </Text>
      <div className="flex flex-wrap gap-2">
        {patterns.map((pattern, idx) => (
          <Tag
            key={idx}
            color={type === "error" ? "red" : "green"}
            icon={type === "error" ? <AlertTriangle size={12} /> : <CheckCircle size={12} />}
          >
            {pattern.length > 50 ? pattern.substring(0, 50) + "..." : pattern}
          </Tag>
        ))}
      </div>
    </div>
  );
}

function InsightCard({ insight }: { insight: Insight }) {
  return (
    <Card size="small" className="insight-card">
      <div className="flex items-start justify-between">
        <div className="flex items-center gap-2">
          {categoryIcons[insight.category] || <Lightbulb size={14} />}
          <Text strong className="text-sm">
            {insight.title}
          </Text>
        </div>
        <Tag color={categoryColors[insight.category] || "default"}>
          {(insight.confidence * 100).toFixed(0)}%
        </Tag>
      </div>
      <Text type="secondary" className="text-xs block mt-1">
        {insight.content.length > 100
          ? insight.content.substring(0, 100) + "..."
          : insight.content}
      </Text>
      <div className="flex items-center gap-2 mt-2">
        {insight.tags.slice(0, 3).map((tag, idx) => (
          <Tag key={idx} className="text-xs">
            {tag}
          </Tag>
        ))}
        {insight.usage_count > 0 && <Badge count={insight.usage_count} size="small" title="Usage count" />}
      </div>
    </Card>
  );
}

export function ReflectionPanel({
  taskId,
  taskDescription,
  onReflectionComplete,
  initialReflection = null,
  isRefecting: initialIsRefecting = false,
}: ReflectionPanelProps) {
  const [isRefecting, setIsRefecting] = useState(initialIsRefecting);
  const [reflection, setReflection] = useState<ReflectionData | null>(initialReflection);
  const [insights, setInsights] = useState<Insight[]>([]);
  const [progress, setProgress] = useState(0);

  useEffect(() => {
    if (isRefecting && progress < 100) {
      const timer = setTimeout(() => {
        setProgress((prev) => {
          const newProgress = prev + 10;
          if (newProgress >= 100) {
            const newReflection: ReflectionData = {
              task_id: taskId || "unknown",
              timestamp: new Date().toISOString(),
              quality_score: 7,
              quality_analysis: "Task completed successfully with good efficiency. Some minor improvements possible.",
              efficiency_analysis: "Total duration: 5000ms. Duration per iteration: 500ms. Execution was efficient.",
              error_patterns: ["Consider adding retry logic for network operations"],
              reusable_patterns: [
                "Successfully completed task with tool combination: search -> analyze -> report",
              ],
              knowledge_suggestions: [
                "Cache intermediate results for similar tasks",
                "Document error handling patterns",
              ],
              improvement_suggestions: [
                "Quality score is good but could be improved with better verification",
                "Consider parallel execution for independent subtasks",
              ],
              overall_summary:
                "Task 'Analysis' succeeded in 5000ms with quality score 7/10. 10 iterations, 3 tools used. 1 error patterns identified. 1 reusable patterns found.",
            };
            setReflection(newReflection);
            setIsRefecting(false);
            onReflectionComplete?.(newReflection);

            const newInsights: Insight[] = [
              {
                id: "1",
                category: "success_pattern",
                title: "Tool Sequence Pattern",
                content: "Successfully completed task with tool combination: search -> analyze -> report",
                confidence: 0.8,
                tags: ["reusable", "workflow"],
                usage_count: 5,
                created_at: new Date().toISOString(),
              },
              {
                id: "2",
                category: "optimization",
                title: "Performance Optimization",
                content: "Task took 5000ms. Consider caching, parallel execution, or algorithm optimization.",
                confidence: 0.7,
                tags: ["performance", "optimization"],
                usage_count: 2,
                created_at: new Date().toISOString(),
              },
            ];
            setInsights(newInsights);
          }
          return newProgress;
        });
      }, 200);

      return () => clearTimeout(timer);
    }
  }, [isRefecting, progress, taskId, onReflectionComplete]);

  const handleStartReflection = () => {
    setIsRefecting(true);
    setReflection(null);
    setProgress(0);
    setInsights([]);
  };

  const handleReset = () => {
    setIsRefecting(false);
    setReflection(null);
    setProgress(0);
    setInsights([]);
  };

  if (!reflection && !isRefecting) {
    return (
      <Card size="small" className="reflection-panel">
        <div className="flex items-center justify-center h-32 text-gray-400">
          <Brain size={24} className="mr-2" />
          <Text type="secondary">No reflection available</Text>
        </div>
        {taskDescription && (
          <div className="mt-4">
            <Button
              type="primary"
              icon={<Brain size={14} />}
              onClick={handleStartReflection}
              block
            >
              Start Reflection
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
            <span>Reflecting...</span>
          </div>
        }
      >
        <div className="flex items-center justify-center h-40">
          <Progress
            type="circle"
            percent={progress}
            size={80}
            strokeColor="#1890ff"
          />
        </div>
        <Text type="secondary" className="block text-center">
          Analyzing task execution...
        </Text>
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
            <span>Reflection</span>
            <Tag color="purple">{taskId || "unknown"}</Tag>
          </div>
          <Button
            type="text"
            size="small"
            icon={<RefreshCw size={14} />}
            onClick={handleReset}
          />
        </div>
      }
    >
      {reflection && (
        <>
          <div className="mb-4">
            <QualityScore score={reflection.quality_score} />
          </div>

          <div className="grid grid-cols-2 gap-4 mb-4">
            <div>
              <Text type="secondary" className="text-xs">
                Error Patterns
              </Text>
              <div className="text-lg font-medium text-red-500">
                {reflection.error_patterns.length}
              </div>
            </div>
            <div>
              <Text type="secondary" className="text-xs">
                Reusable Patterns
              </Text>
              <div className="text-lg font-medium text-green-500">
                {reflection.reusable_patterns.length}
              </div>
            </div>
          </div>

          <AnalysisSection
            title="Quality Analysis"
            icon={<CheckCircle size={14} className="text-green-500" />}
            content={reflection.quality_analysis}
            type="success"
          />

          <AnalysisSection
            title="Efficiency Analysis"
            icon={<Clock size={14} className="text-blue-500" />}
            content={reflection.efficiency_analysis}
            type="info"
          />

          <PatternList patterns={reflection.error_patterns} type="error" />
          <PatternList patterns={reflection.reusable_patterns} type="success" />

          {reflection.improvement_suggestions.length > 0 && (
            <div className="mb-3">
              <Text strong className="mb-2 block">
                Improvement Suggestions
              </Text>
              {reflection.improvement_suggestions.map((suggestion, idx) => (
                <Alert
                  key={idx}
                  type="warning"
                  message={suggestion}
                  className="mb-2"
                />
              ))}
            </div>
          )}

          {insights.length > 0 && (
            <div className="mt-4">
              <div className="flex items-center justify-between mb-2">
                <Text strong>Generated Insights</Text>
                <Badge count={insights.length} />
              </div>
              <div className="space-y-2">
                {insights.map((insight) => <InsightCard key={insight.id} insight={insight} />)}
              </div>
            </div>
          )}

          <Alert
            type="info"
            message="Summary"
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

  const startReflection = (_taskId: string, _taskDescription: string) => {
    setIsRefecting(true);
    setReflection(null);
  };

  const completeReflection = (refl: ReflectionData) => {
    setReflection(refl);
    setIsRefecting(false);
  };

  const reset = () => {
    setReflection(null);
    setIsRefecting(false);
    setInsights([]);
  };

  return {
    reflection,
    isRefecting,
    insights,
    startReflection,
    completeReflection,
    reset,
    ReflectionPanel,
  };
}
