import NodeRenderer from "markstream-react";

interface ReportMarkdownProps {
  content: string;
  isDark?: boolean;
}

/**
 * 静态报告 Markdown 渲染封装（分析师卡片 / 辩论卡片 / 风险矩阵 / 估值面板等）。
 *
 * markstream-react 默认面向"流式 token"场景，两个默认值会坑到静态完整内容：
 *   1. deferNodesUntilVisible 默认 true —— 节点进入视口才真正渲染，否则显示占位；
 *   2. final 默认 undefined      —— 解析器认为流未结束，尾部未闭合构造保持 loading 占位。
 *
 * 这些卡片渲染的是**已完整**的报告文本，且外层普遍是 `maxHeight + overflow:auto` 的
 * 滚动容器。内容一长，后半部分节点落在滚动裁剪区、永远不进入视口 →
 * IntersectionObserver 判定"不可见" → 永久停留在"待显示"占位，一直渲染不出来。
 *
 * 因此这里固定：
 *   - final={true}                  告诉解析器内容已完整，别再挂 loading 占位；
 *   - deferNodesUntilVisible={false} 禁用视口懒渲染，完整内容一次性全部渲染。
 */
export function ReportMarkdown({ content, isDark }: ReportMarkdownProps) {
  return (
    <NodeRenderer
      content={content}
      isDark={isDark}
      final
      deferNodesUntilVisible={false}
    />
  );
}
