import { Tag } from "antd";
import React, { memo } from "react";
import { useTranslation } from "react-i18next";
import type { NodeProps } from "reactflow";
import type { ContainerNodeData } from "./ContainerNode";
import { ContainerNode } from "./ContainerNode";

const ORANGE_BASE = "#fa8c16";

interface LoopNodeData extends ContainerNodeData {
  loopType?: "count" | "condition" | "forEach";
  maxIterations?: number;
  loopCondition?: string;
  collectionVar?: string;
}

const LoopNodeComponent: React.FC<NodeProps<LoopNodeData>> = ({
  data,
  selected,
}) => {
  const { t } = useTranslation();
  const loopType = data.loopType || "count";

  const getLoopDescription = (): string => {
    switch (loopType) {
      case "count":
        return data.maxIterations
          ? `${data.maxIterations}x`
          : t("workflow.loopNode.notConfigured");
      case "condition":
        return data.loopCondition || t("workflow.loopNode.notConfigured");
      case "forEach":
        return data.collectionVar
          ? `∈ ${data.collectionVar}`
          : t("workflow.loopNode.notConfigured");
      default:
        return t("workflow.loopNode.notConfigured");
    }
  };

  return (
    <ContainerNode
      data={data}
      selected={selected}
      icon="🔁"
      childLabel={t("workflow.loopNode.steps", { defaultValue: "Steps" })}
      extraTags={
        <>
          <Tag
            style={{
              margin: 0,
              fontSize: 9,
              padding: "0 4px",
              background: `${ORANGE_BASE}20`,
              border: `1px solid ${ORANGE_BASE}50`,
              color: ORANGE_BASE,
            }}
          >
            {loopType.toUpperCase()}
          </Tag>
          <Tag
            style={{
              margin: 0,
              fontSize: 9,
              padding: "0 4px",
              background: "transparent",
              border: `1px solid ${ORANGE_BASE}50`,
              color: ORANGE_BASE,
            }}
          >
            {getLoopDescription()}
          </Tag>
        </>
      }
    />
  );
};

export const LoopNode = memo(LoopNodeComponent);
