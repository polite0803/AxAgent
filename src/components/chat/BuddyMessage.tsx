import type { BuddyMessage as BuddyMessageType } from "@/stores/feature/buddyStore";
import { Typography } from "antd";
import { useMemo } from "react";

const { Text } = Typography;

// 心情 → 背景色映射
const moodColors: Record<BuddyMessageType["mood"], { bg: string; border: string }> = {
  happy: { bg: "#f6ffed", border: "#b7eb8f" },
  proud: { bg: "#e6f7ff", border: "#91d5ff" },
  curious: { bg: "#fff7e6", border: "#ffd591" },
  snarky: { bg: "#fff1f0", border: "#ffa39e" },
  concerned: { bg: "#f9f0ff", border: "#d3adf7" },
  excited: { bg: "#fff0f6", border: "#ffadd2" },
};

// 心情 → 中文标签映射
const moodLabels: Record<BuddyMessageType["mood"], string> = {
  happy: "开心",
  proud: "自豪",
  curious: "好奇",
  snarky: "毒舌",
  concerned: "关心",
  excited: "兴奋",
};

interface BuddyMessageBubbleProps {
  message: BuddyMessageType;
  buddyEmoji: string;
  buddyName: string;
}

export function BuddyMessageBubble({ message, buddyEmoji, buddyName }: BuddyMessageBubbleProps) {
  const colors = useMemo(() => moodColors[message.mood], [message.mood]);
  const moodLabel = moodLabels[message.mood];

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
      {/* 头部：emoji + 名字 + 心情标签 */}
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

      {/* 消息正文 */}
      <Text style={{ fontSize: 13, color: "#333", lineHeight: 1.5 }}>
        {message.text}
      </Text>
    </div>
  );
}
