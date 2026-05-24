import { useSkillExtensionStore } from "@/stores";
import { theme } from "antd";
import { Wifi, WifiOff } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { SkillStatusBar } from "./../skill/SkillStatusBar";

/**
 * 全局底部状态栏 — 始终可见（匹配原型设计 .statusbar）。
 * 左侧：技能扩展注册的状态项
 * 右侧：连接状态、模型信息等
 */
export function GlobalStatusBar() {
  const { token } = theme.useToken();
  const { t } = useTranslation();
  const count = useSkillExtensionStore((s) => s.statusBarItems.length);
  const [connected] = useState(true); // TODO: 后续接入真实连接状态

  return (
    <div className="statusbar">
      {/* 左侧：技能状态项 */}
      {count > 0 && <SkillStatusBar alignment="left" />}

      <div style={{ flex: 1 }} />

      {/* 右侧：连接状态 */}
      <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
        {connected
          ? <Wifi size={12} style={{ color: token.colorSuccess }} />
          : <WifiOff size={12} style={{ color: token.colorError }} />}
        <span>{connected ? t("status.connected") : t("status.disconnected")}</span>
      </div>
    </div>
  );
}
