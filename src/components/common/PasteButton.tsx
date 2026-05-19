import { Button, message, theme } from "antd";
import { ClipboardPaste } from "lucide-react";
import React, { useCallback } from "react";

export interface PasteButtonProps {
  /** 粘贴成功后的回调，传入剪贴板文本 */
  onPaste: (text: string) => void;
  /** Icon size in px (default: 14) */
  size?: number;
  /** Additional inline style */
  style?: React.CSSProperties;
  /** 按钮提示文本 */
  tooltip?: string;
}

/**
 * 粘贴按钮 — 从系统剪贴板读取文本并回调。
 * 专为 Android / 移动端交互优化，减少手动输入。
 */
export const PasteButton: React.FC<PasteButtonProps> = ({
  onPaste,
  size = 14,
  style,
  tooltip,
}) => {
  const { token } = theme.useToken();
  const [loading, setLoading] = React.useState(false);

  const handleClick = useCallback(async () => {
    setLoading(true);
    try {
      const text = await navigator.clipboard.readText();
      if (text) {
        onPaste(text);
      } else {
        message.info("剪贴板为空");
      }
    } catch {
      // 如果浏览器不支持 clipboard API（非 HTTPS / WebView 限制），
      // 降级提示用户手动粘贴
      message.info("请长按输入框手动粘贴");
    } finally {
      setLoading(false);
    }
  }, [onPaste]);

  return (
    <Button
      type="text"
      size="small"
      loading={loading}
      icon={<ClipboardPaste size={size} style={{ color: token.colorTextSecondary }} />}
      onClick={handleClick}
      title={tooltip ?? "从剪贴板粘贴"}
      style={style}
    />
  );
};
