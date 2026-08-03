// SPDX-License-Identifier: AGPL-3.0-only

import { useTracerStore } from "@/stores/devtools/tracerStore";
import { DislikeOutlined, LikeOutlined } from "@ant-design/icons";
import { App as AntdApp, Button, Input, Space, Typography } from "antd";
import { useState } from "react";
import { useTranslation } from "react-i18next";

const {} = Input;
const { Text } = Typography;

/** 反馈记录条目 */
export interface FeedbackEntry {
  traceId: string;
  rating: "like" | "dislike";
  comment?: string;
  createdAt: number;
}

interface FeedbackCollectorProps {
  traceId: string;
}

export function FeedbackCollector({ traceId }: FeedbackCollectorProps) {
  const { t } = useTranslation();
  const { notification } = AntdApp.useApp();
  const submitFeedback = useTracerStore((s) => s.submitFeedback);
  const [rating, setRating] = useState<"like" | "dislike" | null>(null);
  const [comment, setComment] = useState("");
  const [submitted, setSubmitted] = useState(false);
  const [submitting, setSubmitting] = useState(false);

  const handleSubmit = async () => {
    setSubmitting(true);
    try {
      await submitFeedback(traceId, rating!, comment || undefined);
      setSubmitted(true);
      notification.success({
        message: t("trace.feedback.thanks"),
        description: t("trace.feedback.received"),
        placement: "bottomRight",
      });
    } catch (e: unknown) {
      notification.error({
        message: t("trace.feedback.error"),
        description: e instanceof Error ? e.message : t("trace.feedback.errorRetry"),
        placement: "bottomRight",
      });
    } finally {
      setSubmitting(false);
    }
  };

  if (submitted) {
    return (
      <div style={{ textAlign: "center", padding: 16 }}>
        <Text type="secondary">{t("trace.feedback.submitted")}</Text>
      </div>
    );
  }

  return (
    <div style={{ padding: 12 }}>
      <Text style={{ display: "block", marginBottom: 12 }}>
        {t("trace.feedback.question")}
      </Text>

      <Space size={12} style={{ marginBottom: rating === "dislike" ? 12 : 0 }}>
        <Button
          icon={<LikeOutlined />}
          type={rating === "like" ? "primary" : "default"}
          onClick={() => setRating("like")}
        >
          {t("trace.feedback.helpful")}
        </Button>
        <Button
          icon={<DislikeOutlined />}
          type={rating === "dislike" ? "primary" : "default"}
          danger={rating === "dislike"}
          onClick={() => setRating("dislike")}
        >
          {t("trace.feedback.notHelpful")}
        </Button>
      </Space>

      {rating === "dislike" && (
        <div style={{ marginTop: 12 }}>
          <Input.TextArea
            rows={3}
            placeholder={t("trace.feedback.commentPlaceholder")}
            value={comment}
            onChange={(e) => setComment(e.target.value)}
          />
        </div>
      )}

      {rating !== null && (
        <div style={{ marginTop: 12 }}>
          <Button type="primary" size="small" loading={submitting} onClick={handleSubmit}>
            {t("trace.feedback.submit")}
          </Button>
        </div>
      )}
    </div>
  );
}
