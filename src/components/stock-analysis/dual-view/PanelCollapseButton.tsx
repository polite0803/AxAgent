import { Copy } from "lucide-react";
import { useTranslation } from "react-i18next";

interface PanelCollapseButtonProps {
  /** 折叠为 chat bubble 的内容(纯文本/Markdown) */
  content: string;
  /** 复制成功后的回调,可触发 antd message */
  onCopied?: () => void;
}

/**
 * PanelCollapseButton — panel 头部的"折叠为 chat bubble"按钮(降级实现)
 * Phase 9.5 试点:不向 chat 注入消息(避免污染 conversations),改为复制内容到剪贴板
 * 后续 Phase 9.6/9.7 接入更多 panel 时,会基于此 API 演进
 */
export function PanelCollapseButton({ content, onCopied }: PanelCollapseButtonProps) {
  const { t } = useTranslation();
  return (
    <button
      type="button"
      className="sa-header-back"
      title={t("dualView.collapseToBubble")}
      onClick={async () => {
        try {
          await navigator.clipboard.writeText(content);
          onCopied?.();
        } catch {
          // 复制失败 → 降级为 select+execCommand
          const ta = document.createElement("textarea");
          ta.value = content;
          document.body.appendChild(ta);
          ta.select();
          try {
            document.execCommand("copy");
            onCopied?.();
          } catch { /* ignore */ }
          document.body.removeChild(ta);
        }
      }}
    >
      <Copy size={12} /> {t("dualView.collapseToBubble")}
    </button>
  );
}
