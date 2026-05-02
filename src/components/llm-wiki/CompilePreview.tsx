import type { CompiledPage } from "@/types/llmWiki";
import { EyeOutlined, SaveOutlined } from "@ant-design/icons";
import { Button, Card, Divider, Empty, message, Modal, Space, Spin, Tag, Typography } from "antd";
import { useState } from "react";
import { useTranslation } from "react-i18next";

const { Title, Text } = Typography;

interface CompilePreviewProps {
  onCompile: () => Promise<CompiledPage[]>;
  onSave: (pages: CompiledPage[]) => Promise<void>;
}

export function CompilePreview({ onCompile, onSave }: CompilePreviewProps) {
  const { t } = useTranslation();
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [compiling, setCompiling] = useState(false);
  const [saving, setSaving] = useState(false);
  const [compiledPages, setCompiledPages] = useState<CompiledPage[]>([]);
  const [selectedPageIndex, setSelectedPageIndex] = useState(0);

  const handleOpen = async () => {
    setIsModalOpen(true);
    setCompiledPages([]);
    await handleCompile();
  };

  const handleCompile = async () => {
    setCompiling(true);
    try {
      const pages = await onCompile();
      setCompiledPages(pages);
      setSelectedPageIndex(0);
      if (pages.length === 0) {
        message.warning(t("wiki.compile.noPages", "No pages were generated"));
      }
    } catch (e) {
      message.error(String(e));
    }
    setCompiling(false);
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      await onSave(compiledPages);
      message.success(t("wiki.compile.saved", "Pages saved successfully"));
      setIsModalOpen(false);
    } catch (e) {
      message.error(String(e));
    }
    setSaving(false);
  };

  const getQualityScore = (page: CompiledPage): number => {
    let score = 1.0;
    if (page.content.length < 100) { score -= 0.2; }
    if (!page.content.includes("[[")) { score -= 0.1; }
    if (page.content.toLowerCase().includes("我不知道")) { score -= 0.3; }
    if (page.content.toLowerCase().includes("无法确定")) { score -= 0.3; }
    return Math.max(0, Math.min(1, score));
  };

  const getQualityColor = (score: number) => {
    if (score >= 0.8) { return "success"; }
    if (score >= 0.5) { return "warning"; }
    return "error";
  };

  const renderContent = (content: string) => {
    return content.split("\n").map((line, i) => {
      if (line.startsWith("# ")) {
        return <h2 key={i} className="text-xl font-bold mt-4 mb-2">{line.substring(2)}</h2>;
      }
      if (line.startsWith("## ")) {
        return <h3 key={i} className="text-lg font-semibold mt-3 mb-2">{line.substring(3)}</h3>;
      }
      if (line.startsWith("### ")) {
        return <h4 key={i} className="text-base font-medium mt-2 mb-1">{line.substring(4)}</h4>;
      }
      if (line.startsWith("- ")) {
        return <li key={i} className="ml-4">{line.substring(2)}</li>;
      }
      if (line.match(/^\[\[.+\]\]$/)) {
        const linkText = line.slice(2, -2);
        return (
          <span key={i}>
            <a
              className="text-blue-500 hover:underline cursor-pointer"
              onClick={() => {
                const targetIndex = compiledPages.findIndex(p => p.title === linkText);
                if (targetIndex >= 0) { setSelectedPageIndex(targetIndex); }
              }}
            >
              [[{linkText}]]
            </a>
            {" "}
          </span>
        );
      }
      if (line.trim() === "") {
        return <br key={i} />;
      }
      return <p key={i} className="my-1">{line}</p>;
    });
  };

  const selectedPage = compiledPages[selectedPageIndex];
  const qualityScore = selectedPage ? getQualityScore(selectedPage) : 0;

  return (
    <>
      <Button type="primary" icon={<EyeOutlined />} onClick={handleOpen}>
        {t("wiki.compile.preview", "Preview Compile")}
      </Button>

      <Modal
        title={t("wiki.compile.previewTitle", "Compile Preview")}
        open={isModalOpen}
        onCancel={() => setIsModalOpen(false)}
        width={1200}
        footer={[
          <Button key="recompile" onClick={handleCompile} loading={compiling}>
            {t("wiki.compile.recompile", "Recompile")}
          </Button>,
          <Button key="close" onClick={() => setIsModalOpen(false)}>
            {t("common.cancel", "Cancel")}
          </Button>,
          <Button
            key="save"
            type="primary"
            icon={<SaveOutlined />}
            loading={saving}
            disabled={compiledPages.length === 0}
            onClick={handleSave}
          >
            {t("wiki.compile.saveAll", "Save All ({{count}} pages)", { count: compiledPages.length })}
          </Button>,
        ]}
      >
        {compiling
          ? (
            <div className="flex items-center justify-center py-20">
              <Spin size="large" tip={t("wiki.compile.compiling", "Compiling...")} />
            </div>
          )
          : compiledPages.length === 0
          ? <Empty description={t("wiki.compile.noResults", "No pages generated")} />
          : (
            <div className="flex gap-4">
              <div className="w-64 border-r pr-4 overflow-auto max-h-150">
                <Text type="secondary" className="block mb-2">
                  {t("wiki.compile.pages", "{{count}} pages", { count: compiledPages.length })}
                </Text>
                {compiledPages.map((page, index) => {
                  const score = getQualityScore(page);
                  return (
                    <Card
                      key={index}
                      size="small"
                      className={`mb-2 cursor-pointer ${index === selectedPageIndex ? "border-blue-500" : ""}`}
                      onClick={() => setSelectedPageIndex(index)}
                    >
                      <div className="flex items-center justify-between">
                        <Text strong className="truncate flex-1">{page.title}</Text>
                        <Tag color={getQualityColor(score)} className="ml-2">
                          {Math.round(score * 100)}%
                        </Tag>
                      </div>
                      <Text type="secondary" className="text-xs">
                        {page.page_type} | {page.source_ids.length} sources
                      </Text>
                    </Card>
                  );
                })}
              </div>

              <div className="flex-1 overflow-auto max-h-150">
                {selectedPage && (
                  <>
                    <div className="flex items-center justify-between mb-4">
                      <Title level={4} className="m-0">{selectedPage.title}</Title>
                      <Tag color={getQualityColor(qualityScore)} className="text-lg px-3 py-1">
                        {t("wiki.qualityScore", "Quality")}: {Math.round(qualityScore * 100)}%
                      </Tag>
                    </div>

                    <Space className="mb-4">
                      <Tag>{selectedPage.page_type}</Tag>
                      <Text type="secondary">
                        {selectedPage.content.length} characters
                      </Text>
                      <Text type="secondary">
                        {selectedPage.content.split("[[").length - 1} links
                      </Text>
                    </Space>

                    <Divider className="my-3" />

                    <div className="prose max-w-none">
                      {renderContent(selectedPage.content)}
                    </div>
                  </>
                )}
              </div>
            </div>
          )}
      </Modal>
    </>
  );
}
