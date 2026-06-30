// SPDX-License-Identifier: AGPL-3.0-only
// Phase 4: NLParserPanel — 自然语言解析面板

import { useWorkflowStore } from "@/stores/feature/workflowStore";
import type { NLParseResult } from "@/types/workflow";
import { Button, Input, Progress, Space, Tag, Typography } from "antd";
import { useCallback, useState } from "react";

const { TextArea } = Input;
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
  const [prompt, setPrompt] = useState("");
  const [constraints, setConstraints] = useState("");
  const [result, setResult] = useState<NLParseResult | null>(null);

  const isParsing = useWorkflowStore((s) => s.isParsing);
  const parseProgress = useWorkflowStore((s) => s.parseProgress);
  const parseNaturalLanguage = useWorkflowStore((s) => s.parseNaturalLanguage);

  const handleParse = useCallback(async () => {
    if (!prompt.trim()) return;
    const res = await parseNaturalLanguage({ prompt: prompt.trim(), constraints: constraints ? [constraints.trim()] : undefined });
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
        <Text strong style={{ display: "block", marginBottom: 6 }}>自然语言描述</Text>
        <TextArea
          rows={5}
          value={prompt}
          onChange={(e) => setPrompt(e.target.value)}
          placeholder={`例如：${placeholderExamples[Math.floor(Math.random() * placeholderExamples.length)]}`}
          disabled={isParsing}
          style={{ fontSize: 13 }}
        />
      </div>

      <div>
        <Text strong style={{ display: "block", marginBottom: 6 }}>约束条件（可选）</Text>
        <TextArea
          rows={2}
          value={constraints}
          onChange={(e) => setConstraints(e.target.value)}
          placeholder="例如：执行时间不超过 5 分钟、出错时重试 3 次"
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
        {isParsing ? "解析中..." : "解析生成"}
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
          <Title level={5} style={{ marginTop: 0 }}>解析结果</Title>

          <div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 12 }}>
            <Progress
              type="circle"
              percent={Math.round(result.confidence * 100)}
              size={64}
              strokeColor="#1677ff"
            />
            <div>
              <Text>置信度: <Text strong>{Math.round(result.confidence * 100)}%</Text></Text>
              <br />
              <Text type="secondary" style={{ fontSize: 12 }}>
                {result.workflow.nodes.length} 节点 · {result.workflow.edges.length} 连线 ·{" "}
                {Object.keys(result.workflow.variables).length} 变量
              </Text>
            </div>
          </div>

          {result.suggestions.length > 0 && (
            <div style={{ marginBottom: 12 }}>
              <Text strong style={{ display: "block", marginBottom: 4 }}>AI 建议</Text>
              <Space direction="vertical" size={4}>
                {result.suggestions.map((s, i) => (
                  <Tag key={i} color="processing">{s}</Tag>
                ))}
              </Space>
            </div>
          )}

          <Button type="primary" block onClick={handleApply}>
            应用此方案
          </Button>
        </div>
      )}
    </div>
  );
}
