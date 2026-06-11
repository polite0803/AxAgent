// SPDX-License-Identifier: AGPL-3.0-only

import { Button, Tooltip } from "antd";
import { Sparkles } from "lucide-react";
import React from "react";
import { useTranslation } from "react-i18next";

export interface AIAssistButtonProps {
  /** 按钮是否处于加载/生成中状态 */
  loading?: boolean;
  /** 按钮是否禁用 */
  disabled?: boolean;
  /** 点击回调 */
  onClick: () => void;
  /** 自定义标签；缺省使用 i18n 键 workflow.aiAssist.optimize */
  labelKey?: "optimize" | "suggest" | "generate" | "rewrite" | "contextComplete";
  /** 自定义图标尺寸（默认 12） */
  iconSize?: number;
  /** 自定义 className（用于覆盖布局） */
  className?: string;
  /** 自定义 style */
  style?: React.CSSProperties;
  /** 是否显示为紧凑模式（更小尺寸） */
  compact?: boolean;
  /** Tooltip 内容（缺省为 i18n 通用提示） */
  tooltip?: string;
}

/**
 * 节点级 AI 辅助按钮：统一的 Sparkles 图标 + 加载态 + i18n 提示。
 * 用法：
 *   <AIAssistButton loading={generating} onClick={handleAI} labelKey="optimize" />
 */
export const AIAssistButton: React.FC<AIAssistButtonProps> = ({
  loading,
  disabled,
  onClick,
  labelKey = "optimize",
  iconSize = 12,
  className,
  style,
  compact,
  tooltip,
}) => {
  const { t } = useTranslation();
  const label = t(`workflow.aiAssist.btn.${labelKey}`);
  const tip = tooltip ?? t("workflow.aiAssist.tooltip");
  const btn = (
    <Button
      type="text"
      size={compact ? "small" : "small"}
      icon={<Sparkles size={iconSize} />}
      onClick={onClick}
      loading={loading}
      disabled={disabled}
      className={className}
      style={{ padding: compact ? "0 6px" : "0 8px", fontSize: 12, ...style }}
    >
      {label}
    </Button>
  );
  return <Tooltip title={tip}>{btn}</Tooltip>;
};
