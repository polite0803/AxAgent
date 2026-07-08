/**
 * timeline-jump 解析计划：把 evidence (tabKey, panelKey) 翻译成具体 UI 动作
 *
 * 抽离到独立文件以满足 react-refresh/only-export-components 规则（页面组件
 * 文件不应 export 非组件内容）。
 */
export interface TimelineJumpPlan {
  activeTab?: string;
  sheetTab?: string;
  scrollTo?: string;
  navigateTo?: string;
}

const VALID_TABS = new Set([
  "market",
  "analysts",
  "debate",
  "value",
  "risk",
  "reflection",
  "evolution",
]);

const SHEET_PANELS = new Set([
  "holdings",
  "index",
  "sectors",
  "north",
  "events",
  "announcements",
  "concepts",
  "optionpcr",
  "industry",
  "flash",
]);

/**
 * 把 evidence 抽象层 (tabKey, panelKey) 映射到实际 UI 动作。
 *
 * 设计原则：
 *   - activeTab 只取 tabs 数组里真实存在的 key，避免死链
 *   - panelKey 可能是 sheet panel key（侧栏）或抽象名（decision/trade），
 *     各自走不同通道
 *   - 不匹配时退回到"滚动到决策 hero"作为安全兜底
 */
export function resolveTimelineJump(tabKey?: string, panelKey?: string): TimelineJumpPlan {
  // ── 抽象面板名优先处理 ──
  if (panelKey === "decision") {
    // portfolio-mgr / rule-check 的 evidence — 跳到顶部决策 hero
    return { scrollTo: "decision-banner-top" };
  }
  if (panelKey === "trade") {
    // trader 的 evidence — 跳到交易页
    return { navigateTo: "/trade" };
  }

  // ── sheet panel（侧栏） ──
  if (panelKey && SHEET_PANELS.has(panelKey)) {
    const plan: TimelineJumpPlan = { sheetTab: panelKey };
    // market 类 evidence 切到 market tab 让主区/侧栏并排
    if (tabKey === "market") { plan.activeTab = "market"; }
    return plan;
  }

  // ── 主区 tab ──
  if (panelKey && VALID_TABS.has(panelKey)) {
    return { activeTab: panelKey };
  }

  // ── 兜底：tabKey 是有效 tab 时切到该 tab，否则滚到决策 hero ──
  if (tabKey && VALID_TABS.has(tabKey)) {
    return { activeTab: tabKey };
  }
  return { scrollTo: "decision-banner-top" };
}
