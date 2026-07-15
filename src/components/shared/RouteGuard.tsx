// SPDX-License-Identifier: AGPL-3.0-only

import { Result } from "antd";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";

interface RouteGuardProps {
  /** 是否允许访问；false 渲染 404。undefined 视为尚未判定（加载中）。 */
  allowed: boolean | undefined;
  /** 404 时的描述文案，缺省使用 error.pageNotFound。 */
  subTitle?: string;
  children: ReactNode;
}

/**
 * 通用路由守卫：将「不存在 / 无权限 → 404」的语义从页面组件内收敛到路由层。
 *
 * 动态 UI 等按 schemaId 解析的路由，其 schema 在本地 store 中暂无 owner/visibility
 * 字段，因此此处仅做存在性判定；未来如需基于所有权的权限判定，扩展 allowed
 * 谓词即可，调用方无需改动。
 */
export function RouteGuard({ allowed, subTitle, children }: RouteGuardProps) {
  const { t } = useTranslation();
  if (allowed === undefined) {
    return null;
  }
  if (!allowed) {
    return (
      <div style={{ padding: 48, textAlign: "center" }}>
        <Result status="404" title="404" subTitle={subTitle ?? t("error.pageNotFound")} />
      </div>
    );
  }
  return <>{children}</>;
}
