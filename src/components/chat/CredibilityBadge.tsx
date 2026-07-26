// SPDX-License-Identifier: AGPL-3.0-only

import { Tooltip } from "@/components/layout/Tooltip";
import { StarFilled, StarOutlined } from "@ant-design/icons";
import { Progress, Space, Tag, Typography } from "antd";
import { useTranslation } from "react-i18next";
const { Text } = Typography;

interface CredibilityBadgeProps {
  score: number;
  showLabel?: boolean;
  showStars?: boolean;
  size?: "small" | "default";
}

export function CredibilityBadge({
  score,
  showLabel = true,
  showStars = false,
  size = "default",
}: CredibilityBadgeProps) {
  const { t } = useTranslation();
  const normalizedScore = Math.max(0, Math.min(1, score));

  if (showStars) {
    const starCount = Math.round(normalizedScore * 5);
    const stars = [];
    {
      /* static 5-star rating display, safe to use index as key */
    }
    for (let i = 0; i < 5; i++) {
      stars.push(
        i < starCount
          ? (
            <StarFilled
              key={i}
              style={{ color: "#faad14", fontSize: size === "small" ? 12 : 14 }}
            />
          )
          : (
            <StarOutlined
              key={i}
              style={{ color: "#d9d9d9", fontSize: size === "small" ? 12 : 14 }}
            />
          ),
      );
    }
    return (
      <Tooltip
        title={t("credibility.score", {
          score: Math.round(normalizedScore * 100),
        })}
      >
        <Space size="small">{stars}</Space>
      </Tooltip>
    );
  }

  const colorMap = {
    high: "green",
    medium: "orange",
    low: "red",
  };

  const level = normalizedScore >= 0.7 ? "high" : normalizedScore >= 0.4 ? "medium" : "low";
  const labelMap: Record<string, string> = {
    high: t("credibility.high"),
    medium: t("credibility.medium"),
    low: t("credibility.low"),
  };

  if (showLabel) {
    return (
      <Tooltip
        title={t("credibility.scoreLabel", {
          score: Math.round(normalizedScore * 100),
        })}
      >
        <Tag
          color={colorMap[level]}
          className={size === "small" ? "text-xs" : ""}
        >
          {labelMap[level]}
        </Tag>
      </Tooltip>
    );
  }

  return (
    <Tooltip
      title={t("credibility.score", {
        score: Math.round(normalizedScore * 100),
      })}
    >
      <Tag
        color={colorMap[level]}
        className={size === "small" ? "text-xs" : ""}
      >
        {Math.round(normalizedScore * 100)}%
      </Tag>
    </Tooltip>
  );
}

interface CredibilityBarProps {
  score: number;
  showValue?: boolean;
  height?: number;
}

export function CredibilityBar({
  score,
  showValue = true,
  height = 8,
}: CredibilityBarProps) {
  const normalizedScore = Math.max(0, Math.min(1, score)) * 100;

  const color = normalizedScore >= 70
    ? "#52c41a"
    : normalizedScore >= 40
    ? "#faad14"
    : "#ff4d4f";

  return (
    <div className="flex items-center gap-2">
      <Progress
        percent={normalizedScore}
        showInfo={false}
        strokeColor={color}
        style={{ width: 100, margin: 0, height }}
      />
      {showValue && <Text type="secondary">{Math.round(normalizedScore)}%</Text>}
    </div>
  );
}

interface CredibilityIndicatorProps {
  factors: {
    authority: number;
    consistency: number;
    recency: number;
    objectivity: number;
  };
}

export function CredibilityIndicator({ factors }: CredibilityIndicatorProps) {
  const { t } = useTranslation();
  const { authority, consistency, recency, objectivity } = factors;

  return (
    <Space orientation="vertical" size="small" style={{ width: "100%" }}>
      <div className="flex items-center justify-between">
        <Text type="secondary" className="text-sm">
          {t("credibility.authority")}
        </Text>
        <CredibilityBar score={authority} showValue={false} />
      </div>
      <div className="flex items-center justify-between">
        <Text type="secondary" className="text-sm">
          {t("credibility.consistency")}
        </Text>
        <CredibilityBar score={consistency} showValue={false} />
      </div>
      <div className="flex items-center justify-between">
        <Text type="secondary" className="text-sm">
          {t("credibility.recency")}
        </Text>
        <CredibilityBar score={recency} showValue={false} />
      </div>
      <div className="flex items-center justify-between">
        <Text type="secondary" className="text-sm">
          {t("credibility.objectivity")}
        </Text>
        <CredibilityBar score={objectivity} showValue={false} />
      </div>
    </Space>
  );
}
