/**
 * 移动端适配实用组件
 *
 * - ResponsiveTable: 自动横向滚动的响应式表格
 * - useIsMobile: 移动端检测 Hook
 * - MobileOnly / DesktopOnly: 按端显示
 */

import { useUIStore } from "@/stores";
import type { ReactNode } from "react";

/** 检测当前是否为移动端 */
export function useIsMobile(): boolean {
  return useUIStore((s) => s.deviceLayout) === "mobile";
}

/** 仅在移动端显示 */
export function MobileOnly({ children }: { children: ReactNode }) {
  const isMobile = useIsMobile();
  return isMobile ? <>{children}</> : null;
}

/** 仅在桌面端显示 */
export function DesktopOnly({ children }: { children: ReactNode }) {
  const isMobile = useIsMobile();
  return isMobile ? null : <>{children}</>;
}

/**
 * 响应式表格容器
 *
 * 包装 Ant Design Table 或原生 table，在移动端自动启用横向滚动。
 * 用法:
 * ```tsx
 * <ResponsiveTable>
 *   <Table dataSource={...} columns={...} />
 * </ResponsiveTable>
 * ```
 */
export function ResponsiveTable({
  children,
  className = "",
}: {
  children: ReactNode;
  className?: string;
}) {
  const isMobile = useIsMobile();

  return (
    <div
      className={isMobile ? `overflow-x-auto ${className}` : className}
      style={isMobile ? { WebkitOverflowScrolling: "touch", marginBottom: 4 } : undefined}
    >
      {children}
    </div>
  );
}

/**
 * 移动端紧凑卡片容器
 *
 * 在移动端把横向内容改为纵向堆叠，常用于"指标列表"场景。
 */
export function MobileCardGrid({
  children,
  cols = 2,
}: {
  children: ReactNode;
  cols?: number;
}) {
  const isMobile = useIsMobile();

  return (
    <div
      className={`grid gap-2`}
      style={{
        gridTemplateColumns: isMobile ? "1fr 1fr" : `repeat(${cols}, 1fr)`,
      }}
    >
      {children}
    </div>
  );
}
