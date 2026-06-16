// SPDX-License-Identifier-Identifier: AGPL-3.0-only

import { useWorkflowEditorStore } from "@/stores";
import { Button, Divider, Input, Select, theme } from "antd";
import React, { useEffect, useMemo } from "react";
import { useTranslation } from "react-i18next";
import type { WorkflowNode, WorkflowRefNode } from "../../types";
import { BasePropertyPanel } from "./BasePropertyPanel";

interface Props {
  node: WorkflowNode;
  onUpdate: (u: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}

export const WorkflowRefPropertyPanel: React.FC<Props> = ({ node, onUpdate, onDelete }) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const n = node as unknown as WorkflowRefNode;
  const c = n.config || {
    target_workflow_id: "",
    input_mapping: {},
    output_var: "",
    context_mode: "inherit" as const,
  };
  const sc = (k: string, v: unknown) => onUpdate({ config: { ...c, [k]: v } });

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

  const handleAddInputMapping = () => {
    const existingKeys = Object.keys(c.input_mapping || {});
    let nextIndex = existingKeys.length + 1;
    let newKey = t("workflow.paramKey", { n: nextIndex, defaultValue: `param_${nextIndex}` });
    while (Object.prototype.hasOwnProperty.call(c.input_mapping || {}, newKey)) {
      nextIndex += 1;
      newKey = t("workflow.paramKey", { n: nextIndex, defaultValue: `param_${nextIndex}` });
    }
    sc("input_mapping", { ...c.input_mapping, [newKey]: "" });
  };

  const handleUpdateInputMapping = (key: string, value: string) => {
    sc("input_mapping", { ...c.input_mapping, [key]: value });
  };

  const handleDeleteInputMapping = (key: string) => {
    const newMapping = { ...c.input_mapping };
    delete newMapping[key];
    sc("input_mapping", newMapping);
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      <div>
        <label style={{ display: "block", color: token.colorTextTertiary, fontSize: 12, marginBottom: 4 }}>
          {t("workflow.workflowRef.targetWorkflow", { defaultValue: "Target Workflow" })}
        </label>
        <Select
          value={c.target_workflow_id || undefined}
          onChange={(v) => sc("target_workflow_id", v)}
          size="small"
          style={{ width: "100%" }}
          placeholder={t("workflow.props.selectSubWorkflow")}
          showSearch
          optionFilterProp="label"
          options={workflowOptions}
        />
      </div>
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
        <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>
          {t("workflow.workflowRef.contextMode", { defaultValue: "Context Mode" })}
        </label>
        <Select
          value={c.context_mode || "inherit"}
          onChange={(v) => sc("context_mode", v)}
          size="small"
          style={{ width: 140 }}
          options={[
            { value: "inherit", label: t("workflow.workflowRef.inherit", { defaultValue: "Inherit" }) },
            { value: "isolated", label: t("workflow.workflowRef.isolated", { defaultValue: "Isolated" }) },
          ]}
        />
      </div>
      <Divider style={{ margin: "8px 0", borderColor: token.colorBorderSecondary }} />
      <div>
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 4 }}>
          <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>
            {t("workflow.props.inputMapping")}
          </label>
          <Button type="link" size="small" onClick={handleAddInputMapping}>
            {t("workflow.props.addMapping")}
          </Button>
        </div>
        <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
          {Object.entries(c.input_mapping || {}).map(([key, value]) => (
            <div key={key} style={{ display: "flex", gap: 4, alignItems: "center" }}>
              <Input
                value={key}
                size="small"
                disabled
                style={{ width: 80, fontSize: 12 }}
              />
              <span style={{ color: token.colorTextTertiary, fontSize: 12 }}>←</span>
              <Input
                value={String(value)}
                onChange={(e) =>
                  handleUpdateInputMapping(key, e.target.value)}
                size="small"
                placeholder={t("workflow.props.variable")}
                style={{ flex: 1 }}
              />
              <Button
                type="link"
                size="small"
                danger
                onClick={() =>
                  handleDeleteInputMapping(key)}
              >
                {t("workflow.props.delete")}
              </Button>
            </div>
          ))}
          {Object.keys(c.input_mapping || {}).length === 0 && (
            <div style={{ color: token.colorTextTertiary, fontSize: 12, textAlign: "center", padding: 8 }}>
              {t("workflow.props.clickToAddMapping")}
            </div>
          )}
        </div>
      </div>
      <div>
        <label style={{ display: "block", color: token.colorTextTertiary, fontSize: 12, marginBottom: 4 }}>
          {t("workflow.props.outputVariable")}
        </label>
        <Input
          value={c.output_var ?? ""}
          onChange={(e) => sc("output_var", e.target.value)}
          size="small"
        />
      </div>
      <Divider style={{ margin: "8px 0", borderColor: token.colorBorderSecondary }} />
      <BasePropertyPanel node={node} onUpdate={onUpdate} onDelete={onDelete} />
    </div>
  );
};
