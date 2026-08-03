// SPDX-License-Identifier: AGPL-3.0-only
// Phase 4: NLParserPanel — 自然语言解析面板

import { useWorkflowStore } from "@/stores/feature/workflowStore";
import type { NLParseResult } from "@/types";
import { Button, Input, Progress, Space, Tag, Typography } from "antd";
import { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";

const {} = Input;
const { Text, Title } = Typography;

interface NLParserPanelProps {
  onApply: (result: NLParseResult) => void;
}

const placeholderExamples = [
  "每天早上 8 点抓取指定网站的最新文章，用 AI 总结后发送到企业微信群",
  "收到新邮件后，用 AI 分析内容，如果是重要邮件就发通知并创建待办",
  "用户提交表单后，先验证数据，然后写入数据库并发送确认邮件",
];

export function NLParserPanel({ onApply }: NLParserPanelProps) {
  const { t } = useTranslation();
  const [prompt, setPrompt] = useState("");
  const [constraints, setConstraints] = useState("");
  const [result, setResult] = useState<NLParseResult | null>(null);

  const isParsing = useWorkflowStore((s) => s.isParsing);
  const parseProgress = useWorkflowStore((s) => s.parseProgress);
  const parseNaturalLanguage = useWorkflowStore((s) => s.parseNaturalLanguage);

  // 在组件挂载时选择一个随机的 placeholder 示例
  const [placeholderExample] = useState(
    () => placeholderExamples[Math.floor(Math.random() * placeholderExamples.length)],
  );

  const handleParse = useCallback(async () => {
    if (!prompt.trim()) { return; }
    const res = await parseNaturalLanguage({
      prompt: prompt.trim(),
      constraints: constraints ? [constraints.trim()] : undefined,
    });
    setResult(res);
  }, [prompt, constraints, parseNaturalLanguage]);

  const handleApply = useCallback(() => {
    if (result) {
      onApply(result);
      setResult(null);
      setPrompt("");
      setConstraints("");
    }
  }, [result, onApply]);

  const hasContent = prompt.trim().length > 0;

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
      <div>
        <Text strong style={{ display: "block", marginBottom: 6 }}>{t("workflow.nlParser.naturalLanguageDesc")}</Text>
        <Input.TextArea
          rows={5}
          value={prompt}
          onChange={(e) => setPrompt(e.target.value)}
          placeholder={t("workflow.nlParser.examplePlaceholder", { example: placeholderExample })}
          disabled={isParsing}
          style={{ fontSize: 13 }}
        />
      </div>

      <div>
        <Text strong style={{ display: "block", marginBottom: 6 }}>{t("workflow.nlParser.constraintsOptional")}</Text>
        <Input.TextArea
          rows={2}
          value={constraints}
          onChange={(e) => setConstraints(e.target.value)}
          placeholder={t("workflow.nlParser.constraintsPlaceholder")}
          disabled={isParsing}
          style={{ fontSize: 13 }}
        />
      </div>

      <Button
        type="primary"
        onClick={handleParse}
        loading={isParsing}
        disabled={!hasContent || isParsing}
        block
      >
        {isParsing ? t("workflow.nlParser.parsing") : t("workflow.nlParser.parseGenerate")}
      </Button>

      {isParsing && (
        <div style={{ padding: "12px 0" }}>
          <Progress percent={50} status="active" showInfo={false} strokeColor="#1677ff" />
          <Text type="secondary" style={{ display: "block", marginTop: 6, fontSize: 12, textAlign: "center" }}>
            {parseProgress}
          </Text>
        </div>
      )}

      {result && !isParsing && (
        <div
          style={{
            border: "1px solid var(--color-border-secondary)",
            borderRadius: 8,
            padding: 16,
            backgroundColor: "var(--color-fill-tertiary)",
          }}
        >
          <Title level={5} style={{ marginTop: 0 }}>{t("workflow.nlParser.parseResult")}</Title>

          <div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 12 }}>
            <Progress
              type="circle"
              percent={Math.round(result.confidence * 100)}
              size={64}
              strokeColor="#1677ff"
            />
            <div>
              <Text>
                {t("workflow.nlParser.confidence")}: <Text strong>{Math.round(result.confidence * 100)}%</Text>
              </Text>
              <br />
              <Text type="secondary" style={{ fontSize: 12 }}>
                {t("workflow.nlParser.resultSummary", {
                  nodes: result.workflow.nodes.length,
                  edges: result.workflow.edges.length,
                  variables: Object.keys(result.workflow.variables).length,
                })}
              </Text>
            </div>
          </div>

          {result.suggestions.length > 0 && (
            <div style={{ marginBottom: 12 }}>
              <Text strong style={{ display: "block", marginBottom: 4 }}>{t("workflow.nlParser.aiSuggestion")}</Text>
              <Space orientation="vertical" size={4}>
                {result.suggestions.map((s, i) => (
                  // FIXME: suggestions 是字符串数组，无稳定唯一标识
                  <Tag key={`suggestion-${i}`} color="processing">{s}</Tag>
                ))}
              </Space>
            </div>
          )}

          <Button type="primary" block onClick={handleApply}>
            {t("workflow.nlParser.applySolution")}
          </Button>
        </div>
      )}
    </div>
  );
}
