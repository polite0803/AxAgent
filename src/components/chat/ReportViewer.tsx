import { Button, Card, Divider, Select, Space, Tabs, Tag, Typography } from 'antd';
import {
  CopyOutlined,
  DownloadOutlined,
  FileTextOutlined,
  CheckCircleOutlined,
} from '@ant-design/icons';
import { useState } from 'react';
import { CredibilityBadge } from './CredibilityBadge';

const { Text, Title } = Typography;

type ReportFormat = 'markdown' | 'html' | 'json';

interface Citation {
  id: string;
  sourceUrl: string;
  sourceTitle: string;
  sourceType: string;
  credibility: number;
}

interface ResearchReport {
  id: string;
  topic: string;
  content: string;
  citations: Citation[];
  summary: string;
  createdAt?: string;
}

interface ReportViewerProps {
  report: ResearchReport | null;
  onCopy?: (content: string) => void;
  onExport?: (format: ReportFormat, content: string) => void;
  onReset?: () => void;
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

export function ReportViewer({ report, onCopy, onExport, onReset }: ReportViewerProps) {
  const [selectedFormat, setSelectedFormat] = useState<ReportFormat>('markdown');

  if (!report) {
    return (
      <Card className="h-full">
        <div className="flex items-center justify-center h-64 text-gray-400">
          <div className="text-center">
            <FileTextOutlined style={{ fontSize: 48 }} className="mb-4" />
            <div>暂无生成报告</div>
          </div>
        </div>
      </Card>
    );
  }

  const handleCopy = () => {
    navigator.clipboard.writeText(report.content);
    onCopy?.(report.content);
  };

  const handleExport = (format: ReportFormat) => {
    let content = report.content;
    if (format === 'json') {
      content = JSON.stringify(
        {
          topic: report.topic,
          summary: report.summary,
          content: report.content,
          citations: report.citations,
        },
        null,
        2
      );
    }
    onExport?.(format, content);
  };

  const renderMarkdownPreview = () => (
    <pre
      style={{
        whiteSpace: 'pre-wrap',
        fontFamily: 'inherit',
        fontSize: '14px',
        lineHeight: 1.6,
        background: '#fafafa',
        padding: '16px',
        borderRadius: '8px',
        maxHeight: '500px',
        overflow: 'auto',
      }}
    >
      {report.content}
    </pre>
  );

  const renderHtmlPreview = () => (
    <div
      style={{
        background: '#fff',
        padding: '16px',
        borderRadius: '8px',
        border: '1px solid #f0f0f0',
        maxHeight: '500px',
        overflow: 'auto',
      }}
    >
      <div dangerouslySetInnerHTML={{ __html: report.content.replace(/#\s+(.+)/g, '<h1>$1</h1>').replace(/##\s+(.+)/g, '<h2>$1</h2>').replace(/\n/g, '<br/>') }} />
    </div>
  );

  const tabItems = [
    {
      key: 'preview',
      label: '预览',
      children: selectedFormat === 'markdown' ? renderMarkdownPreview() : renderHtmlPreview(),
    },
    {
      key: 'references',
      label: (
        <span>
          参考文献 <Tag>{report.citations.length}</Tag>
        </span>
      ),
      children: (
        <div className="references-list">
          {report.citations.length > 0 ? (
            <ol style={{ paddingLeft: 20 }}>
              {report.citations.map((citation) => (
                <li key={citation.id} className="mb-2">
                  <a href={citation.sourceUrl} target="_blank" rel="noopener noreferrer">
                    {citation.sourceTitle}
                  </a>
                  <Space size="small" className="ml-2">
                    <Tag>{getSourceTypeName(citation.sourceType)}</Tag>
                    <CredibilityBadge score={citation.credibility} size="small" />
                  </Space>
                </li>
              ))}
            </ol>
          ) : (
            <Text type="secondary">暂无参考文献</Text>
          )}
        </div>
      ),
    },
    {
      key: 'summary',
      label: '摘要',
      children: (
        <Card className="bg-gray-50">
          <Text>{report.summary || '暂无摘要'}</Text>
        </Card>
      ),
    },
  ];

  return (
    <div className="report-viewer">
      <Card>
        <div className="flex items-center justify-between mb-4">
          <Title level={4} className="mb-0">
            {report.topic}
          </Title>
          {report.createdAt && (
            <Text type="secondary" className="text-sm">
              生成时间: {new Date(report.createdAt).toLocaleString()}
            </Text>
          )}
        </div>

        <Divider className="my-3" />

        <div className="flex items-center justify-between mb-4">
          <Space>
            <Select
              value={selectedFormat}
              onChange={setSelectedFormat}
              options={[
                { value: 'markdown', label: 'Markdown' },
                { value: 'html', label: 'HTML' },
              ]}
              style={{ width: 120 }}
            />
            <Button icon={<DownloadOutlined />} onClick={() => handleExport(selectedFormat)}>
              导出
            </Button>
          </Space>
          <Space>
            <Button icon={<CopyOutlined />} onClick={handleCopy}>
              复制报告
            </Button>
            {onReset && (
              <Button onClick={onReset} type="primary">
                开始新研究
              </Button>
            )}
          </Space>
        </div>

        <Tabs items={tabItems} defaultActiveKey="preview" />

        {report.citations.filter((c) => c.credibility < 0.5).length > 0 && (
          <div className="mt-4 p-3 bg-yellow-50 border border-yellow-200 rounded">
            <Text type="warning" className="text-sm">
              <CheckCircleOutlined className="mr-1" />
              注意: 报告中包含 {report.citations.filter((c) => c.credibility < 0.5).length} 个低可信度来源，请谨慎使用
            </Text>
          </div>
        )}
      </Card>
    </div>
  );
}

interface ReportOutlineViewProps {
  outline: { title: string; sections: { id: string; title: string; description: string }[] } | null;
  onSectionClick?: (sectionId: string) => void;
}

export function ReportOutlineView({ outline, onSectionClick }: ReportOutlineViewProps) {
  if (!outline) {
    return (
      <Card size="small">
        <Text type="secondary">暂无大纲</Text>
      </Card>
    );
  }

  return (
    <Card size="small" title={outline.title}>
      <div className="space-y-2">
        {outline.sections.map((section, index) => (
          <div
            key={section.id}
            className="cursor-pointer hover:bg-gray-50 p-2 rounded"
            onClick={() => onSectionClick?.(section.id)}
          >
            <Text strong>
              {index + 1}. {section.title}
            </Text>
            {section.description && (
              <Text type="secondary" className="block text-sm">
                {section.description}
              </Text>
            )}
          </div>
        ))}
      </div>
    </Card>
  );
}

export default ReportViewer;
