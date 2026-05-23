import { useSkillExtensionStore } from "@/stores";
import { theme } from "antd";
import { Wifi, WifiOff } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useLocation } from "react-router-dom";
import { SkillStatusBar } from "./../skill/SkillStatusBar";

/**
 * 全局底部状态栏 — 28px 高度，仅在对话页可见。
 * 左侧：技能扩展注册的状态项
 * 右侧：连接状态、模型信息等
 */
export function GlobalStatusBar() {
  const { token } = theme.useToken();
  const { t } = useTranslation();
  const location = useLocation();
  const isChatPage = location.pathname === "/" || location.pathname === "";
  const count = useSkillExtensionStore((s) => s.statusBarItems.length);
  const [connected] = useState(true); // TODO: 后续接入真实连接状态

  // 仅在对话页显示
  if (!isChatPage) {
    return null;
  }

  return (
    <div
      style={{
        height: 28,
        minHeight: 28,
        display: "flex",
        alignItems: "center",
        padding: "0 12px",
        borderTop: `1px solid ${token.colorBorderSecondary}`,
        backgroundColor: token.colorBgContainer,
        gap: 12,
        fontSize: 11.5,
        color: token.colorTextQuaternary,
      }}
    >
      {/* 左侧：技能状态项 */}
      {count > 0 && <SkillStatusBar alignment="left" />}

      <div style={{ flex: 1 }} />

      {/* 右侧：连接状态 */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 4,
        }}
      >
        {connected
          ? <Wifi size={12} style={{ color: token.colorSuccess }} />
          : <WifiOff size={12} style={{ color: token.colorError }} />}
        <span>{connected ? t("status.connected") : t("status.disconnected")}</span>
      </div>
    </div>
  );
}
