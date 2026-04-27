import { useResolvedAvatarSrc } from "@/hooks/useResolvedAvatarSrc";
import { CHAT_ICON_COLORS } from "@/lib/iconColors";
import type { McpServer } from "@/types";
import { Avatar, theme } from "antd";
import { FileSearch, Globe, Plug } from "lucide-react";
import React from "react";

/** Icons for builtin MCP servers, keyed by server name. Returns icon at given size. */
const BUILTIN_ICON_FACTORY: Record<string, (size: number) => React.ReactNode> = {
  "@axagent/fetch": (s) => <Globe size={s} color={CHAT_ICON_COLORS.Globe} />,
  "@axagent/search-file": (s) => <FileSearch size={s} color={CHAT_ICON_COLORS.FileSearch} />,
};

/** Static 16px icons for external use. */
export const BUILTIN_ICONS: Record<string, React.ReactNode> = {
  "@axagent/fetch": <Globe size={16} color={CHAT_ICON_COLORS.Globe} />,
  "@axagent/search-file": <FileSearch size={16} color={CHAT_ICON_COLORS.FileSearch} />,
};

/**
 * Renders the appropriate icon for an MCP server:
 * - Builtin servers → fixed icon from BUILTIN_ICONS
 * - Custom servers with emoji/url/file iconType → user-chosen icon
 * - Default → Plug icon
 */
export function McpServerIcon({ server, size = 24 }: { server: McpServer; size?: number }) {
  const resolvedSrc = useResolvedAvatarSrc(
    (server.iconType as "icon" | "emoji" | "url" | "file") ?? "icon",
    server.iconValue ?? "",
  );
  const { token } = theme.useToken();

  // Builtin servers: use fixed icon
  if (server.source === "builtin" && BUILTIN_ICON_FACTORY[server.name]) {
    return (
      <span
        style={{
          width: size,
          height: size,
          display: "inline-flex",
          alignItems: "center",
          justifyContent: "center",
          flexShrink: 0,
        }}
      >
        {BUILTIN_ICON_FACTORY[server.name](size * 0.7)}
      </span>
    );
  }

  // Custom: emoji
  if (server.iconType === "emoji" && server.iconValue) {
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
          fontSize: size * 0.6,
          lineHeight: 1,
          flexShrink: 0,
        }}
      >
        {server.iconValue}
      </span>
    );
  }

  // Custom: url or file image
  if ((server.iconType === "url" || server.iconType === "file") && server.iconValue) {
    const src = server.iconType === "file" ? resolvedSrc : server.iconValue;
    return <Avatar size={size} src={src} style={{ flexShrink: 0 }} />;
  }

  // Default: plug icon
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
        flexShrink: 0,
      }}
    >
      <Plug size={size * 0.6} color={CHAT_ICON_COLORS.Plug} />
    </span>
  );
}
