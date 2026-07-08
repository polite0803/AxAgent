import { useRightPanel } from "@/hooks/useRightPanel";
import { getDualView, isDualViewEnabled } from "@/lib/dualView";
import type { Message } from "@/types";
import { Maximize2 } from "lucide-react";
import { useTranslation } from "react-i18next";

interface ChatBubbleExpandButtonProps {
  /** 当前 bubble 对应的 Message,读取 meta.bubbleMeta.dualViewId */
  message: Message;
  /** 显式控制是否显示;默认按 meta.bubbleMeta.dualViewId 自动判断 */
  forceShow?: boolean;
}

/**
 * ChatBubbleExpandButton — 聊天里 AI 输出末尾的小图标
 * 点击 → 跳到 stock-analysis 侧栏相应 panel 并闪烁高亮
 * 前提:message.meta.bubbleMeta.dualViewId 已设置,且对应 dual view 启用
 */
export function ChatBubbleExpandButton({ message, forceShow }: ChatBubbleExpandButtonProps) {
  const { t } = useTranslation();
  const { navigateTo } = useRightPanel();
  const dualViewId = (message.meta as Record<string, unknown>)?.bubbleMeta
    ? ((message.meta as Record<string, unknown>).bubbleMeta as Record<string, unknown>).dualViewId as string | undefined
    : undefined;

  if (!dualViewId) { return null; }
  if (!isDualViewEnabled(dualViewId)) { return null; }
  if (forceShow === false) { return null; }
  const view = getDualView(dualViewId);
  if (!view) { return null; }

  return (
    <button
      type="button"
      className="msg-action-btn"
      title={t("dualView.expandToPanel", { title: view.title })}
      onClick={() => navigateTo(view.defaultTab, dualViewId)}
    >
      <Maximize2 size={12} />
      <span className="ml-1 text-[10px]">{t("dualView.expand", { title: view.title })}</span>
    </button>
  );
}
