import { useState, useCallback } from "react";
import {
  Card,
  Input,
  Button,
  Tag,
  Space,
  Typography,
  Rate,
  Empty,
  Modal,
  message,
  Tabs,
  List,
  Spin,
  Form,
} from "antd";
import {
  DownloadOutlined,
  UploadOutlined,
  StarOutlined,
  DownloadOutlined as DLOutlined,
} from "@ant-design/icons";
import { reviewApi, ReviewResponse, MarketplaceStats } from "@/lib/reviewApi";

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
    if (!selectedTemplate) return;

    setSubmittingReview(true);
    try {
      if (myReview) {
        await reviewApi.updateReview(myReview.id, values);
        message.success("Review updated successfully");
      } else {
        await reviewApi.createReview({
          marketplace_id: selectedTemplate.id,
          rating: values.rating,
          comment: values.comment,
        });
        message.success("Review submitted successfully");
      }
      loadReviews(selectedTemplate.id);
      reviewForm.resetFields();
    } catch (error) {
      message.error(error instanceof Error ? error.message : "Failed to submit review");
    } finally {
      setSubmittingReview(false);
    }
  };

  const handleDeleteReview = async () => {
    if (!myReview) return;

    try {
      await reviewApi.deleteReview(myReview.id);
      message.success("Review deleted successfully");
      if (selectedTemplate) {
        loadReviews(selectedTemplate.id);
      }
    } catch (error) {
      message.error(error instanceof Error ? error.message : "Failed to delete review");
    }
  };

  const filteredTemplates = templates.filter((t) => {
    const matchesSearch =
      t.name.toLowerCase().includes(searchText.toLowerCase()) ||
      t.description?.toLowerCase().includes(searchText.toLowerCase());
    const matchesCategory = selectedCategory === "All" || t.category === selectedCategory;
    return matchesSearch && matchesCategory;
  });

  const handleDownload = (template: MarketplaceTemplate) => {
    message.success(`Downloading "${template.name}"...`);
  };

  const handleImport = () => {
    message.info("Import functionality - select a workflow file to import");
  };

  const renderTemplateCard = (template: MarketplaceTemplate) => (
    <Card
      hoverable
      className="marketplace-card"
      onClick={() => handleTemplateClick(template)}
      cover={
        <div className="flex items-center justify-center h-32 bg-linear-to-br from-blue-50 to-indigo-100">
          <span className="text-4xl">📄</span>
        </div>
      }
    >
      <Card.Meta
        title={
          <Space>
            {template.name}
            {template.isFeatured && <Tag color="gold">Featured</Tag>}
          </Space>
        }
        description={
          <div>
            <Text type="secondary" className="text-sm block mb-2">
              {template.description}
            </Text>
            <Space className="mt-2">
              <Tag icon={<StarOutlined />}>{template.rating}</Tag>
              <Text type="secondary" className="text-xs">
                <DownloadOutlined /> {template.downloads}
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
          Customer Reviews
        </Title>
        {stats && (
          <div className="flex items-center gap-2">
            <Rate disabled value={stats.rating_average} allowHalf />
            <Text>{stats.rating_average.toFixed(1)}</Text>
            <Text type="secondary">({stats.total_reviews} reviews)</Text>
          </div>
        )}
      </div>

      <div className="border p-4 rounded">
        <Title level={5} className="m-0 mb-4">
          {myReview ? "Your Review" : "Write a Review"}
        </Title>
        {myReview ? (
          <div className="space-y-2">
            <Rate disabled value={myReview.rating} />
            {myReview.comment && <p>{myReview.comment}</p>}
            <Text type="secondary" className="text-xs">
              Posted on {formatDate(myReview.created_at)}
            </Text>
            <div className="flex gap-2 mt-2">
              <Button
                size="small"
                onClick={() => {
                  reviewForm.setFieldsValue({ rating: myReview.rating, comment: myReview.comment || "" });
                }}
              >
                Edit
              </Button>
              <Button size="small" danger onClick={handleDeleteReview}>
                Delete
              </Button>
            </div>
          </div>
        ) : (
          <Form form={reviewForm} onFinish={handleSubmitReview} layout="vertical">
            <Form.Item name="rating" label="Rating" rules={[{ required: true, message: "Please select a rating" }]}>
              <Rate />
            </Form.Item>
            <Form.Item name="comment" label="Comment">
              <Input.TextArea rows={3} placeholder="Share your experience..." />
            </Form.Item>
            <Form.Item>
              <Button type="primary" htmlType="submit" loading={submittingReview}>
                Submit Review
              </Button>
            </Form.Item>
          </Form>
        )}
      </div>

      <Spin spinning={loadingReviews}>
        <List
          header={<Title level={5}>All Reviews</Title>}
          dataSource={reviews}
          locale={{ emptyText: "No reviews yet" }}
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
    <div className="flex h-full">
      <aside className="w-56 border-r p-4 bg-white">
        <Title level={5} className="mb-4">
          Categories
        </Title>
        <div className="flex flex-col gap-1">
          {categories.map((cat) => (
            <Button
              key={cat}
              type={selectedCategory === cat ? "primary" : "text"}
              className="text-left justify-start"
              onClick={() => setSelectedCategory(cat)}
              block
            >
              {cat}
            </Button>
          ))}
        </div>

        <Title level={5} className="mt-6 mb-4">
          Quick Actions
        </Title>
        <div className="flex flex-col gap-2">
          <Button icon={<UploadOutlined />} onClick={handleImport} block>
            Import Workflow
          </Button>
        </div>
      </aside>

      <main className="flex-1 overflow-y-auto p-6 bg-gray-50">
        <div className="flex items-center justify-between mb-6">
          <Title level={4} className="m-0">
            Workflow Marketplace
          </Title>
          <Space>
            <Search
              placeholder="Search workflows..."
              allowClear
              onSearch={setSearchText}
              onChange={(e) => setSearchText(e.target.value)}
              className="w-64"
            />
          </Space>
        </div>

        <Tabs
          activeKey={activeTab}
          onChange={setActiveTab}
          items={[
            {
              key: "templates",
              label: "Templates",
              children: (
                <div className="grid grid-cols-3 gap-4">
                  {filteredTemplates.length > 0 ? (
                    filteredTemplates.map((t) => (
                      <div key={t.id} className="relative">
                        {renderTemplateCard(t)}
                        <Button
                          type="primary"
                          icon={<DLOutlined />}
                          className="absolute top-2 right-2"
                          size="small"
                          onClick={(e) => {
                            e.stopPropagation();
                            handleDownload(t);
                          }}
                        />
                      </div>
                    ))
                  ) : (
                    <Empty description="No templates found" className="col-span-3" />
                  )}
                </div>
              ),
            },
            {
              key: "featured",
              label: "Featured",
              children: (
                <div className="grid grid-cols-3 gap-4">
                  {templates
                    .filter((t) => t.isFeatured)
                    .map((t) => (
                      <div key={t.id} className="relative">
                        {renderTemplateCard(t)}
                        <Button
                          type="primary"
                          icon={<DLOutlined />}
                          className="absolute top-2 right-2"
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

      <Modal
        title={selectedTemplate?.name}
        open={isDetailOpen}
        onCancel={handleCloseDetail}
        footer={[
          <Button key="close" onClick={handleCloseDetail}>
            Close
          </Button>,
          <Button
            key="download"
            type="primary"
            icon={<DownloadOutlined />}
            onClick={() => selectedTemplate && handleDownload(selectedTemplate)}
          >
            Download
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
                  label: "Details",
                  children: (
                    <Space direction="vertical" className="w-full" size="large">
                      <div>
                        <Text type="secondary">Description</Text>
                        <div>{selectedTemplate.description}</div>
                      </div>
                      <div className="flex gap-8">
                        <div>
                          <Text type="secondary">Category</Text>
                          <div>
                            <Tag>{selectedTemplate.category}</Tag>
                          </div>
                        </div>
                        <div>
                          <Text type="secondary">Author</Text>
                          <div>{selectedTemplate.author}</div>
                        </div>
                        <div>
                          <Text type="secondary">Downloads</Text>
                          <div>{selectedTemplate.downloads}</div>
                        </div>
                      </div>
                      <div>
                        <Text type="secondary">Rating</Text>
                        <div>
                          <Rate disabled defaultValue={selectedTemplate.rating} allowHalf />
                          <Text className="ml-2">({selectedTemplate.rating})</Text>
                        </div>
                      </div>
                      {selectedTemplate.tags && (
                        <div>
                          <Text type="secondary">Tags</Text>
                          <div className="flex gap-1 mt-1">
                            {selectedTemplate.tags.map((tag) => (
                              <Tag key={tag}>{tag}</Tag>
                            ))}
                          </div>
                        </div>
                      )}
                    </Space>
                  ),
                },
                {
                  key: "reviews",
                  label: "Reviews",
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
