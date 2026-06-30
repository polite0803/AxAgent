// SPDX-License-Identifier: AGPL-3.0-only

// IPC 重连横幅 — 当 WebSocket 断开时显示重连提示
// 注：完整实现在后续远程同步中补充，当前为桩组件

import { Alert } from "antd";
import { useTranslation } from "react-i18next";

export interface IpcReconnectBannerProps {
  healthy: boolean;
}

export function IpcReconnectBanner({ healthy }: IpcReconnectBannerProps) {
  const { t } = useTranslation();

  if (healthy) {
    return null;
  }

  return (
    <Alert
      type="warning"
      message={t("ipc.reconnecting")}
      banner
      showIcon
      closable
    />
  );
}
