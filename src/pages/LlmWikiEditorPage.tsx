import { useLlmWikiStore, WikiPage } from "@/stores/feature/llmWikiStore";
import { CheckCircleOutlined, EyeOutlined, LeftOutlined, SaveOutlined, WarningOutlined } from "@ant-design/icons";
import { Button, Card, Descriptions, message, Space, Spin, Tag, theme, Tooltip, Typography } from "antd";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

const { Title, Text } = Typography;

interface LlmWikiEditorPageProps {
  wikiId: string;
  pageId: string;
  onBack: () => void;
}

export function LlmWikiEditorPage({ wikiId, pageId, onBack }: LlmWikiEditorPageProps) {
  const { token } = theme.useToken();
  const { t } = useTranslation();
  const { pages, compileWiki } = useLlmWikiStore();

  const [wikiPage, setWikiPage] = useState<WikiPage | null>(null);
  const [compiledContent, setCompiledContent] = useState("");
  const [loading, setLoading] = useState(true);
  const [compiling, setCompiling] = useState(false);
  const [_hasChanges, setHasChanges] = useState(false);

  const loadPage = useCallback(async () => {
    setLoading(true);
    const page = pages.find((p) => p.id === pageId);
    if (page) {
      setWikiPage(page);
      setCompiledContent("");
      setHasChanges(false);
    }
    setLoading(false);
  }, [pageId, pages]);

  useEffect(() => {
    loadPage();
  }, [loadPage]);

  const handleRecompile = async () => {
    if (!wikiPage) { return; }
    setCompiling(true);
    try {
      const result = await compileWiki(wikiId, [pageId]);
      if (result && result.updated_pages.length > 0) {
        setCompiledContent(result.updated_pages[0].content);
        message.success(t("wiki.llm.compileSuccess"));
      } else if (result && result.errors.length > 0) {
        message.error(result.errors[0]);
      }
    } catch (e) {
      message.error(String(e));
    }
    setCompiling(false);
  };

  const handleViewInGraph = () => {
    window.location.href = `/wiki/graph?wikiId=${wikiId}&focus=${pageId}`;
  };

  const getQualityColor = (score?: number) => {
    if (!score) { return "default"; }
    if (score >= 0.8) { return "success"; }
    if (score >= 0.5) { return "warning"; }
    return "error";
  };

  const getQualityLabel = (score?: number) => {
    if (!score) { return t("wiki.llm.noScore", "No Score"); }
    return `${Math.round(score * 100)}%`;
  };

  if (loading) {
    return (
      <div className="h-full flex items-center justify-center" style={{ backgroundColor: token.colorBgElevated }}>
        <Spin size="large" />
      </div>
    );
  }

  if (!wikiPage) {
    return (
      <div className="h-full flex items-center justify-center" style={{ backgroundColor: token.colorBgElevated }}>
        <span>{t("wiki.pageNotFound", "Page not found")}</span>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col" style={{ overflow: "hidden", backgroundColor: token.colorBgElevated }}>
      <div className="flex items-center gap-2 p-3 border-b" style={{ borderColor: token.colorBorderSecondary }}>
        <Button icon={<LeftOutlined />} onClick={onBack} type="text" />
        <Title level={4} className="m-0 flex-1">{wikiPage.title}</Title>
        <Space>
          <Tooltip title={t("wiki.llm.viewInGraph", "View in Graph")}>
            <Button icon={<EyeOutlined />} onClick={handleViewInGraph} type="text" />
          </Tooltip>
          <Button
            icon={<SaveOutlined />}
            type="primary"
            onClick={handleRecompile}
            loading={compiling}
          >
            {t("wiki.llm.recompile", "Recompile")}
          </Button>
        </Space>
      </div>

      <div className="flex-1 overflow-auto p-4">
        <Card className="mb-4">
          <Descriptions size="small" column={4}>
            <Descriptions.Item label={t("wiki.pageType", "Page Type")}>
              <Tag color="blue">{wikiPage.pageType}</Tag>
            </Descriptions.Item>
            <Descriptions.Item label={t("wiki.qualityScore", "Quality Score")}>
              <Tag color={getQualityColor(wikiPage.qualityScore)}>
                {getQualityLabel(wikiPage.qualityScore)}
              </Tag>
            </Descriptions.Item>
            <Descriptions.Item label={t("wiki.lastCompiled", "Last Compiled")}>
              {wikiPage.lastCompiledAt
                ? new Date(wikiPage.lastCompiledAt * 1000).toLocaleString()
                : "-"}
            </Descriptions.Item>
            <Descriptions.Item label={t("wiki.lastLinted", "Last Linted")}>
              {wikiPage.lastLintedAt
                ? new Date(wikiPage.lastLintedAt * 1000).toLocaleString()
                : "-"}
            </Descriptions.Item>
          </Descriptions>
        </Card>

        {wikiPage.qualityScore !== undefined && wikiPage.qualityScore < 0.5 && (
          <Card className="mb-4" style={{ backgroundColor: token.colorWarningBg }}>
            <Space>
              <WarningOutlined style={{ color: token.colorWarning }} />
              <Text type="warning">
                {t("wiki.llm.lowQualityWarning", "This page has low quality score. Consider editing or recompiling.")}
              </Text>
            </Space>
          </Card>
        )}

        {compiledContent && (
          <Card
            className="mb-4"
            title={t("wiki.llm.newCompiledContent", "New Compiled Content")}
            extra={
              <Tag icon={<CheckCircleOutlined />} color="success">
                {t("wiki.llm.readyToSave", "Ready to Save")}
              </Tag>
            }
          >
            <pre className="whitespace-pre-wrap font-mono text-sm p-4 bg-gray-50 rounded">
              {compiledContent}
            </pre>
          </Card>
        )}

        <Card title={t("wiki.sources", "Sources")}>
          {wikiPage.sourceIds && wikiPage.sourceIds.length > 0
            ? (
              <ul className="list-disc pl-5">
                {wikiPage.sourceIds.map((sourceId) => <li key={sourceId}>{sourceId}</li>)}
              </ul>
            )
            : <Text type="secondary">{t("wiki.noSources", "No sources linked")}</Text>}
        </Card>
      </div>
    </div>
  );
}
