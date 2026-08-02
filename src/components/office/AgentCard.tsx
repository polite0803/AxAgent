// SPDX-License-Identifier: AGPL-3.0-only

/**
 * AgentCard — 办公室成员卡片（展示状态 + token + 角色）。
 *
 * 在右侧操作面板的「成员列表」中使用，点击触发 DM 面板切换。
 */

import type { FleetMember, FleetMemberStatus } from "@/types";
import { Tag, theme, Tooltip } from "antd";
import { Bot, Trash2, User } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

const STATUS_COLOR: Record<FleetMemberStatus, string> = {
  idle: "#52c41a",
  busy: "#1677ff",
  paused: "#faad14",
  error: "#ff4d4f",
  offline: "#8c8c8c",
};

export interface AgentCardProps {
  member: FleetMember;
  /** 是否高亮（当前选中 DM 目标） */
  highlighted?: boolean;
  /** 点击卡片回调 */
  onClick?: (member: FleetMember) => void;
  /** 移除成员回调（提供时悬停显示删除按钮） */
  onRemove?: (member: FleetMember) => void;
}

export function AgentCard({ member, highlighted, onClick, onRemove }: AgentCardProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const [hover, setHover] = useState(false);

  const formatTokens = (n: number) => {
    if (n >= 1_000_000) { return `${(n / 1_000_000).toFixed(1)}M`; }
    if (n >= 1_000) { return `${(n / 1_000).toFixed(1)}K`; }
    return String(n);
  };

  return (
    <div
      onClick={() => onClick?.(member)}
      style={{
        display: "flex",
        alignItems: "center",
        gap: 10,
        padding: "8px 10px",
        borderRadius: 8,
        cursor: onClick ? "pointer" : "default",
        background: highlighted ? `${token.colorPrimaryBg}` : "transparent",
        border: highlighted ? `1px solid ${token.colorPrimaryBorder}` : "1px solid transparent",
        transition: "all 0.15s",
      }}
      onMouseEnter={(e) => {
        setHover(true);
        if (onClick) {
          (e.currentTarget as HTMLDivElement).style.background = highlighted
            ? `${token.colorPrimaryBg}`
            : `${token.colorFillQuaternary}`;
        }
      }}
      onMouseLeave={(e) => {
        setHover(false);
        if (onClick) {
          (e.currentTarget as HTMLDivElement).style.background = highlighted
            ? `${token.colorPrimaryBg}`
            : "transparent";
        }
      }}
    >
      {/* 状态色条 */}
      <div
        style={{
          width: 4,
          height: 36,
          borderRadius: 2,
          background: STATUS_COLOR[member.status],
          flexShrink: 0,
        }}
      />
      {/* 头像 */}
      <div
        style={{
          width: 32,
          height: 32,
          borderRadius: 6,
          background: `${STATUS_COLOR[member.status]}18`,
          color: STATUS_COLOR[member.status],
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          flexShrink: 0,
        }}
      >
        <Bot size={16} />
      </div>
      {/* 信息 */}
      <div style={{ flex: 1, minWidth: 0 }}>
        <div
          style={{
            fontSize: 13,
            fontWeight: 600,
            color: token.colorText,
            whiteSpace: "nowrap",
            overflow: "hidden",
            textOverflow: "ellipsis",
          }}
        >
          {member.displayName}
        </div>
        <div
          style={{
            fontSize: 11,
            color: token.colorTextTertiary,
            display: "flex",
            alignItems: "center",
            gap: 6,
            marginTop: 2,
          }}
        >
          <User size={10} />
          <span style={{ fontFamily: "monospace" }}>{member.agentSlug}</span>
        </div>
      </div>
      {/* 状态标签 */}
      <Tooltip title={t(`office.memberStatus.${member.status}`)}>
        <Tag
          color={STATUS_COLOR[member.status]}
          style={{ margin: 0, fontSize: 10, lineHeight: "16px", padding: "0 6px" }}
        >
          {t(`office.memberStatus.${member.status}`)}
        </Tag>
      </Tooltip>
      {/* token 统计 */}
      <div style={{ textAlign: "right", flexShrink: 0 }}>
        <div style={{ fontSize: 11, color: token.colorTextSecondary, fontWeight: 500 }}>
          {formatTokens(member.todayTokens)}
        </div>
        <div style={{ fontSize: 10, color: token.colorTextQuaternary }}>
          {formatTokens(member.totalTokens)}
        </div>
      </div>
      {/* 移除成员（悬停显示） */}
      {onRemove && hover && (
        <Tooltip title={t("office.removeMember.button")}>
          <div
            role="button"
            aria-label={`remove-${member.agentSlug}`}
            onClick={(e) => {
              e.stopPropagation();
              onRemove(member);
            }}
            style={{
              flexShrink: 0,
              width: 22,
              height: 22,
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              borderRadius: 4,
              color: token.colorError,
              background: `${token.colorErrorBg}`,
              cursor: "pointer",
            }}
          >
            <Trash2 size={12} />
          </div>
        </Tooltip>
      )}
    </div>
  );
}
