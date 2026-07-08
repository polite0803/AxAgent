// 接入面板:AnalystReportGrid(分析师报告)
// 完整 panel 复用原组件(从 store 读 analystReports),
// chat bubble 用 CompactAnalystSummary 渲染传入的 analystReports 快照。
import { registerDualView } from "@/lib/dualView";
import { AnalystReportGrid } from "../AnalystReportGrid";
import { CompactAnalystSummary } from "./CompactAnalystSummary";

registerDualView({
  id: "analysts",
  title: "分析师报告",
  icon: "Users",
  defaultTab: "analyze",
  compact: (data: unknown) => <CompactAnalystSummary data={data} />,
  panel: () => <AnalystReportGrid />,
});
