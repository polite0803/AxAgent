import { useWorkflowEditorStore } from "@/stores";
import { Button, Divider, Input, Modal, Select, Tabs, theme } from "antd";
import { Trash2 } from "lucide-react";
import React from "react";
import { useTranslation } from "react-i18next";
import { type JsonSchema, NODE_TYPE_MAP, type WorkflowEdge, type WorkflowNode } from "../types";
import {
  AgentPropertyPanel,
  AggregatorPropertyPanel,
  ApprovalPropertyPanel,
  CodePropertyPanel,
  ConditionPropertyPanel,
  DatabaseQueryPropertyPanel,
  DataTransformerPropertyPanel,
  DebatePropertyPanel,
  DelayPropertyPanel,
  DocumentParserPropertyPanel,
  EmailPropertyPanel,
  EndPropertyPanel,
  FileOperationPropertyPanel,
  HttpRequestPropertyPanel,
  LlmClassifierPropertyPanel,
  LLMPropertyPanel,
  LoggingPropertyPanel,
  LoopPropertyPanel,
  MergePropertyPanel,
  NotificationPropertyPanel,
  ParallelPropertyPanel,
  SubWorkflowPropertyPanel,
  SwitchPropertyPanel,
  ToolPropertyPanel,
  TriggerPropertyPanel,
  ValidationPropertyPanel,
  VectorRetrievePropertyPanel,
  WebhookSendPropertyPanel,
} from "./PropertyPanels";

interface RightPanelProps {
  width: number;
  selectedNodeId: string | null;
  selectedEdge: WorkflowEdge | null;
}

function NodePropertyPanel({
  selectedNode,
  onUpdate,
  onDelete,
}: {
  selectedNode: WorkflowNode;
  onUpdate: (updates: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}) {
  const { t } = useTranslation();

  switch (selectedNode.type) {
    case "trigger":
      return (
        <TriggerPropertyPanel
          node={selectedNode}
          onUpdate={onUpdate}
          onDelete={onDelete}
        />
      );
    case "agent":
      return (
        <AgentPropertyPanel
          node={selectedNode}
          onUpdate={onUpdate}
          onDelete={onDelete}
        />
      );
    case "llm":
      return (
        <LLMPropertyPanel
          node={selectedNode}
          onUpdate={onUpdate}
          onDelete={onDelete}
        />
      );
    case "condition":
      return (
        <ConditionPropertyPanel
          node={selectedNode}
          onUpdate={onUpdate}
          onDelete={onDelete}
        />
      );
    case "parallel":
      return (
        <ParallelPropertyPanel
          node={selectedNode}
          onUpdate={onUpdate}
          onDelete={onDelete}
        />
      );
    case "loop":
      return (
        <LoopPropertyPanel
          node={selectedNode}
          onUpdate={onUpdate}
          onDelete={onDelete}
        />
      );
    case "merge":
      return (
        <MergePropertyPanel
          node={selectedNode}
          onUpdate={onUpdate}
          onDelete={onDelete}
        />
      );
    case "delay":
      return (
        <DelayPropertyPanel
          node={selectedNode}
          onUpdate={onUpdate}
          onDelete={onDelete}
        />
      );
    case "tool":
      return (
        <ToolPropertyPanel
          node={selectedNode}
          onUpdate={onUpdate}
          onDelete={onDelete}
        />
      );
    case "code":
      return (
        <CodePropertyPanel
          node={selectedNode}
          onUpdate={onUpdate}
          onDelete={onDelete}
        />
      );
    case "subWorkflow":
      return (
        <SubWorkflowPropertyPanel
          node={selectedNode}
          onUpdate={onUpdate}
          onDelete={onDelete}
        />
      );
    case "documentParser":
      return (
        <DocumentParserPropertyPanel
          node={selectedNode}
          onUpdate={onUpdate}
          onDelete={onDelete}
        />
      );
    case "vectorRetrieve":
      return (
        <VectorRetrievePropertyPanel
          node={selectedNode}
          onUpdate={onUpdate}
          onDelete={onDelete}
        />
      );
    case "validation":
      return (
        <ValidationPropertyPanel
          node={selectedNode}
          onUpdate={onUpdate}
          onDelete={onDelete}
        />
      );
    case "end":
      return (
        <EndPropertyPanel
          node={selectedNode}
          onUpdate={onUpdate}
          onDelete={onDelete}
        />
      );
    case "debate":
      return (
        <DebatePropertyPanel
          node={selectedNode}
          onUpdate={onUpdate}
          onDelete={onDelete}
        />
      );
    case "httpRequest":
      return (
        <HttpRequestPropertyPanel
          node={selectedNode}
          onUpdate={onUpdate}
          onDelete={onDelete}
        />
      );
    case "switch":
      return (
        <SwitchPropertyPanel
          node={selectedNode}
          onUpdate={onUpdate}
          onDelete={onDelete}
        />
      );
    case "databaseQuery":
      return (
        <DatabaseQueryPropertyPanel
          node={selectedNode}
          onUpdate={onUpdate}
          onDelete={onDelete}
        />
      );
    case "notification":
      return (
        <NotificationPropertyPanel
          node={selectedNode}
          onUpdate={onUpdate}
          onDelete={onDelete}
        />
      );
    case "approval":
      return (
        <ApprovalPropertyPanel
          node={selectedNode}
          onUpdate={onUpdate}
          onDelete={onDelete}
        />
      );
    case "fileOperation":
      return (
        <FileOperationPropertyPanel
          node={selectedNode}
          onUpdate={onUpdate}
          onDelete={onDelete}
        />
      );
    case "dataTransformer":
      return (
        <DataTransformerPropertyPanel
          node={selectedNode}
          onUpdate={onUpdate}
          onDelete={onDelete}
        />
      );
    case "webhookSend":
      return (
        <WebhookSendPropertyPanel
          node={selectedNode}
          onUpdate={onUpdate}
          onDelete={onDelete}
        />
      );
    case "logging":
      return (
        <LoggingPropertyPanel
          node={selectedNode}
          onUpdate={onUpdate}
          onDelete={onDelete}
        />
      );
    case "llmClassifier":
      return (
        <LlmClassifierPropertyPanel
          node={selectedNode}
          onUpdate={onUpdate}
          onDelete={onDelete}
        />
      );
    case "aggregator":
      return (
        <AggregatorPropertyPanel
          node={selectedNode}
          onUpdate={onUpdate}
          onDelete={onDelete}
        />
      );
    case "email":
      return (
        <EmailPropertyPanel
          node={selectedNode}
          onUpdate={onUpdate}
          onDelete={onDelete}
        />
      );
    default:
      return (
        <div style={{ padding: 20, textAlign: "center" }}>
          <div style={{ opacity: 0.45, marginBottom: 12 }}>
            {t("workflow.rightPanel.unsupportedNodeType")}
          </div>
          <Button type="text" danger onClick={onDelete}>
            {t("workflow.rightPanel.deleteNode")}
          </Button>
        </div>
      );
  }
}

/** 深度限制（防止 5.2 深度嵌套 JSON 触发栈溢出 / RangeError） */
const MAX_JSON_DEPTH = 32;

/** 安全 JSON 解析：超深嵌套抛错 */
function safeJsonParse(text: string, maxDepth = MAX_JSON_DEPTH): unknown {
  // 先用一次正常解析拿到值，再深度检查
  const parsed = JSON.parse(text);
  const depth = (function getDepth(v: unknown, d: number): number {
    if (v === null || typeof v !== "object") { return d; }
    if (d > maxDepth) { return d; }
    let max = d;
    for (const k of Object.keys(v as Record<string, unknown>)) {
      const child = (v as Record<string, unknown>)[k];
      const cd = getDepth(child, d + 1);
      if (cd > max) { max = cd; }
      if (max > maxDepth) { return max; }
    }
    return max;
  })(parsed, 0);
  if (depth > maxDepth) {
    throw new Error(`JSON depth exceeds ${maxDepth}`);
  }
  return parsed;
}

/** 极简 JSON Schema 形态校验：仅校验 type / properties / required 字段类型 */
function validateJsonSchema(value: unknown): value is JsonSchema {
  if (value === null || typeof value !== "object") { return false; }
  const v = value as Record<string, unknown>;
  if (v.type !== undefined && typeof v.type !== "string") { return false; }
  if (v.properties !== undefined) {
    if (typeof v.properties !== "object" || v.properties === null) { return false; }
  }
  if (v.required !== undefined && !Array.isArray(v.required)) { return false; }
  if (v.items !== undefined && (typeof v.items !== "object" || v.items === null)) { return false; }
  return true;
}

/**
 * Schema 编辑弹窗：替代 window.prompt，提供深度限制 + 形状校验 + 错误提示。
 */
const SchemaEditorModal: React.FC<{
  open: boolean;
  title: string;
  initial: string;
  onCancel: () => void;
  onConfirm: (parsed: JsonSchema | undefined) => void;
}> = ({ open, title, initial, onCancel, onConfirm }) => {
  const { t } = useTranslation();
  const [text, setText] = React.useState(initial);
  const [error, setError] = React.useState<string | null>(null);
  React.useEffect(() => {
    if (open) {
      setText(initial);
      setError(null);
    }
  }, [open, initial]);

  const handleOk = () => {
    if (text.trim() === "") {
      onConfirm(undefined);
      return;
    }
    try {
      const parsed = safeJsonParse(text);
      if (!validateJsonSchema(parsed)) {
        setError(t("workflow.rightPanel.invalidSchemaShape"));
        return;
      }
      onConfirm(parsed as JsonSchema);
    } catch (e) {
      setError(
        t("workflow.rightPanel.invalidJson") + `: ${e instanceof Error ? e.message : String(e)}`,
      );
    }
  };

  return (
    <Modal
      title={title}
      open={open}
      onOk={handleOk}
      onCancel={onCancel}
      okText={t("common.confirm")}
      cancelText={t("common.cancel")}
      width={640}
      destroyOnClose
    >
      <Input.TextArea
        value={text}
        onChange={(e) => {
          setText(e.target.value);
          if (error) { setError(null); }
        }}
        rows={16}
        style={{ fontFamily: "monospace", fontSize: 12 }}
        spellCheck={false}
      />
      {error && <div style={{ color: "#ff4d4f", fontSize: 12, marginTop: 6 }}>{error}</div>}
    </Modal>
  );
};

function TemplateSettings({
  currentTemplate,
}: {
  currentTemplate: {
    name: string;
    description?: string;
    icon?: string;
    tags?: string[];
    inputSchema?: JsonSchema;
    outputSchema?: JsonSchema;
  } | null;
}) {
  const { t } = useTranslation();
  const { token } = theme.useToken();

  if (!currentTemplate) {
    return null;
  }

  const [schemaModal, setSchemaModal] = React.useState<{
    open: boolean;
    kind: "input" | "output";
    initial: string;
  }>({ open: false, kind: "input", initial: "" });

  const openSchemaModal = (kind: "input" | "output") => {
    const schema = kind === "input"
      ? currentTemplate?.inputSchema
      : currentTemplate?.outputSchema;
    setSchemaModal({
      open: true,
      kind,
      initial: schema ? JSON.stringify(schema, null, 2) : "",
    });
  };

  const handleSchemaConfirm = (parsed: JsonSchema | undefined) => {
    if (schemaModal.kind === "input") {
      useWorkflowEditorStore
        .getState()
        .updateTemplateMetadata({ inputSchema: parsed });
    } else {
      useWorkflowEditorStore
        .getState()
        .updateTemplateMetadata({ outputSchema: parsed });
    }
    setSchemaModal((s) => ({ ...s, open: false }));
  };

  return (
    <div style={{ padding: 12 }}>
      <div style={{ marginBottom: 12 }}>
        <label
          style={{
            display: "block",
            color: token.colorTextTertiary,
            fontSize: 12,
            marginBottom: 4,
          }}
        >
          {t("workflow.rightPanel.name")}
        </label>
        <Input
          value={currentTemplate.name}
          size="small"
          onChange={(e) =>
            useWorkflowEditorStore
              .getState()
              .updateTemplateMetadata({ name: e.target.value })}
        />
      </div>

      <div style={{ marginBottom: 12 }}>
        <label
          style={{
            display: "block",
            color: token.colorTextTertiary,
            fontSize: 12,
            marginBottom: 4,
          }}
        >
          {t("workflow.rightPanel.description")}
        </label>
        <Input.TextArea
          id="right-panel-input-textarea-125"
          value={currentTemplate.description || ""}
          rows={3}
          size="small"
          onChange={(e) =>
            useWorkflowEditorStore
              .getState()
              .updateTemplateMetadata({ description: e.target.value })}
        />
      </div>

      <div style={{ marginBottom: 12 }}>
        <label
          style={{
            display: "block",
            color: token.colorTextTertiary,
            fontSize: 12,
            marginBottom: 4,
          }}
        >
          {t("workflow.rightPanel.icon")}
        </label>
        <Select
          value={currentTemplate.icon}
          size="small"
          style={{ width: "100%" }}
          onChange={(icon) => useWorkflowEditorStore.getState().updateTemplateMetadata({ icon })}
          options={[
            { value: "Bot", label: t("workflow.rightPanel.iconBot") },
            { value: "Code", label: t("workflow.rightPanel.iconCode") },
            { value: "FileText", label: t("workflow.rightPanel.iconDocument") },
            {
              value: "GitBranch",
              label: t("workflow.rightPanel.iconGitBranch"),
            },
            { value: "Zap", label: t("workflow.rightPanel.iconZap") },
            { value: "Layers", label: t("workflow.rightPanel.iconLayers") },
          ]}
        />
      </div>

      <Divider style={{ margin: "8px 0", borderColor: token.colorBorderSecondary }} />

      <div>
        <label
          style={{
            display: "block",
            color: token.colorTextTertiary,
            fontSize: 12,
            marginBottom: 4,
          }}
        >
          {t("workflow.rightPanel.tags")}
        </label>
        <Select
          mode="tags"
          value={currentTemplate.tags || []}
          size="small"
          style={{ width: "100%" }}
          onChange={(tags) => useWorkflowEditorStore.getState().updateTemplateMetadata({ tags })}
          placeholder={t("workflow.rightPanel.addTagsPlaceholder")}
          options={[
            { value: "ai", label: t("workflow.rightPanel.tagAi") },
            {
              value: "automation",
              label: t("workflow.rightPanel.tagAutomation"),
            },
            { value: "workflow", label: t("workflow.rightPanel.tagWorkflow") },
            { value: "agent", label: t("workflow.rightPanel.tagAgent") },
            { value: "chatbot", label: t("workflow.rightPanel.tagChatbot") },
            {
              value: "data-processing",
              label: t("workflow.rightPanel.tagDataProcessing"),
            },
          ]}
        />
      </div>

      <div>
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
            marginBottom: 4,
          }}
        >
          <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>
            {t("workflow.rightPanel.inputSchema")}
          </label>
          <Button
            type="link"
            size="small"
            onClick={() => openSchemaModal("input")}
            style={{ fontSize: 11 }}
          >
            {currentTemplate?.inputSchema
              ? t("workflow.rightPanel.edit")
              : t("workflow.rightPanel.add")}
          </Button>
        </div>
        {currentTemplate?.inputSchema && (
          <pre
            style={{
              fontSize: 10,
              color: token.colorTextTertiary,
              background: token.colorBgContainer,
              padding: 8,
              borderRadius: 4,
              maxHeight: 120,
              overflow: "auto",
              whiteSpace: "pre-wrap",
              margin: 0,
            }}
          >
            {JSON.stringify(currentTemplate.inputSchema, null, 2)}
          </pre>
        )}
      </div>

      <div>
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
            marginBottom: 4,
          }}
        >
          <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>
            {t("workflow.rightPanel.outputSchema")}
          </label>
          <Button
            type="link"
            size="small"
            onClick={() => openSchemaModal("output")}
            style={{ fontSize: 11 }}
          >
            {currentTemplate?.outputSchema
              ? t("workflow.rightPanel.edit")
              : t("workflow.rightPanel.add")}
          </Button>
        </div>
        {currentTemplate?.outputSchema && (
          <pre
            style={{
              fontSize: 10,
              color: token.colorTextTertiary,
              background: token.colorBgContainer,
              padding: 8,
              borderRadius: 4,
              maxHeight: 120,
              overflow: "auto",
              whiteSpace: "pre-wrap",
              margin: 0,
            }}
          >
            {JSON.stringify(currentTemplate.outputSchema, null, 2)}
          </pre>
        )}
      </div>

      <SchemaEditorModal
        open={schemaModal.open}
        title={schemaModal.kind === "input"
          ? t("workflow.rightPanel.inputSchema")
          : t("workflow.rightPanel.outputSchema")}
        initial={schemaModal.initial}
        onCancel={() => setSchemaModal((s) => ({ ...s, open: false }))}
        onConfirm={handleSchemaConfirm}
      />
    </div>
  );
}

export const RightPanel: React.FC<RightPanelProps> = React.memo(
  ({ width, selectedNodeId, selectedEdge }) => {
    const { t } = useTranslation();
    const { token } = theme.useToken();
    const deleteNode = useWorkflowEditorStore((state) => state.deleteNode);
    const deleteEdge = useWorkflowEditorStore((state) => state.deleteEdge);
    const updateNode = useWorkflowEditorStore((state) => state.updateNode);
    const currentTemplate = useWorkflowEditorStore(
      (state) => state.currentTemplate,
    );
    const selectedNode = useWorkflowEditorStore(
      (state) => (selectedNodeId ? state.nodes.find((n) => n.id === selectedNodeId) ?? null : null),
    );
    const [localEdgeLabel, setLocalEdgeLabel] = React.useState("");
    React.useEffect(() => {
      setLocalEdgeLabel(selectedEdge?.label || "");
    }, [selectedEdge?.id, selectedEdge?.label]);

    const handleUpdateNode = (updates: Partial<WorkflowNode>) => {
      if (selectedNode) {
        updateNode(selectedNode.id, updates);
      }
    };

    const handleDeleteNode = () => {
      if (selectedNode) {
        deleteNode(selectedNode.id);
      }
    };

    const renderNodeProperties = () => {
      if (!selectedNode) {
        return (
          <div style={{ padding: 20, textAlign: "center", color: token.colorTextTertiary }}>
            {t("workflow.rightPanel.selectNodeToEdit")}
          </div>
        );
      }

      const nodeTypeInfo = NODE_TYPE_MAP[selectedNode.type] || {
        labelKey: "",
        color: token.colorTextTertiary,
      };

      return (
        <div style={{ padding: 12 }}>
          <div style={{ marginBottom: 12 }}>
            <div
              style={{
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
              }}
            >
              <span
                style={{
                  color: token.colorTextTertiary,
                  fontSize: 12,
                  textTransform: "uppercase",
                }}
              >
                {t("workflow.rightPanel.nodeType")}
              </span>
              <span
                style={{
                  background: `${nodeTypeInfo.color}20`,
                  color: nodeTypeInfo.color,
                  padding: "2px 8px",
                  borderRadius: 4,
                  fontSize: 12,
                  fontWeight: 500,
                }}
              >
                {nodeTypeInfo.labelKey
                  ? t(nodeTypeInfo.labelKey)
                  : selectedNode.type}
              </span>
            </div>
          </div>

          <Divider style={{ margin: "8px 0", borderColor: token.colorBorderSecondary }} />

          <NodePropertyPanel
            selectedNode={selectedNode}
            onUpdate={handleUpdateNode}
            onDelete={handleDeleteNode}
          />
        </div>
      );
    };

    const renderEdgeProperties = () => {
      if (!selectedEdge) {
        return (
          <div style={{ padding: 20, textAlign: "center", color: token.colorTextTertiary }}>
            {t("workflow.rightPanel.selectEdgeToEdit")}
          </div>
        );
      }

      return (
        <div style={{ padding: 12 }}>
          <div style={{ marginBottom: 12 }}>
            <label
              style={{
                display: "block",
                color: token.colorTextTertiary,
                fontSize: 12,
                marginBottom: 4,
              }}
            >
              {t("workflow.rightPanel.edgeType")}
            </label>
            <Select
              value={selectedEdge.edge_type}
              onChange={(edge_type) =>
                useWorkflowEditorStore
                  .getState()
                  .updateEdge(selectedEdge.id, { edge_type })}
              size="small"
              style={{ width: "100%" }}
              options={[
                {
                  value: "direct",
                  label: t("workflow.rightPanel.edgeTypeDirect"),
                },
                {
                  value: "conditionTrue",
                  label: t("workflow.rightPanel.edgeTypeConditionTrue"),
                },
                {
                  value: "conditionFalse",
                  label: t("workflow.rightPanel.edgeTypeConditionFalse"),
                },
                {
                  value: "loopBack",
                  label: t("workflow.rightPanel.edgeTypeLoopBack"),
                },
                {
                  value: "parallelBranch",
                  label: t("workflow.rightPanel.edgeTypeParallelBranch"),
                },
                {
                  value: "merge",
                  label: t("workflow.rightPanel.edgeTypeMerge"),
                },
                {
                  value: "error",
                  label: t("workflow.rightPanel.edgeTypeError"),
                },
                {
                  value: "debateRound",
                  label: t("workflow.rightPanel.edgeTypeDebateRound"),
                },
                {
                  value: "grouping",
                  label: t("workflow.rightPanel.edgeTypeGrouping"),
                },
              ]}
            />
          </div>

          <div style={{ marginBottom: 12 }}>
            <label
              style={{
                display: "block",
                color: token.colorTextTertiary,
                fontSize: 12,
                marginBottom: 4,
              }}
            >
              {t("workflow.rightPanel.label")}
            </label>
            <Input
              id="right-panel-input-124"
              value={localEdgeLabel}
              onChange={(e) => setLocalEdgeLabel(e.target.value)}
              onBlur={() => {
                if (localEdgeLabel !== (selectedEdge?.label || "")) {
                  useWorkflowEditorStore
                    .getState()
                    .updateEdge(selectedEdge!.id, { label: localEdgeLabel || undefined });
                }
              }}
              size="small"
              placeholder={t("workflow.rightPanel.edgeLabelPlaceholder")}
            />
          </div>

          <Divider style={{ margin: "8px 0", borderColor: token.colorBorderSecondary }} />

          <Button
            type="text"
            danger
            icon={<Trash2 size={14} />}
            onClick={() => deleteEdge(selectedEdge.id)}
            style={{ width: "100%", justifyContent: "flex-start" }}
          >
            {t("workflow.rightPanel.deleteEdge")}
          </Button>
        </div>
      );
    };

    return (
      <div
        className="workflow-side-panel"
        style={{
          width,
          background: token.colorBgContainer,
          borderLeft: `1px solid ${token.colorBorderSecondary}`,
          display: "flex",
          flexDirection: "column",
          overflow: "hidden",
        }}
      >
        <Tabs
          defaultActiveKey="properties"
          size="small"
          style={{ height: "100%" }}
          items={[
            {
              key: "properties",
              label: t("workflow.rightPanel.properties"),
              children: (
                <div
                  style={{ height: "100%", overflow: "auto" }}
                >
                  {selectedNode
                    ? (
                      renderNodeProperties()
                    )
                    : selectedEdge
                    ? (
                      renderEdgeProperties()
                    )
                    : (
                      <div
                        style={{
                          padding: 20,
                          textAlign: "center",
                          color: token.colorTextTertiary,
                        }}
                      >
                        {t("workflow.rightPanel.selectNodeOrEdge")}
                      </div>
                    )}
                </div>
              ),
            },
            {
              key: "settings",
              label: t("workflow.rightPanel.settings"),
              children: (
                <div
                  style={{ height: "100%", overflow: "auto" }}
                >
                  <TemplateSettings currentTemplate={currentTemplate} />
                </div>
              ),
            },
          ]}
        />
      </div>
    );
  },
);
