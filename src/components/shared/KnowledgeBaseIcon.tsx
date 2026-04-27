import { useResolvedAvatarSrc } from "@/hooks/useResolvedAvatarSrc";
import { CHAT_ICON_COLORS } from "@/lib/iconColors";
import type { AvatarType } from "@/stores";
import type { KnowledgeBase } from "@/types";
import { Avatar, theme } from "antd";
import { BookOpen } from "lucide-react";

interface KnowledgeBaseIconProps {
  kb: KnowledgeBase;
  size?: number;
}

export function KnowledgeBaseIcon({ kb, size = 16 }: KnowledgeBaseIconProps) {
  const resolvedSrc = useResolvedAvatarSrc((kb.iconType as AvatarType) ?? "icon", kb.iconValue ?? "");
  const { token } = theme.useToken();

  if (kb.iconType === "emoji" && kb.iconValue) {
    return (
      <span
        style={{
          width: size,
          height: size,
          borderRadius: "50%",
          backgroundColor: token.colorFillSecondary,
          display: "inline-flex",
          alignItems: "center",
          justifyContent: "center",
          fontSize: size * 0.7,
          lineHeight: 1,
          flexShrink: 0,
        }}
      >
        {kb.iconValue}
      </span>
    );
  }
  if ((kb.iconType === "url" || kb.iconType === "file") && kb.iconValue) {
    const src = kb.iconType === "file" ? resolvedSrc : kb.iconValue;
    return <Avatar size={size} src={src} style={{ flexShrink: 0 }} />;
  }
  return <BookOpen size={size} style={{ flexShrink: 0, color: CHAT_ICON_COLORS.BookOpen }} />;
}
