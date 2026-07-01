// SPDX-License-Identifier: AGPL-3.0-only

import type { DynamicUIProps } from "@/types";
import { Alert, Spin, Table } from "antd";
import type { TableColumnsType } from "antd";
import { useTranslation } from "react-i18next";

export const DataTable: React.FC<DynamicUIProps> = ({
  schema,
  dataContext,
}) => {
  const { t } = useTranslation();
  const {
    columns = [],
    dataSource: staticData,
    pagination,
    rowSelection,
    showHeader = true,
    size = "middle",
    dataLoading,
    dataError,
  } = schema.props as {
    columns: TableColumnsType<Record<string, unknown>>;
    dataSource?: Record<string, unknown>[];
    pagination?: boolean | { pageSize?: number };
    rowSelection?: Record<string, unknown>;
    showHeader?: boolean;
    size?: "small" | "middle" | "large";
    dataLoading?: boolean;
    dataError?: Error | null;
  };

  if (dataError) {
    return (
      <Alert
        type="error"
        message={t("dynamicUI.dataLoadFailed")}
        description={dataError.message}
        showIcon
      />
    );
  }

  const data = staticData
      || (dataContext
        && Array.isArray(
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
    <Spin spinning={!!dataLoading}>
      <Table<Record<string, unknown>>
        columns={columns}
        dataSource={data}
        loading={!!dataLoading}
        pagination={pagination === false ? false : { pageSize: 10, ...(pagination as object) }}
        rowSelection={rowSelection
          ? (rowSelection as TableColumnsType<Record<string, unknown>>[0] extends {
            rowSelection: infer R;
          } ? R
            : never)
          : undefined}
        showHeader={showHeader}
        size={size}
        rowKey={(record) => String(record.id || record.key || "")}
        style={schema.style as React.CSSProperties}
      />
    </Spin>
  );
};
