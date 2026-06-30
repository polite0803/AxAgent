// SPDX-License-Identifier: AGPL-3.0-only

import type { DynamicUIProps } from "@/types";
import { Table } from "antd";
import type { TableColumnsType } from "antd";

/**
 * 数据表格，基于 Ant Design Table。
 * 支持分页、排序、行选择。
 */
export const DataTable: React.FC<DynamicUIProps> = ({
  schema,
  dataContext,
}) => {
  const {
    columns = [],
    dataSource: staticData,
    pagination,
    rowSelection,
    showHeader = true,
    size = "middle",
  } = schema.props as {
    columns: TableColumnsType<Record<string, unknown>>;
    dataSource?: Record<string, unknown>[];
    pagination?: boolean | { pageSize?: number };
    rowSelection?: Record<string, unknown>;
    showHeader?: boolean;
    size?: "small" | "middle" | "large";
  };

  const data =
    staticData
    || (dataContext &&
        Array.isArray(
          (dataContext as Record<string, unknown>)[schema.id],
        ))
      ? (
        (dataContext as Record<string, unknown>)[schema.id] as Record<
          string,
          unknown
        >[]
      )
      : [];

  return (
    <Table<Record<string, unknown>>
      columns={columns}
      dataSource={data}
      pagination={
        pagination === false ? false : { pageSize: 10, ...(pagination as object) }
      }
      rowSelection={
        rowSelection
          ? (rowSelection as TableColumnsType<Record<string, unknown>>[0] extends {
              rowSelection: infer R;
            }
              ? R
              : never)
          : undefined
      }
      showHeader={showHeader}
      size={size}
      rowKey={(record) => String(record.id || record.key || "")}
      style={schema.style as React.CSSProperties}
    />
  );
};

export default DataTable;
