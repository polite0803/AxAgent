import { Dropdown, message, Tag } from "antd";
import type { MenuProps } from "antd";
import React, { memo } from "react";
import { useTranslation } from "react-i18next";
import { Handle, type NodeProps, Position } from "reactflow";

interface SemanticMatchInfo {
  existing_skill_id: string;
  existing_skill_name: string;
  similarity_score: number;
  match_reasons: string[];
}

interface AtomicSkillNodeData {
  id: string;
  type: string;
  title: string;
  description?: string;
  color: string;
  nodeType: string;
  enabled: boolean;
  skillId?: string;
  skillName?: string;
  entryType?: string;
  inputMapping?: Record<string, string>;
  outputVar?: string;
  semanticMatch?: SemanticMatchInfo;
  onSemanticAction?: (nodeId: string, action: "replace" | "keep" | "upgrade_existing") => void;
  onUpgradeRequest?: (
    nodeId: string,
    existingSkillId: string,
    existingSkillName: string,
    generatedSkillName: string,
    generatedSkillDescription: string,
  ) => void;
}

const ENTRY_TYPE_COLORS: Record<string, string> = {
  builtin: "#1890ff",
  mcp: "#722ed1",
  local: "#52c41a",
  plugin: "#fa8c16",
};

const ENTRY_TYPE_LABELS: Record<string, string> = {
  builtin: "内置",
  mcp: "MCP",
  local: "本地",
  plugin: "插件",
};

const AtomicSkillNodeComponent: React.FC<NodeProps<AtomicSkillNodeData>> = ({ data, selected }) => {
  const color = "#13c2c2";
  const skillName = data.skillName || "未选择原子Skill";
  const entryType = data.entryType || "builtin";
  const entryColor = ENTRY_TYPE_COLORS[entryType] || color;
  const inputMapping = data.inputMapping || {};
  const outputVar = data.outputVar;
  const inputCount = Object.keys(inputMapping).length;
  const { t } = useTranslation("chat");

  const getMatchBadgeColor = (score: number) => {
    if (score >= 0.9) { return "#f5222d"; }
    if (score >= 0.8) { return "#fa8c16"; }
    if (score >= 0.7) { return "#faad14"; }
    return "#52c41a";
  };

  const handleMenuClick: MenuProps["onClick"] = ({ key }) => {
    const action = key as "replace" | "keep" | "upgrade_existing";
    if (action === "upgrade_existing" && data.onUpgradeRequest && data.semanticMatch) {
      data.onUpgradeRequest(
        data.id,
        data.semanticMatch.existing_skill_id,
        data.semanticMatch.existing_skill_name,
        data.title,
        data.description || "",
      );
      return;
    }
    if (data.onSemanticAction && data.semanticMatch) {
      data.onSemanticAction(data.id, action);
      message.success(t("workflow.semanticCheckApplied"));
    }
  };

  const matchMenuItems: MenuProps["items"] = data.semanticMatch
    ? [
      {
        key: "replace",
        label: (
          <span style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <span>🔄</span>
            <span>{t("workflow.replaceWithExisting")}</span>
            <Tag color="blue" style={{ margin: 0, fontSize: 10 }}>{data.semanticMatch.existing_skill_name}</Tag>
          </span>
        ),
      },
      {
        key: "upgrade_existing",
        label: (
          <span style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <span>⬆️</span>
            <span>{t("workflow.upgradeExisting")}</span>
          </span>
        ),
      },
      { type: "divider" },
      {
        key: "keep",
        label: (
          <span style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <span>✨</span>
            <span>{t("workflow.keepGeneratedSkill")}</span>
          </span>
        ),
      },
    ]
    : [];

  return (
    <div
      style={{
        minWidth: 180,
        maxWidth: 220,
        opacity: data.enabled ? 1 : 0.5,
        filter: data.enabled ? "none" : "grayscale(100%)",
      }}
    >
      <div
        style={{
          background: "#1e1e1e",
          border: `2px solid ${selected ? "#1890ff" : color}`,
          borderRadius: 8,
          overflow: "hidden",
          boxShadow: selected ? `0 0 0 2px ${color}40` : "none",
          transition: "all 0.2s",
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
          <span style={{ fontSize: 14 }}>⚛️</span>
          <span style={{ fontSize: 11, color, fontWeight: 600 }}>原子Skill</span>
          <Tag
            style={{
              margin: 0,
              fontSize: 9,
              padding: "0 4px",
              background: `${entryColor}20`,
              border: `1px solid ${entryColor}50`,
              color: entryColor,
              lineHeight: "16px",
            }}
          >
            {ENTRY_TYPE_LABELS[entryType] || entryType}
          </Tag>
          {data.semanticMatch && (
            <Dropdown
              menu={{ items: matchMenuItems, onClick: handleMenuClick }}
              trigger={["click"]}
              placement="bottomRight"
            >
              <Tag
                style={{
                  margin: 0,
                  fontSize: 9,
                  padding: "0 4px",
                  background: `${getMatchBadgeColor(data.semanticMatch.similarity_score)}20`,
                  border: `1px solid ${getMatchBadgeColor(data.semanticMatch.similarity_score)}50`,
                  color: getMatchBadgeColor(data.semanticMatch.similarity_score),
                  cursor: "pointer",
                  lineHeight: "16px",
                  fontWeight: 600,
                }}
              >
                {Math.round(data.semanticMatch.similarity_score * 100)}% 匹配
              </Tag>
            </Dropdown>
          )}
        </div>

        <div style={{ padding: "10px 12px" }}>
          <div
            style={{
              fontSize: 13,
              color: "#fff",
              fontWeight: 500,
              marginBottom: 6,
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
            }}
          >
            {data.title}
          </div>

          <div
            style={{
              fontSize: 11,
              color: entryColor,
              marginBottom: 6,
              padding: "4px 6px",
              background: `${entryColor}15`,
              borderRadius: 4,
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
              fontWeight: 500,
            }}
          >
            {skillName}
          </div>

          <div style={{ display: "flex", flexWrap: "wrap", gap: 4 }}>
            {inputCount > 0 && (
              <Tag
                style={{
                  margin: 0,
                  fontSize: 9,
                  padding: "0 4px",
                  background: "#252525",
                  border: "1px solid #444",
                  color: "#aaa",
                }}
              >
                📥 {inputCount} 输入
              </Tag>
            )}

            {outputVar && (
              <Tag
                style={{
                  margin: 0,
                  fontSize: 9,
                  padding: "0 4px",
                  background: "#1890ff20",
                  border: "1px solid #1890ff50",
                  color: "#1890ff",
                }}
              >
                📤 {outputVar}
              </Tag>
            )}
          </div>
        </div>
      </div>

      <Handle
        type="target"
        position={Position.Top}
        style={{ background: color, border: "none", width: 8, height: 8 }}
      />

      <Handle
        type="source"
        position={Position.Bottom}
        style={{ background: color, border: "none", width: 8, height: 8 }}
      />
    </div>
  );
};

export const AtomicSkillNode = memo(AtomicSkillNodeComponent);
