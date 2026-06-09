import { Tag } from "antd";
import React, { memo } from "react";
import { useTranslation } from "react-i18next";
import type { NodeProps } from "reactflow";
import type { MergeStrategy } from "../types/workflow.types";
import type { ContainerNodeData } from "./ContainerNode";
import { ContainerNode } from "./ContainerNode";

const ORANGE_BASE = "#fa8c16";

interface ParallelNodeData extends ContainerNodeData {
  branches?: number;
  waitStrategy?: "all" | "any" | "race";
  aggregation?: MergeStrategy;
  autoInputFromParent?: boolean;
}

const ParallelNodeComponent: React.FC<NodeProps<ParallelNodeData>> = ({
  data,
  selected,
}) => {
  const { t } = useTranslation();
  const isDecorative = data.kind === "decorative";
  const branches = data.branches || 2;

  const getWaitStrategyLabel = (strategy?: string): string => {
    switch (strategy) {
      case "all":
        return t("workflow.parallelNode.waitAll");
      case "any":
        return t("workflow.parallelNode.waitAny");
      case "race":
        return t("workflow.parallelNode.race");
      default:
        return strategy || "";
    }
  };

  const extraTags = isDecorative
    ? (
      <Tag
        style={{
          margin: 0,
          fontSize: 9,
          padding: "0 4px",
          background: "transparent",
          border: `1px dashed ${ORANGE_BASE}50`,
          color: ORANGE_BASE,
          opacity: 0.7,
        }}
      >
        {t("workflow.parallelNode.decorative")}
      </Tag>
    )
    : (
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
          {branches} {t("workflow.parallelNode.branches")}
        </Tag>
        {data.waitStrategy && (
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
            {getWaitStrategyLabel(data.waitStrategy)}
          </Tag>
        )}
      </>
    );

  return (
    <ContainerNode
      data={data}
      selected={selected}
      icon={isDecorative ? "📦" : "⚡"}
      childLabel={t("workflow.parallelNode.branches", { defaultValue: "Branches" })}
      extraTags={extraTags}
      disableHandles={isDecorative}
    />
  );
};

export const ParallelNode = memo(ParallelNodeComponent);
