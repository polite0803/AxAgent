// SPDX-License-Identifier: AGPL-3.0-only

import { resolveDynamicArray } from "@/lib/dynamicUI/utils";
import type { DynamicUIProps } from "@/types";
import { Tree } from "antd";
import type { TreeDataNode } from "antd";

/**
 * 树形控件，基于 Ant Design Tree。
 */
export const TreeView: React.FC<DynamicUIProps> = ({ schema, dataContext }) => {
  const treeData = resolveDynamicArray<TreeDataNode>(
    schema.props.treeData as TreeDataNode[] | undefined,
    dataContext,
    schema.id,
  );

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
