// SPDX-License-Identifier: AGPL-3.0-only

import type { NLParseResult, WorkflowDefinition } from "@/types/workflow";
import { Button, Card, List, Progress, Radio, Statistic, Tag, Typography, theme } from "antd";
import { Lightbulb, Workflow } from "lucide-react";
import React, { useMemo, useState } from "react";

const { Text } = Typography;

interface NLParseResultViewProps {
  result: NLParseResult;
  onApply: (workflow: WorkflowDefinition) => void;
  loading?: boolean;
}

export const NLParseResultView: React.FC<NLParseResultViewProps> = React.memo(({ result, onApply, loading }) => {
  const { token } = theme.useToken();
  const percent = Math.round(result.confidence * 100);

  const confidenceColor = percent >= 80 ? token.colorSuccess : percent >= 60 ? token.colorPrimary : token.colorWarning;

  const alternatives = result.alternatives ?? [];
  const [selectedAltIndex, setSelectedAltIndex] = useState(0);
  const hasAlternatives = alternatives.length > 0;

  const displayWorkflow = useMemo(() => {
    if (hasAlternatives && selectedAltIndex < alternatives.length) {
      return alternatives[selectedAltIndex];
    }
    return result.workflow;
  }, [hasAlternatives, selectedAltIndex, alternatives, result.workflow]);

  const nodeCount = displayWorkflow?.nodes?.length ?? 0;
  const edgeCount = displayWorkflow?.edges?.length ?? 0;
  const variableCount = Object.keys(displayWorkflow?.variables ?? {}).length;

  const nodeTypeCounts = useMemo(() => {
    if (!displayWorkflow?.nodes) return {};
    const counts: Record<string, number> = {};
    for (const n of displayWorkflow.nodes) {
      const t = n.type || "unknown";
      counts[t] = (counts[t] || 0) + 1;
    }
    return counts;
  }, [displayWorkflow]);

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      {/* 置信度圆环 + 摘要 */}
      <Card
        size="small"
        style={{ background: token.colorBgContainer, border: `1px solid ${token.colorBorderSecondary}` }}
        styles={{ body: { padding: "12px" } }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: 16 }}>
          <div style={{ flexShrink: 0 }}>
            <Progress
              type="circle"
              percent={percent}
              size={72}
              strokeColor={confidenceColor}
              format={() => (
                <span style={{ fontSize: 18, fontWeight: 600, color: confidenceColor }}>
                  {percent}%
                </span>
              )}
            />
          </div>
          <div style={{ flex: 1, minWidth: 0 }}>
            <Text strong style={{ fontSize: 13 }}>解析置信度</Text>
            <div style={{ display: "flex", gap: 16, marginTop: 8 }}>
              <Statistic
                title="节点"
                value={nodeCount}
                valueStyle={{ fontSize: 18, fontWeight: 600, color: token.colorText }}
                prefix={<Workflow size={12} />}
              />
              <Statistic
                title="边"
                value={edgeCount}
                valueStyle={{ fontSize: 18, fontWeight: 600, color: token.colorText }}
              />
              <Statistic
                title="变量"
                value={variableCount}
                valueStyle={{ fontSize: 18, fontWeight: 600, color: token.colorText }}
              />
            </div>
            {/* 节点类型分布 */}
            <div style={{ marginTop: 8, display: "flex", flexWrap: "wrap", gap: 4 }}>
              {Object.entries(nodeTypeCounts).map(([t, c]) => (
                <Tag key={t} color="blue" style={{ fontSize: 10, margin: 0, padding: "0 4px", lineHeight: "16px" }}>
                  {t} x{c}
                </Tag>
              ))}
            </div>
          </div>
        </div>
      </Card>

      {/* 备选方案 */}
      {hasAlternatives && (
        <Card
          size="small"
          style={{ background: token.colorBgContainer, border: `1px solid ${token.colorBorderSecondary}` }}
          styles={{ body: { padding: "8px 12px" } }}
        >
          <Text strong style={{ fontSize: 12, display: "block", marginBottom: 6 }}>
            备选方案
          </Text>
          <Radio.Group
            value={selectedAltIndex}
            onChange={(e) => setSelectedAltIndex(e.target.value)}
            size="small"
            style={{ display: "flex", flexDirection: "column", gap: 4 }}
          >
            {alternatives.map((alt, idx) => (
              <Radio key={idx} value={idx} style={{ fontSize: 12 }}>
                方案 {idx + 1}：{alt.name ?? `${alt.nodes?.length ?? 0} 节点 / ${alt.edges?.length ?? 0} 边`}
              </Radio>
            ))}
          </Radio.Group>
        </Card>
      )}

      {/* AI 建议列表 */}
      {result.suggestions && result.suggestions.length > 0 && (
        <Card
          size="small"
          style={{ background: token.colorBgContainer, border: `1px solid ${token.colorBorderSecondary}` }}
          styles={{ body: { padding: "8px 12px" } }}
        >
          <Text strong style={{ fontSize: 12, display: "block", marginBottom: 6 }}>
            AI 建议
          </Text>
          <List
            size="small"
            dataSource={result.suggestions}
            split={false}
            renderItem={(item, idx) => (
              <List.Item key={idx} style={{ padding: "2px 0", border: "none" }}>
                <div style={{ display: "flex", alignItems: "flex-start", gap: 6 }}>
                  <Lightbulb size={12} style={{ marginTop: 2, color: token.colorWarning, flexShrink: 0 }} />
                  <Text style={{ fontSize: 12, color: token.colorTextSecondary }}>{item}</Text>
                </div>
              </List.Item>
            )}
          />
        </Card>
      )}

      {/* 应用按钮 */}
      <Button
        type="primary"
        block
        size="middle"
        icon={<Workflow size={14} />}
        loading={loading}
        onClick={() => onApply(displayWorkflow)}
        style={{ fontWeight: 500 }}
      >
        应用此方案
      </Button>
    </div>
  );
});
