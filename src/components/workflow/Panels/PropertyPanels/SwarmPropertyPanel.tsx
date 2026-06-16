// SPDX-License-Identifier-Identifier: AGPL-3.0-only

import { useWorkflowEditorStore } from "@/stores";
import { Button, Divider, Input, InputNumber, Tag, theme } from "antd";
import { Plus, Trash2 } from "lucide-react";
import React from "react";
import { useTranslation } from "react-i18next";
import type { SwarmNode, WorkflowNode } from "../../types";
import { BasePropertyPanel } from "./BasePropertyPanel";

interface Props {
  node: WorkflowNode;
  onUpdate: (u: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}

export const SwarmPropertyPanel: React.FC<Props> = ({ node, onUpdate, onDelete }) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const n = node as unknown as SwarmNode;
  const c = n.config || {
    agent_steps: [],
    max_rounds: 3,
    topic_var: "",
    output_var: "",
  };
  const sc = (k: string, v: unknown) => onUpdate({ config: { ...c, [k]: v } });

  const allNodes = useWorkflowEditorStore((s) => s.nodes);
  const addNode = useWorkflowEditorStore((s) => s.addNode);
  const setParentRef = useWorkflowEditorStore((s) => s.setParentRef);
  const agentSteps: string[] = c.agent_steps || [];

  const childNodes = agentSteps
    .map((id) => allNodes.find((n) => n.id === id))
    .filter(Boolean) as WorkflowNode[];

  const addAgent = () => {
    const id = `node-${crypto.randomUUID()}`;
    const position = { x: 50 + agentSteps.length * 30, y: 80 + agentSteps.length * 80 };
    const newNode: WorkflowNode = {
      id,
      type: "agent",
      title: t("workflow.swarmNode.newAgent", { defaultValue: "Agent" }) + ` ${agentSteps.length + 1}`,
      position,
      config: {
        system_prompt: "",
        context_sources: [],
        output_var: `${id}_output`,
        tools: [],
        exposed_tools: [],
        output_mode: "text",
      },
    } as unknown as WorkflowNode;
    addNode(newNode);
    setParentRef(id, n.id);
    sc("agent_steps", [...agentSteps, id]);
  };

  const removeAgent = (stepId: string) => {
    const updated = agentSteps.filter((id) => id !== stepId);
    sc("agent_steps", updated);
    setParentRef(stepId, null);
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      <div>
        <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>
          {t("workflow.nodeConfig.topic_var", { defaultValue: "Topic Variable" })}
        </label>
        <Input
          value={c.topic_var}
          onChange={(e) => sc("topic_var", e.target.value)}
          size="small"
        />
      </div>
      <div>
        <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>
          {t("workflow.nodeConfig.max_rounds", { defaultValue: "Max Rounds" })}
        </label>
        <InputNumber
          value={c.max_rounds}
          onChange={(v) => sc("max_rounds", v ?? 3)}
          size="small"
          min={1}
          max={50}
          style={{ width: "100%" }}
        />
      </div>
      <Divider style={{ margin: "8px 0" }}>
        {t("workflow.swarmNode.agents", { defaultValue: "Agents (child nodes)" })}
      </Divider>
      {childNodes.map((child) => (
        <div
          key={child.id}
          style={{
            border: `1px solid ${token.colorBorderSecondary}`,
            borderRadius: 6,
            padding: 8,
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            gap: 8,
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: 6, flex: 1, minWidth: 0 }}>
            <Tag color="cyan" style={{ margin: 0, fontSize: 11, flexShrink: 0 }}>
              Agent
            </Tag>
            <span style={{ fontSize: 12, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
              {child.title}
            </span>
          </div>
          <Button
            type="text"
            size="small"
            icon={<Trash2 size={14} />}
            onClick={() => removeAgent(child.id)}
            danger
          />
        </div>
      ))}
      {agentSteps.length === 0 && (
        <div style={{ fontSize: 11, color: token.colorTextTertiary, textAlign: "center" }}>
          {t("workflow.swarmNode.noAgentsHint", { defaultValue: "Add Agent nodes inside this container" })}
        </div>
      )}
      <Button
        type="dashed"
        size="small"
        icon={<Plus size={14} />}
        onClick={addAgent}
        style={{ width: "100%" }}
      >
        {t("workflow.swarmNode.addAgent", { defaultValue: "Add Agent" })}
      </Button>
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
      <Divider style={{ margin: "8px 0" }} />
      <BasePropertyPanel node={node} onUpdate={onUpdate} onDelete={onDelete} />
    </div>
  );
};
