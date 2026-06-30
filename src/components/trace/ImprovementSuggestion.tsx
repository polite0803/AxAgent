// SPDX-License-Identifier: AGPL-3.0-only

import { Button, Card, Typography } from "antd";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";

const { Text, Paragraph } = Typography;

// ── Types ──

export interface ImprovementSuggestion {
  id: string;
  problem: string;
  suggestion: string;
  expectedImprovement: string;
}

/** @deprecated 使用 ImprovementSuggestion */
export type ImprovementSuggestionItem = ImprovementSuggestion;

// ── Mock ──

function buildMockSuggestions(): ImprovementSuggestion[] {
  return [
    {
      id: "sug_001",
      problem: "工具调用 `search_file` 和 `read_file` 本可并行执行，但实际串行执行，浪费了 2.1s 等待时间。",
      suggestion: "将无依赖的工具调用标记为可并行，Agent 应自动识别独立操作并合并到同一批执行。",
      expectedImprovement: "预计减少 25% 总执行时间（约 2.1s）",
    },
    {
      id: "sug_002",
      problem: "系统提示词包含大量冗余工具定义，当前会话仅使用了 2/15 个工具，Token 浪费约 800。",
      suggestion: "根据会话上下文动态裁剪工具列表，仅加载当前任务可能用到的工具定义。",
      expectedImprovement: "每次会话节省约 800 Token，累计可降低 20% Token 成本",
    },
    {
      id: "sug_003",
      problem: "错误处理策略过于保守：遇到权限错误后直接终止，未尝试备用路径。",
      suggestion: "在技能配置中添加 fallback 路径列表，当主路径失败时自动切换到备用路径。",
      expectedImprovement: "预计将错误率从 8% 降至 3%",
    },
    {
      id: "sug_004",
      problem: "对话历史中包含了大量已解决的工具调用中间结果，占用上下文窗口。",
      suggestion: "启用对话自动压缩，在 Token 超阈值时自动摘要旧消息，保留关键信息。",
      expectedImprovement: "可多支持 40% 的对话轮次，降低 Token 溢出风险",
    },
  ];
}

// ── Component ──

interface ImprovementSuggestionProps {
  traceId: string;
}

export function ImprovementSuggestion({ traceId: _traceId }: ImprovementSuggestionProps) {
  const { t } = useTranslation();
  const suggestions = useMemo(() => buildMockSuggestions(), []);

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      {suggestions.length === 0
        ? <Text type="secondary">{t("trace.improvement.noSuggestions", "暂无改进建议")}</Text>
        : (
          suggestions.map((item) => (
            <Card
              key={item.id}
              size="small"
              style={{ borderLeft: "3px solid #1890ff" }}
            >
              <div style={{ marginBottom: 8 }}>
                <Text type="danger" strong style={{ fontSize: 12 }}>
                  {t("trace.improvement.problem", "问题")}:
                </Text>
                <Paragraph style={{ margin: "4px 0", fontSize: 13 }}>{item.problem}</Paragraph>
              </div>
              <div style={{ marginBottom: 8 }}>
                <Text type="warning" strong style={{ fontSize: 12 }}>
                  {t("trace.improvement.suggestion", "建议")}:
                </Text>
                <Paragraph style={{ margin: "4px 0", fontSize: 13 }}>{item.suggestion}</Paragraph>
              </div>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <Text type="success" style={{ fontSize: 12 }}>
                  {item.expectedImprovement}
                </Text>
                <Button type="primary" size="small">
                  {t("trace.improvement.apply", "应用改进")}
                </Button>
              </div>
            </Card>
          ))
        )}
    </div>
  );
}
