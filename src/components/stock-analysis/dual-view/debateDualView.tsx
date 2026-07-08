// 注册试点 2:DebatePanel
// 完整 panel 复用原组件(从 store 读 debateRounds),
// chat bubble 用 CompactDebateNode 渲染传入的 rounds 数据。
import { registerDualView } from "@/lib/dualView";
import { DebatePanel } from "../DebatePanel";
import { CompactDebateNode } from "./CompactDebateNode";

registerDualView({
  id: "debate",
  title: "辩论",
  icon: "ArrowLeftRight",
  defaultTab: "analyze",
  compact: (data: unknown) => <CompactDebateNode data={data} />,
  panel: () => <DebatePanel />,
});
