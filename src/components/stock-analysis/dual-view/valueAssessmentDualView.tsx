// 注册试点 1:ValueAssessmentPanel
// 估值评估的"完整 panel 视图"复用原组件(它内部从 store 读 valueAssessments["value-investor"]),
// "chat bubble 视图"用 CompactValueAssessment 渲染传入的 report 字符串。
import { registerDualView } from "@/lib/dualView";
import { ValueAssessmentPanel } from "../ValueAssessmentPanel";
import { CompactValueAssessment } from "./CompactValueAssessment";

registerDualView({
  id: "value",
  title: "估值评估",
  icon: "Banknote",
  defaultTab: "analyze",
  compact: (data: unknown) => <CompactValueAssessment data={data as { report: string } | string} />,
  // panel 模式不传 data(组件自己读 store);仅在 bubble 模式才需要 data
  panel: () => <ValueAssessmentPanel />,
});
