// SPDX-License-Identifier: AGPL-3.0-only

import { Button } from "antd";
import { ArrowUp, Square } from "lucide-react";
import { useTranslation } from "react-i18next";

interface SendControlsProps {
  /** 是否正在流式输出（决定显示停止还是发送按钮） */
  streaming: boolean;
  /** 输入是否为空（为空时禁用发送） */
  hasContent: boolean;
  /** 发送回调 */
  onSend: () => void;
  /** 停止生成回调 */
  onCancel: () => void;
}

/**
 * 输入框右下角的发送 / 停止按钮。
 * 流式输出期间显示停止按钮，否则显示发送按钮。
 */
export function SendControls({
  streaming,
  hasContent,
  onSend,
  onCancel,
}: SendControlsProps) {
  const { t } = useTranslation();

  if (streaming) {
    return (
      <Button
        shape="circle"
        size="small"
        danger
        data-testid="stop-generation-btn"
        icon={<Square size={14} />}
        onClick={onCancel}
        style={{ flexShrink: 0, alignSelf: "flex-end" }}
      />
    );
  }

  return (
    <Button
      type="primary"
      shape="circle"
      size="small"
      data-testid="send-btn"
      aria-label={t("chat.sendMessage")}
      icon={<ArrowUp size={16} />}
      onClick={onSend}
      disabled={!hasContent || streaming}
      style={{ flexShrink: 0, alignSelf: "flex-end", width: 36, height: 36 }}
      className={hasContent && !streaming ? "ax-glow-shadow" : ""}
    />
  );
}
