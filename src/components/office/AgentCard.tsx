// i18n-exempt: 业务逻辑/格式化/日志字符串，非 UI 展示文本
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * AgentCard — 办公室成员卡片（展示状态 + token + 角色）。
 *
 * 在右侧操作面板的「成员列表」中使用，点击触发 DM 面板切换。
 *
 * 角色 Tag 显示策略：
 * - 通过 agent_slug + role 推断业务角色（投研 / 数据 / 策略 / 交易 / 风控 / 管理 / 通用）
 * - 角色 Tag 颜色与 sprites.ts 中的 ROLE_COLORS 保持一致（Ant Design 色板）
 * - 与状态 Tag 正交：角色反映"是谁"，状态反映"在干什么"
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

/**
 * 业务角色 → Ant Design Tag color 映射。
 *
 * 颜色与 sprites.ts 的 ROLE_COLORS 保持一致（Ant Design 6 色板），
 * 但用 Ant Design Tag 接受的字符串色名而非数字 hex。
 */
const ROLE_TAG_COLOR: Record<string, string> = {
  research: "blue",
  analyst: "blue",
  researcher: "blue",
  data: "cyan",
  data_room: "cyan",
  strategy: "magenta",
  strategist: "magenta",
  trading: "red",
  trader: "red",
  risk: "orange",
  risk_manager: "orange",
  meeting: "purple",
  manager: "purple",
  ceo: "purple",
  default: "green",
};

/** Ant Design Tag color 名称 → hex 值（用于头像背景色 alpha 混合） */
const TAG_COLOR_HEX: Record<string, string> = {
  blue: "#1677ff",
  cyan: "#13c2c2",
  magenta: "#eb2f96",
  red: "#f5222d",
  orange: "#fa8c16",
  purple: "#722ed1",
  green: "#52c41a",
};

/**
 * 业务角色 i18n key 后缀（对应 office.roleTag.* 翻译）。
 *
 * 与 ROLE_TAG_COLOR 的键一致，取首选项作为 i18n key。
 */
const ROLE_I18N_KEY: Record<string, string> = {
  research: "research",
  analyst: "research",
  researcher: "research",
  data: "data",
  data_room: "data",
  strategy: "strategy",
  strategist: "strategy",
  trading: "trading",
  trader: "trading",
  risk: "risk",
  risk_manager: "risk",
  meeting: "manager",
  manager: "manager",
  ceo: "manager",
};

/**
 * 根据 agent_slug + role 推断业务角色 Tag 信息。
 *
 * 返回 `{ color, labelKey }`，labelKey 对应 `office.roleTag.${key}` i18n。
 * 都不匹配时返回 default 绿色通用助手。
 */
function resolveRoleTag(agentSlug?: string, role?: string): { color: string; labelKey: string } {
  const slug = (agentSlug ?? "").toLowerCase();
  // 1. slug 直接命中
  if (slug && ROLE_TAG_COLOR[slug]) {
    return { color: ROLE_TAG_COLOR[slug], labelKey: ROLE_I18N_KEY[slug] };
  }
  // 2. slug 包含角色关键词
  if (slug) {
    for (const key of Object.keys(ROLE_TAG_COLOR)) {
      if (key !== "default" && slug.includes(key)) {
        return { color: ROLE_TAG_COLOR[key], labelKey: ROLE_I18N_KEY[key] };
      }
    }
  }
  // 3. role 关键词匹配
  const roleLower = (role ?? "").toLowerCase();
  if (roleLower) {
    for (const key of Object.keys(ROLE_TAG_COLOR)) {
      if (key !== "default" && roleLower.includes(key)) {
        return { color: ROLE_TAG_COLOR[key], labelKey: ROLE_I18N_KEY[key] };
      }
    }
    // 中文角色名匹配
    if (roleLower.includes("投研") || roleLower.includes("研究员")) {
      return { color: ROLE_TAG_COLOR.research, labelKey: "research" };
    }
    if (roleLower.includes("数据")) { return { color: ROLE_TAG_COLOR.data, labelKey: "data" }; }
    if (roleLower.includes("策略")) {
      return { color: ROLE_TAG_COLOR.strategy, labelKey: "strategy" };
    }
    if (roleLower.includes("交易") || roleLower.includes("交易员")) {
      return { color: ROLE_TAG_COLOR.trading, labelKey: "trading" };
    }
    if (roleLower.includes("风控")) { return { color: ROLE_TAG_COLOR.risk, labelKey: "risk" }; }
    if (roleLower.includes("经理") || roleLower.includes("管理")) {
      return { color: ROLE_TAG_COLOR.manager, labelKey: "manager" };
    }
  }
  return { color: ROLE_TAG_COLOR.default, labelKey: "default" };
}

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

  // 推断角色 Tag（基于 agent_slug + role）
  const roleTag = resolveRoleTag(member.agentSlug, member.role);

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
      {/* 头像 — 背景色用角色色，图标色用状态色（双映射） */}
      <div
        style={{
          width: 32,
          height: 32,
          borderRadius: 6,
          background: `${TAG_COLOR_HEX[roleTag.color] ?? TAG_COLOR_HEX.green}18`,
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
            display: "flex",
            alignItems: "center",
            gap: 6,
            fontSize: 13,
            fontWeight: 600,
            color: token.colorText,
          }}
        >
          <span
            style={{
              whiteSpace: "nowrap",
              overflow: "hidden",
              textOverflow: "ellipsis",
              flex: 1,
              minWidth: 0,
            }}
          >
            {member.displayName}
          </span>
          {/* 角色 Tag — 颜色由角色决定 */}
          <Tag
            color={roleTag.color}
            style={{ margin: 0, fontSize: 10, lineHeight: "16px", padding: "0 6px", flexShrink: 0 }}
          >
            {t(`office.roleTag.${roleTag.labelKey}`)}
          </Tag>
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
          <span
            style={{ fontFamily: "monospace", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}
          >
            {member.agentSlug}
          </span>
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
