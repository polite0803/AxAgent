// SPDX-License-Identifier: AGPL-3.0-only

import { useRecommendationStore } from "@/stores/devtools/recommendationStore";
import { Alert, Button, Card, Divider, Input, Progress, Space, Spin, Tag, Typography } from "antd";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

const {} = Input;
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
    if (score >= 0.8) {
      return "green";
    }
    if (score >= 0.6) {
      return "blue";
    }
    if (score >= 0.4) {
      return "orange";
    }
    return "red";
  };

  return (
    <div style={{ padding: "24px" }}>
      <Card title={t("recommendation.title")}>
        <Space orientation="vertical" style={{ width: "100%" }} size="large">
          <div>
            <Title level={5}>{t("recommendation.taskDescription")}</Title>
            <Input.TextArea
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

          {error && <Alert type="error" title={error} showIcon />}

          {isLoading && (
            <div style={{ textAlign: "center", padding: "40px" }}>
              <Spin size="large" />
              <Paragraph>
                Analyzing task and generating recommendations…
              </Paragraph>
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
                <div className="divide-y divide-gray-100">
                  {recommendations.tools.map((item) => (
                    <div
                      key={item.tool_id}
                      style={{
                        padding: "12px 0",
                        display: "flex",
                        justifyContent: "space-between",
                        alignItems: "flex-start",
                      }}
                    >
                      <div style={{ flex: 1 }}>
                        <div style={{ fontWeight: 500 }}>
                          {item.tool_name}
                        </div>
                        <div
                          style={{
                            color: "var(--text-secondary, rgba(0,0,0,0.45))",
                            fontSize: 13,
                            marginTop: 2,
                          }}
                        >
                          <div>
                            {item.reasons.map((reason, _idx) => (
                              <Tag key={reason} style={{ marginBottom: "4px" }}>
                                {reason}
                              </Tag>
                            ))}
                          </div>
                        </div>
                      </div>
                      <Tag color={getScoreColor(item.score)}>
                        Score: {(item.score * 100).toFixed(0)}%
                      </Tag>
                    </div>
                  ))}
                </div>
              </div>

              {recommendations.alternatives.length > 0 && (
                <>
                  <Divider />
                  <div>
                    <Title level={5}>
                      {t("recommendation.alternativeApproaches")}
                    </Title>
                    <div className="divide-y divide-gray-100">
                      {recommendations.alternatives.map((alt) => (
                        <div key={alt.description} style={{ padding: "12px 0" }}>
                          <div style={{ fontWeight: 500 }}>
                            {alt.description}
                          </div>
                          <div
                            style={{
                              color: "var(--text-secondary, rgba(0,0,0,0.45))",
                              fontSize: 13,
                              marginTop: 2,
                            }}
                          >
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
                          </div>
                        </div>
                      ))}
                    </div>
                  </div>
                </>
              )}
            </>
          )}

          {!recommendations && !isLoading && !error && (
            <div
              style={{ textAlign: "center", padding: "40px", color: "#999" }}
            >
              <Paragraph>
                Enter a task description and click "Get Recommendations" to see tool suggestions.
              </Paragraph>
            </div>
          )}
        </Space>
      </Card>

      {availableTools.length > 0 && (
        <Card
          title={t("recommendation.availableTools")}
          style={{ marginTop: "16px" }}
        >
          <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-4">
            {availableTools.map((tool) => (
              <div key={tool.name}>
                <Card size="small" title={tool.name}>
                  <Paragraph type="secondary" ellipsis={{ rows: 2 }}>
                    {tool.description}
                  </Paragraph>
                  <div>
                    {tool.categories.map((cat) => <Tag key={cat}>{cat}</Tag>)}
                  </div>
                </Card>
              </div>
            ))}
          </div>
        </Card>
      )}
    </div>
  );
}
