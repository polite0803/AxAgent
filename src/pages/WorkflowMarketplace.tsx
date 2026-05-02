import { MarketplaceStats, reviewApi, ReviewResponse } from "@/lib/reviewApi";
import { DownloadOutlined, DownloadOutlined as DLOutlined, StarOutlined, UploadOutlined } from "@ant-design/icons";
import {
  Button,
  Card,
  Empty,
  Form,
  Input,
  List,
  message,
  Modal,
  Rate,
  Space,
  Spin,
  Tabs,
  Tag,
  theme,
  Typography,
} from "antd";
import { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";

const { Title, Text } = Typography;
const { Search } = Input;

interface MarketplaceTemplate {
  id: string;
  name: string;
  description?: string;
  category: string;
  author: string;
  downloads: number;
  rating: number;
  isFeatured: boolean;
  icon: string;
  tags?: string[];
}

const mockTemplates: MarketplaceTemplate[] = [
  {
    id: "1",
    name: "Document Summarizer",
    description: "Automatically summarize long documents into concise summaries",
    category: "Productivity",
    author: "System",
    downloads: 1250,
    rating: 4.5,
    isFeatured: true,
    icon: "FileText",
    tags: ["summarization", "documents"],
  },
  {
    id: "2",
    name: "Code Review Assistant",
    description: "AI-powered code review with best practices suggestions",
    category: "Development",
    author: "Community",
    downloads: 890,
    rating: 4.8,
    isFeatured: true,
    icon: "Code",
    tags: ["code", "review"],
  },
  {
    id: "3",
    name: "Data Pipeline Builder",
    description: "Build and manage ETL data pipelines",
    category: "Data",
    author: "Community",
    downloads: 567,
    rating: 4.2,
    isFeatured: false,
    icon: "Database",
    tags: ["data", "pipeline"],
  },
  {
    id: "4",
    name: "Customer Support Bot",
    description: "Automated customer support workflow",
    category: "Business",
    author: "System",
    downloads: 2100,
    rating: 4.6,
    isFeatured: true,
    icon: "CustomerService",
    tags: ["support", "automation"],
  },
];

const categories = ["All", "Productivity", "Development", "Data", "Automation", "AI", "Business"];

function formatDate(timestamp: number): string {
  return new Date(timestamp * 1000).toLocaleDateString();
}

export function WorkflowMarketplace() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const [templates] = useState<MarketplaceTemplate[]>(mockTemplates);
  const [searchText, setSearchText] = useState("");
  const [selectedCategory, setSelectedCategory] = useState("All");
  const [selectedTemplate, setSelectedTemplate] = useState<MarketplaceTemplate | null>(null);
  const [isDetailOpen, setIsDetailOpen] = useState(false);
  const [activeTab, setActiveTab] = useState("templates");

  const [reviews, setReviews] = useState<ReviewResponse[]>([]);
  const [myReview, setMyReview] = useState<ReviewResponse | null>(null);
  const [stats, setStats] = useState<MarketplaceStats | null>(null);
  const [loadingReviews, setLoadingReviews] = useState(false);
  const [submittingReview, setSubmittingReview] = useState(false);

  const [reviewForm] = Form.useForm();

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
      console.error("Failed to load reviews:", error);
    } finally {
      setLoadingReviews(false);
    }
  }, []);

  const handleTemplateClick = (template: MarketplaceTemplate) => {
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

  const handleSubmitReview = async (values: { rating: number; comment?: string }) => {
    if (!selectedTemplate) { return; }

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
    } catch (error) {
      message.error(error instanceof Error ? error.message : t("review.failedToSubmit"));
    } finally {
      setSubmittingReview(false);
    }
  };

  const handleDeleteReview = async () => {
    if (!myReview) { return; }

    try {
      await reviewApi.deleteReview(myReview.id);
      message.success(t("review.deletedSuccess"));
      if (selectedTemplate) {
        loadReviews(selectedTemplate.id);
      }
    } catch (error) {
      message.error(error instanceof Error ? error.message : t("review.failedToDelete"));
    }
  };

  const filteredTemplates = templates.filter((t) => {
    const matchesSearch = t.name.toLowerCase().includes(searchText.toLowerCase())
      || t.description?.toLowerCase().includes(searchText.toLowerCase());
    const matchesCategory = selectedCategory === "All" || t.category === selectedCategory;
    return matchesSearch && matchesCategory;
  });

  const handleDownload = (template: MarketplaceTemplate) => {
    message.success(t("marketplace.downloading", { name: template.name }));
  };

  const handleImport = () => {
    message.info(t("marketplace.importInfo"));
  };

  const renderTemplateCard = (template: MarketplaceTemplate) => (
    <Card
      hoverable
      className="marketplace-card"
      onClick={() => handleTemplateClick(template)}
      cover={
        <div
          className="flex items-center justify-center h-32"
          style={{
            backgroundColor: token.colorBgContainer,
            borderBottom: `1px solid ${token.colorBorderSecondary}`,
          }}
        >
          <span style={{ fontSize: 48 }}>📄</span>
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
            {template.isFeatured && <Tag color="gold" style={{ margin: 0 }}>{t("marketplace.featured")}</Tag>}
          </Space>
        }
        description={
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            <Text type="secondary" style={{ fontSize: 12, display: "block" }}>
              {template.description}
            </Text>
            <Space size={8}>
              <Tag color="blue" style={{ margin: 0, fontSize: 12 }}>
                <StarOutlined style={{ fontSize: 12, marginRight: 4 }} />
                {template.rating}
              </Tag>
              <Text type="secondary" style={{ fontSize: 12 }}>
                <DownloadOutlined style={{ fontSize: 12, marginRight: 4 }} />
                {template.downloads}
              </Text>
            </Space>
          </div>
        }
      />
    </Card>
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
            <Text type="secondary">({stats.total_reviews} {t("marketplace.reviews")})</Text>
          </div>
        )}
      </div>

      <div className="border p-4 rounded">
        <Title level={5} className="m-0 mb-4">
          {myReview ? t("marketplace.yourReview") : t("marketplace.writeReview")}
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
                    reviewForm.setFieldsValue({ rating: myReview.rating, comment: myReview.comment || "" });
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
            <Form form={reviewForm} onFinish={handleSubmitReview} layout="vertical">
              <Form.Item
                name="rating"
                label={t("common.rating")}
                rules={[{ required: true, message: t("review.ratingRequired") }]}
              >
                <Rate />
              </Form.Item>
              <Form.Item name="comment" label={t("common.comment")}>
                <Input.TextArea rows={3} placeholder={t("review.commentPlaceholder")} />
              </Form.Item>
              <Form.Item>
                <Button type="primary" htmlType="submit" loading={submittingReview}>
                  {t("marketplace.submitReview")}
                </Button>
              </Form.Item>
            </Form>
          )}
      </div>

      <Spin spinning={loadingReviews}>
        <List
          header={<Title level={5}>{t("marketplace.allReviews")}</Title>}
          dataSource={reviews}
          locale={{ emptyText: t("marketplace.noReviews") }}
          renderItem={(item) => (
            <List.Item>
              <List.Item.Meta
                avatar={<Rate disabled value={item.rating} />}
                title={`User ${item.user_id.slice(0, 8)}`}
                description={
                  <div>
                    {item.comment && <p>{item.comment}</p>}
                    <Text type="secondary" className="text-xs">
                      {formatDate(item.created_at)}
                    </Text>
                  </div>
                }
              />
            </List.Item>
          )}
        />
      </Spin>
    </div>
  );

  return (
    <div className="flex h-full" style={{ backgroundColor: token.colorBgElevated }}>
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
          {categories.map((cat) => (
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
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 24 }}>
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
                <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: 16 }}>
                  {filteredTemplates.length > 0
                    ? (
                      filteredTemplates.map((t) => (
                        <div key={t.id} style={{ position: "relative" }}>
                          {renderTemplateCard(t)}
                          <Button
                            type="primary"
                            icon={<DLOutlined />}
                            style={{ position: "absolute", top: 8, right: 8 }}
                            size="small"
                            onClick={(e) => {
                              e.stopPropagation();
                              handleDownload(t);
                            }}
                          />
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
            {
              key: "featured",
              label: t("marketplace.featured"),
              children: (
                <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: 16 }}>
                  {templates
                    .filter((t) => t.isFeatured)
                    .map((t) => (
                      <div key={t.id} style={{ position: "relative" }}>
                        {renderTemplateCard(t)}
                        <Button
                          type="primary"
                          icon={<DLOutlined />}
                          style={{ position: "absolute", top: 8, right: 8 }}
                          size="small"
                          onClick={(e) => {
                            e.stopPropagation();
                            handleDownload(t);
                          }}
                        />
                      </div>
                    ))}
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
            onClick={() => selectedTemplate && handleDownload(selectedTemplate)}
          >
            {t("common.download")}
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
                    <Space direction="vertical" className="w-full" size="large">
                      <div>
                        <Text type="secondary">{t("common.description")}</Text>
                        <div>{selectedTemplate.description}</div>
                      </div>
                      <div className="flex gap-8">
                        <div>
                          <Text type="secondary">{t("common.category")}</Text>
                          <div>
                            <Tag>{selectedTemplate.category}</Tag>
                          </div>
                        </div>
                        <div>
                          <Text type="secondary">{t("marketplace.author")}</Text>
                          <div>{selectedTemplate.author}</div>
                        </div>
                        <div>
                          <Text type="secondary">{t("marketplace.downloads")}</Text>
                          <div>{selectedTemplate.downloads}</div>
                        </div>
                      </div>
                      <div>
                        <Text type="secondary">{t("common.rating")}</Text>
                        <div>
                          <Rate disabled defaultValue={selectedTemplate.rating} allowHalf />
                          <Text className="ml-2">({selectedTemplate.rating})</Text>
                        </div>
                      </div>
                      {selectedTemplate.tags && (
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
    </div>
  );
}
