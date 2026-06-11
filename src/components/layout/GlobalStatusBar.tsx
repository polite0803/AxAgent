// SPDX-License-Identifier: AGPL-3.0-only

import { useSkillExtensionStore } from "@/stores";
import { SkillStatusBar } from "./../skill/SkillStatusBar";
import { BackendStatusIndicator } from "./BackendStatusIndicator";

/**
 * 全局底部状态栏 — 始终可见。
 * 左侧：技能扩展注册的状态项。
 * 右侧：后台任务状态。
 * 本应用内置引擎，无需显示连接状态。
 */
export function GlobalStatusBar() {
  const count = useSkillExtensionStore((s) => s.statusBarItems.length);

  return (
    <div className="statusbar">
      {count > 0 && <SkillStatusBar alignment="left" />}
      <div className="ml-auto">
        <BackendStatusIndicator />
      </div>
    </div>
  );
}
