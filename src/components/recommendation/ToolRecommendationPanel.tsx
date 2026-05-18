import { useRecommendationStore } from "@/stores/devtools/recommendationStore";
import { Alert, Button, Card, Divider, Input, List, Progress, Space, Spin, Tag, Typography } from "antd";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

const { TextArea } = Input;
const { Title, Text, Paragraph } = Typography;

export function ToolRecommendationPanel() {
  const { t } = useTranslation();
  const {
    recommendations,
    isLoading,
    error,
    setCurrentTask,
    getRecommendations,
    clearRecommendations,
    fetchAvailableTools,
    availableTools,
  } = useRecommendationStore();

  const [localTask, setLocalTask] = useState("");

  useEffect(() => {
    fetchAvailableTools();
  }, [fetchAvailableTools]);

  const handleAnalyze = () => {
    if (localTask.trim()) {
      setCurrentTask(localTask);
      getRecommendations(localTask);
    }
  };

  const handleClear = () => {
    setLocalTask("");
    clearRecommendations();
  };

  const getScoreColor = (score: number) => {
    if (score >= 0.8) { return "green"; }
    if (score >= 0.6) { return "blue"; }
    if (score >= 0.4) { return "orange"; }
    return "red";
  };

  return (
    <div style={{ padding: "24px" }}>
      <Card title={t("recommendation.title")}>
        <Space direction="vertical" style={{ width: "100%" }} size="large">
          <div>
            <Title level={5}>{t("recommendation.taskDescription")}</Title>
            <TextArea
              placeholder={t("devtools.toolRecommender.taskPlaceholder")}
              value={localTask}
              onChange={(e) => setLocalTask(e.target.value)}
              rows={3}
              autoSize={{ minRows: 2, maxRows: 5 }}
            />
          </div>

          <Space>
            <Button
              type="primary"
              onClick={handleAnalyze}
              loading={isLoading}
              disabled={!localTask.trim()}
            >
              Get Recommendations
            </Button>
            <Button onClick={handleClear} disabled={!localTask.trim()}>
              Clear
            </Button>
          </Space>

          {error && <Alert type="error" message={error} showIcon />}

          {isLoading && (
            <div style={{ textAlign: "center", padding: "40px" }}>
              <Spin size="large" />
              <Paragraph>Analyzing task and generating recommendations…</Paragraph>
            </div>
          )}

          {recommendations && !isLoading && (
            <>
              <Divider />

              <div>
                <Title level={5}>{t("recommendation.analysisResult")}</Title>
                <Progress
                  percent={Math.round(recommendations.confidence * 100)}
                  status={recommendations.confidence >= 0.7 ? "success" : "active"}
                  strokeColor={recommendations.confidence >= 0.7 ? "#52c41a" : "#1890ff"}
                />
                <Paragraph>
                  <Text strong>Reasoning:</Text>
                  <Text>{recommendations.reasoning}</Text>
                </Paragraph>
              </div>

              <Divider />

              <div>
                <Title level={5}>{t("recommendation.recommendedTools")}</Title>
                <List
                  itemLayout="horizontal"
                  dataSource={recommendations.tools}
                  renderItem={(item) => (
                    <List.Item
                      actions={[
                        <Tag color={getScoreColor(item.score)} key={item.tool_id}>
                          Score: {(item.score * 100).toFixed(0)}%
                        </Tag>,
                      ]}
                    >
                      <List.Item.Meta
                        title={item.tool_name}
                        description={
                          <div>
                            {item.reasons.map((reason, _idx) => (
                              <Tag key={reason} style={{ marginBottom: "4px" }}>
                                {reason}
                              </Tag>
                            ))}
                          </div>
                        }
                      />
                    </List.Item>
                  )}
                />
              </div>

              {recommendations.alternatives.length > 0 && (
                <>
                  <Divider />
                  <div>
                    <Title level={5}>{t("recommendation.alternativeApproaches")}</Title>
                    <List
                      itemLayout="horizontal"
                      dataSource={recommendations.alternatives}
                      renderItem={(alt) => (
                        <List.Item>
                          <List.Item.Meta
                            title={alt.description}
                            description={
                              <div>
                                <Text type="secondary">Tools:</Text>
                                {alt.tools.map((tool, _idx) => <Tag key={tool}>{tool}</Tag>)}
                                <br />
                                <Text type="secondary">Tradeoffs:</Text>
                                {alt.tradeoffs.map((tradeoff, _idx) => (
                                  <Tag key={tradeoff} color="default">
                                    {tradeoff}
                                  </Tag>
                                ))}
                              </div>
                            }
                          />
                        </List.Item>
                      )}
                    />
                  </div>
                </>
              )}
            </>
          )}

          {!recommendations && !isLoading && !error && (
            <div style={{ textAlign: "center", padding: "40px", color: "#999" }}>
              <Paragraph>
                Enter a task description and click "Get Recommendations" to see tool suggestions.
              </Paragraph>
            </div>
          )}
        </Space>
      </Card>

      {availableTools.length > 0 && (
        <Card title={t("recommendation.availableTools")} style={{ marginTop: "16px" }}>
          <List
            grid={{ gutter: 16, xs: 1, sm: 2, md: 3, lg: 4 }}
            dataSource={availableTools}
            renderItem={(tool) => (
              <List.Item>
                <Card size="small" title={tool.name}>
                  <Paragraph type="secondary" ellipsis={{ rows: 2 }}>
                    {tool.description}
                  </Paragraph>
                  <div>
                    {tool.categories.map((cat) => (
                      <Tag key={cat}>
                        {cat}
                      </Tag>
                    ))}
                  </div>
                </Card>
              </List.Item>
            )}
          />
        </Card>
      )}
    </div>
  );
}
