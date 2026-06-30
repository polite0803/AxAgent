// SPDX-License-Identifier: AGPL-3.0-only

import type { DynamicUIProps } from "@/types";
import { Tree } from "antd";
import type { TreeDataNode } from "antd";

/**
 * 树形控件，基于 Ant Design Tree。
 */
export const TreeView: React.FC<DynamicUIProps> = ({ schema, dataContext }) => {
  const treeData =
    (schema.props.treeData as TreeDataNode[])
    || (dataContext &&
        Array.isArray((dataContext as Record<string, unknown>)[schema.id]))
      ? (
        (dataContext as Record<string, unknown>)[schema.id] as TreeDataNode[]
      )
      : [];

  const {
    checkable = false,
    showLine = false,
    showIcon = false,
    defaultExpandAll = false,
  } = schema.props as {
    checkable?: boolean;
    showLine?: boolean;
    showIcon?: boolean;
    defaultExpandAll?: boolean;
  };

  return (
    <Tree
      treeData={treeData}
      checkable={checkable}
      showLine={showLine}
      showIcon={showIcon}
      defaultExpandAll={defaultExpandAll}
      style={schema.style as React.CSSProperties}
    />
  );
};

export default TreeView;
