// EdgeMarkerDefs.tsx — 全局 SVG marker 定义
// 每个自定义边类型使用的箭头标记。从 BaseEdge.tsx 提取为独立组件，
// 避免 50+ 条边各自重复渲染 7 个 <marker>（原来重复 350+ 个 DOM 元素）。
//
// 在 WorkflowEditor.tsx 的 <ReactFlow> 内部包含一次即可。

import { theme } from "antd";
import React from "react";

const ORANGE_BASE = "#fa8c16";
const PURPLE_BASE = "#722ed1";

const EdgeMarkerDefs: React.FC = () => {
  const { token } = theme.useToken();

  return (
    <defs>
      <marker
        id="arrow-default"
        viewBox="0 0 10 10"
        refX="8"
        refY="5"
        markerWidth="6"
        markerHeight="6"
        orient="auto-start-reverse"
      >
        <path d="M 0 0 L 10 5 L 0 10 z" fill={token.colorTextQuaternary} />
      </marker>
      <marker
        id="arrow-direct"
        viewBox="0 0 10 10"
        refX="8"
        refY="5"
        markerWidth="6"
        markerHeight="6"
        orient="auto-start-reverse"
      >
        <path d="M 0 0 L 10 5 L 0 10 z" fill={token.colorTextQuaternary} />
      </marker>
      <marker
        id="arrow-conditionTrue"
        viewBox="0 0 10 10"
        refX="8"
        refY="5"
        markerWidth="6"
        markerHeight="6"
        orient="auto-start-reverse"
      >
        <path d="M 0 0 L 10 5 L 0 10 z" fill={token.colorSuccess} />
      </marker>
      <marker
        id="arrow-conditionFalse"
        viewBox="0 0 10 10"
        refX="8"
        refY="5"
        markerWidth="6"
        markerHeight="6"
        orient="auto-start-reverse"
      >
        <path d="M 0 0 L 10 5 L 0 10 z" fill={token.colorError} />
      </marker>
      <marker
        id="arrow-loopBack"
        viewBox="0 0 10 10"
        refX="8"
        refY="5"
        markerWidth="6"
        markerHeight="6"
        orient="auto-start-reverse"
      >
        <path d="M 0 0 L 10 5 L 0 10 z" fill={ORANGE_BASE} />
      </marker>
      <marker
        id="arrow-error"
        viewBox="0 0 10 10"
        refX="8"
        refY="5"
        markerWidth="6"
        markerHeight="6"
        orient="auto-start-reverse"
      >
        <path d="M 0 0 L 10 5 L 0 10 z" fill={token.colorError} />
      </marker>
      <marker
        id="arrow-parallelBranch"
        viewBox="0 0 10 10"
        refX="8"
        refY="5"
        markerWidth="6"
        markerHeight="6"
        orient="auto-start-reverse"
      >
        <path d="M 0 0 L 10 5 L 0 10 z" fill={PURPLE_BASE} />
      </marker>
      <marker
        id="arrow-merge"
        viewBox="0 0 10 10"
        refX="8"
        refY="5"
        markerWidth="6"
        markerHeight="6"
        orient="auto-start-reverse"
      >
        <path d="M 0 0 L 10 5 L 0 10 z" fill={token.colorPrimary} />
      </marker>
    </defs>
  );
};

export default EdgeMarkerDefs;
