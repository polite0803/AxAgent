import { Tag } from "antd";
import React, { memo } from "react";
import { useTranslation } from "react-i18next";
import { Handle, type NodeProps, Position } from "reactflow";

interface ConditionNodeData {
  id: string;
  type: string;
  title: string;
  description?: string;
  color: string;
  nodeType: string;
  enabled: boolean;
  conditions?: Array<{
    var_path: string;
    operator: string;
    value: unknown;
  }>;
  logicalOp?: "and" | "or";
  validationState?: "error" | "warning";
}

const ConditionNodeComponent: React.FC<NodeProps<ConditionNodeData>> = ({
  data,
  selected,
}) => {
  const { t } = useTranslation();
  const color = "#fa8c16";
  const conditions = data.conditions || [];
  const logicalOp = data.logicalOp || "and";

  const getBorderColor = () => {
    if (data.validationState === "error") {
      return "#ff4d4f";
    }
    if (data.validationState === "warning") {
      return "#faad14";
    }
    if (selected) {
      return "#1890ff";
    }
    return color;
  };

  const borderColor = getBorderColor();

  const getOperatorLabel = (op: string): string => {
    const labels: Record<string, string> = {
      eq: "=",
      ne: "≠",
      gt: ">",
      lt: "<",
      gte: "≥",
      lte: "≤",
      contains: t("workflow.conditionNode.opContains"),
      notContains: t("workflow.conditionNode.opNotContains"),
      startsWith: t("workflow.conditionNode.opStartsWith"),
      endsWith: t("workflow.conditionNode.opEndsWith"),
      regexMatch: t("workflow.conditionNode.opRegexMatch"),
      isEmpty: t("workflow.conditionNode.opIsEmpty"),
      isNotEmpty: t("workflow.conditionNode.opIsNotEmpty"),
    };
    return labels[op] || op;
  };

  const formatValue = (value: unknown): string => {
    if (value === null || value === undefined) {
      return "";
    }
    if (typeof value === "string") {
      return value.length > 10 ? `${value.slice(0, 10)}...` : value;
    }
    if (typeof value === "number") {
      return String(value);
    }
    return JSON.stringify(value).slice(0, 10);
  };

  return (
    <div
      style={{
        minWidth: 200,
        maxWidth: 260,
        opacity: data.enabled ? 1 : 0.5,
        filter: data.enabled ? "none" : "grayscale(100%)",
      }}
    >
      <div
        style={{
          background: "#1e1e1e",
          border: `2px solid ${borderColor}`,
          borderRadius: 8,
          overflow: "hidden",
          boxShadow: selected ? `0 0 0 2px ${borderColor}40` : "none",
          transition: "box-shadow 0.2s, transform 0.2s",
        }}
      >
        <div
          style={{
            padding: "8px 12px",
            borderBottom: `1px solid ${color}30`,
            display: "flex",
            alignItems: "center",
            gap: 8,
            background: `${color}15`,
          }}
        >
          <span style={{ fontSize: 14 }}>🔀</span>
          <span
            style={{
              fontSize: 12,
              color: color,
              fontWeight: 600,
            }}
          >
            {t("workflow.conditionNode.title")}
          </span>
          <Tag
            style={{
              margin: 0,
              fontSize: 9,
              padding: "0 4px",
              background: `${color}30`,
              border: "none",
              color: "#fff",
            }}
          >
            {logicalOp.toUpperCase()}
          </Tag>
        </div>

        <div style={{ padding: "10px 12px" }}>
          <div
            style={{
              fontSize: 13,
              color: "#fff",
              fontWeight: 500,
              marginBottom: 8,
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
            }}
          >
            {data.title}
          </div>

          {conditions.length > 0
            ? (
              <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                {conditions.slice(0, 3).map((condition, _index) => (
                  <div
                    key={`${condition.var_path}-${condition.operator}-${String(condition.value)}`}
                    style={{
                      fontSize: 12,
                      color: "#aaa",
                      padding: "4px 6px",
                      background: "#252525",
                      borderRadius: 4,
                      display: "flex",
                      alignItems: "center",
                      gap: 4,
                      overflow: "hidden",
                    }}
                  >
                    <span
                      style={{
                        color: "#888",
                        overflow: "hidden",
                        textOverflow: "ellipsis",
                        whiteSpace: "nowrap",
                        flex: 1,
                      }}
                    >
                      {condition.var_path || t("workflow.conditionNode.notSet")}
                    </span>
                    <span style={{ color: color, fontWeight: 500 }}>
                      {getOperatorLabel(condition.operator)}
                    </span>
                    <span
                      style={{
                        color: "#52c41a",
                        overflow: "hidden",
                        textOverflow: "ellipsis",
                        whiteSpace: "nowrap",
                        maxWidth: 60,
                      }}
                    >
                      {formatValue(condition.value)}
                    </span>
                  </div>
                ))}
                {conditions.length > 3 && (
                  <div
                    style={{
                      fontSize: 9,
                      color: "#666",
                      textAlign: "center",
                    }}
                  >
                    {t("workflow.conditionNode.moreConditions", {
                      count: conditions.length - 3,
                    })}
                  </div>
                )}
              </div>
            )
            : (
              <div
                style={{
                  fontSize: 12,
                  color: "#666",
                  textAlign: "center",
                  padding: 8,
                  background: "#252525",
                  borderRadius: 4,
                }}
              >
                {t("workflow.conditionNode.clickToEdit")}
              </div>
            )}
        </div>
      </div>

      <Handle
        type="target"
        position={Position.Top}
        style={{
          background: color,
          border: "none",
          width: 8,
          height: 8,
        }}
      />

      <Handle
        type="source"
        position={Position.Bottom}
        id="true"
        style={{
          background: "#52c41a",
          border: "none",
          width: 8,
          height: 8,
          left: "30%",
        }}
      />

      <Handle
        type="source"
        position={Position.Bottom}
        id="false"
        style={{
          background: "#ff4d4f",
          border: "none",
          width: 8,
          height: 8,
          left: "70%",
        }}
      />

      <div
        style={{
          display: "flex",
          justifyContent: "space-around",
          marginTop: 4,
        }}
      >
        <Tag color="green" style={{ margin: 0, fontSize: 9 }}>
          {t("workflow.conditionNode.true")}
        </Tag>
        <Tag color="red" style={{ margin: 0, fontSize: 9 }}>
          {t("workflow.conditionNode.false")}
        </Tag>
      </div>
    </div>
  );
};

export const ConditionNode = memo(ConditionNodeComponent);
