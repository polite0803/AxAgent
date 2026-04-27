import { useWorkflowEditorStore } from "@/stores";
import { Button, Card, Empty, Input, message, Tabs, Tag } from "antd";
import { Lightbulb, MessageSquare, Sparkles, Wand2 } from "lucide-react";
import React, { useState } from "react";
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
      message.warning("请输入工作流描述");
      return;
    }
    setIsGenerating(true);
    try {
      const result = await onGenerateWorkflow(generatePrompt);
      if (result) {
        setNodes(result.nodes);
        setEdges(result.edges);
        message.success("工作流已生成");
      }
    } catch (error) {
      message.error("生成失败");
    } finally {
      setIsGenerating(false);
    }
  };

  const handleOptimize = async () => {
    if (!optimizePrompt.trim()) {
      message.warning("请输入要优化的 Prompt");
      return;
    }
    setIsOptimizing(true);
    setOptimizedResult(null);
    try {
      const result = await onOptimizePrompt(optimizePrompt);
      if (result) {
        setOptimizedResult(result);
        message.success("Prompt 已优化");
      }
    } catch (error) {
      message.error("优化失败");
    } finally {
      setIsOptimizing(false);
    }
  };

  const handleRecommend = async () => {
    if (!recommendContext.trim()) {
      message.warning("请输入上下文信息");
      return;
    }
    setIsRecommending(true);
    setRecommendedNodes(null);
    try {
      const result = await onRecommendNodes(recommendContext);
      if (result) {
        setRecommendedNodes(result);
        message.success("推荐已生成");
      }
    } catch (error) {
      message.error("推荐失败");
    } finally {
      setIsRecommending(false);
    }
  };

  const handleCopyOptimized = () => {
    if (optimizedResult) {
      navigator.clipboard.writeText(optimizedResult);
      message.success("已复制到剪贴板");
    }
  };

  const tabItems = [
    {
      key: "generate",
      label: (
        <span style={{ display: "flex", alignItems: "center", gap: 6 }}>
          <Wand2 size={14} />
          生成工作流
        </span>
      ),
      children: (
        <div style={{ padding: "16px 0" }}>
          <div style={{ marginBottom: 16 }}>
            <label style={{ display: "block", color: "#999", fontSize: 12, marginBottom: 8 }}>
              描述你想要的工作流
            </label>
            <Input.TextArea
              placeholder="例如：创建一个代码审查工作流，首先探索代码库结构，然后进行 Bug 检测和安全审查，最后生成总结报告"
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
              {isGenerating ? "生成中..." : "生成工作流"}
            </Button>
          </div>

          <div style={{ color: "#666", fontSize: 11 }}>
            <strong>当前画布状态：</strong>
            {nodes.length} 个节点，{edges.length} 条边
            <br />
            生成的工作流将替换当前画布内容
          </div>
        </div>
      ),
    },
    {
      key: "optimize",
      label: (
        <span style={{ display: "flex", alignItems: "center", gap: 6 }}>
          <MessageSquare size={14} />
          优化 Prompt
        </span>
      ),
      children: (
        <div style={{ padding: "16px 0" }}>
          <div style={{ marginBottom: 16 }}>
            <label style={{ display: "block", color: "#999", fontSize: 12, marginBottom: 8 }}>
              输入要优化的 Agent Prompt
            </label>
            <Input.TextArea
              placeholder="粘贴你的 Prompt..."
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
            {isOptimizing ? "优化中..." : "优化 Prompt"}
          </Button>

          {optimizedResult && (
            <div>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
                <label style={{ color: "#999", fontSize: 12 }}>优化结果</label>
                <Button type="text" size="small" onClick={handleCopyOptimized}>
                  复制
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
          推荐节点
        </span>
      ),
      children: (
        <div style={{ padding: "16px 0" }}>
          <div style={{ marginBottom: 16 }}>
            <label style={{ display: "block", color: "#999", fontSize: 12, marginBottom: 8 }}>
              描述当前工作流的上下文和目标
            </label>
            <Input.TextArea
              placeholder="例如：我正在构建一个自动化测试工作流，需要对代码变更进行单元测试和集成测试"
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
            {isRecommending ? "推荐中..." : "获取推荐"}
          </Button>

          {recommendedNodes && (
            <div>
              <label style={{ color: "#999", fontSize: 12, marginBottom: 8, display: "block" }}>
                推荐的节点类型
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
                从左侧节点面板拖拽这些节点到画布上
              </div>
            </div>
          )}

          {recommendedNodes && recommendedNodes.length === 0 && (
            <Empty description="暂无推荐" image={Empty.PRESENTED_IMAGE_SIMPLE} />
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
          <span style={{ fontWeight: 500, color: "#fff" }}>AI 助手</span>
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
