// 接入面板:RiskMatrix
// 完整 panel 复用原组件(从 store 读 riskAssessments),
// chat bubble 用 CompactRiskSummary 渲染传入的 riskAssessments 快照。
import { registerDualView } from "@/lib/dualView";
import { RiskMatrix } from "../RiskMatrix";
import { CompactRiskSummary } from "./CompactRiskSummary";

registerDualView({
  id: "risk",
  title: "风险矩阵",
  icon: "Shield",
  defaultTab: "analyze",
  compact: (data: unknown) => <CompactRiskSummary data={data} />,
  panel: () => <RiskMatrix />,
});
