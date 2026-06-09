import { Divider, Input, Select, Switch, theme } from "antd";
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
  const c = n.config || {
    strategy: "all",
    input_sources: [],
    wait_for_all: true,
    weights: [],
    summarize_prompt: "",
    summarize_model: "",
    output_var: "",
  };

  const sc = (k: string, v: unknown) => onUpdate({ config: { ...c, [k]: v } });

  const isWeighted = c.strategy === "weighted";
  const isLlmSummarize = c.strategy === "llm_summarize";

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      {/* 聚合策略 */}
      <div>
        <label
          style={{ color: token.colorTextTertiary, fontSize: 12, marginBottom: 4, display: "block" }}
        >
          {t("workflow.props.aggregStrategy", { defaultValue: "Aggregation Strategy" })}
        </label>
        <Select
          value={c.strategy}
          onChange={(v) => sc("strategy", v)}
          size="small"
          style={{ width: "100%" }}
          options={[
            { value: "all", label: t("workflow.props.aggregAll", { defaultValue: "All (array)" }) },
            {
              value: "concat",
              label: t("workflow.props.aggregConcat", { defaultValue: "Concat (string)" }),
            },
            { value: "sum", label: t("workflow.props.aggregSum", { defaultValue: "Sum (numeric)" }) },
            {
              value: "merge",
              label: t("workflow.props.aggregMerge", { defaultValue: "Merge (object)" }),
            },
            {
              value: "weighted",
              label: t("workflow.props.aggregWeighted", { defaultValue: "Weighted (numeric)" }),
            },
            {
              value: "llm_summarize",
              label: t("workflow.props.aggregLlmSummarize", {
                defaultValue: "LLM Summarize (text)",
              }),
            },
            { value: "count", label: t("workflow.props.aggregCount", { defaultValue: "Count" }) },
          ]}
        />
        <div style={{ fontSize: 11, color: token.colorTextTertiary, marginTop: 2 }}>
          {c.strategy === "concat"
            && t("workflow.props.aggregConcatHint", { defaultValue: "Joins string values together" })}
          {c.strategy === "merge"
            && t("workflow.props.aggregMergeHint", {
              defaultValue: "Shallow-merges JSON objects (latter overwrites)",
            })}
          {c.strategy === "weighted"
            && t("workflow.props.aggregWeightedHint", {
              defaultValue: "Weighted sum: enter comma-separated weights below",
            })}
          {c.strategy === "llm_summarize"
            && t("workflow.props.aggregLlmHint", {
              defaultValue: "Summarizes text inputs. LLM call pending - currently concats.",
            })}
        </div>
      </div>

      {/* 等待策略 */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
        }}
      >
        <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>
          {t("workflow.props.waitForAll", { defaultValue: "Wait for all inputs" })}
        </label>
        <Switch
          size="small"
          checked={c.wait_for_all !== false}
          onChange={(checked) => sc("wait_for_all", checked)}
        />
      </div>
      <div style={{ fontSize: 11, color: token.colorTextTertiary }}>
        {c.wait_for_all !== false
          ? t("workflow.props.waitForAllHint", {
            defaultValue: "Aggregate only when all inputs are ready",
          })
          : t("workflow.props.waitForAnyHint", {
            defaultValue: "Aggregate from first available inputs (race mode)",
          })}
      </div>

      {/* 加权策略：权重配置 */}
      {isWeighted && (
        <div>
          <label
            style={{
              color: token.colorTextTertiary,
              fontSize: 12,
              marginBottom: 4,
              display: "block",
            }}
          >
            {t("workflow.props.weights", { defaultValue: "Weights (comma-separated)" })}
          </label>
          <Input
            value={(c.weights ?? []).join(", ")}
            onChange={(e) => {
              const vals = e.target.value
                .split(",")
                .map((s) => parseFloat(s.trim()))
                .filter((n) => !isNaN(n));
              sc("weights", vals.length > 0 ? vals : undefined);
            }}
            size="small"
            placeholder={t("workflow.props.weightsPlaceholder", {
              defaultValue: "e.g. 0.5, 1.0, 1.5",
            })}
          />
        </div>
      )}

      {/* LLM 摘要策略：提示词 + 模型 */}
      {isLlmSummarize && (
        <>
          <div>
            <label
              style={{
                color: token.colorTextTertiary,
                fontSize: 12,
                marginBottom: 4,
                display: "block",
              }}
            >
              {t("workflow.props.summarizePrompt", { defaultValue: "Summarize prompt" })}
            </label>
            <Input.TextArea
              value={c.summarize_prompt || ""}
              onChange={(e) => sc("summarize_prompt", e.target.value || undefined)}
              size="small"
              rows={2}
              placeholder={t("workflow.props.summarizePromptPlaceholder", {
                defaultValue: "Describe how to summarize...",
              })}
            />
          </div>
          <div>
            <label
              style={{
                color: token.colorTextTertiary,
                fontSize: 12,
                marginBottom: 4,
                display: "block",
              }}
            >
              {t("workflow.props.summarizeModel", { defaultValue: "Model (optional)" })}
            </label>
            <Input
              value={c.summarize_model || ""}
              onChange={(e) => sc("summarize_model", e.target.value || undefined)}
              size="small"
              placeholder={t("workflow.props.defaultModel", { defaultValue: "Use default model" })}
            />
          </div>
        </>
      )}

      <Divider style={{ margin: "8px 0", borderColor: token.colorBorderSecondary }} />
      <BasePropertyPanel node={node} onUpdate={onUpdate} onDelete={onDelete} />
    </div>
  );
};
