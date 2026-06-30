// SPDX-License-Identifier: AGPL-3.0-only

// 时间旅行模式切换开关 — 切换当前会话的 mode（chat / agent / workflow）
// 注：完整实现在后续远程同步中补充，当前为桩组件

import { Button } from "antd";
import { Clock } from "lucide-react";
import { useTranslation } from "react-i18next";

export function ModeSwitch() {
  const { t } = useTranslation();

  return (
    <Button
      type="text"
      size="small"
      icon={<Clock size={14} />}
      title={t("timeTravel.modeSwitch")}
    />
  );
}
