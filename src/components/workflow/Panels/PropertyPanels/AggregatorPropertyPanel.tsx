import { Divider, Select, theme } from "antd";
import React from "react";
import { useTranslation } from "react-i18next";
import type { AggregatorNode, WorkflowNode } from "../../types";
import { BasePropertyPanel } from "./BasePropertyPanel";
interface Props {
  node: WorkflowNode;
  onUpdate: (u: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}
export const AggregatorPropertyPanel: React.FC<Props> = ({ node, onUpdate, onDelete }) => {
  const { token } = theme.useToken();
  const { t } = useTranslation();
  const n = node as unknown as AggregatorNode;
  const c = n.config || { strategy: "all", input_sources: [], output_var: "" };
  const sc = (k: string, v: unknown) => onUpdate({ config: { ...c, [k]: v } });
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      <div>
        <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>{t("workflow.props.aggregation")}</label>
        <Select
          value={c.strategy}
          onChange={(v) => sc("strategy", v)}
          size="small"
          style={{ width: "100%" }}
          options={[{ value: "all", label: t("workflow.props.aggregationAll") }, { value: "any", label: t("workflow.props.aggregationAny") }, { value: "race", label: t("workflow.props.aggregationRace") }, {
            value: "majority",
            label: t("workflow.props.aggregationMajority"),
          }]}
        />
      </div>
      <Divider style={{ margin: "8px 0" }} />
      <BasePropertyPanel node={node} onUpdate={onUpdate} onDelete={onDelete} />
    </div>
  );
};
