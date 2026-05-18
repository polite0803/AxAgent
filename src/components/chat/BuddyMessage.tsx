import type { BuddyMessage as BuddyMessageType } from "@/stores/feature/buddyStore";
import { Typography } from "antd";
import { useMemo } from "react";
import React from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

// 心情 → 背景色映射
const moodColors: Record<
  BuddyMessageType["mood"],
  { bg: string; border: string }
> = {
  happy: { bg: "#f6ffed", border: "#b7eb8f" },
  proud: { bg: "#e6f7ff", border: "#91d5ff" },
  curious: { bg: "#fff7e6", border: "#ffd591" },
  snarky: { bg: "#fff1f0", border: "#ffa39e" },
  concerned: { bg: "#f9f0ff", border: "#d3adf7" },
  excited: { bg: "#fff0f6", border: "#ffadd2" },
};

// 心情 → i18n key 映射
const moodLabelKeys: Record<BuddyMessageType["mood"], string> = {
  happy: "buddy.mood.happy",
  proud: "buddy.mood.proud",
  curious: "buddy.mood.curious",
  snarky: "buddy.mood.snarky",
  concerned: "buddy.mood.concerned",
  excited: "buddy.mood.excited",
};

interface BuddyMessageBubbleProps {
  message: BuddyMessageType;
  buddyEmoji: string;
  buddyName: string;
}

export const BuddyMessageBubble = React.memo(
  function BuddyMessageBubble({
    message,
    buddyEmoji,
    buddyName,
  }: BuddyMessageBubbleProps) {
    const { t } = useTranslation();
    const colors = useMemo(() => moodColors[message.mood], [message.mood]);
    const moodLabel = t(moodLabelKeys[message.mood]);

    return (
      <div
        style={{
          background: colors.bg,
          border: `1px solid ${colors.border}`,
          borderRadius: 12,
          padding: "8px 12px",
          marginBottom: 8,
          maxWidth: 260,
        }}
      >
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: 6,
            marginBottom: 4,
          }}
        >
          <Text style={{ fontSize: 14 }}>{buddyEmoji}</Text>
          <Text style={{ fontSize: 12, color: "#666", fontWeight: 500 }}>
            {buddyName}
          </Text>
          <Text
            style={{
              fontSize: 10,
              color: colors.border,
              background: "#fff",
              padding: "0 6px",
              borderRadius: 8,
              lineHeight: "18px",
            }}
          >
            {moodLabel}
          </Text>
        </div>

        <Text style={{ fontSize: 13, color: "#333", lineHeight: 1.5 }}>
          {message.text}
        </Text>
      </div>
    );
  },
  (prevProps, nextProps) => {
    return (
      prevProps.message.text === nextProps.message.text
      && prevProps.message.mood === nextProps.message.mood
      && prevProps.message.timestamp === nextProps.message.timestamp
      && prevProps.buddyEmoji === nextProps.buddyEmoji
      && prevProps.buddyName === nextProps.buddyName
    );
  },
);
