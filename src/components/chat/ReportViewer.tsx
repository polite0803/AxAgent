import type { Citation } from "@/types";
import { CheckCircleOutlined, CopyOutlined, DownloadOutlined, FileTextOutlined } from "@ant-design/icons";
import { Button, Card, Divider, Select, Space, Tabs, Tag, Typography } from "antd";
import DOMPurify from "dompurify";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { CredibilityBadge } from "./CredibilityBadge";

const { Text, Title } = Typography;

type ReportFormat = "markdown" | "html" | "json";

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

export function ReportViewer({ report, onCopy, onExport, onReset }: ReportViewerProps) {
  const { t } = useTranslation();
  const getSourceTypeName = (sourceType: string): string =>
    t(`report.sourceType.${sourceType.toLowerCase()}`, sourceType);
  const [selectedFormat, setSelectedFormat] = useState<ReportFormat>("markdown");

  const sanitizedHtml = useMemo(() => {
    if (!report) { return ""; }
    const rawHtml = report.content
      .replace(/#\s+(.+)/g, "<h1>$1</h1>")
      .replace(/##\s+(.+)/g, "<h2>$1</h2>")
      .replace(/\n/g, "<br/>");
    return DOMPurify.sanitize(rawHtml, {
      ALLOWED_TAGS: [
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "p",
        "br",
        "hr",
        "ul",
        "ol",
        "li",
        "strong",
        "em",
        "b",
        "i",
        "u",
        "s",
        "a",
        "code",
        "pre",
        "blockquote",
        "table",
        "thead",
        "tbody",
        "tr",
        "th",
        "td",
        "span",
        "div",
        "img",
        "sub",
        "sup",
      ],
      ALLOWED_ATTR: ["href", "src", "alt", "title", "target", "rel"],
    });
  }, [report?.content]);

  if (!report) {
    return (
      <Card className="h-full">
        <div className="flex items-center justify-center h-64 text-zinc-400">
          <div className="text-center">
            <FileTextOutlined style={{ fontSize: 48 }} className="mb-4" />
            <div>{t("reportViewer.noReport")}</div>
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
    if (format === "json") {
      content = JSON.stringify(
        {
          topic: report.topic,
          summary: report.summary,
          content: report.content,
          citations: report.citations,
        },
        null,
        2,
      );
    }
    onExport?.(format, content);
  };

  const renderMarkdownPreview = () => (
    <pre
      style={{
        whiteSpace: "pre-wrap",
        fontFamily: "inherit",
        fontSize: "14px",
        lineHeight: 1.6,
        background: "#fafafa",
        padding: "16px",
        borderRadius: "8px",
        maxHeight: "500px",
        overflow: "auto",
      }}
    >
      {report.content}
    </pre>
  );

  const renderHtmlPreview = () => {
    return (
      <div
        style={{
          background: "#fff",
          padding: "16px",
          borderRadius: "8px",
          border: "1px solid #f0f0f0",
          maxHeight: "500px",
          overflow: "auto",
        }}
      >
        <div dangerouslySetInnerHTML={{ __html: sanitizedHtml }} />
      </div>
    );
  };

  const tabItems = [
    {
      key: "preview",
      label: t("reportViewer.preview"),
      children: selectedFormat === "markdown" ? renderMarkdownPreview() : renderHtmlPreview(),
    },
    {
      key: "references",
      label: (
        <span>
          {t("reportViewer.references")} <Tag>{report.citations.length}</Tag>
        </span>
      ),
      children: (
        <div className="references-list">
          {report.citations.length > 0
            ? (
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
            )
            : <Text type="secondary">{t("reportViewer.noReferences")}</Text>}
        </div>
      ),
    },
    {
      key: "summary",
      label: t("reportViewer.summary"),
      children: (
        <Card className="bg-zinc-50">
          <Text>{report.summary || t("reportViewer.noSummary")}</Text>
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
              {t("reportViewer.generatedAt")} {new Date(report.createdAt).toLocaleString()}
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
                { value: "markdown", label: "Markdown" },
                { value: "html", label: "HTML" },
              ]}
              style={{ width: 120 }}
            />
            <Button icon={<DownloadOutlined />} onClick={() => handleExport(selectedFormat)}>
              {t("reportViewer.export")}
            </Button>
          </Space>
          <Space>
            <Button icon={<CopyOutlined />} onClick={handleCopy}>
              {t("reportViewer.copyReport")}
            </Button>
            {onReset && (
              <Button onClick={onReset} type="primary">
                {t("reportViewer.startNewResearch")}
              </Button>
            )}
          </Space>
        </div>

        <Tabs items={tabItems} defaultActiveKey="preview" />

        {report.citations.filter((c) => c.credibility < 0.5).length > 0 && (
          <div className="mt-4 p-3 bg-yellow-50 border border-yellow-200 rounded">
            <Text type="warning" className="text-sm">
              <CheckCircleOutlined className="mr-1" />
              {t("reportViewer.lowCredibilityWarning", {
                count: report.citations.filter((c) => c.credibility < 0.5).length,
              })}
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
  const { t } = useTranslation();
  if (!outline) {
    return (
      <Card size="small">
        <Text type="secondary">{t("reportViewer.noOutline")}</Text>
      </Card>
    );
  }

  return (
    <Card size="small" title={outline.title}>
      <div className="space-y-2">
        {outline.sections.map((section, index) => (
          <div
            key={section.id}
            className="cursor-pointer hover:bg-zinc-50 p-2 rounded"
            role="button"
            tabIndex={0}
            onKeyDown={(e) => {
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                onSectionClick?.(section.id);
              }
            }}
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
