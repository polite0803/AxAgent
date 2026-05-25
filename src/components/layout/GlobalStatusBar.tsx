import { useSkillExtensionStore } from "@/stores";
import { SkillStatusBar } from "./../skill/SkillStatusBar";

/**
 * 全局底部状态栏 — 始终可见。
 * 左侧：技能扩展注册的状态项。
 * 本应用内置引擎，无需显示连接状态。
 */
export function GlobalStatusBar() {
  const count = useSkillExtensionStore((s) => s.statusBarItems.length);

  return (
    <div className="statusbar">
      {count > 0 && <SkillStatusBar alignment="left" />}
    </div>
  );
}
