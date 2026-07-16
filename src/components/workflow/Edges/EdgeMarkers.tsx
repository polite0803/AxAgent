// SPDX-License-Identifier: AGPL-3.0-only

/**
 * 边箭头标记的共享定义。
 *
 * 原先每条边都在自身 `<defs>` 中重复声明 9 个 `<marker>`，节点多时产生大量
 * 冗余 DOM。这里集中渲染一次，所有边通过 `url(#arrow-<type>)` 引用。
 *
 * 颜色统一取自 antd 主题 token（loopBack/parallelBranch 使用稳定的语义色），
 * 不再依赖 `var(--orange)` / `var(--purple)` 之类的游离 CSS 变量。
 */

import { theme } from "antd";
import React from "react";

const ORANGE_BASE = "#fa8c16";
const PURPLE_BASE = "#722ed1";

interface MarkerDef {
  id: string;
  color: string;
}

export const EdgeMarkers: React.FC = () => {
  const { token } = theme.useToken();

  const markers: MarkerDef[] = [
    { id: "arrow-default", color: token.colorBorderSecondary },
    { id: "arrow-direct", color: token.colorBorderSecondary },
    { id: "arrow-conditionTrue", color: token.colorSuccess },
    { id: "arrow-conditionFalse", color: token.colorError },
    { id: "arrow-loopBack", color: ORANGE_BASE },
    { id: "arrow-error", color: token.colorError },
    { id: "arrow-parallelBranch", color: PURPLE_BASE },
    { id: "arrow-merge", color: token.colorPrimary },
  ];

  return (
    <svg
      aria-hidden
      style={{ position: "absolute", width: 0, height: 0, overflow: "hidden" }}
    >
      <defs>
        {markers.map((m) => (
          <marker
            key={m.id}
            id={m.id}
            viewBox="0 0 10 10"
            refX="8"
            refY="5"
            markerWidth="6"
            markerHeight="6"
            orient="auto-start-reverse"
          >
            <path d="M 0 0 L 10 5 L 0 10 z" fill={m.color} />
          </marker>
        ))}
      </defs>
    </svg>
  );
};
