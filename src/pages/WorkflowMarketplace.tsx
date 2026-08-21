// i18n-exempt: 业务逻辑/API 描述/日志字符串，非 UI 展示文本
// SPDX-License-Identifier: AGPL-3.0-only

import { ImportExportModal } from "@/components/workflow/Templates/ImportExportModal";
import { useAgentContext } from "@/hooks/useAgentContext";
import { showBackendError } from "@/lib/errorI18n";
import { invoke, logIpcError } from "@/lib/invoke";
import { MarketplaceStats, reviewApi, ReviewResponse } from "@/lib/reviewApi";
import { DownloadOutlined, StarOutlined, UploadOutlined } from "@ant-design/icons";
import { App, Button, Card, Empty, Form, Input, Modal, Rate, Space, Spin, Tabs, Tag, theme, Typography } from "antd";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

const { Title, Text } = Typography;
const { Search } = Input;

/**
 * 目录条目类型，与后端 `axagent_harness::marketplace::CatalogItem` 对齐
 * （serde rename_all = "camelCase" 序列化）。
 */
interface CatalogItem {
  id: string;
  /** "workflow_template" 或 "skill" */
  itemType: string;
  name: string;
  description?: string;
  category?: string;
  author?: string;
  tags: string[];
  version?: string;
  /** 是否已安装到本地 */
  installed: boolean;
  ratingAverage?: number;
  downloadCount?: number;
  createdAt: number;
  updatedAt: number;
}

/** 市场目录查询参数（与后端 CatalogQuery 对齐）。 */
interface CatalogQuery {
  keyword?: string;
  category?: string;
  itemType?: string;
  limit?: number;
  offset?: number;
}

/** 市场目录分页结果（与后端 CatalogPage 对齐）。 */
interface CatalogPage {
  items: CatalogItem[];
  total: number;
  offset: number;
  limit: number;
}

const CATEGORIES = [
  "All",
  "Productivity",
  "Development",
  "Data",
  "Automation",
  "AI",
  "Business",
];

function formatDate(timestamp: number): string {
  return new Date(timestamp * 1000).toLocaleDateString();
}

/**
 * Extracted component for rendering a template card.
 */
function TemplateCard({
  template,
  onTemplateClick,
}: {
  template: CatalogItem;
  onTemplateClick: (template: CatalogItem) => void;
}) {
  const { t } = useTranslation();
  const { token } = theme.useToken();

  return (
    <Card
      hoverable
      className="marketplace-card"
      onClick={() => onTemplateClick(template)}
      cover={
        <div
          className="flex items-center justify-center h-32"
          style={{
            backgroundColor: token.colorBgContainer,
            borderBottom: `1px solid ${token.colorBorderSecondary}`,
          }}
        >
          <span style={{ fontSize: 48 }}>
            {template.itemType === "skill" ? "⚡" : "📄"}
          </span>
        </div>
      }
      styles={{
        body: { padding: "16px" },
      }}
    >
      <Card.Meta
        title={
          <Space size={4}>
            <Text strong>{template.name}</Text>
            {template.installed && (
              <Tag color="green" style={{ margin: 0 }}>
                {t("marketplace.installed")}
              </Tag>
            )}
          </Space>
        }
        description={
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            <Text type="secondary" style={{ fontSize: 12, display: "block" }}>
              {template.description ?? t("marketplace.noDescription")}
            </Text>
            <Space size={8}>
              {template.ratingAverage !== undefined && template.ratingAverage > 0 && (
                <Tag color="blue" style={{ margin: 0, fontSize: 12 }}>
                  <StarOutlined style={{ fontSize: 12, marginRight: 4 }} />
                  {template.ratingAverage.toFixed(1)}
                </Tag>
              )}
              {template.downloadCount !== undefined && template.downloadCount > 0 && (
                <Text type="secondary" style={{ fontSize: 12 }}>
                  <DownloadOutlined style={{ fontSize: 12, marginRight: 4 }} />
                  {template.downloadCount}
                </Text>
              )}
            </Space>
          </div>
        }
      />
    </Card>
  );
}

export function WorkflowMarketplace() {
  const { t } = useTranslation();
  const { message } = App.useApp();
  const { token } = theme.useToken();

  // ── Agent 上下文注入：告知 Agent 当前页面是工作流市场 ──
  useAgentContext({
    page: "workflow-marketplace",
    url: "/marketplace",
    quickActions: [
      { id: "browse-templates", description: "浏览/搜索工作流模板市场" },
      { id: "install-template", description: "安装工作流模板到本地", requireConfirmation: true },
      { id: "import-template", description: "从本地文件导入工作流模板", requireConfirmation: true },
    ],
  });

  const [importModalOpen, setImportModalOpen] = useState(false);
  const [templates, setTemplates] = useState<CatalogItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [searchText, setSearchText] = useState("");
  const [selectedCategory, setSelectedCategory] = useState("All");
  const [selectedTemplate, setSelectedTemplate] = useState<CatalogItem | null>(null);
  const [isDetailOpen, setIsDetailOpen] = useState(false);
  const [activeTab, setActiveTab] = useState("templates");
  const [installing, setInstalling] = useState(false);

  const [reviews, setReviews] = useState<ReviewResponse[]>([]);
  const [myReview, setMyReview] = useState<ReviewResponse | null>(null);
  const [stats, setStats] = useState<MarketplaceStats | null>(null);
  const [loadingReviews, setLoadingReviews] = useState(false);
  const [submittingReview, setSubmittingReview] = useState(false);

  const [reviewForm] = Form.useForm();

  // ── 加载市场目录 ──
  const loadCatalog = useCallback(async () => {
    setLoading(true);
    try {
      const query: CatalogQuery = {
        keyword: searchText.trim() || undefined,
        category: selectedCategory !== "All" ? selectedCategory : undefined,
        limit: 100,
        offset: 0,
      };
      const page = await invoke<CatalogPage>("list_marketplace_catalog", { query });
      setTemplates(page.items);
    } catch (e) {
      showBackendError(message, e, { context: "list_marketplace_catalog" });
    } finally {
      setLoading(false);
    }
  }, [searchText, selectedCategory, message]);

  useEffect(() => {
    void loadCatalog();
  }, [loadCatalog]);

  const loadReviews = useCallback(async (marketplaceId: string) => {
    setLoadingReviews(true);
    try {
      const [reviewsData, myReviewData, statsData] = await Promise.all([
        reviewApi.getReviews(marketplaceId),
        reviewApi.getMyReview(marketplaceId),
        reviewApi.getStats(marketplaceId),
      ]);
      setReviews(reviewsData);
      setMyReview(myReviewData);
      setStats(statsData);
    } catch (error) {
      logIpcError("Failed to load reviews")(error);
    } finally {
      setLoadingReviews(false);
    }
  }, []);

  const handleTemplateClick = (template: CatalogItem) => {
    setSelectedTemplate(template);
    setIsDetailOpen(true);
    loadReviews(template.id);
  };

  const handleCloseDetail = () => {
    setIsDetailOpen(false);
    setReviews([]);
    setMyReview(null);
    setStats(null);
    reviewForm.resetFields();
  };

  const handleSubmitReview = async (values: {
    rating: number;
    comment?: string;
  }) => {
    if (!selectedTemplate) {
      return;
    }

    setSubmittingReview(true);
    try {
      if (myReview) {
        await reviewApi.updateReview(myReview.id, values);
        message.success(t("review.updatedSuccess"));
      } else {
        await reviewApi.createReview({
          marketplace_id: selectedTemplate.id,
          rating: values.rating,
          comment: values.comment,
        });
        message.success(t("review.submittedSuccess"));
      }
      loadReviews(selectedTemplate.id);
      reviewForm.resetFields();
    } catch (e) {
      showBackendError(message, e, { context: "submitReview" });
    } finally {
      setSubmittingReview(false);
    }
  };

  const handleDeleteReview = async () => {
    if (!myReview) {
      return;
    }

    try {
      await reviewApi.deleteReview(myReview.id);
      message.success(t("review.deletedSuccess"));
      if (selectedTemplate) {
        loadReviews(selectedTemplate.id);
      }
    } catch (e) {
      showBackendError(message, e, { context: "deleteReview" });
    }
  };

  // ── 安装模板：调用后端 install_marketplace_template ──
  const handleDownload = async (template: CatalogItem) => {
    // 技能类目录项已恒为 installed=true，无需安装
    if (template.itemType === "skill" || template.installed) {
      message.info(t("marketplace.alreadyInstalled"));
      return;
    }
    setInstalling(true);
    try {
      await invoke<void>("install_marketplace_template", { templateId: template.id });
      message.success(t("marketplace.installSuccess", { name: template.name }));
      // 刷新目录以反映 installed 状态变更
      void loadCatalog();
      if (selectedTemplate?.id === template.id) {
        setSelectedTemplate({ ...template, installed: true });
      }
    } catch (e) {
      showBackendError(message, e, { context: "install_marketplace_template" });
    } finally {
      setInstalling(false);
    }
  };

  const handleImport = () => {
    setImportModalOpen(true);
  };

  const handleImportSubmit = async (jsonData: string) => {
    return await invoke<{ id: string; warnings: string[]; errors: string[] }>(
      "import_workflow_template",
      { json_data: jsonData },
    );
  };

  const filteredTemplates = templates.filter((item) => {
    const matchesSearch = item.name.toLowerCase().includes(searchText.toLowerCase())
      || item.description?.toLowerCase().includes(searchText.toLowerCase());
    const matchesCategory = selectedCategory === "All" || item.category === selectedCategory;
    return matchesSearch && matchesCategory;
  });

  const featuredTemplates = filteredTemplates.filter((item) =>
    item.ratingAverage !== undefined && item.ratingAverage >= 4.5
  );

  const renderReviewsTab = () => (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <Title level={5} className="m-0">
          {t("marketplace.customerReviews")}
        </Title>
        {stats && (
          <div className="flex items-center gap-2">
            <Rate disabled value={stats.rating_average} allowHalf />
            <Text>{stats.rating_average.toFixed(1)}</Text>
            <Text type="secondary">
              ({stats.total_reviews} {t("marketplace.reviews")})
            </Text>
          </div>
        )}
      </div>

      <div className="border p-4 rounded">
        <Title level={5} className="m-0 mb-4">
          {myReview
            ? t("marketplace.yourReview")
            : t("marketplace.writeReview")}
        </Title>
        {myReview
          ? (
            <div className="space-y-2">
              <Rate disabled value={myReview.rating} />
              {myReview.comment && <p>{myReview.comment}</p>}
              <Text type="secondary" className="text-xs">
                {t("common.postedOn")} {formatDate(myReview.created_at)}
              </Text>
              <div className="flex gap-2 mt-2">
                <Button
                  size="small"
                  onClick={() => {
                    reviewForm.setFieldsValue({
                      rating: myReview.rating,
                      comment: myReview.comment || "",
                    });
                  }}
                >
                  {t("common.edit")}
                </Button>
                <Button size="small" danger onClick={handleDeleteReview}>
                  {t("common.delete")}
                </Button>
              </div>
            </div>
          )
          : (
            <Form
              form={reviewForm}
              onFinish={handleSubmitReview}
              layout="vertical"
            >
              <Form.Item
                name="rating"
                label={t("common.rating")}
                rules={[{ required: true, message: t("review.ratingRequired") }]}
              >
                <Rate />
              </Form.Item>
              <Form.Item name="comment" label={t("common.comment")}>
                <Input.TextArea
                  name="comment"
                  rows={3}
                  placeholder={t("review.commentPlaceholder")}
                />
              </Form.Item>
              <Form.Item>
                <Button
                  type="primary"
                  htmlType="submit"
                  loading={submittingReview}
                >
                  {t("marketplace.submitReview")}
                </Button>
              </Form.Item>
            </Form>
          )}
      </div>

      <Spin spinning={loadingReviews}>
        <Title level={5}>{t("marketplace.allReviews")}</Title>
        {reviews.length === 0
          ? <Empty description={t("marketplace.noReviews")} />
          : (
            <div className="divide-y divide-gray-100">
              {reviews.map((item) => (
                <div key={item.id} className="py-3">
                  <Space align="start" size={12}>
                    <Rate disabled value={item.rating} />
                    <div>
                      <Text strong>{`User ${item.user_id.slice(0, 8)}`}</Text>
                      <div>
                        {item.comment && <p>{item.comment}</p>}
                        <Text type="secondary" className="text-xs">
                          {formatDate(item.created_at)}
                        </Text>
                      </div>
                    </div>
                  </Space>
                </div>
              ))}
            </div>
          )}
      </Spin>
    </div>
  );

  return (
    <div
      className="flex h-full"
      style={{ backgroundColor: token.colorBgElevated }}
    >
      <aside
        className="w-56 border-r p-4"
        style={{
          backgroundColor: token.colorBgContainer,
          borderRight: `1px solid ${token.colorBorder}`,
        }}
      >
        <Title level={5} style={{ marginBottom: 16, marginTop: 0 }}>
          {t("marketplace.categories")}
        </Title>
        <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
          {CATEGORIES.map((cat) => (
            <Button
              key={cat}
              type={selectedCategory === cat ? "primary" : "text"}
              style={{ textAlign: "left", justifyContent: "flex-start" }}
              onClick={() => setSelectedCategory(cat)}
              block
            >
              {cat}
            </Button>
          ))}
        </div>

        <Title level={5} style={{ marginTop: 24, marginBottom: 16 }}>
          {t("marketplace.quickActions")}
        </Title>
        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          <Button icon={<UploadOutlined />} onClick={handleImport} block>
            {t("marketplace.importWorkflow")}
          </Button>
        </div>
      </aside>

      <main
        className="flex-1 overflow-y-auto p-6"
        style={{ backgroundColor: token.colorBgContainer }}
      >
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            marginBottom: 24,
          }}
        >
          <Title level={4} style={{ margin: 0 }}>
            {t("marketplace.title")}
          </Title>
          <Space>
            <Search
              placeholder={t("marketplace.searchWorkflows")}
              allowClear
              onSearch={setSearchText}
              onChange={(e) => setSearchText(e.target.value)}
              style={{ width: 256 }}
            />
          </Space>
        </div>

        <Tabs
          activeKey={activeTab}
          onChange={setActiveTab}
          items={[
            {
              key: "templates",
              label: t("marketplace.templates"),
              children: (
                <Spin spinning={loading}>
                  <div
                    style={{
                      display: "grid",
                      gridTemplateColumns: "repeat(3, 1fr)",
                      gap: 16,
                    }}
                  >
                    {filteredTemplates.length > 0
                      ? (
                        filteredTemplates.map((item) => (
                          <div key={item.id} style={{ position: "relative" }}>
                            <TemplateCard
                              template={item}
                              onTemplateClick={handleTemplateClick}
                            />
                            {!item.installed && (
                              <Button
                                type="primary"
                                icon={<DownloadOutlined />}
                                style={{ position: "absolute", top: 8, right: 8 }}
                                size="small"
                                loading={installing}
                                onClick={(e) => {
                                  e.stopPropagation();
                                  void handleDownload(item);
                                }}
                              />
                            )}
                          </div>
                        ))
                      )
                      : (
                        <div style={{ gridColumn: "span 3" }}>
                          <Empty description={t("marketplace.noTemplatesFound")} />
                        </div>
                      )}
                  </div>
                </Spin>
              ),
            },
            {
              key: "featured",
              label: t("marketplace.featured"),
              children: (
                <div
                  style={{
                    display: "grid",
                    gridTemplateColumns: "repeat(3, 1fr)",
                    gap: 16,
                  }}
                >
                  {featuredTemplates.length > 0
                    ? (
                      featuredTemplates.map((item) => (
                        <div key={item.id} style={{ position: "relative" }}>
                          <TemplateCard
                            template={item}
                            onTemplateClick={handleTemplateClick}
                          />
                          {!item.installed && (
                            <Button
                              type="primary"
                              icon={<DownloadOutlined />}
                              style={{ position: "absolute", top: 8, right: 8 }}
                              size="small"
                              loading={installing}
                              onClick={(e) => {
                                e.stopPropagation();
                                void handleDownload(item);
                              }}
                            />
                          )}
                        </div>
                      ))
                    )
                    : (
                      <div style={{ gridColumn: "span 3" }}>
                        <Empty description={t("marketplace.noTemplatesFound")} />
                      </div>
                    )}
                </div>
              ),
            },
          ]}
        />
      </main>
      <style>
        {`
        .marketplace-card {
          transition: border-color 0.2s, box-shadow 0.2s;
        }
        .marketplace-card:hover {
          border-color: ${token.colorPrimary} !important;
          box-shadow: 0 4px 12px rgba(0, 0, 0, 0.08);
        }
      `}
      </style>

      <Modal
        title={selectedTemplate?.name}
        open={isDetailOpen}
        onCancel={handleCloseDetail}
        footer={[
          <Button key="close" onClick={handleCloseDetail}>
            {t("common.close")}
          </Button>,
          <Button
            key="download"
            type="primary"
            icon={<DownloadOutlined />}
            loading={installing}
            disabled={selectedTemplate?.installed}
            onClick={() => selectedTemplate && void handleDownload(selectedTemplate)}
          >
            {selectedTemplate?.installed
              ? t("marketplace.installed")
              : t("common.download")}
          </Button>,
        ]}
        width={700}
      >
        {selectedTemplate && (
          <div className="py-4">
            <Tabs
              items={[
                {
                  key: "details",
                  label: t("marketplace.details"),
                  children: (
                    <Space orientation="vertical" className="w-full" size="large">
                      <div>
                        <Text type="secondary">{t("common.description")}</Text>
                        <div>{selectedTemplate.description ?? t("marketplace.noDescription")}</div>
                      </div>
                      <div className="flex gap-8">
                        {selectedTemplate.category && (
                          <div>
                            <Text type="secondary">{t("common.category")}</Text>
                            <div>
                              <Tag>{selectedTemplate.category}</Tag>
                            </div>
                          </div>
                        )}
                        {selectedTemplate.author && (
                          <div>
                            <Text type="secondary">
                              {t("marketplace.author")}
                            </Text>
                            <div>{selectedTemplate.author}</div>
                          </div>
                        )}
                        {selectedTemplate.downloadCount !== undefined && (
                          <div>
                            <Text type="secondary">
                              {t("marketplace.downloads")}
                            </Text>
                            <div>{selectedTemplate.downloadCount}</div>
                          </div>
                        )}
                      </div>
                      {selectedTemplate.ratingAverage !== undefined
                        && selectedTemplate.ratingAverage > 0 && (
                        <div>
                          <Text type="secondary">{t("common.rating")}</Text>
                          <div>
                            <Rate
                              disabled
                              value={selectedTemplate.ratingAverage}
                              allowHalf
                            />
                            <Text className="ml-2">
                              ({selectedTemplate.ratingAverage.toFixed(1)})
                            </Text>
                          </div>
                        </div>
                      )}
                      {selectedTemplate.tags.length > 0 && (
                        <div>
                          <Text type="secondary">{t("common.tags")}</Text>
                          <div className="flex gap-1 mt-1">
                            {selectedTemplate.tags.map((tag) => <Tag key={tag}>{tag}</Tag>)}
                          </div>
                        </div>
                      )}
                    </Space>
                  ),
                },
                {
                  key: "reviews",
                  label: t("marketplace.reviews"),
                  children: renderReviewsTab(),
                },
              ]}
            />
          </div>
        )}
      </Modal>

      <ImportExportModal
        open={importModalOpen}
        onClose={() => setImportModalOpen(false)}
        onImport={handleImportSubmit}
        onExport={async () => null}
        templates={[]}
      />
    </div>
  );
}
