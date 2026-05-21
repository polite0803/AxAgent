import { Button, message, theme } from "antd";
import { ClipboardPaste } from "lucide-react";
import React, { useCallback, useEffect } from "react";
import { useTranslation } from "react-i18next";

export interface PasteButtonProps {
  /** 粘贴成功后的回调，传入剪贴板文本 */
  onPaste: (text: string) => void;
  /** Icon size in px (default: 14) */
  size?: number;
  /** Additional inline style */
  style?: React.CSSProperties;
  /** 按钮提示文本（i18n key） */
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
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const [loading, setLoading] = React.useState(false);
  const mountedRef = React.useRef(true);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  const handleClick = useCallback(async () => {
    if (!mountedRef.current) { return; }
    setLoading(true);
    try {
      let text: string;
      // Tauri 环境优先使用原生 clipboard 插件（可靠，无需 HTTPS 权限）
      try {
        const { readText } = await import("@tauri-apps/plugin-clipboard-manager");
        text = await readText();
      } catch {
        // 浏览器降级：需要 secure context + clipboard-read 权限
        text = await navigator.clipboard.readText();
      }
      if (!mountedRef.current) { return; }
      if (text) {
        onPaste(text);
      } else {
        message.info(t("pasteButton.emptyClipboard"));
      }
    } catch {
      message.info(t("pasteButton.clipboardUnavailable"));
    } finally {
      if (mountedRef.current) { setLoading(false); }
    }
  }, [onPaste, t]);

  return (
    <Button
      type="text"
      size="small"
      loading={loading}
      icon={<ClipboardPaste size={size} style={{ color: token.colorTextSecondary }} />}
      onClick={handleClick}
      title={tooltip ?? t("pasteButton.pasteFromClipboard")}
      style={style}
    />
  );
};
