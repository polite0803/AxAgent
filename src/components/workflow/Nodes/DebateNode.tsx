import { Tag } from "antd";
import React, { memo } from "react";
import { useTranslation } from "react-i18next";
import type { NodeProps } from "reactflow";
import type { ContainerNodeData } from "./ContainerNode";
import { ContainerNode } from "./ContainerNode";

const BLUE_BASE = "#1890ff";

interface DebateNodeData extends ContainerNodeData {
  debaterSteps?: string[];
  maxRounds?: number;
  convergencePrompt?: string;
}

const DebateNodeComponent: React.FC<NodeProps<DebateNodeData>> = ({
  data,
  selected,
}) => {
  const { t } = useTranslation();
  const debaterCount = data.debaterSteps?.length || data.childCount || 0;
  const maxRounds = data.maxRounds || 2;

  return (
    <ContainerNode
      data={data}
      selected={selected}
      icon="⚖️"
      childLabel={t("workflow.debateNode.debaters", { defaultValue: "Debaters" })}
      extraTags={
        <>
          <Tag
            style={{
              margin: 0,
              fontSize: 9,
              padding: "0 4px",
              background: `${BLUE_BASE}20`,
              border: `1px solid ${BLUE_BASE}50`,
              color: BLUE_BASE,
            }}
          >
            {debaterCount} {t("workflow.debateNode.debaters", { defaultValue: "debaters" })}
          </Tag>
          <Tag
            style={{
              margin: 0,
              fontSize: 9,
              padding: "0 4px",
              background: "transparent",
              border: `1px solid ${BLUE_BASE}50`,
              color: BLUE_BASE,
            }}
          >
            {maxRounds} {t("workflow.debateNode.rounds", { defaultValue: "rounds" })}
          </Tag>
        </>
      }
    />
  );
};

export const DebateNode = memo(DebateNodeComponent);
