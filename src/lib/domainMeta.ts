// SPDX-License-Identifier: AGPL-3.0-only
// ! 前端能力域（8+1）唯一权威映射
//
// 对齐后端 axagent_harness::CapabilityDomain（8 个业务功能域 + System 内部域）。
// 本文件是前端「域」概念的单一真相源：
//   - 侧栏导航按域分组
//   - 页面/行业按业务本质归入唯一标准域
//   - 大导航用域作为一级组织轴（能力发现也以域一级过滤）
//
// 设计约束（与后端 capability.rs 一致）：
//   - 只允许 8 个业务域 + System 内部域，禁止引入自定义/产品线域。
//   - 业务线（AxInvest/AxOPC）通过标签表达，不占域轴。
//   - General 是唯一兜底域。
//   - System 仅配合 SystemOnly，永不进入检索与导航。

import type { CapabilityDomain } from "@/types/capability";

// ── 域元数据 ──────────────────────────────────────

export interface CapabilityDomainMeta {
  /** 域 id（与后端 CapabilityDomain 完全一致，snake_case） */
  id: CapabilityDomain;
  /** 导航扇区显示名 i18n key */
  labelKey: string;
  /** 域一处聚合入口路径（用户选择的「域路径」导航目标） */
  path: string;
  /** 主题色（用于图标/标签高亮） */
  color: string;
  /** 排序权重（决定侧栏分组顺序） */
  order: number;
}

/** 8 个业务功能域（不含 System，System 永不进入导航） */
export const CAPABILITY_DOMAIN_META: readonly CapabilityDomainMeta[] = [
  {
    id: "general",
    labelKey: "domain.general",
    path: "/general",
    color: "#8c8c8c",
    order: 0,
  },
  {
    id: "finance",
    labelKey: "domain.finance",
    path: "/finance",
    color: "#d4380d",
    order: 1,
  },
  {
    id: "automation",
    labelKey: "domain.automation",
    path: "/automation",
    color: "#722ed1",
    order: 2,
  },
  {
    id: "devops",
    labelKey: "domain.devops",
    path: "/devops",
    color: "#13c2c2",
    order: 3,
  },
  {
    id: "data_analysis",
    labelKey: "domain.dataAnalysis",
    path: "/data-analysis",
    color: "#2f54eb",
    order: 4,
  },
  {
    id: "content_creation",
    labelKey: "domain.contentCreation",
    path: "/content-creation",
    color: "#eb2f96",
    order: 5,
  },
  {
    id: "ai_media",
    labelKey: "domain.aiMedia",
    path: "/ai-media",
    color: "#fa8c16",
    order: 6,
  },
  {
    id: "communication",
    labelKey: "domain.communication",
    path: "/communication",
    color: "#52c41a",
    order: 7,
  },
];

/** 按 id 快速索引域元数据 */
export const CAPABILITY_DOMAIN_BY_ID: ReadonlyMap<CapabilityDomain, CapabilityDomainMeta> = new Map(
  CAPABILITY_DOMAIN_META.map((m) => [m.id, m] as const),
);

/** 域 id 集合（用于校验） */
export const CAPABILITY_DOMAIN_IDS: readonly CapabilityDomain[] = CAPABILITY_DOMAIN_META.map(
  (m) => m.id,
);

// ── 导航项归域表 ──────────────────────────────────
//
// 将侧栏内置导航项（NavItem.key）按业务本质归入唯一标准域。
// 这是「导航以域为标准」的权威归域来源。

/** 导航项 key → 标准域 id */
export const NAV_ITEM_DOMAIN_MAP: Readonly<Record<string, CapabilityDomain>> = {
  // 通用域
  chat: "general",
  // 金融域
  "finance-investment": "finance",
  "finance-analysis": "finance",
  "finance-accounting": "finance",
  // 自动化域
  "automation-operations": "automation",
  "automation-sales": "automation",
  "automation-projects": "automation",
  "automation-consulting": "automation",
  "automation-ecommerce": "automation",
  // 运维域
  "devops-software": "devops",
  "devops-security": "devops",
  // 数据分析域
  "data-geospatial": "data_analysis",
  "data-ai-research": "data_analysis",
  // 内容创作域
  "content-media": "content_creation",
  "content-design": "content_creation",
  "content-education": "content_creation",
  // AI 媒体域
  "ai-media-game": "ai_media",
  // 通信域
  "communication-message": "communication",
};

/** 根据导航项 key 解析其所属标准域；未知项兜底 general */
export function domainForNavKey(key: string): CapabilityDomain {
  return NAV_ITEM_DOMAIN_MAP[key] ?? "general";
}
