import { useTimeAnchorStore } from "@/stores/feature/timeAnchorStore";
import { Button } from "antd";
import { Sparkles, X } from "lucide-react";
import { useTranslation } from "react-i18next";

/**
 * TimeAnchorTour — 首次进入时间旅行模式的引导气泡
 *
 * 行为：
 *   - 只在 `tourSeen === false` 时渲染
 *   - 定位在 ModeSwitch pill 上方（用 absolute 定位 + transform）
 *   - 用户点击「知道了」或右上角关闭按钮 → markTourSeen
 *   - 跨刷新不重复弹（tourSeen 字段已持久化到 localStorage）
 *
 * 设计目标（HCI §10.1 L1 引导层）：
 *   - 不阻塞用户操作
 *   - 一次性出现
 *   - 措辞直白：解释能力、解释风险、告诉用户下一步
 */
export function TimeAnchorTour() {
  const { t } = useTranslation();
  const tourSeen = useTimeAnchorStore((s) => s.tourSeen);
  const markTourSeen = useTimeAnchorStore((s) => s.markTourSeen);

  if (tourSeen) { return null; }

  const onDismiss = () => {
    markTourSeen();
  };

  return (
    <div
      data-testid="time-anchor-tour"
      role="dialog"
      aria-label={t("timeTravel.tour.title")}
      style={{
        position: "absolute",
        top: "calc(100% + 10px)",
        right: 0,
        zIndex: 1100,
        width: 320,
        padding: 14,
        background: "var(--ax-bg, #fff)",
        border: "1px solid var(--ax-warning, #f59e0b)",
        borderRadius: 10,
        boxShadow: "0 6px 24px rgba(0,0,0,0.12)",
        fontSize: 12,
        lineHeight: 1.5,
        color: "var(--ax-text, #111827)",
      }}
    >
      {/* 指向 pill 的小三角 */}
      <span
        aria-hidden
        style={{
          position: "absolute",
          top: -6,
          right: 18,
          width: 10,
          height: 10,
          background: "var(--ax-bg, #fff)",
          borderTop: "1px solid var(--ax-warning, #f59e0b)",
          borderLeft: "1px solid var(--ax-warning, #f59e0b)",
          transform: "rotate(45deg)",
        }}
      />
      <div
        style={{
          display: "flex",
          alignItems: "flex-start",
          gap: 8,
          marginBottom: 8,
        }}
      >
        <Sparkles size={14} color="var(--ax-warning, #f59e0b)" style={{ flexShrink: 0, marginTop: 1 }} />
        <div style={{ flex: 1, fontWeight: 600, fontSize: 13 }}>
          {t("timeTravel.tour.title")}
        </div>
        <button
          type="button"
          onClick={onDismiss}
          aria-label="Close"
          style={{
            background: "transparent",
            border: 0,
            cursor: "pointer",
            padding: 0,
            color: "var(--ax-text-tertiary, #9ca3af)",
            display: "flex",
          }}
        >
          <X size={12} />
        </button>
      </div>
      <p style={{ margin: "0 0 8px 0", color: "var(--ax-text-secondary, #4b5563)" }}>
        {t("timeTravel.tour.body")}
      </p>
      <p
        style={{
          margin: "0 0 10px 0",
          padding: "6px 8px",
          background: "var(--ax-bg-soft, #f9fafb)",
          borderRadius: 6,
          color: "var(--ax-text-tertiary, #6b7280)",
          fontSize: 11,
        }}
      >
        {t("timeTravel.tour.stepAnchor")}
      </p>
      <div style={{ display: "flex", justifyContent: "flex-end" }}>
        <Button type="primary" size="small" onClick={onDismiss}>
          {t("timeTravel.tour.gotIt")}
        </Button>
      </div>
    </div>
  );
}
