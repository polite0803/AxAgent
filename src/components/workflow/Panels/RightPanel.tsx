import { useWorkflowEditorStore } from "@/stores";
import { Button, Divider, Input, message, Select, Tabs } from "antd";
import { Trash2 } from "lucide-react";
import React from "react";
import { useTranslation } from "react-i18next";
import { type JsonSchema, NODE_TYPE_MAP, type WorkflowEdge, type WorkflowNode } from "../types";
import {
  AgentPropertyPanel,
  CodePropertyPanel,
  ConditionPropertyPanel,
  DelayPropertyPanel,
  DocumentParserPropertyPanel,
  EndPropertyPanel,
  LLMPropertyPanel,
  LoopPropertyPanel,
  MergePropertyPanel,
  ParallelPropertyPanel,
  SubWorkflowPropertyPanel,
  ToolPropertyPanel,
  TriggerPropertyPanel,
  ValidationPropertyPanel,
  VectorRetrievePropertyPanel,
} from "./PropertyPanels";

interface RightPanelProps {
  selectedNode: WorkflowNode | null;
  selectedEdge: WorkflowEdge | null;
}

/**
 * Extracted component for rendering the property panel based on node type.
 * Fixes react-doctor/no-render-in-render by moving renderPropertyPanel() out of renderNodeProperties.
 */
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
    default:
      return (
        <div style={{ color: "#666", textAlign: "center", padding: 20 }}>
          {t("workflow.rightPanel.unsupportedNodeType")}
        </div>
      );
  }
}

/**
 * Extracted component for rendering template settings.
 * Fixes react-doctor/no-render-in-render by moving renderTemplateSettings() out of RightPanel.
 */
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

  if (!currentTemplate) {
    return null;
  }

  return (
    <div style={{ padding: 12 }}>
      <div style={{ marginBottom: 12 }}>
        <label
          style={{
            display: "block",
            color: "#999",
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
            color: "#999",
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
            color: "#999",
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

      <Divider style={{ margin: "8px 0", borderColor: "#333" }} />

      <div>
        <label
          style={{
            display: "block",
            color: "#999",
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

      {/* Input Schema 编辑器 */}
      <div>
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
            marginBottom: 4,
          }}
        >
          <label style={{ color: "#999", fontSize: 12 }}>
            {t("workflow.rightPanel.inputSchema")}
          </label>
          <Button
            type="link"
            size="small"
            onClick={() => {
              try {
                const schema = currentTemplate?.inputSchema
                  ? JSON.stringify(currentTemplate.inputSchema, null, 2)
                  : "";
                const newSchema = window.prompt(
                  t("workflow.rightPanel.editSchema"),
                  schema,
                );
                if (newSchema !== null) {
                  if (newSchema.trim() === "") {
                    useWorkflowEditorStore
                      .getState()
                      .updateTemplateMetadata({ inputSchema: undefined });
                  } else {
                    useWorkflowEditorStore
                      .getState()
                      .updateTemplateMetadata({ inputSchema: JSON.parse(newSchema) });
                  }
                }
              } catch {
                message.error(t("workflow.rightPanel.invalidJson"));
              }
            }}
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
              color: "#888",
              background: "#1a1a2e",
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

      {/* Output Schema 编辑器 */}
      <div>
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
            marginBottom: 4,
          }}
        >
          <label style={{ color: "#999", fontSize: 12 }}>
            {t("workflow.rightPanel.outputSchema")}
          </label>
          <Button
            type="link"
            size="small"
            onClick={() => {
              try {
                const schema = currentTemplate?.outputSchema
                  ? JSON.stringify(currentTemplate.outputSchema, null, 2)
                  : "";
                const newSchema = window.prompt(
                  t("workflow.rightPanel.editSchema"),
                  schema,
                );
                if (newSchema !== null) {
                  if (newSchema.trim() === "") {
                    useWorkflowEditorStore
                      .getState()
                      .updateTemplateMetadata({ outputSchema: undefined });
                  } else {
                    useWorkflowEditorStore
                      .getState()
                      .updateTemplateMetadata({ outputSchema: JSON.parse(newSchema) });
                  }
                }
              } catch {
                message.error(t("workflow.rightPanel.invalidJson"));
              }
            }}
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
              color: "#888",
              background: "#1a1a2e",
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
    </div>
  );
}

export const RightPanel: React.FC<RightPanelProps> = React.memo(
  ({ selectedNode, selectedEdge }) => {
    const { t } = useTranslation();
    const deleteNode = useWorkflowEditorStore((state) => state.deleteNode);
    const deleteEdge = useWorkflowEditorStore((state) => state.deleteEdge);
    const updateNode = useWorkflowEditorStore((state) => state.updateNode);
    const currentTemplate = useWorkflowEditorStore(
      (state) => state.currentTemplate,
    );

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
          <div style={{ padding: 20, textAlign: "center", color: "#666" }}>
            {t("workflow.rightPanel.selectNodeToEdit")}
          </div>
        );
      }

      const nodeTypeInfo = NODE_TYPE_MAP[selectedNode.type] || {
        labelKey: "",
        color: "#999",
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
                  color: "#999",
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

          <Divider style={{ margin: "8px 0", borderColor: "#333" }} />

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
          <div style={{ padding: 20, textAlign: "center", color: "#666" }}>
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
                color: "#999",
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
              ]}
            />
          </div>

          <div style={{ marginBottom: 12 }}>
            <label
              style={{
                display: "block",
                color: "#999",
                fontSize: 12,
                marginBottom: 4,
              }}
            >
              {t("workflow.rightPanel.label")}
            </label>
            <Input
              id="right-panel-input-124"
              value={selectedEdge.label || ""}
              onChange={(e) =>
                useWorkflowEditorStore
                  .getState()
                  .updateEdge(selectedEdge.id, { label: e.target.value })}
              size="small"
              placeholder={t("workflow.rightPanel.edgeLabelPlaceholder")}
            />
          </div>

          <Divider style={{ margin: "8px 0", borderColor: "#333" }} />

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
        style={{
          width: 320,
          background: "#252525",
          borderLeft: "1px solid #333",
          display: "flex",
          flexDirection: "column",
          overflow: "hidden",
        }}
      >
        <Tabs
          defaultActiveKey="properties"
          size="small"
          style={{ height: "100%", overflow: "auto" }}
          items={[
            {
              key: "properties",
              label: t("workflow.rightPanel.properties"),
              children: (
                <div
                  style={{ overflow: "auto", maxHeight: "calc(100vh - 120px)" }}
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
                          color: "#666",
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
                  style={{ overflow: "auto", maxHeight: "calc(100vh - 120px)" }}
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
