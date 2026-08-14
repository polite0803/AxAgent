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
import {
  Building2,
  Code,
  Cpu,
  DollarSign,
  GraduationCap,
  LineChart,
  MapPin,
  MessageSquare,
  Palette,
  PieChart,
  Rocket,
  Shield,
  ShoppingBag,
  Target,
  TrendingUp,
  Users,
} from "lucide-react";

export interface NavItem {
  key: string;
  icon: React.ReactNode;
  labelKey: string;
  path: string;
  isPlugin: boolean;
  pluginName?: string;
}

const industryNav = (key: string, icon: React.ReactNode, labelKey: string, path: string): NavItem => ({
  key,
  icon,
  labelKey,
  path,
  isPlugin: false,
});

/** 内置导航项（按域归组，见 NAV_ITEM_DOMAIN_MAP） */
export const builtinNavItems: NavItem[] = [
  {
    key: "chat",
    icon: <Icon icon="fluent:chat-20-filled" size={17} />,
    labelKey: "nav.chat",
    path: BUILTIN_PAGE_PATH.chat,
    isPlugin: false,
  },
  // ── 金融域 ──
  {
    key: "invest",
    icon: <LineChart size={17} />,
    labelKey: "nav.invest",
    path: BUILTIN_PAGE_PATH.invest,
    isPlugin: false,
  },
  industryNav(
    "opc-industry-finance-invest",
    <TrendingUp size={17} />,
    "opc.industries.finance_invest",
    BUILTIN_PAGE_PATH.opcIndustryFinanceInvest,
  ),
  industryNav(
    "opc-industry-accounting",
    <PieChart size={17} />,
    "opc.industries.accounting",
    BUILTIN_PAGE_PATH.opcIndustryAccounting,
  ),
  // ── 自动化域 ──
  {
    key: "opc",
    icon: <Building2 size={17} />,
    labelKey: "nav.opc",
    path: BUILTIN_PAGE_PATH.opc,
    isPlugin: false,
  },
  industryNav(
    "opc-industry-sales-growth",
    <DollarSign size={17} />,
    "opc.industries.sales_growth",
    BUILTIN_PAGE_PATH.opcIndustrySalesGrowth,
  ),
  industryNav(
    "opc-industry-project-management",
    <Target size={17} />,
    "opc.industries.project_management",
    BUILTIN_PAGE_PATH.opcIndustryProjectManagement,
  ),
  industryNav(
    "opc-industry-industry-consulting",
    <Users size={17} />,
    "opc.industries.industry_consulting",
    BUILTIN_PAGE_PATH.opcIndustryIndustryConsulting,
  ),
  industryNav(
    "opc-industry-ecommerce",
    <ShoppingBag size={17} />,
    "opc.industries.ecommerce",
    BUILTIN_PAGE_PATH.opcIndustryEcommerce,
  ),
  // ── 运维域 ──
  industryNav(
    "opc-industry-software-dev",
    <Code size={17} />,
    "opc.industries.software_dev",
    BUILTIN_PAGE_PATH.opcIndustrySoftwareDev,
  ),
  industryNav(
    "opc-industry-security",
    <Shield size={17} />,
    "opc.industries.security",
    BUILTIN_PAGE_PATH.opcIndustrySecurity,
  ),
  // ── 数据分析域 ──
  industryNav(
    "opc-industry-geospatial",
    <MapPin size={17} />,
    "opc.industries.geospatial",
    BUILTIN_PAGE_PATH.opcIndustryGeospatial,
  ),
  industryNav(
    "opc-industry-ai-research",
    <Cpu size={17} />,
    "opc.industries.ai_research",
    BUILTIN_PAGE_PATH.opcIndustryAiResearch,
  ),
  // ── 内容创作域 ──
  industryNav(
    "opc-industry-content-media",
    <MessageSquare size={17} />,
    "opc.industries.content_media",
    BUILTIN_PAGE_PATH.opcIndustryContentMedia,
  ),
  industryNav(
    "opc-industry-design",
    <Palette size={17} />,
    "opc.industries.design",
    BUILTIN_PAGE_PATH.opcIndustryDesign,
  ),
  industryNav(
    "opc-industry-education",
    <GraduationCap size={17} />,
    "opc.industries.education",
    BUILTIN_PAGE_PATH.opcIndustryEducation,
  ),
  // ── AI 媒体域 ──
  industryNav(
    "opc-industry-game-dev",
    <Rocket size={17} />,
    "opc.industries.game_dev",
    BUILTIN_PAGE_PATH.opcIndustryGameDev,
  ),
];

/** 按标准域过滤内置导航项 */
export function navItemsByDomain(domain: CapabilityDomain): NavItem[] {
  return builtinNavItems.filter((n) => domainForNavKey(n.key) === domain);
}
