import { Button, Divider, Input, Select } from "antd";
import { Plus, Trash2 } from "lucide-react";
import React from "react";
import { useTranslation } from "react-i18next";
import type { CompareOperator, Condition, ConditionNode, LogicalOperator, WorkflowNode } from "../../types";
import { BasePropertyPanel } from "./BasePropertyPanel";

interface ConditionPropertyPanelProps {
  node: WorkflowNode;
  onUpdate: (updates: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}

export const ConditionPropertyPanel: React.FC<ConditionPropertyPanelProps> = ({
  node,
  onUpdate,
  onDelete,
}) => {
  const { t } = useTranslation();
  const conditionNode = node as ConditionNode;
  const config = conditionNode.config || {
    conditions: [],
    logical_op: "and" as LogicalOperator,
  };

  const OPERATOR_OPTIONS: { value: CompareOperator; label: string }[] = [
    { value: "eq", label: "=" },
    { value: "ne", label: "≠" },
    { value: "gt", label: ">" },
    { value: "lt", label: "<" },
    { value: "gte", label: "≥" },
    { value: "lte", label: "≤" },
    { value: "contains", label: t("workflow.props.opContains") },
    { value: "notContains", label: t("workflow.props.opNotContains") },
    { value: "startsWith", label: t("workflow.props.opStartsWith") },
    { value: "endsWith", label: t("workflow.props.opEndsWith") },
    { value: "regexMatch", label: t("workflow.props.opRegexMatch") },
    { value: "isEmpty", label: t("workflow.props.opIsEmpty") },
    { value: "isNotEmpty", label: t("workflow.props.opIsNotEmpty") },
  ];

  const handleAddCondition = () => {
    const newCondition: Condition = {
      var_path: "",
      operator: "eq",
      value: "",
    };
    onUpdate({
      config: {
        ...config,
        conditions: [...config.conditions, newCondition],
      },
    });
  };

  const handleUpdateCondition = (
    index: number,
    updates: Partial<Condition>,
  ) => {
    const newConditions = [...config.conditions];
    newConditions[index] = { ...newConditions[index], ...updates };
    onUpdate({
      config: {
        ...config,
        conditions: newConditions,
      },
    });
  };

  const handleDeleteCondition = (index: number) => {
    const newConditions = config.conditions.filter((_, i) => i !== index);
    onUpdate({
      config: {
        ...config,
        conditions: newConditions,
      },
    });
  };

  const handleLogicalOpChange = (logical_op: LogicalOperator) => {
    onUpdate({
      config: {
        ...config,
        logical_op,
      },
    });
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      <div>
        <label
          style={{
            display: "block",
            color: "#999",
            fontSize: 12,
            marginBottom: 4,
          }}
        >
          {t("workflow.props.logicalOp")}
        </label>
        <Select
          value={config.logical_op}
          onChange={handleLogicalOpChange}
          size="small"
          style={{ width: "100%" }}
          options={[
            { value: "and", label: t("workflow.props.andAllTrue") },
            { value: "or", label: t("workflow.props.orAnyTrue") },
          ]}
        />
      </div>

      <div>
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            marginBottom: 8,
          }}
        >
          <label style={{ color: "#999", fontSize: 12 }}>
            {t("workflow.props.conditions")} ({config.conditions.length})
          </label>
          <Button
            type="dashed"
            size="small"
            icon={<Plus size={12} />}
            onClick={handleAddCondition}
          >
            {t("workflow.props.addCondition")}
          </Button>
        </div>

        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          {config.conditions.map((condition, index) => (
            <div
              key={`${condition.var_path}-${condition.operator}-${String(condition.value)}-${index}`}
              style={{
                padding: 8,
                background: "#1e1e1e",
                borderRadius: 6,
                border: "1px solid #333",
              }}
            >
              <div style={{ marginBottom: 8 }}>
                <Input
                  id="condition-property-panel-input-87"
                  value={condition.var_path}
                  onChange={(e) => handleUpdateCondition(index, { var_path: e.target.value })}
                  size="small"
                  placeholder={t("workflow.props.conditionVarPath")}
                />
              </div>

              <div
                style={{
                  display: "flex",
                  gap: 4,
                  marginBottom: condition.operator === "isEmpty"
                      || condition.operator === "isNotEmpty"
                    ? 0
                    : 8,
                }}
              >
                <Select
                  value={condition.operator}
                  onChange={(value) => handleUpdateCondition(index, { operator: value })}
                  size="small"
                  style={{ flex: 1 }}
                  options={OPERATOR_OPTIONS}
                />

                {condition.operator !== "isEmpty"
                  && condition.operator !== "isNotEmpty" && (
                  <Input
                    id="condition-property-panel-input-88"
                    value={String(condition.value || "")}
                    onChange={(e) => handleUpdateCondition(index, { value: e.target.value })}
                    size="small"
                    placeholder={t("workflow.props.conditionValue")}
                    style={{ flex: 1 }}
                  />
                )}

                <Button
                  type="text"
                  danger
                  size="small"
                  icon={<Trash2 size={12} />}
                  onClick={() => handleDeleteCondition(index)}
                />
              </div>
            </div>
          ))}

          {config.conditions.length === 0 && (
            <div
              style={{
                color: "#666",
                fontSize: 12,
                textAlign: "center",
                padding: 16,
              }}
            >
              {t("workflow.props.clickToAddCondition")}
            </div>
          )}
        </div>
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
