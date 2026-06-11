// SPDX-License-Identifier: AGPL-3.0-only

import type { NodeProps } from "@xyflow/react";
import { Tag } from "antd";
import React, { memo } from "react";
import { useTranslation } from "react-i18next";
import type { ContainerNodeData } from "./ContainerNode";
import { ContainerNode } from "./ContainerNode";

const PINK_BASE = "#eb2f96";

interface SwarmNodeData extends ContainerNodeData {
  agentSteps?: string[];
  maxRounds?: number;
}

const SwarmNodeComponent: React.FC<NodeProps> = ({ data: _data, selected }) => {
  const data = _data as unknown as SwarmNodeData;
  const { t } = useTranslation();
  const agentCount = data.agentSteps?.length || data.childCount || 0;
  const maxRounds = data.maxRounds || 3;

  return (
    <ContainerNode
      data={data}
      selected={selected}
      icon="🧠"
      childLabel={t("workflow.swarmNode.agents", { defaultValue: "Agents" })}
      extraTags={
        <>
          <Tag
            style={{
              margin: 0,
              fontSize: 9,
              padding: "0 4px",
              background: `${PINK_BASE}20`,
              border: `1px solid ${PINK_BASE}50`,
              color: PINK_BASE,
            }}
          >
            {agentCount} {t("workflow.swarmNode.agents", { defaultValue: "agents" })}
          </Tag>
          <Tag
            style={{
              margin: 0,
              fontSize: 9,
              padding: "0 4px",
              background: "transparent",
              border: `1px solid ${PINK_BASE}50`,
              color: PINK_BASE,
            }}
          >
            {maxRounds} {t("workflow.swarmNode.rounds", { defaultValue: "rounds" })}
          </Tag>
        </>
      }
    />
  );
};

export const SwarmNode = memo(SwarmNodeComponent);
