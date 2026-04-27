import React, { useState, useCallback } from 'react';
import { Card, Button, Progress, List, Tag, Space, Typography, Input, Alert, Divider } from 'antd';
import {
  SearchOutlined,
  PauseOutlined,
  PlayCircleOutlined,
  StopOutlined,
  FileTextOutlined,
  LinkOutlined,
  CheckCircleOutlined,
  ClockCircleOutlined,
  StarOutlined,
} from '@ant-design/icons';

const { Title, Text, Paragraph } = Typography;
const { TextArea } = Input;

type ResearchPhase = 'planning' | 'searching' | 'extracting' | 'analyzing' | 'synthesizing' | 'reporting';
type ResearchStatus = 'pending' | 'in_progress' | 'paused' | 'completed' | 'failed';

interface SearchResult {
  id: string;
  sourceType: string;
  url: string;
  title: string;
  snippet: string;
  credibilityScore: number | null;
  relevanceScore: number;
}

interface Citation {
  id: string;
  sourceUrl: string;
  sourceTitle: string;
  sourceType: string;
  credibility: number;
  inReport: boolean;
}

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

const phaseSteps: { key: ResearchPhase; label: string; icon: React.ReactNode }[] = [
  { key: 'planning', label: '规划', icon: <ClockCircleOutlined /> },
  { key: 'searching', label: '搜索', icon: <SearchOutlined /> },
  { key: 'extracting', label: '提取', icon: <LinkOutlined /> },
  { key: 'analyzing', label: '分析', icon: <StarOutlined /> },
  { key: 'synthesizing', label: '综合', icon: <CheckCircleOutlined /> },
  { key: 'reporting', label: '报告', icon: <FileTextOutlined /> },
];

function getSourceTypeColor(sourceType: string): string {
  const colorMap: Record<string, string> = {
    web: 'blue',
    academic: 'green',
    wikipedia: 'cyan',
    github: 'purple',
    documentation: 'orange',
    news: 'magenta',
    blog: 'gold',
    forum: 'default',
    unknown: 'default',
  };
  return colorMap[sourceType.toLowerCase()] || 'default';
}

function getSourceTypeName(sourceType: string): string {
  const nameMap: Record<string, string> = {
    web: '网页',
    academic: '学术',
    wikipedia: '维基百科',
    github: 'GitHub',
    documentation: '文档',
    news: '新闻',
    blog: '博客',
    forum: '论坛',
    unknown: '未知',
  };
  return nameMap[sourceType.toLowerCase()] || sourceType;
}

function CredibilityBadge({ score }: { score: number }) {
  if (score >= 0.8) {
    return <Tag color="green">高可信度</Tag>;
  } else if (score >= 0.5) {
    return <Tag color="orange">中可信度</Tag>;
  } else {
    return <Tag color="red">低可信度</Tag>;
  }
}

function PhaseProgress({ currentPhase, percentage }: { currentPhase: ResearchPhase; percentage: number }) {
  const currentIndex = phaseSteps.findIndex((p) => p.key === currentPhase);

  return (
    <div className="phase-progress">
      <div className="flex items-center justify-between mb-2">
        {phaseSteps.map((step, index) => {
          const isCompleted = index < currentIndex;
          const isCurrent = index === currentIndex;
          return (
            <div
              key={step.key}
              className={`flex flex-col items-center ${isCompleted ? 'text-green-500' : isCurrent ? 'text-blue-500' : 'text-gray-400'}`}
            >
              <div
                className={`w-8 h-8 rounded-full flex items-center justify-center ${
                  isCompleted ? 'bg-green-500 text-white' : isCurrent ? 'bg-blue-500 text-white' : 'bg-gray-200'
                }`}
              >
                {step.icon}
              </div>
              <Text className="text-xs mt-1">{step.label}</Text>
            </div>
          );
        })}
      </div>
      <Progress percent={percentage} showInfo={false} strokeColor="#1890ff" />
    </div>
  );
}

export function ResearchPanel({ className }: ResearchPanelProps) {
  const [topic, setTopic] = useState('');
  const [isResearching, setIsResearching] = useState(false);
  const [isPaused, setIsPaused] = useState(false);
  const [state, setState] = useState<ResearchState | null>(null);
  const [report, setReport] = useState<ResearchReport | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const startResearch = useCallback(async () => {
    if (!topic.trim()) return;

    setLoading(true);
    setError(null);

    try {
      setIsResearching(true);
      setState({
        id: crypto.randomUUID(),
        topic,
        status: 'in_progress',
        currentPhase: 'planning',
        searchResults: [],
        citations: [],
        progress: {
          phase: 'planning',
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
              currentPhase: 'searching',
              progress: { ...prev.progress, phase: 'searching', percentage: 30 },
            }
          : null
      );

      await new Promise((resolve) => setTimeout(resolve, 800));

      const mockResults: SearchResult[] = [
        {
          id: '1',
          sourceType: 'web',
          url: 'https://example.com/article',
          title: '关于人工智能的最新研究',
          snippet: '本文探讨了人工智能技术的发展现状和未来趋势...',
          credibilityScore: 0.7,
          relevanceScore: 0.9,
        },
        {
          id: '2',
          sourceType: 'academic',
          url: 'https://scholar.google.com/scholar?q=ai',
          title: '深度学习研究综述',
          snippet: '本文综述了深度学习在各个领域的应用和研究进展...',
          credibilityScore: 0.9,
          relevanceScore: 0.95,
        },
        {
          id: '3',
          sourceType: 'wikipedia',
          url: 'https://en.wikipedia.org/wiki/Artificial_intelligence',
          title: 'Artificial Intelligence - Wikipedia',
          snippet: 'Artificial intelligence (AI) is the intelligence exhibited by machines...',
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
              currentPhase: 'extracting',
              progress: { ...prev.progress, phase: 'extracting', percentage: 50 },
            }
          : null
      );

      await new Promise((resolve) => setTimeout(resolve, 600));

      const mockCitations: Citation[] = mockResults.map((r, idx) => ({
        id: `citation-${idx}`,
        sourceUrl: r.url,
        sourceTitle: r.title,
        sourceType: r.sourceType,
        credibility: r.credibilityScore || 0.5,
        inReport: true,
      }));

      setState((prev) =>
        prev
          ? {
              ...prev,
              citations: mockCitations,
              progress: { ...prev.progress, citationsAdded: mockCitations.length },
            }
          : null
      );

      setState((prev) =>
        prev
          ? {
              ...prev,
              currentPhase: 'analyzing',
              progress: { ...prev.progress, phase: 'analyzing', percentage: 70 },
            }
          : null
      );

      await new Promise((resolve) => setTimeout(resolve, 500));

      setState((prev) =>
        prev
          ? {
              ...prev,
              currentPhase: 'synthesizing',
              progress: { ...prev.progress, phase: 'synthesizing', percentage: 85 },
            }
          : null
      );

      await new Promise((resolve) => setTimeout(resolve, 400));

      setState((prev) =>
        prev
          ? {
              ...prev,
              currentPhase: 'reporting',
              progress: { ...prev.progress, phase: 'reporting', percentage: 95 },
            }
          : null
      );

      await new Promise((resolve) => setTimeout(resolve, 600));

      const mockReport: ResearchReport = {
        id: crypto.randomUUID(),
        topic,
        summary: `本报告基于 ${mockCitations.length} 个来源，对「${topic}」进行了深入分析。`,
        content: `# 关于「${topic}」的研究报告

## 摘要

本报告基于对 ${mockCitations.length} 个来源的研究，对「${topic}」进行了深入分析。

## 主要发现

${mockResults.map((r, idx) => `### 发现 ${idx + 1}: ${r.title}\n\n${r.snippet}\n`).join('\n')}

## 结论

通过对 ${mockCitations.length} 个来源的深入研究和分析，我们对「${topic}」有了更全面的认识。

## 参考文献

${mockCitations.map((c, idx) => `[${idx + 1}] ${c.sourceTitle} - ${c.sourceUrl}`).join('\n')}
`,
        citations: mockCitations,
      };

      setReport(mockReport);

      setState((prev) =>
        prev
          ? {
              ...prev,
              status: 'completed',
              progress: { ...prev.progress, percentage: 100 },
            }
          : null
      );
    } catch (err) {
      setError(err instanceof Error ? err.message : '研究过程出错');
      setState((prev) =>
        prev ? { ...prev, status: 'failed' } : null
      );
    } finally {
      setLoading(false);
    }
  }, [topic]);

  const pauseResearch = useCallback(() => {
    setIsPaused(true);
    setState((prev) =>
      prev ? { ...prev, status: 'paused' } : null
    );
  }, []);

  const resumeResearch = useCallback(() => {
    setIsPaused(false);
    setState((prev) =>
      prev ? { ...prev, status: 'in_progress' } : null
    );
  }, []);

  const stopResearch = useCallback(() => {
    setIsResearching(false);
    setIsPaused(false);
    setState((prev) =>
      prev ? { ...prev, status: 'failed' } : null
    );
  }, []);

  const resetResearch = useCallback(() => {
    setTopic('');
    setState(null);
    setReport(null);
    setError(null);
    setIsResearching(false);
    setIsPaused(false);
  }, []);

  return (
    <Card className={className} style={{ height: '100%', overflow: 'auto' }}>
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
          <SearchOutlined size={20} />
          <Title level={4} style={{ margin: 0 }}>
            研究型 Agent
          </Title>
        </div>
        {isResearching && (
          <Button
            type="text"
            danger
            icon={<StopOutlined />}
            onClick={stopResearch}
          >
            停止
          </Button>
        )}
      </div>

      {!isResearching && !state && (
        <div className="research-start">
          <Paragraph type="secondary" className="mb-4">
            输入研究主题，AI 将自动搜索、分析多个信息源并生成研究报告。
          </Paragraph>
          <TextArea
            placeholder="输入研究主题，例如：人工智能的发展历史和未来趋势"
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
            开始研究
          </Button>
        </div>
      )}

      {state && (
        <div className="research-progress">
          <div className="mb-4">
            <Text strong>研究主题：</Text>
            <Paragraph className="mb-2">{state.topic}</Paragraph>
          </div>

          {state.status !== 'completed' && state.status !== 'failed' && (
            <PhaseProgress
              currentPhase={state.currentPhase}
              percentage={state.progress.percentage}
            />
          )}

          <Divider />

          {state.status === 'in_progress' && (
            <div className="flex gap-2 mb-4">
              <Button
                icon={<PauseOutlined />}
                onClick={pauseResearch}
                disabled={isPaused}
              >
                暂停
              </Button>
            </div>
          )}

          {state.status === 'paused' && (
            <div className="flex gap-2 mb-4">
              <Button
                type="primary"
                icon={<PlayCircleOutlined />}
                onClick={resumeResearch}
              >
                继续
              </Button>
            </div>
          )}

          {error && (
            <Alert
              message="错误"
              description={error}
              type="error"
              showIcon
              className="mb-4"
            />
          )}

          <Divider />

          {state.searchResults.length > 0 && (
            <div className="sources-section mb-4">
              <Title level={5}>搜索结果 ({state.searchResults.length})</Title>
              <List
                size="small"
                dataSource={state.searchResults}
                renderItem={(item) => (
                  <List.Item>
                    <List.Item.Meta
                      title={
                        <Space>
                          <a href={item.url} target="_blank" rel="noopener noreferrer">
                            {item.title}
                          </a>
                          <Tag color={getSourceTypeColor(item.sourceType)}>
                            {getSourceTypeName(item.sourceType)}
                          </Tag>
                        </Space>
                      }
                      description={
                        <div>
                          <Paragraph
                            ellipsis={{ rows: 2 }}
                            className="mb-1"
                          >
                            {item.snippet}
                          </Paragraph>
                          {item.credibilityScore !== null && (
                            <CredibilityBadge score={item.credibilityScore} />
                          )}
                        </div>
                      }
                    />
                  </List.Item>
                )}
              />
            </div>
          )}

          {state.citations.length > 0 && (
            <div className="citations-section mb-4">
              <Title level={5}>
                引用 ({state.citations.length})
              </Title>
              <List
                size="small"
                dataSource={state.citations}
                renderItem={(item) => (
                  <List.Item>
                    <Space>
                      <CheckCircleOutlined style={{ color: item.inReport ? '#52c41a' : '#d9d9d9' }} />
                      <Text>{item.sourceTitle}</Text>
                      <Tag>{getSourceTypeName(item.sourceType)}</Tag>
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
          <Title level={5}>生成的研究报告</Title>

          <div className="flex gap-2 mb-4">
            <Button
              icon={<FileTextOutlined />}
              onClick={() => navigator.clipboard.writeText(report.content)}
            >
              复制报告
            </Button>
            <Button onClick={resetResearch}>开始新研究</Button>
          </div>

          <Card className="report-preview" style={{ background: '#fafafa' }}>
            <pre style={{ whiteSpace: 'pre-wrap', fontFamily: 'inherit', fontSize: '14px' }}>
              {report.content}
            </pre>
          </Card>
        </div>
      )}
    </Card>
  );
}

export default ResearchPanel;
