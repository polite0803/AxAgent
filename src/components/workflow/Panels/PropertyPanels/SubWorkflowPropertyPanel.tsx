import { useWorkflowEditorStore } from "@/stores";
import { Button, Divider, Input, Select, Switch } from "antd";
import React, { useEffect, useMemo } from "react";
import { useTranslation } from "react-i18next";
import type { SubWorkflowNode, WorkflowNode } from "../../types";
import { BasePropertyPanel } from "./BasePropertyPanel";

interface SubWorkflowPropertyPanelProps {
  node: WorkflowNode;
  onUpdate: (updates: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}

export const SubWorkflowPropertyPanel: React.FC<
  SubWorkflowPropertyPanelProps
> = ({ node, onUpdate, onDelete }) => {
  const { t } = useTranslation();
  const subWorkflowNode = node as SubWorkflowNode;
  const config = subWorkflowNode.config || {
    sub_workflow_id: "",
    input_mapping: {},
    output_var: "",
    is_async: false,
  };

  const { templates, loadTemplates, currentTemplate } = useWorkflowEditorStore();

  useEffect(() => {
    if (templates.length === 0) {
      loadTemplates();
    }
  }, [templates.length, loadTemplates]);

  const workflowOptions = useMemo(
    () => templates.flatMap((t) => t.id !== currentTemplate?.id ? [{ value: t.id, label: t.name }] : []),
    [templates, currentTemplate?.id],
  );

  const handleConfigChange = (key: string, value: unknown) => {
    onUpdate({ config: { ...config, [key]: value } });
  };

  const handleAddInputMapping = () => {
    onUpdate({
      config: {
        ...config,
        input_mapping: {
          ...config.input_mapping,
          [`param_${Object.keys(config.input_mapping).length + 1}`]: "",
        },
      },
    });
  };

  const handleUpdateInputMapping = (key: string, value: string) => {
    onUpdate({
      config: {
        ...config,
        input_mapping: {
          ...config.input_mapping,
          [key]: value,
        },
      },
    });
  };

  const handleDeleteInputMapping = (key: string) => {
    const newMapping = { ...config.input_mapping };
    delete newMapping[key];
    onUpdate({
      config: {
        ...config,
        input_mapping: newMapping,
      },
    });
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      <div>
        <label
          htmlFor="sub-workflow-select"
          style={{
            display: "block",
            color: "#999",
            fontSize: 12,
            marginBottom: 4,
          }}
        >
          Sub Workflow
        </label>
        <Select
          id="sub-workflow-select"
          value={config.sub_workflow_id || undefined}
          onChange={(value) => handleConfigChange("sub_workflow_id", value)}
          size="small"
          style={{ width: "100%" }}
          placeholder={t("workflow.props.selectSubWorkflow")}
          showSearch
          optionFilterProp="label"
          options={workflowOptions}
        />
      </div>

      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
        }}
      >
        <label style={{ color: "#999", fontSize: 12 }}>
          {t("workflow.props.asyncExecution")}
        </label>
        <Switch
          size="small"
          checked={config.is_async ?? false}
          onChange={(checked) => handleConfigChange("is_async", checked)}
        />
      </div>

      <div>
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            marginBottom: 4,
          }}
        >
          <label style={{ color: "#999", fontSize: 12 }}>
            {t("workflow.props.inputMapping")}
          </label>
          <Button type="link" size="small" onClick={handleAddInputMapping}>
            {t("workflow.props.addMapping")}
          </Button>
        </div>

        <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
          {Object.entries(config.input_mapping || {}).map(([key, value]) => (
            <div
              key={key}
              style={{ display: "flex", gap: 4, alignItems: "center" }}
            >
              <Input
                id="sub-workflow-property-panel-input-107"
                value={key}
                size="small"
                disabled
                style={{ width: 80, fontSize: 12 }}
              />
              <span style={{ color: "#666", fontSize: 12 }}>←</span>
              <Input
                id="sub-workflow-property-panel-input-108"
                value={String(value)}
                onChange={(e) => handleUpdateInputMapping(key, e.target.value)}
                size="small"
                placeholder={t("workflow.props.variable")}
                style={{ flex: 1 }}
              />
              <Button
                type="link"
                size="small"
                danger
                onClick={() => handleDeleteInputMapping(key)}
              >
                {t("workflow.props.delete")}
              </Button>
            </div>
          ))}

          {Object.keys(config.input_mapping || {}).length === 0 && (
            <div
              style={{
                color: "#666",
                fontSize: 12,
                textAlign: "center",
                padding: 8,
              }}
            >
              {t("workflow.props.clickToAddMapping")}
            </div>
          )}
        </div>
      </div>

      <div>
        <label
          style={{
            display: "block",
            color: "#999",
            fontSize: 12,
            marginBottom: 4,
          }}
        >
          {t("workflow.props.outputVariable")}
        </label>
        <Input
          id="sub-workflow-property-panel-input-109"
          value={config.output_var || ""}
          onChange={(e) => handleConfigChange("output_var", e.target.value)}
          size="small"
          placeholder="subworkflow_output"
        />
      </div>

      <Divider style={{ margin: "8px 0", borderColor: "#333" }} />

      <div
        style={{ borderTop: "1px solid #333", paddingTop: 12, marginTop: 4 }}
      >
        <BasePropertyPanel
          node={node}
          onUpdate={onUpdate}
          onDelete={onDelete}
        />
      </div>
    </div>
  );
};
