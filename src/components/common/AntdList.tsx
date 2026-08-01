// SPDX-License-Identifier: AGPL-3.0-only
// antd v6 中 List 已弃用，此文件提供基于 Flex 的兼容实现
// 保持 API 与原 antd List 一致，避免修改 10+ 消费文件

import { Flex } from "antd";
import type { CSSProperties, ReactNode } from "react";

export interface ListProps<T> {
  dataSource?: T[];
  renderItem?: (item: T, index: number) => ReactNode;
  size?: "small" | "default" | "large";
  bordered?: boolean;
  split?: boolean;
  loading?: boolean;
  header?: ReactNode;
  footer?: ReactNode;
  pagination?: unknown;
  style?: CSSProperties;
  className?: string;
  children?: ReactNode;
}

export interface ListItemProps {
  style?: CSSProperties;
  className?: string;
  onClick?: () => void;
  children?: ReactNode;
  actions?: ReactNode[];
  extra?: ReactNode;
  main?: ReactNode;
}

export function AntdListItem({
  style,
  className,
  onClick,
  children,
  actions,
  extra,
  main,
}: ListItemProps) {
  const itemStyle: CSSProperties = {
    display: "flex",
    alignItems: "center",
    padding: "8px 0",
    cursor: onClick ? "pointer" : "default",
    width: "100%",
    ...style,
  };

  return (
    <div
      className={className}
      style={itemStyle}
      onClick={onClick}
      role={onClick ? "button" : undefined}
      tabIndex={onClick ? 0 : undefined}
    >
      {main ?? children}
      {extra && <div style={{ marginLeft: "auto", flexShrink: 0 }}>{extra}</div>}
      {actions && actions.length > 0 && (
        <div style={{ marginLeft: 8, display: "flex", gap: 8 }}>
          {actions.map((action, idx) => <span key={idx}>{action}</span>)}
        </div>
      )}
    </div>
  );
}

export function AntdList<T>({
  dataSource,
  renderItem,
  size = "default",
  bordered,
  split = true,
  loading,
  header,
  footer,
  style,
  className,
  children,
}: ListProps<T>) {
  const padding = size === "small" ? "4px 8px" : size === "large" ? "16px" : "8px 12px";
  const borderStyle: CSSProperties = bordered
    ? { border: "1px solid var(--color-border-secondary, #f0f0f0)", borderRadius: 8 }
    : {};

  const items: ReactNode[] = [];

  if (header) {
    items.push(
      <div
        key="header"
        style={{
          padding,
          borderBottom: split ? "1px solid var(--color-border-secondary, #f0f0f0)" : "none",
          fontWeight: 500,
        }}
      >
        {header}
      </div>,
    );
  }

  if (loading) {
    items.push(
      <div key="loading" style={{ padding: 24, textAlign: "center", color: "var(--color-text-secondary)" }}>
        Loading...
      </div>,
    );
  } else if (dataSource && renderItem) {
    dataSource.forEach((item, index) => {
      const rendered = renderItem(item, index);
      items.push(
        <div
          key={index}
          style={{
            padding,
            borderBottom: split && index < dataSource!.length - 1
              ? "1px solid var(--color-border-secondary, #f0f0f0)"
              : "none",
          }}
        >
          {rendered}
        </div>,
      );
    });
  }

  if (children) {
    items.push(<div key="children">{children}</div>);
  }

  if (footer) {
    items.push(
      <div
        key="footer"
        style={{ padding, borderTop: split ? "1px solid var(--color-border-secondary, #f0f0f0)" : "none" }}
      >
        {footer}
      </div>,
    );
  }

  return (
    <Flex
      vertical
      className={className}
      style={{ width: "100%", ...borderStyle, ...style }}
    >
      {items}
    </Flex>
  );
}

// 兼容导出：保持原来的命名导出名 "List"，但使用新的 AntdList 实现
// 使用 Object.assign 挂载静态 Item 属性，并通过类型断言保留泛型签名

type ListComponent =
  & (<T>(props: ListProps<T>) => ReactNode)
  & { Item: typeof AntdListItem };

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const _list = Object.assign(AntdList, { Item: AntdListItem }) as any;
export const List: ListComponent = _list;
export const ListItem = AntdListItem;
