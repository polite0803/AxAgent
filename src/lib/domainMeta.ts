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
// 这是「导航以域为标准」的权威归域来源，行业/业务入口在此收敛。
// 历史业务线（invest/opc）不再是域轴，而是作为 finance/automation 域下的具体导航项。

/** 导航项 key → 标准域 id */
export const NAV_ITEM_DOMAIN_MAP: Readonly<Record<string, CapabilityDomain>> = {
  // 通用工作台（对话/知识/记忆等）
  chat: "general",
  knowledge: "general",
  memory: "general",
  wiki: "general",
  settings: "general",
  marketplace: "general",
  "multi-agent": "general",
  // 金融：投资业务 + 金融财务类行业
  invest: "finance",
  "opc-industry-finance-invest": "finance",
  "opc-industry-accounting": "finance",
  // 自动化：一人公司运营 + 销售/项目管理/电商/咨询类行业
  opc: "automation",
  "opc-industry-sales-growth": "automation",
  "opc-industry-project-management": "automation",
  "opc-industry-industry-consulting": "automation",
  "opc-industry-ecommerce": "automation",
  // 运维：终端/文件/网关 + 软件研发/安全类行业
  terminal: "devops",
  files: "devops",
  gateway: "devops",
  "opc-industry-software-dev": "devops",
  "opc-industry-security": "devops",
  // 数据分析：GIS/地理 + 科研类行业
  "opc-industry-geospatial": "data_analysis",
  "opc-industry-ai-research": "data_analysis",
  // 内容创作：内容/设计/教育类行业
  "opc-industry-content-media": "content_creation",
  "opc-industry-design": "content_creation",
  "opc-industry-education": "content_creation",
  // AI 媒体：游戏等媒体创作类行业
  "opc-industry-game-dev": "ai_media",
  // 通信：暂无内置导航项，预留
  // communication: (无)
};

/** 根据导航项 key 解析其所属标准域；未知项兜底 general */
export function domainForNavKey(key: string): CapabilityDomain {
  return NAV_ITEM_DOMAIN_MAP[key] ?? "general";
}
