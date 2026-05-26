import { useWorkflowEditorStore } from "@/stores";
import { Button, Card, Empty, Input, message, Tabs, Tag, theme } from "antd";
import { Lightbulb, MessageSquare, Plus, Sparkles, Wand2 } from "lucide-react";
import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { setDragPayload } from "../dndState";
import type { WorkflowEdge, WorkflowNode } from "../types";

interface AIPanelProps {
  onGenerateWorkflow: (
    prompt: string,
  ) => Promise<
    {
      nodes: WorkflowNode[];
      edges: WorkflowEdge[];
      explanation?: string;
    } | null
  >;
  onOptimizePrompt: (prompt: string) => Promise<string | null>;
  onRecommendNodes: (
    context: string,
  ) => Promise<
    Array<{
      node_type: string;
      label: string;
      description: string;
      confidence: number;
    }> | null
  >;
  onClose: () => void;
}

export const AIPanel: React.FC<AIPanelProps> = ({
  onGenerateWorkflow,
  onOptimizePrompt,
  onRecommendNodes,
  onClose,
}) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const [activeTab, setActiveTab] = useState("generate");
  const [generatePrompt, setGeneratePrompt] = useState("");
  const [optimizePrompt, setOptimizePrompt] = useState("");
  const [recommendContext, setRecommendContext] = useState("");
  const [isGenerating, setIsGenerating] = useState(false);
  const [isOptimizing, setIsOptimizing] = useState(false);
  const [isRecommending, setIsRecommending] = useState(false);
  const [optimizedResult, setOptimizedResult] = useState<string | null>(null);
  const [recommendedNodes, setRecommendedNodes] = useState<
    Array<{
      node_type: string;
      label: string;
      description: string;
      confidence: number;
    }> | null
  >(null);
  const [generationExplanation, setGenerationExplanation] = useState<
    string | null
  >(null);

  const { nodes, edges, setNodes, setEdges } = useWorkflowEditorStore();

  const handleGenerate = async () => {
    if (!generatePrompt.trim()) {
      message.warning(t("workflow.aiPanel.enterWorkflowDesc"));
      return;
    }
    // 画布已有节点时弹出确认
    if (nodes.length > 0 || edges.length > 0) {
      const { Modal: AntModal } = await import("antd");
      const confirmed = await new Promise<boolean>((resolve) => {
        AntModal.confirm({
          title: t("workflow.aiPanel.replaceConfirmTitle"),
          content: t("workflow.aiPanel.replaceConfirmContent", { nodes: nodes.length, edges: edges.length }),
          okText: t("common.confirm"),
          cancelText: t("common.cancel"),
          onOk: () => resolve(true),
          onCancel: () => resolve(false),
        });
      });
      if (!confirmed) { return; }
    }
    setIsGenerating(true);
    try {
      const result = await onGenerateWorkflow(generatePrompt);
      if (result) {
        setNodes(result.nodes);
        setEdges(result.edges);
        setGenerationExplanation(result.explanation || null);
        message.success(t("workflow.aiPanel.workflowGenerated"));
      }
    } catch (error) {
      console.error("[AIPanel] generate failed:", error);
      message.error(t("workflow.aiPanel.generationFailed"));
    } finally {
      setIsGenerating(false);
    }
  };

  const handleOptimize = async () => {
    if (!optimizePrompt.trim()) {
      message.warning(t("workflow.aiPanel.enterPromptToOptimize"));
      return;
    }
    setIsOptimizing(true);
    setOptimizedResult(null);
    try {
      const result = await onOptimizePrompt(optimizePrompt);
      if (result) {
        setOptimizedResult(result);
        message.success(t("workflow.aiPanel.promptOptimized"));
      }
    } catch (error) {
      console.error("[AIPanel] optimize failed:", error);
      message.error(t("workflow.aiPanel.optimizationFailed"));
    } finally {
      setIsOptimizing(false);
    }
  };

  const handleRecommend = async () => {
    if (!recommendContext.trim()) {
      message.warning(t("workflow.aiPanel.enterContext"));
      return;
    }
    setIsRecommending(true);
    setRecommendedNodes(null);
    try {
      const result = await onRecommendNodes(recommendContext);
      if (result) {
        setRecommendedNodes(result);
        message.success(t("workflow.aiPanel.recommendationGenerated"));
      }
    } catch (error) {
      console.error("[AIPanel] recommend failed:", error);
      message.error(t("workflow.aiPanel.recommendationFailed"));
    } finally {
      setIsRecommending(false);
    }
  };

  const handleCopyOptimized = () => {
    if (optimizedResult) {
      navigator.clipboard.writeText(optimizedResult);
      message.success(t("workflow.aiPanel.copiedToClipboard"));
    }
  };

  const tabItems = [
    {
      key: "generate",
      label: (
        <span style={{ display: "flex", alignItems: "center", gap: 6 }}>
          <Wand2 size={14} />
          {t("workflow.aiPanel.tabGenerateWorkflow")}
        </span>
      ),
      children: (
        <div style={{ padding: "16px 0" }}>
          <div style={{ marginBottom: 16 }}>
            <label
              style={{
                display: "block",
                color: token.colorTextSecondary,
                fontSize: 12,
                marginBottom: 8,
              }}
            >
              {t("workflow.aiPanel.describeWorkflow")}
            </label>
            <Input.TextArea
              id="a-i-panel-input-textarea-70"
              placeholder={t("workflow.aiPanel.generatePlaceholder")}
              value={generatePrompt}
              onChange={(e) => setGeneratePrompt(e.target.value)}
              rows={6}
              style={{
                background: token.colorBgContainer,
                fontSize: 13,
              }}
            />
          </div>

          <div style={{ marginBottom: 12 }}>
            <Button
              type="primary"
              icon={<Sparkles size={14} />}
              onClick={handleGenerate}
              loading={isGenerating}
              disabled={isGenerating}
              style={{ width: "100%" }}
            >
              {isGenerating
                ? t("workflow.aiPanel.generating")
                : t("workflow.aiPanel.generateBtn")}
            </Button>
          </div>

          {generationExplanation && (
            <Card
              size="small"
              style={{
                background: token.colorBgContainer,
                border: `1px solid ${token.colorBorderSecondary}`,
                marginBottom: 12,
              }}
              styles={{ body: { padding: 12 } }}
            >
              <div
                style={{
                  display: "flex",
                  justifyContent: "space-between",
                  alignItems: "flex-start",
                  marginBottom: 4,
                }}
              >
                <span
                  style={{
                    color: token.colorTextSecondary,
                    fontSize: 12,
                    fontWeight: 500,
                  }}
                >
                  {t("workflow.aiPanel.explanation")}
                </span>
                <Button
                  type="text"
                  size="small"
                  onClick={() => setGenerationExplanation(null)}
                  style={{
                    color: token.colorTextTertiary,
                    fontSize: 12,
                    minWidth: "auto",
                    padding: "0 4px",
                  }}
                >
                  ✕
                </Button>
              </div>
              <pre
                style={{
                  whiteSpace: "pre-wrap",
                  fontSize: 12,
                  color: token.colorTextSecondary,
                  margin: 0,
                }}
              >
                {generationExplanation}
              </pre>
            </Card>
          )}

          <div style={{ color: token.colorTextTertiary, fontSize: 12 }}>
            <strong>{t("workflow.aiPanel.currentCanvasState")}</strong>
            {t("workflow.aiPanel.canvasStatus", {
              nodes: nodes.length,
              edges: edges.length,
            })}
            <br />
            {t("workflow.aiPanel.replaceCanvasWarning")}
          </div>
        </div>
      ),
    },
    {
      key: "optimize",
      label: (
        <span style={{ display: "flex", alignItems: "center", gap: 6 }}>
          <MessageSquare size={14} />
          {t("workflow.aiPanel.tabOptimizePrompt")}
        </span>
      ),
      children: (
        <div style={{ padding: "16px 0" }}>
          <div style={{ marginBottom: 16 }}>
            <label
              style={{
                display: "block",
                color: token.colorTextSecondary,
                fontSize: 12,
                marginBottom: 8,
              }}
            >
              {t("workflow.aiPanel.enterAgentPrompt")}
            </label>
            <Input.TextArea
              id="a-i-panel-input-textarea-71"
              placeholder={t("workflow.aiPanel.optimizePlaceholder")}
              value={optimizePrompt}
              onChange={(e) => setOptimizePrompt(e.target.value)}
              rows={6}
              style={{
                background: token.colorBgContainer,
                fontSize: 13,
              }}
            />
          </div>

          <Button
            type="primary"
            icon={<Sparkles size={14} />}
            onClick={handleOptimize}
            loading={isOptimizing}
            disabled={isOptimizing}
            style={{ width: "100%", marginBottom: 16 }}
          >
            {isOptimizing
              ? t("workflow.aiPanel.optimizing")
              : t("workflow.aiPanel.optimizeBtn")}
          </Button>

          {optimizedResult && (
            <div>
              <div
                style={{
                  display: "flex",
                  justifyContent: "space-between",
                  alignItems: "center",
                  marginBottom: 8,
                }}
              >
                <label
                  style={{ color: token.colorTextSecondary, fontSize: 12 }}
                >
                  {t("workflow.aiPanel.optimizedResult")}
                </label>
                <Button type="text" size="small" onClick={handleCopyOptimized}>
                  {t("workflow.aiPanel.copy")}
                </Button>
              </div>
              <Card
                size="small"
                style={{
                  background: token.colorBgContainer,
                  border: `1px solid ${token.colorBorderSecondary}`,
                }}
                styles={{ body: { padding: 12 } }}
              >
                <pre
                  style={{
                    whiteSpace: "pre-wrap",
                    fontSize: 12,
                    color: token.colorTextSecondary,
                    margin: 0,
                  }}
                >
                  {optimizedResult}
                </pre>
              </Card>
            </div>
          )}
        </div>
      ),
    },
    {
      key: "recommend",
      label: (
        <span style={{ display: "flex", alignItems: "center", gap: 6 }}>
          <Lightbulb size={14} />
          {t("workflow.aiPanel.tabRecommend")}
        </span>
      ),
      children: (
        <div style={{ padding: "16px 0" }}>
          <div style={{ marginBottom: 16 }}>
            <label
              style={{
                display: "block",
                color: token.colorTextSecondary,
                fontSize: 12,
                marginBottom: 8,
              }}
            >
              {t("workflow.aiPanel.describeContext")}
            </label>
            <Input.TextArea
              id="a-i-panel-input-textarea-72"
              placeholder={t("workflow.aiPanel.recommendPlaceholder")}
              value={recommendContext}
              onChange={(e) => setRecommendContext(e.target.value)}
              rows={4}
              style={{
                background: token.colorBgContainer,
                fontSize: 13,
              }}
            />
          </div>

          <Button
            type="primary"
            icon={<Sparkles size={14} />}
            onClick={handleRecommend}
            loading={isRecommending}
            disabled={isRecommending}
            style={{ width: "100%", marginBottom: 16 }}
          >
            {isRecommending
              ? t("workflow.aiPanel.recommending")
              : t("workflow.aiPanel.getRecommendation")}
          </Button>

          {recommendedNodes && (
            <div>
              <label
                style={{
                  color: token.colorTextSecondary,
                  fontSize: 12,
                  marginBottom: 8,
                  display: "block",
                }}
              >
                {t("workflow.aiPanel.recommendedNodeTypes")}
              </label>
              <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                {recommendedNodes.map((node, _index) => (
                  <Card
                    key={`${node.node_type}-${node.label}`}
                    size="small"
                    style={{
                      background: token.colorBgContainer,
                      border: `1px solid ${token.colorBorderSecondary}`,
                      cursor: "pointer",
                      transition: "border-color 0.2s",
                    }}
                    styles={{ body: { padding: "8px 12px" } }}
                    hoverable
                    onClick={() => {
                      setDragPayload({
                        type: node.node_type,
                        label: node.label,
                      });
                      message.info(
                        t("workflow.aiPanel.clickCanvasToAdd", {
                          label: node.label,
                        }),
                      );
                    }}
                  >
                    <div
                      style={{
                        display: "flex",
                        justifyContent: "space-between",
                        alignItems: "center",
                      }}
                    >
                      <div style={{ flex: 1, minWidth: 0 }}>
                        <div
                          style={{
                            display: "flex",
                            alignItems: "center",
                            gap: 6,
                            marginBottom: 2,
                          }}
                        >
                          <Tag
                            color="blue"
                            style={{
                              fontSize: 12,
                              margin: 0,
                              padding: "0 6px",
                            }}
                          >
                            {node.node_type}
                          </Tag>
                          <span
                            style={{
                              color: token.colorText,
                              fontSize: 12,
                              fontWeight: 500,
                            }}
                          >
                            {node.label}
                          </span>
                        </div>
                        {node.description && (
                          <div
                            style={{
                              color: token.colorTextSecondary,
                              fontSize: 12,
                              marginTop: 4,
                              lineHeight: 1.4,
                            }}
                          >
                            {node.description}
                          </div>
                        )}
                      </div>
                      <div
                        style={{
                          display: "flex",
                          alignItems: "center",
                          gap: 6,
                          marginLeft: 8,
                          flexShrink: 0,
                        }}
                      >
                        <span
                          style={{
                            color: node.confidence >= 0.8
                              ? token.colorSuccess
                              : node.confidence >= 0.5
                              ? token.colorWarning
                              : token.colorError,
                            fontSize: 12,
                            fontWeight: 500,
                          }}
                        >
                          {Math.round(node.confidence * 100)}%
                        </span>
                        <Plus size={14} color={token.colorTextTertiary} />
                      </div>
                    </div>
                  </Card>
                ))}
              </div>
              <div
                style={{
                  color: token.colorTextTertiary,
                  fontSize: 12,
                  marginTop: 12,
                }}
              >
                {t("workflow.aiPanel.dragHint")}
              </div>
            </div>
          )}

          {recommendedNodes && recommendedNodes.length === 0 && (
            <Empty
              description={t("workflow.aiPanel.noRecommendations")}
              image={Empty.PRESENTED_IMAGE_SIMPLE}
            />
          )}
        </div>
      ),
    },
  ];

  return (
    <div
      style={{
        width: "100%",
        height: "100%",
        background: token.colorBgElevated,
        display: "flex",
        flexDirection: "column",
      }}
    >
      <div
        style={{
          padding: "8px 16px",
          borderBottom: `1px solid ${token.colorBorderSecondary}`,
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <Sparkles size={16} color={token.colorPrimary} />
          <span style={{ fontWeight: 500, color: token.colorText }}>
            {t("workflow.aiPanel.aiAssistant")}
          </span>
        </div>
        {onClose && (
          <Button type="text" size="small" onClick={onClose} style={{ color: token.colorTextTertiary }}>
            ✕
          </Button>
        )}
      </div>

      <div style={{ flex: 1, overflow: "auto", padding: "0 16px" }}>
        <Tabs
          activeKey={activeTab}
          onChange={setActiveTab}
          tabPlacement="top"
          size="small"
          items={tabItems}
          style={{ height: "100%" }}
        />
      </div>
    </div>
  );
};
