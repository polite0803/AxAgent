// SPDX-License-Identifier: AGPL-3.0-only

import { Icon as IconifyIcon } from "@iconify/react";
import React from "react";

export interface IconProps {
  /** Iconify icon name, e.g. "lucide:message-square", "mdi:chat", "fluent:chat-20-filled" */
  icon: string;
  size?: number;
  color?: string;
  className?: string;
  style?: React.CSSProperties;
  /** 旋转动画 */
  spin?: boolean;
}

/**
 * 统一图标组件 — 基于 Iconify，支持 Lucide / Material Design / Fluent / Phosphor 等 100+ 图标集。
 * 用法: <Icon icon="mdi:chat" size={20} color="#1677ff" />
 */
export function Icon({ icon, size = 18, color, className, style, spin }: IconProps) {
  return (
    <IconifyIcon
      icon={icon}
      width={size}
      height={size}
      color={color}
      className={className}
      style={style}
      {...(spin ? {} : {})}
    />
  );
}

/** 预定义的高质量导航图标（Material Design / Fluent 填充风格） */
// eslint-disable-next-line react-refresh/only-export-components
export const NAV_ICONS = {
  chat: "fluent:chat-20-filled",
  knowledge: "fluent:book-database-20-filled",
  gateway: "fluent:globe-20-filled",
  terminal: "fluent:prompt-20-filled",
  files: "fluent:folder-20-filled",
  settings: "fluent:settings-20-filled",
} as const;
