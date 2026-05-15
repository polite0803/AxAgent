import { useWorkflowEditorStore } from "@/stores";
import { Button, Divider, Input, Select, Tabs } from "antd";
import { Trash2 } from "lucide-react";
import React from "react";
import { useTranslation } from "react-i18next";
import { NODE_TYPE_MAP, type WorkflowEdge, type WorkflowNode } from "../types";
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

export const RightPanel: React.FC<RightPanelProps> = React.memo(({ selectedNode, selectedEdge }) => {
  const { t } = useTranslation();
  const deleteNode = useWorkflowEditorStore((state) => state.deleteNode);
  const deleteEdge = useWorkflowEditorStore((state) => state.deleteEdge);
  const updateNode = useWorkflowEditorStore((state) => state.updateNode);
  const currentTemplate = useWorkflowEditorStore((state) => state.currentTemplate);

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

    const nodeTypeInfo = NODE_TYPE_MAP[selectedNode.type] || { labelKey: "", color: "#999" };

    const renderPropertyPanel = () => {
      switch (selectedNode.type) {
        case "trigger":
          return (
            <TriggerPropertyPanel
              node={selectedNode}
              onUpdate={handleUpdateNode}
              onDelete={handleDeleteNode}
            />
          );
        case "agent":
          return (
            <AgentPropertyPanel
              node={selectedNode}
              onUpdate={handleUpdateNode}
              onDelete={handleDeleteNode}
            />
          );
        case "llm":
          return (
            <LLMPropertyPanel
              node={selectedNode}
              onUpdate={handleUpdateNode}
              onDelete={handleDeleteNode}
            />
          );
        case "condition":
          return (
            <ConditionPropertyPanel
              node={selectedNode}
              onUpdate={handleUpdateNode}
              onDelete={handleDeleteNode}
            />
          );
        case "parallel":
          return (
            <ParallelPropertyPanel
              node={selectedNode}
              onUpdate={handleUpdateNode}
              onDelete={handleDeleteNode}
            />
          );
        case "loop":
          return (
            <LoopPropertyPanel
              node={selectedNode}
              onUpdate={handleUpdateNode}
              onDelete={handleDeleteNode}
            />
          );
        case "merge":
          return (
            <MergePropertyPanel
              node={selectedNode}
              onUpdate={handleUpdateNode}
              onDelete={handleDeleteNode}
            />
          );
        case "delay":
          return (
            <DelayPropertyPanel
              node={selectedNode}
              onUpdate={handleUpdateNode}
              onDelete={handleDeleteNode}
            />
          );
        case "tool":
          return (
            <ToolPropertyPanel
              node={selectedNode}
              onUpdate={handleUpdateNode}
              onDelete={handleDeleteNode}
            />
          );
        case "code":
          return (
            <CodePropertyPanel
              node={selectedNode}
              onUpdate={handleUpdateNode}
              onDelete={handleDeleteNode}
            />
          );
        case "subWorkflow":
          return (
            <SubWorkflowPropertyPanel
              node={selectedNode}
              onUpdate={handleUpdateNode}
              onDelete={handleDeleteNode}
            />
          );
        case "documentParser":
          return (
            <DocumentParserPropertyPanel
              node={selectedNode}
              onUpdate={handleUpdateNode}
              onDelete={handleDeleteNode}
            />
          );
        case "vectorRetrieve":
          return (
            <VectorRetrievePropertyPanel
              node={selectedNode}
              onUpdate={handleUpdateNode}
              onDelete={handleDeleteNode}
            />
          );
        case "validation":
          return (
            <ValidationPropertyPanel
              node={selectedNode}
              onUpdate={handleUpdateNode}
              onDelete={handleDeleteNode}
            />
          );
        case "end":
          return (
            <EndPropertyPanel
              node={selectedNode}
              onUpdate={handleUpdateNode}
              onDelete={handleDeleteNode}
            />
          );
        default:
          return (
            <div style={{ color: "#666", textAlign: "center", padding: 20 }}>
              {t("workflow.rightPanel.unsupportedNodeType")}
            </div>
          );
      }
    };

    return (
      <div style={{ padding: 12 }}>
        <div style={{ marginBottom: 12 }}>
          <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
            <span style={{ color: "#999", fontSize: 11, textTransform: "uppercase" }}>
              {t("workflow.rightPanel.nodeType")}
            </span>
            <span
              style={{
                background: `${nodeTypeInfo.color}20`,
                color: nodeTypeInfo.color,
                padding: "2px 8px",
                borderRadius: 4,
                fontSize: 11,
                fontWeight: 500,
              }}
            >
              {nodeTypeInfo.labelKey ? t(nodeTypeInfo.labelKey) : selectedNode.type}
            </span>
          </div>
        </div>

        <Divider style={{ margin: "8px 0", borderColor: "#333" }} />

        {renderPropertyPanel()}
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
          <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>
            {t("workflow.rightPanel.edgeType")}
          </label>
          <Select
            value={selectedEdge.edge_type}
            onChange={(edge_type) => useWorkflowEditorStore.getState().updateEdge(selectedEdge.id, { edge_type })}
            size="small"
            style={{ width: "100%" }}
            options={[
              { value: "direct", label: t("workflow.rightPanel.edgeTypeDirect") },
              { value: "conditionTrue", label: t("workflow.rightPanel.edgeTypeConditionTrue") },
              { value: "conditionFalse", label: t("workflow.rightPanel.edgeTypeConditionFalse") },
              { value: "loopBack", label: t("workflow.rightPanel.edgeTypeLoopBack") },
              { value: "parallelBranch", label: t("workflow.rightPanel.edgeTypeParallelBranch") },
              { value: "merge", label: t("workflow.rightPanel.edgeTypeMerge") },
              { value: "error", label: t("workflow.rightPanel.edgeTypeError") },
            ]}
          />
        </div>

        <div style={{ marginBottom: 12 }}>
          <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>
            {t("workflow.rightPanel.label")}
          </label>
          <Input
            id="right-panel-input-124"
            value={selectedEdge.label || ""}
            onChange={(e) => useWorkflowEditorStore.getState().updateEdge(selectedEdge.id, { label: e.target.value })}
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

  const renderTemplateSettings = () => {
    if (!currentTemplate) { return null; }

    return (
      <div style={{ padding: 12 }}>
        <div style={{ marginBottom: 12 }}>
          <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>
            {t("workflow.rightPanel.name")}
          </label>
          <Input
            value={currentTemplate.name}
            size="small"
            onChange={(e) => useWorkflowEditorStore.getState().updateTemplateMetadata({ name: e.target.value })}
          />
        </div>

        <div style={{ marginBottom: 12 }}>
          <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>
            {t("workflow.rightPanel.description")}
          </label>
          <Input.TextArea
            id="right-panel-input-textarea-125"
            value={currentTemplate.description || ""}
            rows={3}
            size="small"
            onChange={(e) => useWorkflowEditorStore.getState().updateTemplateMetadata({ description: e.target.value })}
          />
        </div>

        <div style={{ marginBottom: 12 }}>
          <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>
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
              { value: "GitBranch", label: t("workflow.rightPanel.iconGitBranch") },
              { value: "Zap", label: t("workflow.rightPanel.iconZap") },
              { value: "Layers", label: t("workflow.rightPanel.iconLayers") },
            ]}
          />
        </div>

        <Divider style={{ margin: "8px 0", borderColor: "#333" }} />

        <div>
          <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>
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
              { value: "automation", label: t("workflow.rightPanel.tagAutomation") },
              { value: "workflow", label: t("workflow.rightPanel.tagWorkflow") },
              { value: "agent", label: t("workflow.rightPanel.tagAgent") },
              { value: "chatbot", label: t("workflow.rightPanel.tagChatbot") },
              { value: "data-processing", label: t("workflow.rightPanel.tagDataProcessing") },
            ]}
          />
        </div>
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
              <div style={{ overflow: "auto", maxHeight: "calc(100vh - 120px)" }}>
                {selectedNode
                  ? renderNodeProperties()
                  : selectedEdge
                  ? renderEdgeProperties()
                  : (
                    <div style={{ padding: 20, textAlign: "center", color: "#666" }}>
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
              <div style={{ overflow: "auto", maxHeight: "calc(100vh - 120px)" }}>
                {renderTemplateSettings()}
              </div>
            ),
          },
        ]}
      />
    </div>
  );
});
