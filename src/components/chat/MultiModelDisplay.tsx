import { CopyButton } from "@/components/common/CopyButton";
import { stripAxAgentTags } from "@/lib/chatMarkdown";
import type { Message } from "@/types";
import { ModelIcon } from "@lobehub/icons";
import { Alert, Button, Popconfirm, Tag, theme, Tooltip, Typography } from "antd";
import { Check, Columns2, LayoutList, Rows3, Trash2 } from "lucide-react";
import { OverlayScrollbars } from "overlayscrollbars";
import React, { useEffect, useLayoutEffect, useMemo, useRef } from "react";
import { useTranslation } from "react-i18next";

export type MultiModelDisplayMode = "tabs" | "side-by-side" | "stacked";

/** Error boundary to prevent white-screen crashes in multi-model display */
class MultiModelErrorBoundary extends React.Component<
  { children: React.ReactNode; fallback?: React.ReactNode },
  { hasError: boolean }
> {
  constructor(props: { children: React.ReactNode; fallback?: React.ReactNode }) {
    super(props);
    this.state = { hasError: false };
  }
  static getDerivedStateFromError() {
    return { hasError: true };
  }
  render() {
    if (this.state.hasError) {
      return this.props.fallback ?? <Alert type="warning" message="Multi-model display error" showIcon />;
    }
    return this.props.children;
  }
}

export interface MultiModelDisplayProps {
  versions: Message[];
  activeMessageId: string;
  mode: "side-by-side" | "stacked";
  conversationId: string;
  onSwitchVersion: (parentMessageId: string, messageId: string) => void;
  onDeleteVersion?: (messageId: string) => void;
  renderContent: (msg: Message, isVersionStreaming: boolean) => React.ReactNode;
  getModelDisplayInfo: (
    model_id?: string | null,
    providerId?: string | null,
  ) => { modelName: string; providerName: string };
  streamingMessageId?: string | null;
  multiModelDoneMessageIds: string[];
}

/**
 * Renders multiple model versions side-by-side or stacked.
 * Used when multi_model_display_mode is not 'tabs'.
 *
 * NOTE: This is a thin wrapper that always renders MultiModelDisplayInner
 * to avoid Rules of Hooks violations (all hooks must be called on every render).
 */
export const MultiModelDisplay = React.memo(function MultiModelDisplay(props: MultiModelDisplayProps) {
  return (
    <MultiModelErrorBoundary>
      <MultiModelDisplayInner {...props} />
    </MultiModelErrorBoundary>
  );
});

interface MultiModelDisplayInnerProps
  extends Omit<MultiModelDisplayProps, "multiModelDoneMessageIds" | "conversationId">
{}

function MultiModelDisplayInner({
  versions,
  activeMessageId,
  mode,
  onSwitchVersion,
  onDeleteVersion,
  renderContent,
  getModelDisplayInfo,
  streamingMessageId,
}: MultiModelDisplayInnerProps) {
  const { token } = theme.useToken();
  const { t } = useTranslation();

  const latestByModel = useMemo(() => {
    const modelMap = new Map<string, Message>();
    for (const v of versions) {
      const key = v.model_id ?? "__unknown__";
      const existing = modelMap.get(key);
      if (!existing || v.version_index > existing.version_index) {
        modelMap.set(key, v);
      }
    }
    return Array.from(modelMap.values());
  }, [versions]);

  const parentMessageId = versions[0]?.parent_message_id;

  const scrollRef = useRef<HTMLDivElement>(null);
  const isSideBySide = mode === "side-by-side";
  const latestCount = latestByModel.length;

  useLayoutEffect(() => {
    if (!isSideBySide) { return; }
    const el = scrollRef.current;
    if (!el) { return; }

    const modified: Array<{ el: HTMLElement; prev: string }> = [];
    let cur: HTMLElement | null = el;
    while (cur) {
      if (cur.classList.contains("ant-bubble")) {
        modified.push({ el: cur, prev: cur.style.cssText });
        cur.style.width = "100%";
        cur.style.boxSizing = "border-box";
        break;
      }
      if (cur.classList.contains("ant-bubble-body") || cur.classList.contains("ant-bubble-content")) {
        modified.push({ el: cur, prev: cur.style.cssText });
        cur.style.overflow = "hidden";
        cur.style.minWidth = "0";
        cur.style.width = "100%";
      }
      cur = cur.parentElement;
    }

    return () => {
      for (const item of modified) {
        item.el.style.cssText = item.prev;
      }
    };
  }, [isSideBySide]);

  useEffect(() => {
    if (!isSideBySide) { return; }
    const el = scrollRef.current;
    if (!el) { return; }

    const inst = OverlayScrollbars(
      { target: el, elements: { viewport: el } },
      {
        scrollbars: {
          theme: "os-theme-axagent",
          autoHide: "never",
          clickScroll: true,
        },
        overflow: { x: "scroll", y: "hidden" },
      },
    );

    return () => inst.destroy();
  }, [isSideBySide]);

  if (!versions || versions.length === 0) { return null; }

  if (latestCount <= 1) {
    const msg = latestByModel[0];
    if (!msg) { return null; }
    return <>{renderContent(msg, msg.id === streamingMessageId)}</>;
  }

  const containerStyle: React.CSSProperties = mode === "side-by-side"
    ? {
      display: "flex",
      gap: 12,
      overflowX: "auto",
      paddingBottom: 8,
      width: "100%",
      boxSizing: "border-box",
    }
    : {
      display: "flex",
      flexDirection: "column",
      gap: 12,
    };

  const cardStyle: React.CSSProperties = mode === "side-by-side"
    ? {
      minWidth: 300,
      flex: "0 0 auto",
      width: `calc((100% - ${(latestByModel.length - 1) * 12}px) / ${latestByModel.length})`,
      border: `1px solid ${token.colorBorderSecondary}`,
      borderRadius: token.borderRadiusLG,
      overflow: "hidden",
    }
    : {
      border: `1px solid ${token.colorBorderSecondary}`,
      borderRadius: token.borderRadiusLG,
      overflow: "hidden",
    };

  return (
    <div
      ref={scrollRef}
      style={containerStyle}
      className={mode === "side-by-side" ? "axagent-multi-model-scroll" : undefined}
    >
      {latestByModel.map((vMsg) => {
        const isActive = vMsg.id === activeMessageId;
        const isVersionStreaming = vMsg.id === streamingMessageId;
        const { modelName, providerName } = getModelDisplayInfo(
          vMsg.model_id,
          vMsg.provider_id,
        );

        return (
          <div
            key={vMsg.id}
            style={{
              ...cardStyle,
              borderColor: isActive ? token.colorPrimary : token.colorBorderSecondary,
            }}
          >
            {/* Card header */}
            <div
              style={{
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
                padding: "8px 12px",
                borderBottom: `1px solid ${token.colorBorderSecondary}`,
                backgroundColor: token.colorBgLayout,
              }}
            >
              <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                <ModelIcon model={vMsg.model_id ?? ""} size={20} type="avatar" />
                {providerName && (
                  <Tag
                    style={{
                      fontSize: 11,
                      margin: 0,
                      padding: "0 4px",
                      lineHeight: "18px",
                      color: token.colorPrimary,
                      backgroundColor: token.colorPrimaryBg,
                      border: "none",
                    }}
                  >
                    {providerName}
                  </Tag>
                )}
                <Typography.Text style={{ fontSize: 13 }}>{modelName}</Typography.Text>
                {isVersionStreaming && (
                  <span className="axagent-streaming-dots" aria-hidden="true" style={{ marginLeft: 4 }}>
                    <span />
                    <span />
                    <span />
                  </span>
                )}
              </div>
              {/* Card action buttons */}
              <div style={{ display: "flex", alignItems: "center", gap: 2 }}>
                <CopyButton
                  text={() => stripAxAgentTags(vMsg.content ?? "")}
                  size={13}
                  timeout={3000}
                />
                {/* Delete button with confirmation */}
                {onDeleteVersion && latestByModel.length > 1 && (
                  <Popconfirm
                    title={t("chat.deleteConfirm")}
                    onConfirm={() => onDeleteVersion(vMsg.id)}
                    okText={t("common.confirm")}
                    cancelText={t("common.cancel")}
                  >
                    <Button type="text" size="small" danger icon={<Trash2 size={13} />} />
                  </Popconfirm>
                )}
                {/* Use as context button */}
                <div
                  onClick={() => {
                    if (!isActive && parentMessageId) {
                      onSwitchVersion(parentMessageId, vMsg.id);
                    }
                  }}
                  style={{
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "center",
                    width: 24,
                    height: 24,
                    borderRadius: "50%",
                    cursor: isActive ? "default" : "pointer",
                    backgroundColor: isActive ? token.colorPrimary : "transparent",
                    color: isActive ? "#fff" : token.colorTextSecondary,
                    border: isActive ? "none" : `1px solid ${token.colorBorder}`,
                    transition: "all 0.2s",
                  }}
                >
                  <Check size={14} />
                </div>
              </div>
            </div>
            {/* Card content — key includes mode to force re-mount on layout switch */}
            <div key={`content-${mode}`} style={{ padding: "12px" }}>
              {renderContent(vMsg, isVersionStreaming)}
            </div>
          </div>
        );
      })}
    </div>
  );
}

/**
 * Layout switcher row — rendered below ModelTags.
 * Lets users temporarily override the display mode for a specific message.
 */
export function LayoutSwitcher({
  currentMode,
  onModeChange,
}: {
  currentMode: MultiModelDisplayMode;
  onModeChange: (mode: MultiModelDisplayMode) => void;
}) {
  const { token } = theme.useToken();
  const { t } = useTranslation();

  const modes: { key: MultiModelDisplayMode; icon: React.ReactNode; label: string }[] = [
    { key: "tabs", icon: <LayoutList size={14} />, label: t("settings.multiModelDisplayModeTabs") },
    { key: "side-by-side", icon: <Columns2 size={14} />, label: t("settings.multiModelDisplayModeSideBySide") },
    { key: "stacked", icon: <Rows3 size={14} />, label: t("settings.multiModelDisplayModeStacked") },
  ];

  return (
    <div style={{ display: "flex", alignItems: "center", gap: 2 }}>
      {modes.map(({ key, icon, label }) => (
        <Tooltip key={key} title={label} mouseEnterDelay={0.3}>
          <div
            onClick={() => onModeChange(key)}
            style={{
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              width: 24,
              height: 24,
              borderRadius: token.borderRadiusSM,
              cursor: currentMode === key ? "default" : "pointer",
              backgroundColor: currentMode === key ? token.colorPrimaryBg : "transparent",
              color: currentMode === key ? token.colorPrimary : token.colorTextQuaternary,
              transition: "all 0.2s",
            }}
          >
            {icon}
          </div>
        </Tooltip>
      ))}
    </div>
  );
}
