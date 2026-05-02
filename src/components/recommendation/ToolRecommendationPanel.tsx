import { useRecommendationStore } from "@/stores/devtools/recommendationStore";
import { Alert, Button, Card, Divider, Input, List, Progress, Space, Spin, Tag, Typography } from "antd";
import { useEffect, useState } from "react";

const { TextArea } = Input;
const { Title, Text, Paragraph } = Typography;

export function ToolRecommendationPanel() {
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
      <Card title="Tool Recommendation">
        <Space direction="vertical" style={{ width: "100%" }} size="large">
          <div>
            <Title level={5}>Task Description</Title>
            <TextArea
              placeholder="Describe your task (e.g., 'I need to search for information about Rust programming and write it to a file')"
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
              <Paragraph>Analyzing task and generating recommendations...</Paragraph>
            </div>
          )}

          {recommendations && !isLoading && (
            <>
              <Divider />

              <div>
                <Title level={5}>Analysis Result</Title>
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
                <Title level={5}>Recommended Tools</Title>
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
                            {item.reasons.map((reason, idx) => (
                              <Tag key={idx} style={{ marginBottom: "4px" }}>
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
                    <Title level={5}>Alternative Approaches</Title>
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
                                {alt.tools.map((tool, idx) => <Tag key={idx}>{tool}</Tag>)}
                                <br />
                                <Text type="secondary">Tradeoffs:</Text>
                                {alt.tradeoffs.map((tradeoff, idx) => (
                                  <Tag key={idx} color="default">
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
        <Card title="Available Tools" style={{ marginTop: "16px" }}>
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
