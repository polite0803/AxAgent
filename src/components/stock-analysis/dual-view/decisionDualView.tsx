// 接入面板:DecisionComparisonPanel(决策双视角对比 — 方案 D 双向并存)
// 完整 panel 复用 DecisionComparisonPanel,chat bubble 用 CompactDecisionComparison。
//
// 数据契约:`data` 是 CompactDecisionShape 子集(decisionAction / positionPct /
// confidence / llmDecisionAction / llmDecisionPositionPct / llmConfidence /
// decisionAgreementScore 等)。任何这些字段集合的对象都能塞进来。
import { registerDualView } from "@/lib/dualView";
import { CompactDecisionComparison, type CompactDecisionShape } from "./CompactDecisionComparison";
import { DecisionComparisonPanel } from "./DecisionComparisonPanel";

registerDualView<CompactDecisionShape>({
  id: "decision-comparison",
  title: "决策双视角",
  icon: "SplitSquareHorizontal",
  defaultTab: "analyze",
  compact: (data) => <CompactDecisionComparison data={data} />,
  panel: (data) => <DecisionComparisonPanel data={data} />,
});
