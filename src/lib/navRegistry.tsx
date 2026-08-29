// SPDX-License-Identifier: AGPL-3.0-only
// ! 内置侧栏导航项的唯一权威来源
//
// 所有内置导航项集中在此声明，Sidebar 与 DomainHub（域聚合页）共同复用，
// 禁止在别处重复定义导航项数组。
// 导航以「能力域」为组织轴：每个导航项通过 NAV_ITEM_DOMAIN_MAP（见 domainMeta）
// 归入唯一标准域。

import { Icon } from "@/components/common/Icon";
import { domainForNavKey } from "@/lib/domainMeta";
import { BUILTIN_PAGE_PATH } from "@/lib/pageRegistry";
import type { CapabilityDomain } from "@/types/capability";

export interface NavItem {
  key: string;
  icon: React.ReactNode;
  labelKey: string;
  path: string;
  isPlugin: boolean;
  pluginName?: string;
}

/** 内置导航项 */
export const builtinNavItems: NavItem[] = [
  // ── 通用域（general） ──
  {
    key: "chat",
    icon: <Icon icon="fluent:chat-20-filled" size={17} />,
    labelKey: "nav.chat",
    path: BUILTIN_PAGE_PATH.chat,
    isPlugin: false,
  },
];

/** 按标准域过滤内置导航项 */
export function navItemsByDomain(domain: CapabilityDomain): NavItem[] {
  return builtinNavItems.filter((n) => domainForNavKey(n.key) === domain);
}
