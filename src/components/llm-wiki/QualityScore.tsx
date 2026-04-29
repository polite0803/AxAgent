import { useEffect, useState } from 'react';
import {
  Card,
  Typography,
  Progress,
  Tag,
  Space,
  Spin,
  Button,
  Tooltip,
  Empty,
  List,
  Badge,
} from 'antd';
import {
  CheckCircleOutlined,
  CloseCircleOutlined,
  WarningOutlined,
  ReloadOutlined,
} from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { invoke } from '@/lib/invoke';
import type { LintIssue } from '@/types/llmWiki';

const { Text } = Typography;

interface QualityScoreProps {
  wikiId: string;
  pageId?: string;
  autoRefresh?: boolean;
  refreshInterval?: number;
}

interface QualityDetails {
  score: number;
  issues: LintIssue[];
  factors: {
    name: string;
    impact: number;
    description: string;
  }[];
}

export function QualityScore({
  wikiId,
  pageId,
  autoRefresh = false,
  refreshInterval = 60000,
}: QualityScoreProps) {
  const { t } = useTranslation();
  const [loading, setLoading] = useState(true);
  const [details, setDetails] = useState<QualityDetails | null>(null);
  const [refreshing, setRefreshing] = useState(false);

  useEffect(() => {
    loadQualityScore();
  }, [wikiId, pageId]);

  useEffect(() => {
    if (!autoRefresh) return;
    const interval = setInterval(loadQualityScore, refreshInterval);
    return () => clearInterval(interval);
  }, [autoRefresh, refreshInterval, wikiId, pageId]);

  const loadQualityScore = async () => {
    setRefreshing(true);
    try {
      if (pageId) {
        const result = await invoke<LintIssue[]>('llm_wiki_lint', { wikiId, pageId });
        const score = calculateScore(result || []);
        setDetails({
          score,
          issues: result || [],
          factors: analyzeFactors(result || []),
        });
      } else {
        const result = await invoke<{ pageId: string; issues: LintIssue[] }[]>('llm_wiki_lint_batch', { wikiId });
        const allIssues = result?.flatMap(r => r.issues) || [];
        const score = calculateScore(allIssues);
        setDetails({
          score,
          issues: allIssues,
          factors: analyzeFactors(allIssues),
        });
      }
    } catch (e) {
      console.error('Failed to load quality score:', e);
    }
    setLoading(false);
    setRefreshing(false);
  };

  const calculateScore = (issues: LintIssue[]): number => {
    if (issues.length === 0) return 1.0;
    let score = 1.0;
    for (const issue of issues) {
      switch (issue.severity) {
        case 'Error':
          score -= 0.3;
          break;
        case 'Warning':
          score -= 0.1;
          break;
        case 'Info':
          score -= 0.02;
          break;
      }
    }
    return Math.max(0, Math.min(1, score));
  };

  const analyzeFactors = (issues: LintIssue[]): QualityDetails['factors'] => {
    const factors: QualityDetails['factors'] = [];

    const errorCount = issues.filter(i => i.severity === 'Error').length;
    if (errorCount > 0) {
      factors.push({
        name: t('wiki.quality.errors', 'Errors'),
        impact: -errorCount * 0.3,
        description: t('wiki.quality.errorsDesc', '{{count}} structural errors found', { count: errorCount }),
      });
    }

    const warningCount = issues.filter(i => i.severity === 'Warning').length;
    if (warningCount > 0) {
      factors.push({
        name: t('wiki.quality.warnings', 'Warnings'),
        impact: -warningCount * 0.1,
        description: t('wiki.quality.warningsDesc', '{{count}} potential issues found', { count: warningCount }),
      });
    }

    const infoCount = issues.filter(i => i.severity === 'Info').length;
    if (infoCount > 0) {
      factors.push({
        name: t('wiki.quality.suggestions', 'Suggestions'),
        impact: -infoCount * 0.02,
        description: t('wiki.quality.suggestionsDesc', '{{count}} improvement suggestions', { count: infoCount }),
      });
    }

    return factors;
  };

  const getScoreColor = (score: number) => {
    if (score >= 0.8) return '#52c41a';
    if (score >= 0.5) return '#faad14';
    return '#ff4d4f';
  };

  const getScoreIcon = (score: number) => {
    if (score >= 0.8) return <CheckCircleOutlined style={{ color: '#52c41a' }} />;
    if (score >= 0.5) return <WarningOutlined style={{ color: '#faad14' }} />;
    return <CloseCircleOutlined style={{ color: '#ff4d4f' }} />;
  };

  const getScoreLabel = (score: number) => {
    if (score >= 0.8) return t('wiki.quality.excellent', 'Excellent');
    if (score >= 0.6) return t('wiki.quality.good', 'Good');
    if (score >= 0.4) return t('wiki.quality.fair', 'Fair');
    return t('wiki.quality.poor', 'Poor');
  };

  const getIssueSeverityColor = (severity: string) => {
    switch (severity) {
      case 'error':
        return 'error';
      case 'warning':
        return 'warning';
      case 'info':
        return 'info';
      default:
        return 'default';
    }
  };

  if (loading) {
    return (
      <Card size="small">
        <div className="flex items-center justify-center py-4">
          <Spin size="small" />
        </div>
      </Card>
    );
  }

  if (!details) {
    return (
      <Card size="small">
        <Empty description={t('wiki.quality.noData', 'No quality data available')} />
      </Card>
    );
  }

  const percent = Math.round(details.score * 100);

  return (
    <Card
      size="small"
      title={
        <Space>
          <span>{t('wiki.quality.title', 'Quality Score')}</span>
          {refreshing && <Spin size="small" />}
        </Space>
      }
      extra={
        <Tooltip title={t('wiki.quality.refresh', 'Refresh')}>
          <Button
            type="text"
            size="small"
            icon={<ReloadOutlined spin={refreshing} />}
            onClick={loadQualityScore}
          />
        </Tooltip>
      }
    >
      <div className="flex items-center gap-4 mb-4">
        <Progress
          type="circle"
          percent={percent}
          size={80}
          strokeColor={getScoreColor(details.score)}
          format={() => (
            <span className="text-lg font-bold">{percent}%</span>
          )}
        />
        <div>
          <Space>
            {getScoreIcon(details.score)}
            <Text strong>{getScoreLabel(details.score)}</Text>
          </Space>
          <div className="mt-1">
            <Text type="secondary" className="text-xs">
              {details.issues.length} {t('wiki.quality.issues', 'issues')}
            </Text>
          </div>
        </div>
      </div>

      {details.factors.length > 0 && (
        <div className="mb-4">
          <Text type="secondary" className="text-xs uppercase">
            {t('wiki.quality.factors', 'Contributing Factors')}
          </Text>
          <div className="mt-1">
            {details.factors.map((factor, index) => (
              <div key={index} className="flex items-center gap-2 text-sm">
                <Tag color={factor.impact < -0.2 ? 'error' : factor.impact < -0.05 ? 'warning' : 'default'}>
                  {factor.name}
                </Tag>
                <Text type="secondary">{factor.description}</Text>
              </div>
            ))}
          </div>
        </div>
      )}

      {details.issues.length > 0 && (
        <div>
          <Text type="secondary" className="text-xs uppercase">
            {t('wiki.quality.issueList', 'Issues')}
          </Text>
          <List
            size="small"
            className="mt-1 max-h-40 overflow-auto"
            dataSource={details.issues.slice(0, 10)}
            renderItem={(issue) => (
              <List.Item className="px-0 py-1">
                <Space size="small">
                  <Badge status={getIssueSeverityColor(issue.severity) as any} />
                  <Text className="text-xs">{issue.message}</Text>
                  {issue.line && (
                    <Tag className="text-xs">L{issue.line}</Tag>
                  )}
                </Space>
              </List.Item>
            )}
          />
          {details.issues.length > 10 && (
            <Text type="secondary" className="text-xs">
              +{details.issues.length - 10} {t('wiki.quality.more', 'more')}
            </Text>
          )}
        </div>
      )}
    </Card>
  );
}