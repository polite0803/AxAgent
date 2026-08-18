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
  Edit3,
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

const domainNav = (key: string, icon: React.ReactNode, labelKey: string, path: string): NavItem => ({
  key,
  icon,
  labelKey,
  path,
  isPlugin: false,
});

/** 内置导航项（按 8 个标准能力域归组，对齐后端 CapabilityDomain 职责） */
export const builtinNavItems: NavItem[] = [
  // ── 通用域（general）：文件/Shell/文本/网络/搜索/文档/配置 ──
  {
    key: "chat",
    icon: <Icon icon="fluent:chat-20-filled" size={17} />,
    labelKey: "nav.chat",
    path: BUILTIN_PAGE_PATH.chat,
    isPlugin: false,
  },

  // ── 金融域（finance）：行情、交易、风控、组合管理 ──
  domainNav(
    "finance-investment",
    <LineChart size={17} />,
    "nav.financeInvestment",
    BUILTIN_PAGE_PATH.financeInvestment,
  ),
  domainNav(
    "finance-analysis",
    <TrendingUp size={17} />,
    "nav.financeAnalysis",
    BUILTIN_PAGE_PATH.financeAnalysis,
  ),
  domainNav(
    "finance-accounting",
    <PieChart size={17} />,
    "nav.financeAccounting",
    BUILTIN_PAGE_PATH.financeAccounting,
  ),

  // ── 自动化域（automation）：RPA、定时任务、工作流编排 ──
  domainNav(
    "automation-operations",
    <Building2 size={17} />,
    "nav.automationOperations",
    BUILTIN_PAGE_PATH.automationOperations,
  ),
  domainNav(
    "automation-sales",
    <DollarSign size={17} />,
    "nav.automationSales",
    BUILTIN_PAGE_PATH.automationSales,
  ),
  domainNav(
    "automation-projects",
    <Target size={17} />,
    "nav.automationProjects",
    BUILTIN_PAGE_PATH.automationProjects2,
  ),
  domainNav(
    "automation-consulting",
    <Users size={17} />,
    "nav.automationConsulting",
    BUILTIN_PAGE_PATH.automationConsulting,
  ),
  domainNav(
    "automation-ecommerce",
    <ShoppingBag size={17} />,
    "nav.automationEcommerce",
    BUILTIN_PAGE_PATH.automationEcommerce,
  ),

  // ── 运维域（devops）：CI/CD、部署、监控告警、安全审计、容器编排 ──
  domainNav(
    "devops-software",
    <Code size={17} />,
    "nav.devopsSoftware",
    BUILTIN_PAGE_PATH.devopsSoftware,
  ),
  domainNav(
    "devops-security",
    <Shield size={17} />,
    "nav.devopsSecurity",
    BUILTIN_PAGE_PATH.devopsSecurity,
  ),

  // ── 数据分析域（data_analysis）：SQL 查询、数据可视化、ETL/数据清洗 ──
  domainNav(
    "data-geospatial",
    <MapPin size={17} />,
    "nav.dataGeospatial",
    BUILTIN_PAGE_PATH.dataGeospatial,
  ),
  domainNav(
    "data-ai-research",
    <Cpu size={17} />,
    "nav.dataAiResearch",
    BUILTIN_PAGE_PATH.dataAiResearch,
  ),

  // ── 内容创作域（content_creation）：写作、设计、排版 ──
  domainNav(
    "content-media",
    <Edit3 size={17} />,
    "nav.contentMedia",
    BUILTIN_PAGE_PATH.contentMedia,
  ),
  domainNav(
    "content-design",
    <Palette size={17} />,
    "nav.contentDesign",
    BUILTIN_PAGE_PATH.contentDesign,
  ),
  domainNav(
    "content-education",
    <GraduationCap size={17} />,
    "nav.contentEducation",
    BUILTIN_PAGE_PATH.contentEducation,
  ),

  // ── AI 媒体域（ai_media）：图像/视频/音频的生成与处理 ──
  domainNav(
    "ai-media-game",
    <Rocket size={17} />,
    "nav.aiMediaGame",
    BUILTIN_PAGE_PATH.aiMediaGame,
  ),

  // ── 通信域（communication）：IM、邮件、推送通知 ──
  domainNav(
    "communication-message",
    <MessageSquare size={17} />,
    "nav.communicationMessage",
    BUILTIN_PAGE_PATH.communicationMessage,
  ),
];

/** 按标准域过滤内置导航项 */
export function navItemsByDomain(domain: CapabilityDomain): NavItem[] {
  return builtinNavItems.filter((n) => domainForNavKey(n.key) === domain);
}
