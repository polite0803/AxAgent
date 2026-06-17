// SPDX-License-Identifier: AGPL-3.0-only

import type { NodeProps } from "@xyflow/react";
import { Tag } from "antd";
import React, { memo } from "react";
import { useTranslation } from "react-i18next";
import type { ContainerNodeData } from "./ContainerNode";
import { ContainerNode } from "./ContainerNode";

const ORANGE_BASE = "#fa8c16";

interface AggregatorNodeData extends ContainerNodeData {
  aggregationType?: string;
  inputCount?: number;
}

const AggregatorNodeComponent: React.FC<NodeProps> = ({ data: _data, selected }) => {
  const data = _data as unknown as AggregatorNodeData;
  const { t } = useTranslation();

  return (
    <ContainerNode
      data={data}
      selected={selected}
      icon="📊"
      childLabel={t("workflow.aggregatorNode.inputs", { defaultValue: "Inputs" })}
      extraTags={
        <>
          {data.aggregationType && (
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
              {data.aggregationType.toUpperCase()}
            </Tag>
          )}
        </>
      }
    />
  );
};

export const AggregatorNode = memo(AggregatorNodeComponent);
