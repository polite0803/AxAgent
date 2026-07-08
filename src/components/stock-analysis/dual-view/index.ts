// Side-effect import: 注册所有 dual view 试点
import "./valueAssessmentDualView";
import "./debateDualView";
import "./riskDualView";
import "./recommendationDualView";
import "./analystDualView";
import "./decisionDualView";
// 后续接入(回测、节点时间线等)在此处追加
export { CompactAnalystSummary } from "./CompactAnalystSummary";
export { CompactDebateNode } from "./CompactDebateNode";
export { CompactRecommendation } from "./CompactRecommendation";
export { CompactRiskSummary } from "./CompactRiskSummary";
export { CompactValueAssessment } from "./CompactValueAssessment";
export { DualViewRenderer } from "./DualViewRenderer";
export { PanelCollapseButton } from "./PanelCollapseButton";
