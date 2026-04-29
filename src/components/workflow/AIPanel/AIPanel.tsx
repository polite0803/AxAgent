import { useWorkflowEditorStore } from "@/stores";
import { Button, Card, Empty, Input, message, Tabs, Tag } from "antd";
import { Lightbulb, MessageSquare, Sparkles, Wand2 } from "lucide-react";
import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import type { WorkflowEdge, WorkflowNode } from "../types";

interface AIPanelProps {
  onGenerateWorkflow: (prompt: string) => Promise<{ nodes: WorkflowNode[]; edges: WorkflowEdge[] } | null>;
  onOptimizePrompt: (prompt: string) => Promise<string | null>;
  onRecommendNodes: (context: string) => Promise<string[] | null>;
  onClose: () => void;
}

export const AIPanel: React.FC<AIPanelProps> = ({
  onGenerateWorkflow,
  onOptimizePrompt,
  onRecommendNodes,
}) => {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState("generate");
  const [generatePrompt, setGeneratePrompt] = useState("");
  const [optimizePrompt, setOptimizePrompt] = useState("");
  const [recommendContext, setRecommendContext] = useState("");
  const [isGenerating, setIsGenerating] = useState(false);
  const [isOptimizing, setIsOptimizing] = useState(false);
  const [isRecommending, setIsRecommending] = useState(false);
  const [optimizedResult, setOptimizedResult] = useState<string | null>(null);
  const [recommendedNodes, setRecommendedNodes] = useState<string[] | null>(null);

  const { nodes, edges, setNodes, setEdges } = useWorkflowEditorStore();

  const handleGenerate = async () => {
    if (!generatePrompt.trim()) {
      message.warning(t("workflow.aiPanel.enterWorkflowDesc"));
      return;
    }
    setIsGenerating(true);
    try {
      const result = await onGenerateWorkflow(generatePrompt);
      if (result) {
        setNodes(result.nodes);
        setEdges(result.edges);
        message.success(t("workflow.aiPanel.workflowGenerated"));
      }
    } catch (error) {
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
            <label style={{ display: "block", color: "#999", fontSize: 12, marginBottom: 8 }}>
              {t("workflow.aiPanel.describeWorkflow")}
            </label>
            <Input.TextArea
              placeholder={t("workflow.aiPanel.generatePlaceholder")}
              value={generatePrompt}
              onChange={(e) => setGeneratePrompt(e.target.value)}
              rows={6}
              style={{
                background: "#1a1a1a",
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
              {isGenerating ? t("workflow.aiPanel.generating") : t("workflow.aiPanel.generateBtn")}
            </Button>
          </div>

          <div style={{ color: "#666", fontSize: 11 }}>
            <strong>{t("workflow.aiPanel.currentCanvasState")}</strong>
            {t("workflow.aiPanel.canvasStatus", { nodes: nodes.length, edges: edges.length })}
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
            <label style={{ display: "block", color: "#999", fontSize: 12, marginBottom: 8 }}>
              {t("workflow.aiPanel.enterAgentPrompt")}
            </label>
            <Input.TextArea
              placeholder={t("workflow.aiPanel.optimizePlaceholder")}
              value={optimizePrompt}
              onChange={(e) => setOptimizePrompt(e.target.value)}
              rows={6}
              style={{
                background: "#1a1a1a",
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
            {isOptimizing ? t("workflow.aiPanel.optimizing") : t("workflow.aiPanel.optimizeBtn")}
          </Button>

          {optimizedResult && (
            <div>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
                <label style={{ color: "#999", fontSize: 12 }}>{t("workflow.aiPanel.optimizedResult")}</label>
                <Button type="text" size="small" onClick={handleCopyOptimized}>
                  {t("workflow.aiPanel.copy")}
                </Button>
              </div>
              <Card
                size="small"
                style={{
                  background: "#1a1a1a",
                  border: "1px solid #333",
                }}
                styles={{ body: { padding: 12 } }}
              >
                <pre style={{ whiteSpace: "pre-wrap", fontSize: 12, color: "#ccc", margin: 0 }}>
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
            <label style={{ display: "block", color: "#999", fontSize: 12, marginBottom: 8 }}>
              {t("workflow.aiPanel.describeContext")}
            </label>
            <Input.TextArea
              placeholder={t("workflow.aiPanel.recommendPlaceholder")}
              value={recommendContext}
              onChange={(e) => setRecommendContext(e.target.value)}
              rows={4}
              style={{
                background: "#1a1a1a",
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
            {isRecommending ? t("workflow.aiPanel.recommending") : t("workflow.aiPanel.getRecommendation")}
          </Button>

          {recommendedNodes && (
            <div>
              <label style={{ color: "#999", fontSize: 12, marginBottom: 8, display: "block" }}>
                {t("workflow.aiPanel.recommendedNodeTypes")}
              </label>
              <div style={{ display: "flex", flexWrap: "wrap", gap: 8 }}>
                {recommendedNodes.map((node) => (
                  <Tag
                    key={node}
                    color="blue"
                    style={{ fontSize: 12, padding: "4px 12px" }}
                  >
                    {node}
                  </Tag>
                ))}
              </div>
              <div style={{ color: "#666", fontSize: 11, marginTop: 12 }}>
                {t("workflow.aiPanel.dragHint")}
              </div>
            </div>
          )}

          {recommendedNodes && recommendedNodes.length === 0 && (
            <Empty description={t("workflow.aiPanel.noRecommendations")} image={Empty.PRESENTED_IMAGE_SIMPLE} />
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
        background: "#252525",
        display: "flex",
        flexDirection: "column",
      }}
    >
      <div
        style={{
          padding: "8px 16px",
          borderBottom: "1px solid #333",
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <Sparkles size={16} color="#722ed1" />
          <span style={{ fontWeight: 500, color: "#fff" }}>{t("workflow.aiPanel.aiAssistant")}</span>
        </div>
      </div>

      <div style={{ flex: 1, overflow: "auto", padding: "0 16px" }}>
        <Tabs
          activeKey={activeTab}
          onChange={setActiveTab}
          tabPosition="top"
          size="small"
          items={tabItems}
          style={{ height: "100%" }}
        />
      </div>
    </div>
  );
};
