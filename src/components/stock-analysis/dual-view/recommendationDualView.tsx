// 接入面板:RecommendationPanel(选股器)
// 完整 panel 复用原组件(组件内部自行 invoke),
// chat bubble 用 CompactRecommendation 渲染传入的推荐结果快照。
import { registerDualView } from "@/lib/dualView";
import { RecommendationPanel } from "../RecommendationPanel";
import { CompactRecommendation } from "./CompactRecommendation";

registerDualView({
  id: "screener",
  title: "选股器",
  icon: "Filter",
  defaultTab: "market",
  compact: (data: unknown) => <CompactRecommendation data={data} />,
  panel: () => <RecommendationPanel />,
});
