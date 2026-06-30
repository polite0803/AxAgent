// SPDX-License-Identifier: AGPL-3.0-only

import { DislikeOutlined, LikeOutlined } from "@ant-design/icons";
import { Button, Input, notification, Space, Typography } from "antd";
import { useState } from "react";
import { useTranslation } from "react-i18next";

const { TextArea } = Input;
const { Text } = Typography;

interface FeedbackCollectorProps {
  traceId: string;
}

export default function FeedbackCollector({ traceId: _traceId }: FeedbackCollectorProps) {
  const { t } = useTranslation();
  const [rating, setRating] = useState<"like" | "dislike" | null>(null);
  const [comment, setComment] = useState("");
  const [submitted, setSubmitted] = useState(false);

  const handleSubmit = () => {
    setSubmitted(true);
    notification.success({
      message: t("trace.feedback.thanks", "感谢反馈"),
      description: t("trace.feedback.received", "您的反馈已提交，将用于优化 Agent 执行策略。"),
      placement: "bottomRight",
    });
  };

  if (submitted) {
    return (
      <div style={{ textAlign: "center", padding: 16 }}>
        <Text type="secondary">{t("trace.feedback.submitted", "反馈已提交")}</Text>
      </div>
    );
  }

  return (
    <div style={{ padding: 12 }}>
      <Text style={{ display: "block", marginBottom: 12 }}>
        {t("trace.feedback.question", "这次执行对您有帮助吗？")}
      </Text>

      <Space size={12} style={{ marginBottom: rating === "dislike" ? 12 : 0 }}>
        <Button
          icon={<LikeOutlined />}
          type={rating === "like" ? "primary" : "default"}
          onClick={() => setRating("like")}
        >
          {t("trace.feedback.helpful", "有帮助")}
        </Button>
        <Button
          icon={<DislikeOutlined />}
          type={rating === "dislike" ? "primary" : "default"}
          danger={rating === "dislike"}
          onClick={() => setRating("dislike")}
        >
          {t("trace.feedback.notHelpful", "不满意")}
        </Button>
      </Space>

      {rating === "dislike" && (
        <div style={{ marginTop: 12 }}>
          <TextArea
            rows={3}
            placeholder={t("trace.feedback.commentPlaceholder", "请描述具体问题，帮助我们改进...")}
            value={comment}
            onChange={(e) => setComment(e.target.value)}
          />
        </div>
      )}

      {rating !== null && (
        <div style={{ marginTop: 12 }}>
          <Button type="primary" size="small" onClick={handleSubmit}>
            {t("trace.feedback.submit", "提交反馈")}
          </Button>
        </div>
      )}
    </div>
  );
}
