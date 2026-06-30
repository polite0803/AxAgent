import { List } from "@/components/common/AntdList";
import { useStreamStore } from "@/stores";
import type { Citation } from "@/types";
import {
  CheckCircleOutlined,
  FileTextOutlined,
  PauseOutlined,
  PlayCircleOutlined,
  SearchOutlined,
  StopOutlined,
} from "@ant-design/icons";
import { Alert, Button, Card, Divider, Input, Space, Tag, theme, Typography } from "antd";
import { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";
import { CredibilityBadge } from "./CredibilityBadge";
import { ResearchProgress } from "./ResearchProgress";
import { ResearchSources } from "./ResearchSources";
import { getSourceTypeName, type SearchResult } from "./researchUtils";

const { Title, Text, Paragraph } = Typography;
const { TextArea } = Input;

type ResearchPhase =
  | "planning"
  | "searching"
  | "extracting"
  | "analyzing"
  | "synthesizing"
  | "reporting";
type ResearchStatus =
  | "pending"
  | "in_progress"
  | "paused"
  | "completed"
  | "failed";

interface ResearchProgress {
  phase: ResearchPhase;
  percentage: number;
  currentQuery: string | null;
  sourcesFound: number;
  sourcesProcessed: number;
  citationsAdded: number;
  errors: string[];
}

interface ResearchState {
  id: string;
  topic: string;
  status: ResearchStatus;
  currentPhase: ResearchPhase;
  searchResults: SearchResult[];
  citations: Citation[];
  progress: ResearchProgress;
}

interface ResearchReport {
  id: string;
  topic: string;
  content: string;
  citations: Citation[];
  summary: string;
}

interface ResearchPanelProps {
  className?: string;
}

export function ResearchPanel({ className }: ResearchPanelProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const [topic, setTopic] = useState("");
  const [isResearching, setIsResearching] = useState(false);
  const [isPaused, setIsPaused] = useState(false);
  const [state, setState] = useState<ResearchState | null>(null);
  const [report, setReport] = useState<ResearchReport | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const syncCitations = useStreamStore((s) => s.setCitations);

  const startResearch = useCallback(async () => {
    if (!topic.trim()) {
      return;
    }

    setLoading(true);
    setError(null);

    try {
      setIsResearching(true);
      setState({
        id: crypto.randomUUID(),
        topic,
        status: "in_progress",
        currentPhase: "planning",
        searchResults: [],
        citations: [],
        progress: {
          phase: "planning",
          percentage: 10,
          currentQuery: null,
          sourcesFound: 0,
          sourcesProcessed: 0,
          citationsAdded: 0,
          errors: [],
        },
      });

      await new Promise((resolve) => setTimeout(resolve, 500));

      setState((prev) =>
        prev
          ? {
            ...prev,
            currentPhase: "searching",
            progress: {
              ...prev.progress,
              phase: "searching",
              percentage: 30,
            },
          }
          : null
      );

      await new Promise((resolve) => setTimeout(resolve, 800));

      const mockResults: SearchResult[] = [
        {
          id: "1",
          sourceType: "web",
          url: "https://example.com/article",
          title: t("research.mockTitle1"),
          snippet: t("research.mockSnippet1"),
          credibilityScore: 0.7,
          relevanceScore: 0.9,
        },
        {
          id: "2",
          sourceType: "academic",
          url: "https://scholar.google.com/scholar?q=ai",
          title: t("research.mockTitle2"),
          snippet: t("research.mockSnippet2"),
          credibilityScore: 0.9,
          relevanceScore: 0.95,
        },
        {
          id: "3",
          sourceType: "wikipedia",
          url: "https://en.wikipedia.org/wiki/Artificial_intelligence",
          title: "Artificial Intelligence - Wikipedia",
          snippet: "Artificial intelligence (AI) is the intelligence exhibited by machines...",
          credibilityScore: 0.75,
          relevanceScore: 0.85,
        },
      ];

      setState((prev) =>
        prev
          ? {
            ...prev,
            searchResults: mockResults,
            progress: {
              ...prev.progress,
              sourcesFound: mockResults.length,
            },
          }
          : null
      );

      setState((prev) =>
        prev
          ? {
            ...prev,
            currentPhase: "extracting",
            progress: {
              ...prev.progress,
              phase: "extracting",
              percentage: 50,
            },
          }
          : null
      );

      await new Promise((resolve) => setTimeout(resolve, 600));

      const mockCitations: Citation[] = mockResults.map((r, idx) => ({
        id: `citation-${idx}`,
        sourceUrl: r.url,
        sourceTitle: r.title,
        sourceType: r.sourceType as Citation["sourceType"],
        credibility: r.credibilityScore || 0.5,
        inReport: true,
      }));

      setState((prev) =>
        prev
          ? {
            ...prev,
            citations: mockCitations,
            progress: {
              ...prev.progress,
              citationsAdded: mockCitations.length,
            },
          }
          : null
      );

      syncCitations(mockCitations);

      setState((prev) =>
        prev
          ? {
            ...prev,
            currentPhase: "analyzing",
            progress: {
              ...prev.progress,
              phase: "analyzing",
              percentage: 70,
            },
          }
          : null
      );

      await new Promise((resolve) => setTimeout(resolve, 500));

      setState((prev) =>
        prev
          ? {
            ...prev,
            currentPhase: "synthesizing",
            progress: {
              ...prev.progress,
              phase: "synthesizing",
              percentage: 85,
            },
          }
          : null
      );

      await new Promise((resolve) => setTimeout(resolve, 400));

      setState((prev) =>
        prev
          ? {
            ...prev,
            currentPhase: "reporting",
            progress: {
              ...prev.progress,
              phase: "reporting",
              percentage: 95,
            },
          }
          : null
      );

      await new Promise((resolve) => setTimeout(resolve, 600));

      const mockReport: ResearchReport = {
        id: crypto.randomUUID(),
        topic,
        summary: t("research.mockSummary", {
          count: mockCitations.length,
          topic,
        }),
        content: t("research.mockContent", {
          count: mockCitations.length,
          topic,
          findings: mockResults
            .map((r, idx) =>
              t("research.mockFinding", {
                num: idx + 1,
                title: r.title,
                snippet: r.snippet,
              })
            )
            .join("\n"),
        }),
        citations: mockCitations,
      };

      setReport(mockReport);

      setState((prev) =>
        prev
          ? {
            ...prev,
            status: "completed",
            progress: { ...prev.progress, percentage: 100 },
          }
          : null
      );
    } catch (err) {
      setError(
        err instanceof Error ? err.message : t("research.errorOccurred"),
      );
      setState((prev) => (prev ? { ...prev, status: "failed" } : null));
    } finally {
      setLoading(false);
    }
  }, [topic, syncCitations, t]);

  const pauseResearch = useCallback(() => {
    setIsPaused(true);
    setState((prev) => (prev ? { ...prev, status: "paused" } : null));
  }, []);

  const resumeResearch = useCallback(() => {
    setIsPaused(false);
    setState((prev) => (prev ? { ...prev, status: "in_progress" } : null));
  }, []);

  const stopResearch = useCallback(() => {
    setIsResearching(false);
    setIsPaused(false);
    setState((prev) => (prev ? { ...prev, status: "failed" } : null));
  }, []);

  const resetResearch = useCallback(() => {
    setTopic("");
    setState(null);
    setReport(null);
    setError(null);
    setIsResearching(false);
    setIsPaused(false);
  }, []);

  return (
    <Card className={className} style={{ height: "100%", overflow: "auto" }}>
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
          <SearchOutlined size={20} />
          <Title level={4} style={{ margin: 0 }}>
            {t("research.researchAgent")}
          </Title>
        </div>
        {isResearching && (
          <Button
            type="text"
            danger
            icon={<StopOutlined />}
            onClick={stopResearch}
          >
            {t("research.stop")}
          </Button>
        )}
      </div>

      {!isResearching && !state && (
        <div className="research-start">
          <Paragraph type="secondary" className="mb-4">
            {t("research.inputHint")}
          </Paragraph>
          <TextArea
            placeholder={t("research.topicPlaceholder")}
            value={topic}
            onChange={(e) => setTopic(e.target.value)}
            rows={3}
            className="mb-4"
          />
          <Button
            type="primary"
            icon={<SearchOutlined />}
            onClick={startResearch}
            disabled={!topic.trim() || loading}
            loading={loading}
            block
          >
            {t("research.startResearch")}
          </Button>
        </div>
      )}

      {state && (
        <div className="research-progress">
          <div className="mb-4">
            <Text strong>{t("research.topic")}</Text>
            <Paragraph className="mb-2">{state.topic}</Paragraph>
          </div>

          {state.status !== "completed" && state.status !== "failed" && (
            <ResearchProgress
              currentPhase={state.currentPhase}
              percentage={state.progress.percentage}
              currentQuery={state.progress.currentQuery}
            />
          )}

          <Divider />

          {state.status === "in_progress" && (
            <div className="flex gap-2 mb-4">
              <Button
                icon={<PauseOutlined />}
                onClick={pauseResearch}
                disabled={isPaused}
              >
                {t("research.pause")}
              </Button>
            </div>
          )}

          {state.status === "paused" && (
            <div className="flex gap-2 mb-4">
              <Button
                type="primary"
                icon={<PlayCircleOutlined />}
                onClick={resumeResearch}
              >
                {t("research.continue")}
              </Button>
            </div>
          )}

          {error && (
            <Alert
              message={t("research.error")}
              description={error}
              type="error"
              showIcon
              className="mb-4"
            />
          )}

          <Divider />

          {state.searchResults.length > 0 && (
            <div className="sources-section mb-4">
              <Title level={5}>
                {t("research.searchResults")} ({state.searchResults.length})
              </Title>
              <ResearchSources sources={state.searchResults} />
            </div>
          )}

          {state.citations.length > 0 && (
            <div className="citations-section mb-4">
              <Title level={5}>
                {t("research.citations")} ({state.citations.length})
              </Title>
              <List
                size="small"
                dataSource={state.citations}
                renderItem={(item: any) => (
                  <List.Item>
                    <Space>
                      <CheckCircleOutlined
                        style={{ color: item.inReport ? token.colorSuccess : token.colorTextQuaternary }}
                      />
                      <Text>{item.sourceTitle}</Text>
                      <Tag>{getSourceTypeName(item.sourceType, t)}</Tag>
                      <CredibilityBadge score={item.credibility} />
                    </Space>
                  </List.Item>
                )}
              />
            </div>
          )}
        </div>
      )}

      {report && (
        <div className="report-section">
          <Divider />
          <Title level={5}>{t("research.generatedReport")}</Title>

          <div className="flex gap-2 mb-4">
            <Button
              icon={<FileTextOutlined />}
              onClick={() => navigator.clipboard.writeText(report.content)}
            >
              {t("research.copyReport")}
            </Button>
            <Button onClick={resetResearch}>{t("research.startNew")}</Button>
          </div>

          <Card className="report-preview" style={{ background: token.colorFillQuaternary }}>
            <pre
              style={{
                whiteSpace: "pre-wrap",
                fontFamily: "inherit",
                fontSize: "14px",
              }}
            >
              {report.content}
            </pre>
          </Card>
        </div>
      )}
    </Card>
  );
}
