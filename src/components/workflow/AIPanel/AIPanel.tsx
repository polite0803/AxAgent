// SPDX-License-Identifier: AGPL-3.0-only

import { logIpcError } from "@/lib/invoke";
import { useEvolutionStore } from "@/stores/feature/evolutionStore";
import type { AiChatMessage } from "@/stores/feature/workflowEditorStore";
import { useWorkflowEditorStore } from "@/stores/feature/workflowEditorStore";
import { useWorkflowStore } from "@/stores/feature/workflowStore";
import type { NLParseResult } from "@/types/workflow";
import { App, Button, Card, Empty, Input, Progress, Radio, Tag, theme } from "antd";
import DOMPurify from "dompurify";
import { Lightbulb, MessageSquare, Play, Rocket, Send, Sparkles, StopCircle, Trash2, Wand2 } from "lucide-react";
import React, { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useShallow } from "zustand/react/shallow";
import { setDragPayload } from "../dndState";
import type { WorkflowEdge, WorkflowNode } from "../types/workflow.types";
import { ActionDiffPreview } from "./ActionDiffPreview";
import { EvolutionTab } from "./EvolutionTab";
import { NLParseResultView } from "./NLParseResultView";

const { TextArea } = Input;

interface AIPanelProps {
  onGenerateWorkflow: (
    prompt: string,
    mergeMode?: boolean,
  ) => Promise<{ nodes: WorkflowNode[]; edges: WorkflowEdge[]; explanation?: string } | null>;
  onOptimizePrompt: (prompt: string) => Promise<string | null>;
  onRecommendNodes: (
    context: string,
  ) => Promise<Array<{ node_type: string; label: string; description: string; confidence: number }> | null>;
  onClose: () => void;
  selectedNodeId?: string | null;
  selectedNodePrompt?: string | null;
  onApplyPromptToNode?: (nodeId: string, prompt: string) => void;
  chatMessages: AiChatMessage[];
  chatStreaming: boolean;
  onChatSend: (message: string) => void;
  onChatCancel: () => void;
  onChatClear: () => void;
}

// Phase 4: progress map for NL parse stages (moved outside component for stable reference)
const NL_PARSE_PROGRESS_MAP: Record<string, number> = {
  "正在分析意图": 25,
  "正在匹配节点": 50,
  "正在构建工作流": 75,
  "正在优化": 95,
};

/** Render assistant message content with Markdown-like formatting — defined outside component to avoid re-creation */
function renderAssistantContent(content: string) {
  const lines = content.split("\n");
  return lines.map((line, i) => {
    if (line.startsWith("### ")) {
      return (
        <div key={i} style={{ fontWeight: 600, fontSize: 14, marginTop: 8, marginBottom: 4 }}>{line.slice(4)}</div>
      );
    }
    if (line.startsWith("## ")) {
      return (
        <div key={i} style={{ fontWeight: 700, fontSize: 15, marginTop: 10, marginBottom: 4 }}>{line.slice(3)}</div>
      );
    }
    if (line.startsWith("# ")) {
      return (
        <div key={i} style={{ fontWeight: 700, fontSize: 16, marginTop: 12, marginBottom: 6 }}>{line.slice(2)}</div>
      );
    }
    if (line.startsWith("- ") || line.startsWith("* ")) {
      return (
        <div key={i} style={{ paddingLeft: 16, position: "relative" }}>
          <span style={{ position: "absolute", left: 4 }}>•</span>
          {line.slice(2)}
        </div>
      );
    }
    const numberedMatch = line.match(/^(\d+)\.\s/);
    if (numberedMatch) {
      return <div key={i} style={{ paddingLeft: 16 }}>{line}</div>;
    }
    if (line.startsWith("```")) {
      return null;
    }
    if (line.trim() === "") {
      return <div key={i} style={{ height: 8 }} />;
    }
    const boldText = line.replace(/\*\*(.*?)\*\*/g, "<b>$1</b>");
    const codeText = boldText.replace(
      /`([^`]+)`/g,
      "<code style='background:rgba(0,0,0,0.06);padding:1px 4px;border-radius:3px;font-size:12px'>$1</code>",
    );
    const safeHtml = DOMPurify.sanitize(codeText, { ALLOWED_TAGS: ["b", "code"] });
    return <div key={i} dangerouslySetInnerHTML={{ __html: safeHtml }} />;
  });
}

export const AIPanel: React.FC<AIPanelProps> = ({
  onGenerateWorkflow,
  onOptimizePrompt,
  onRecommendNodes,
  onClose,
  selectedNodeId,
  selectedNodePrompt,
  onApplyPromptToNode,
  chatMessages,
  chatStreaming,
  onChatSend,
  onChatCancel,
  onChatClear,
}) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const { message } = App.useApp();
  // 用 store selector 订阅 nodes/edges 变化，确保 Diff 预览拿到最新数据
  const nodes = useWorkflowEditorStore((s) => s.nodes) as unknown as WorkflowNode[];
  const edges = useWorkflowEditorStore((s) => s.edges) as unknown as WorkflowEdge[];

  // Diff 预览状态（AI Chat action 在应用前必须经过用户确认）
  const {
    pendingAiChatActions,
    applyAiChatAction,
    setPendingAiChatActions,
    clearPendingAiChatActions,
  } = useWorkflowEditorStore(
    useShallow((s) => ({
      pendingAiChatActions: s.pendingAiChatActions,
      applyAiChatAction: s.applyAiChatAction,
      setPendingAiChatActions: s.setPendingAiChatActions,
      clearPendingAiChatActions: s.clearPendingAiChatActions,
    })),
  );

  const [activeTab, setActiveTab] = useState<"chat" | "tools" | "evolution">("chat");
  const [chatInput, setChatInput] = useState("");
  const [toolTab, setToolTab] = useState<"generate" | "optimize" | "recommend">("generate");
  const messagesEndRef = useRef<HTMLDivElement>(null);

  const [generatePrompt, setGeneratePrompt] = useState("");
  const [isGenerating, setIsGenerating] = useState(false);
  const [mergeMode, setMergeMode] = useState(false);

  const [optimizePrompt, setOptimizePrompt] = useState("");
  const [isOptimizing, setIsOptimizing] = useState(false);
  const [optimizedResult, setOptimizedResult] = useState<string | null>(null);

  const [recommendContext, setRecommendContext] = useState("");
  const [isRecommending, setIsRecommending] = useState(false);
  const [recommendedNodes, setRecommendedNodes] = useState<
    Array<{ node_type: string; label: string; description: string; confidence: number }> | null
  >(null);

  const [generateError, setGenerateError] = useState<string | null>(null);
  const [optimizeError, setOptimizeError] = useState<string | null>(null);
  const [recommendError, setRecommendError] = useState<string | null>(null);

  // Phase 3/4: NL parse + evolution state
  const [nlResult, setNlResult] = useState<NLParseResult | null>(null);
  const workflowStore = useWorkflowStore();
  const evolutionStore = useEvolutionStore();
  const [chatContextMessage, setChatContextMessage] = useState<string | null>(null);

  const scrollToBottom = useCallback(() => {
    messagesEndRef.current?.scrollIntoView?.({ behavior: "smooth" });
  }, []);

  useEffect(() => {
    scrollToBottom();
  }, [chatMessages, scrollToBottom]);

  // Phase 3: build chat context on mount
  useEffect(() => {
    const tid = setTimeout(() => {
      try {
        const ctx = useWorkflowEditorStore.getState().buildChatContext();
        setChatContextMessage(ctx);
      } catch {
        setChatContextMessage(null);
      }
    }, 0);
    return () => clearTimeout(tid);
  }, []);

  const handleChatSend = () => {
    if (!chatInput.trim() || chatStreaming) { return; }
    onChatSend(chatInput.trim());
    setChatInput("");
  };

  const handleChatKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleChatSend();
    }
  };

  const handleGenerate = async () => {
    if (!generatePrompt.trim()) {
      message.warning(t("workflow.aiPanel.enterWorkflowDesc"));
      return;
    }
    if (!mergeMode && (nodes.length > 0 || edges.length > 0)) {
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
    setGenerateError(null);
    setNlResult(null);
    try {
      // Phase 4: use NL parse pipeline
      const result = await workflowStore.parseNaturalLanguage({ prompt: generatePrompt });
      if (result) {
        setNlResult(result);
        if (result.confidence >= 0.7) {
          message.success(t("workflow.aiPanel.workflowParsed"));
        } else {
          message.warning(t("workflow.aiPanel.workflowParsedLowConfidence"));
        }
      }
    } catch (error) {
      logIpcError("NL 解析工作流")(error);
      setGenerateError(String(error));
    } finally {
      setIsGenerating(false);
    }
  };

  const handleApplyNLResult = async (workflow: import("@/types/workflow").WorkflowDefinition) => {
    setIsGenerating(true);
    try {
      const result = await onGenerateWorkflow(
        JSON.stringify({ nodes: workflow.nodes, edges: workflow.edges }),
        mergeMode,
      );
      if (result) {
        message.success(t("workflow.aiPanel.workflowGenerated"));
        setNlResult(null);
      }
    } catch (error) {
      logIpcError("AI 生成工作流")(error);
      setGenerateError(String(error));
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
    setOptimizeError(null);
    try {
      const result = await onOptimizePrompt(optimizePrompt);
      if (result) {
        setOptimizedResult(result);
        message.success(t("workflow.aiPanel.promptOptimized"));
      }
    } catch (error) {
      logIpcError("AI 优化提示词")(error);
      setOptimizeError(String(error));
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
    setRecommendError(null);
    try {
      // Phase 3: inject evolution stats into context
      let enrichedContext = recommendContext;
      try {
        const engines = evolutionStore.engines;
        const evoStats: string[] = [];
        for (const [, engine] of Object.entries(engines)) {
          if (engine.running) {
            evoStats.push(`${engine.displayName}: 运行中`);
          }
          if (engine.stats && Object.keys(engine.stats).length > 0) {
            evoStats.push(`${engine.displayName}: ${JSON.stringify(engine.stats)}`);
          }
        }
        if (evoStats.length > 0) {
          enrichedContext = `${recommendContext}\n\n【进化引擎统计】\n${evoStats.join("\n")}`;
        }
      } catch {
        // evolutionStore unavailable, proceed without enrichment
      }
      const result = await onRecommendNodes(enrichedContext);
      if (result) {
        setRecommendedNodes(result);
        message.success(t("workflow.aiPanel.recommendationGenerated"));
      }
    } catch (error) {
      logIpcError("AI 推荐节点")(error);
      setRecommendError(String(error));
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

  const handleApplyToNode = () => {
    if (optimizedResult && selectedNodeId && onApplyPromptToNode) {
      onApplyPromptToNode(selectedNodeId, optimizedResult);
      // Phase 3: trigger evolution record
      try {
        evolutionStore.triggerSkillEvolution(selectedNodeId);
      } catch (e) {
        console.warn("[AIPanel] triggerSkillEvolution fallback:", e);
      }
      message.success(t("workflow.aiPanel.promptAppliedToNode"));
    }
  };

  const handleFillFromSelectedNode = () => {
    if (selectedNodePrompt) {
      setOptimizePrompt(selectedNodePrompt);
    }
  };

  const renderChatMessage = (msg: AiChatMessage) => {
    const isUser = msg.role === "user";
    return (
      <div
        key={msg.id}
        style={{
          display: "flex",
          justifyContent: isUser ? "flex-end" : "flex-start",
          marginBottom: 12,
          padding: "0 12px",
        }}
      >
        <div
          style={{
            maxWidth: "85%",
            background: isUser ? token.colorPrimaryBg : token.colorBgContainer,
            border: `1px solid ${isUser ? token.colorPrimaryBorder : token.colorBorderSecondary}`,
            borderRadius: isUser ? "12px 12px 2px 12px" : "12px 12px 12px 2px",
            padding: "8px 12px",
            fontSize: 13,
            lineHeight: 1.6,
            color: token.colorText,
            whiteSpace: "pre-wrap",
            wordBreak: "break-word",
          }}
        >
          {isUser ? msg.content : renderAssistantContent(msg.content)}
          {msg.isStreaming && (
            <span className="axagent-streaming-dots" style={{ marginLeft: 4 }}>
              <span />
              <span />
              <span />
            </span>
          )}
          {msg.actions && msg.actions.length > 0 && !msg.isStreaming && (
            <div style={{ marginTop: 8, display: "flex", flexDirection: "column", gap: 4 }}>
              {msg.actions.length === 1
                ? (
                  <Button
                    type="primary"
                    size="small"
                    icon={<Play size={12} />}
                    onClick={() => setPendingAiChatActions(msg.id, msg.actions!)}
                    style={{ alignSelf: "flex-start" }}
                  >
                    {getActionLabel(msg.actions[0].action_type)}
                  </Button>
                )
                : (
                  <Button
                    type="primary"
                    size="small"
                    icon={<Play size={12} />}
                    onClick={() => setPendingAiChatActions(msg.id, msg.actions!)}
                    style={{ alignSelf: "flex-start" }}
                  >
                    {t("workflow.aiPanel.actionApplyAll", { count: msg.actions.length })}
                  </Button>
                )}
            </div>
          )}
        </div>
      </div>
    );
  };

  const getActionLabel = (actionType: string): string => {
    switch (actionType) {
      case "generate_workflow":
        return t("workflow.aiPanel.actionGenerateWorkflow");
      case "add_node":
      case "add_nodes":
        return t("workflow.aiPanel.actionAddNodes");
      case "update_node":
      case "modify_node":
        return t("workflow.aiPanel.actionModifyNode");
      case "delete_node":
      case "delete_nodes":
        return t("workflow.aiPanel.actionDeleteNodes");
      case "add_edge":
        return t("workflow.aiPanel.actionAddEdge");
      case "update_edge":
        return t("workflow.aiPanel.actionUpdateEdge");
      case "delete_edge":
        return t("workflow.aiPanel.actionDeleteEdge");
      case "optimize_prompt":
        return t("workflow.aiPanel.actionOptimizePrompt");
      default:
        return t("workflow.aiPanel.actionApply");
    }
  };

  const renderChatTab = () => (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div
        style={{
          flex: 1,
          overflowY: "auto",
          padding: "12px 0",
          minHeight: 0,
        }}
      >
        {/* Phase 3: chat context system message */}
        {chatContextMessage && (
          <div
            style={{
              margin: "0 12px 8px",
              padding: "8px 10px",
              background: token.colorFillTertiary,
              borderRadius: 8,
              fontSize: 11,
              color: token.colorTextTertiary,
              lineHeight: 1.6,
              whiteSpace: "pre-wrap",
              wordBreak: "break-word",
            }}
          >
            {chatContextMessage}
          </div>
        )}
        {chatMessages.length === 0 && (
          <div style={{ padding: "24px 12px", textAlign: "center" }}>
            <Sparkles size={24} color={token.colorTextTertiary} style={{ marginBottom: 8 }} />
            <div style={{ color: token.colorTextSecondary, fontSize: 13, marginBottom: 4 }}>
              {t("workflow.aiPanel.chatWelcome")}
            </div>
            <div style={{ color: token.colorTextTertiary, fontSize: 12 }}>
              {t("workflow.aiPanel.chatWelcomeHint")}
            </div>
          </div>
        )}
        {chatMessages.map(renderChatMessage)}
        <div ref={messagesEndRef} />
      </div>
      <div
        style={{
          borderTop: `1px solid ${token.colorBorderSecondary}`,
          padding: "8px 12px",
          display: "flex",
          gap: 8,
          alignItems: "flex-end",
          background: token.colorBgElevated,
        }}
      >
        <TextArea
          id="a-i-panel-chat-input"
          placeholder={t("workflow.aiPanel.chatPlaceholder")}
          value={chatInput}
          onChange={(e) => setChatInput(e.target.value)}
          onKeyDown={handleChatKeyDown}
          autoSize={{ minRows: 1, maxRows: 4 }}
          style={{ flex: 1, fontSize: 13, background: token.colorBgContainer }}
          disabled={chatStreaming}
        />
        {chatStreaming
          ? (
            <Button
              type="text"
              danger
              icon={<StopCircle size={16} />}
              onClick={onChatCancel}
              style={{ flexShrink: 0 }}
            />
          )
          : (
            <Button
              type="primary"
              icon={<Send size={14} />}
              onClick={handleChatSend}
              disabled={!chatInput.trim()}
              style={{ flexShrink: 0 }}
            />
          )}
        <Button
          type="text"
          icon={<Trash2 size={14} />}
          onClick={onChatClear}
          disabled={chatStreaming || chatMessages.length === 0}
          style={{ flexShrink: 0, color: token.colorTextTertiary }}
        />
      </div>
    </div>
  );

  const renderGenerateTool = () => (
    <div style={{ padding: "12px 0" }}>
      <div style={{ marginBottom: 12 }}>
        <label style={{ display: "block", color: token.colorTextSecondary, fontSize: 12, marginBottom: 8 }}>
          {t("workflow.aiPanel.describeWorkflow")}
        </label>
        <TextArea
          id="a-i-panel-input-textarea-70"
          placeholder={t("workflow.aiPanel.generatePlaceholder")}
          value={generatePrompt}
          onChange={(e) => setGeneratePrompt(e.target.value)}
          rows={4}
          style={{ background: token.colorBgContainer, fontSize: 13 }}
        />
      </div>
      <div style={{ marginBottom: 8, display: "flex", gap: 8 }}>
        <Radio.Group value={mergeMode} onChange={(e) => setMergeMode(e.target.value)} size="small">
          <Radio.Button value={false}>{t("workflow.aiPanel.replaceMode")}</Radio.Button>
          <Radio.Button value={true}>{t("workflow.aiPanel.mergeMode")}</Radio.Button>
        </Radio.Group>
      </div>
      <Button
        type="primary"
        icon={<Sparkles size={14} />}
        onClick={handleGenerate}
        loading={isGenerating && !workflowStore.parseProgress}
        disabled={isGenerating}
        style={{ width: "100%" }}
      >
        {isGenerating && !workflowStore.parseProgress
          ? t("workflow.aiPanel.generating")
          : mergeMode
          ? t("workflow.aiPanel.generateMergeBtn")
          : t("workflow.aiPanel.generateBtn")}
      </Button>
      {/* Phase 4: parse progress bar */}
      {isGenerating && workflowStore.parseProgress && (
        <div style={{ marginTop: 12 }}>
          <div style={{ color: token.colorTextSecondary, fontSize: 12, marginBottom: 4 }}>
            {workflowStore.parseProgress}
          </div>
          <Progress
            percent={NL_PARSE_PROGRESS_MAP[workflowStore.parseProgress] || 10}
            size="small"
            showInfo={false}
            strokeColor={token.colorPrimary}
          />
        </div>
      )}
      {generateError && !isGenerating && (
        <Button type="dashed" size="small" onClick={handleGenerate} style={{ width: "100%", marginTop: 8 }}>
          {t("workflow.aiPanel.retry")}
        </Button>
      )}
      {/* Phase 4: NLParseResultView */}
      {nlResult && !isGenerating && (
        <div style={{ marginTop: 12 }}>
          <NLParseResultView
            result={nlResult}
            onApply={handleApplyNLResult}
            loading={isGenerating}
          />
        </div>
      )}
    </div>
  );

  const renderOptimizeTool = () => (
    <div style={{ padding: "12px 0" }}>
      <div style={{ marginBottom: 12 }}>
        <label style={{ display: "block", color: token.colorTextSecondary, fontSize: 12, marginBottom: 8 }}>
          {t("workflow.aiPanel.enterAgentPrompt")}
        </label>
        <TextArea
          id="a-i-panel-input-textarea-71"
          placeholder={t("workflow.aiPanel.optimizePlaceholder")}
          value={optimizePrompt}
          onChange={(e) => setOptimizePrompt(e.target.value)}
          rows={4}
          style={{ background: token.colorBgContainer, fontSize: 13 }}
        />
      </div>
      <Button
        type="primary"
        icon={<Sparkles size={14} />}
        onClick={handleOptimize}
        loading={isOptimizing}
        disabled={isOptimizing}
        style={{ width: "100%", marginBottom: 8 }}
      >
        {isOptimizing ? t("workflow.aiPanel.optimizing") : t("workflow.aiPanel.optimizeBtn")}
      </Button>
      {selectedNodePrompt && (
        <Button
          type="dashed"
          size="small"
          onClick={handleFillFromSelectedNode}
          style={{ width: "100%", marginBottom: 8 }}
        >
          {t("workflow.aiPanel.fillFromSelectedNode")}
        </Button>
      )}
      {optimizeError && !isOptimizing && (
        <Button type="dashed" size="small" onClick={handleOptimize} style={{ width: "100%", marginBottom: 8 }}>
          {t("workflow.aiPanel.retry")}
        </Button>
      )}
      {optimizedResult && (
        <div>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
            <label style={{ color: token.colorTextSecondary, fontSize: 12 }}>
              {t("workflow.aiPanel.optimizedResult")}
            </label>
            <div style={{ display: "flex", gap: 4 }}>
              {selectedNodeId && onApplyPromptToNode && (
                <Button size="small" type="primary" onClick={handleApplyToNode}>
                  {t("workflow.aiPanel.applyToNode")}
                </Button>
              )}
              <Button type="text" size="small" onClick={handleCopyOptimized}>{t("workflow.aiPanel.copy")}</Button>
            </div>
          </div>
          <Card
            size="small"
            style={{ background: token.colorBgContainer, border: `1px solid ${token.colorBorderSecondary}` }}
            styles={{ body: { padding: 12 } }}
          >
            <pre style={{ whiteSpace: "pre-wrap", fontSize: 12, color: token.colorTextSecondary, margin: 0 }}>
              {optimizedResult}
            </pre>
          </Card>
          {/* Phase 3: evolution history for selected node */}
          {selectedNodeId && (() => {
            try {
              const history = evolutionStore.getSkillEvolutionHistory(selectedNodeId);
              if (history && history.length > 0) {
                const recent = history.slice(-3);
                return (
                  <div style={{ marginTop: 12 }}>
                    <label
                      style={{
                        display: "block",
                        color: token.colorTextSecondary,
                        fontSize: 12,
                        marginBottom: 6,
                      }}
                    >
                      进化历史
                    </label>
                    {recent.map((evt, i) => (
                      <Card
                        key={i}
                        size="small"
                        style={{
                          background: token.colorFillTertiary,
                          border: "none",
                          marginBottom: 4,
                        }}
                        styles={{ body: { padding: "6px 10px" } }}
                      >
                        <div style={{ display: "flex", justifyContent: "space-between", fontSize: 11 }}>
                          <span style={{ color: token.colorTextSecondary }}>
                            v{evt.version} {evt.summary || "优化"}
                          </span>
                          <span style={{ color: token.colorTextTertiary }}>
                            {new Date(evt.timestamp).toLocaleDateString()}
                          </span>
                        </div>
                      </Card>
                    ))}
                  </div>
                );
              }
            } catch {
              // evolutionStore unavailable
            }
            return null;
          })()}
        </div>
      )}
    </div>
  );

  const renderRecommendTool = () => (
    <div style={{ padding: "12px 0" }}>
      <div style={{ marginBottom: 12 }}>
        <label style={{ display: "block", color: token.colorTextSecondary, fontSize: 12, marginBottom: 8 }}>
          {t("workflow.aiPanel.describeContext")}
        </label>
        <TextArea
          id="a-i-panel-input-textarea-72"
          placeholder={t("workflow.aiPanel.recommendPlaceholder")}
          value={recommendContext}
          onChange={(e) => setRecommendContext(e.target.value)}
          rows={3}
          style={{ background: token.colorBgContainer, fontSize: 13 }}
        />
      </div>
      <Button
        type="primary"
        icon={<Sparkles size={14} />}
        onClick={handleRecommend}
        loading={isRecommending}
        disabled={isRecommending}
        style={{ width: "100%", marginBottom: 8 }}
      >
        {isRecommending ? t("workflow.aiPanel.recommending") : t("workflow.aiPanel.getRecommendation")}
      </Button>
      {recommendError && !isRecommending && (
        <Button type="dashed" size="small" onClick={handleRecommend} style={{ width: "100%", marginBottom: 8 }}>
          {t("workflow.aiPanel.retry")}
        </Button>
      )}
      {recommendedNodes && (
        <div>
          <label style={{ color: token.colorTextSecondary, fontSize: 12, marginBottom: 8, display: "block" }}>
            {t("workflow.aiPanel.recommendedNodeTypes")}
          </label>
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            {recommendedNodes.map((node) => (
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
                  setDragPayload({ type: node.node_type, label: node.label });
                  message.info(t("workflow.aiPanel.dragCanvasToAdd", { label: node.label }));
                }}
              >
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                  <div style={{ flex: 1, minWidth: 0 }}>
                    <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 2 }}>
                      <Tag color="blue" style={{ fontSize: 12, margin: 0, padding: "0 6px" }}>{node.node_type}</Tag>
                      <span style={{ color: token.colorText, fontSize: 12, fontWeight: 500 }}>{node.label}</span>
                    </div>
                    {node.description && (
                      <div style={{ color: token.colorTextSecondary, fontSize: 12, marginTop: 4, lineHeight: 1.4 }}>
                        {node.description}
                      </div>
                    )}
                  </div>
                  <div style={{ display: "flex", alignItems: "center", gap: 6, marginLeft: 8, flexShrink: 0 }}>
                    {/* Phase 3: evolution badges */}
                    {(() => {
                      try {
                        const abResults = evolutionStore.getABTestResults(node.node_type);
                        const historyResults = evolutionStore.getSkillEvolutionHistory(node.node_type);
                        const hasABWin = abResults && abResults.length > 0;
                        const hasHistory = historyResults && historyResults.length > 0;
                        return (
                          <>
                            {hasABWin && (
                              <Tag
                                color="green"
                                style={{ fontSize: 10, margin: 0, padding: "0 4px", lineHeight: "16px" }}
                              >
                                已验证有效
                              </Tag>
                            )}
                            {hasHistory && !hasABWin && (
                              <Tag
                                color="blue"
                                style={{ fontSize: 10, margin: 0, padding: "0 4px", lineHeight: "16px" }}
                              >
                                有进化记录
                              </Tag>
                            )}
                          </>
                        );
                      } catch {
                        return null;
                      }
                    })()}
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
                  </div>
                </div>
              </Card>
            ))}
          </div>
          <div style={{ color: token.colorTextTertiary, fontSize: 12, marginTop: 12 }}>
            {t("workflow.aiPanel.dragHintUpdated")}
          </div>
        </div>
      )}
      {recommendedNodes && recommendedNodes.length === 0 && (
        <Empty description={t("workflow.aiPanel.noRecommendations")} image={Empty.PRESENTED_IMAGE_SIMPLE} />
      )}
    </div>
  );

  const renderToolsTab = () => (
    <div style={{ height: "100%", display: "flex", flexDirection: "column" }}>
      <div
        style={{
          display: "flex",
          gap: 4,
          padding: "8px 12px",
          borderBottom: `1px solid ${token.colorBorderSecondary}`,
        }}
      >
        {[
          { key: "generate" as const, icon: <Wand2 size={12} />, label: t("workflow.aiPanel.tabGenerateWorkflow") },
          {
            key: "optimize" as const,
            icon: <MessageSquare size={12} />,
            label: t("workflow.aiPanel.tabOptimizePrompt"),
          },
          { key: "recommend" as const, icon: <Lightbulb size={12} />, label: t("workflow.aiPanel.tabRecommend") },
        ].map((tab) => (
          <Button
            key={tab.key}
            type={toolTab === tab.key ? "primary" : "text"}
            size="small"
            icon={tab.icon}
            onClick={() => setToolTab(tab.key)}
            style={{ fontSize: 12 }}
          >
            {tab.label}
          </Button>
        ))}
      </div>
      <div style={{ flex: 1, overflowY: "auto", padding: "0 12px" }}>
        {toolTab === "generate" && renderGenerateTool()}
        {toolTab === "optimize" && renderOptimizeTool()}
        {toolTab === "recommend" && renderRecommendTool()}
      </div>
    </div>
  );

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", background: token.colorBgElevated }}>
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          padding: "8px 12px",
          borderBottom: `1px solid ${token.colorBorderSecondary}`,
          flexShrink: 0,
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <Sparkles size={14} color={token.colorPrimary} />
          <span style={{ fontSize: 13, fontWeight: 500, color: token.colorText }}>
            {t("workflow.aiPanel.aiAssistant")}
          </span>
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
          <Button
            type="text"
            size="small"
            onClick={onClose}
            style={{ color: token.colorTextTertiary, padding: "0 4px" }}
          >
            ✕
          </Button>
          <Radio.Group
            value={activeTab}
            onChange={(e) => setActiveTab(e.target.value)}
            size="small"
            buttonStyle="solid"
          >
            <Radio.Button value="chat">
              <MessageSquare size={11} style={{ marginRight: 4, verticalAlign: -1 }} />
              {t("workflow.aiPanel.chatMode")}
            </Radio.Button>
            <Radio.Button value="tools">
              <Wand2 size={11} style={{ marginRight: 4, verticalAlign: -1 }} />
              {t("workflow.aiPanel.toolsMode")}
            </Radio.Button>
            <Radio.Button value="evolution">
              <Rocket size={11} style={{ marginRight: 4, verticalAlign: -1 }} />
              进化
            </Radio.Button>
          </Radio.Group>
        </div>
      </div>
      <div style={{ flex: 1, minHeight: 0, overflow: "hidden" }}>
        {activeTab === "chat" && renderChatTab()}
        {activeTab === "tools" && renderToolsTab()}
        {activeTab === "evolution" && (
          <EvolutionTab
            currentWorkflowId={workflowStore.currentWorkflowId ?? null}
            nodes={nodes}
          />
        )}
      </div>
      <ActionDiffPreview
        actions={pendingAiChatActions}
        currentNodes={nodes}
        currentEdges={edges}
        onApply={applyAiChatAction}
        onApplyAll={() => {
          if (pendingAiChatActions) {
            for (const a of pendingAiChatActions) {
              applyAiChatAction(a);
            }
          }
        }}
        onCancel={clearPendingAiChatActions}
      />
    </div>
  );
};
