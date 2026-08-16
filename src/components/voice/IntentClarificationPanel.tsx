// SPDX-License-Identifier: AGPL-3.0-only

import type { IntentClarification } from "@/types";
import { Button, Card, Input, Space, Tag, Typography } from "antd";
import { CheckCircle2, MessageCircleQuestion, Sparkles, XCircle } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

interface IntentClarificationPanelProps {
  clarification: IntentClarification | null;
  onAnswerQuestion: (questionId: string, answer: string) => void;
  onConfirm: () => void;
  onCancel: () => void;
  onRephrase: () => void;
  onSkip: () => void;
}

const { TextArea } = Input;

/**
 * 意图澄清面板
 *
 * 根据意图澄清状态显示不同 UI：
 * - draft: 初始状态，不显示
 * - clarifying: 显示澄清问题列表 + 回答输入
 * - needs_confirmation: 显示意图摘要 + 确认/取消按钮
 * - submitted/cancelled: 显示结果状态
 */
export function IntentClarificationPanel({
  clarification,
  onAnswerQuestion,
  onConfirm,
  onCancel,
  onRephrase,
  onSkip,
}: IntentClarificationPanelProps) {
  const { t } = useTranslation();
  const [answerInputs, setAnswerInputs] = useState<Record<string, string>>({});

  if (!clarification || clarification.state === "draft") {
    return null;
  }

  const stateTagColor: Record<string, string> = {
    clarifying: "blue",
    needs_confirmation: "orange",
    submitted: "green",
    cancelled: "default",
  };

  const stateIcon: Record<string, React.ReactNode> = {
    clarifying: <MessageCircleQuestion size={16} />,
    needs_confirmation: <Sparkles size={16} />,
    submitted: <CheckCircle2 size={16} />,
    cancelled: <XCircle size={16} />,
  };

  const renderClarifying = () => (
    <div className="flex flex-col gap-4">
      <Typography.Text strong>{t("voice.intent.clarificationPrompt")}</Typography.Text>
      {clarification.clarificationQuestions.map((question, idx) => {
        const qid = `q_${idx}`;
        const answer = clarification.clarificationAnswers[qid] ?? "";
        return (
          <div key={qid} className="flex flex-col gap-2">
            <Typography.Text>{question}</Typography.Text>
            <TextArea
              value={answerInputs[qid] ?? answer}
              onChange={(e) => setAnswerInputs((prev) => ({ ...prev, [qid]: e.target.value }))}
              onBlur={() => {
                if (answerInputs[qid]) {
                  onAnswerQuestion(qid, answerInputs[qid]);
                }
              }}
              placeholder={t("voice.intent.clarificationPrompt")}
              autoSize={{ minRows: 1, maxRows: 3 }}
            />
          </div>
        );
      })}
      <Space className="justify-end">
        <Button size="small" onClick={onSkip}>
          {t("voice.intent.skipClarification")}
        </Button>
      </Space>
    </div>
  );

  const renderNeedsConfirmation = () => (
    <div className="flex flex-col gap-4">
      <Typography.Text strong>{t("voice.intent.confirmationPrompt")}</Typography.Text>
      <Card
        size="small"
        className="bg-blue-50/50 dark:bg-blue-900/20"
        styles={{ body: { padding: 12 } }}
      >
        <Space direction="vertical" size={4}>
          <Typography.Text type="secondary">{t("voice.intent.intentSummary")}:</Typography.Text>
          <Typography.Text strong>{clarification.intentSummary}</Typography.Text>
        </Space>
      </Card>
      {clarification.confirmationOptions && clarification.confirmationOptions.length > 0 && (
        <div className="flex flex-wrap gap-2">
          {clarification.confirmationOptions.map((opt, idx) => (
            <Tag key={idx} color="blue">
              {opt}
            </Tag>
          ))}
        </div>
      )}
      <Space className="justify-end">
        <Button size="small" onClick={onRephrase}>
          {t("voice.intent.rephrase")}
        </Button>
        <Button size="small" danger onClick={onCancel}>
          {t("voice.intent.cancel")}
        </Button>
        <Button size="small" type="primary" onClick={onConfirm}>
          {t("voice.intent.confirm")}
        </Button>
      </Space>
    </div>
  );

  const renderResult = () => (
    <div className="flex items-center gap-2">
      {stateIcon[clarification.state]}
      <Tag color={stateTagColor[clarification.state]}>
        {t(`voice.intent.${clarification.state}`)}
      </Tag>
      {clarification.confirmedIntent && (
        <Typography.Text type="secondary" className="ml-2">
          {clarification.confirmedIntent}
        </Typography.Text>
      )}
    </div>
  );

  return (
    <Card
      size="small"
      className="w-full"
      styles={{ body: { padding: 12 } }}
      title={
        <Space>
          <Tag color={stateTagColor[clarification.state]}>
            {stateIcon[clarification.state]}
            <span className="ml-1">{t(`voice.intent.${clarification.state}`)}</span>
          </Tag>
        </Space>
      }
    >
      {clarification.state === "clarifying" && renderClarifying()}
      {clarification.state === "needs_confirmation" && renderNeedsConfirmation()}
      {(clarification.state === "submitted" || clarification.state === "cancelled")
        && renderResult()}
    </Card>
  );
}
