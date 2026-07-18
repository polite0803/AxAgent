// SPDX-License-Identifier: AGPL-3.0-only

// BusinessRole — 业务岗位（如 CEO / CTO / 产品经理）
// 与 AgentRole（抽象执行器）和 AgencyExpert（业务人才）正交：
// - AgentRole 回答「怎么干活」（executor / planner / researcher）
// - BusinessRole 回答「在组织里担什么责」
// - AgencyExpert 回答「具体技能是什么」
// AgentProfile 通过 business_role_id + expert_id 将三者融合。

/**
 * 业务岗位来源
 * - builtin: 内置预设
 * - custom: 用户自定义
 */
export type BusinessRoleSource = "builtin" | "custom";

/** 资历等级（与 AgencyExpert 保持一致） */
export type SeniorityLevel = "junior" | "mid" | "senior" | "expert";

/**
 * 业务岗位 DTO，字段对齐后端 `BusinessRoleDto`
 * （`src-tauri/crates/harness/src/repo_dtos.rs`）。
 *
 * 注意：后端 responsibilities / decision_authority / managed_expert_ids /
 * required_certifications / active_domains 均以 JSON 字符串形式存储，
 * 前端在 store 层做 parse / stringify 转换，UI 层直接使用结构化数组/对象。
 */
export interface BusinessRole {
  id: string;
  name: string;
  description: string | null;
  /** 岗位核心职责列表（已 parse） */
  responsibilities: string[];
  /** 决策权限边界（已 parse，结构由业务自定义） */
  decisionAuthority: Record<string, unknown> | null;
  /** 汇报对象 ID（business_roles.id 自引用） */
  reportsTo: string | null;
  /** 下属专家 ID 列表（已 parse） */
  managedExpertIds: string[];
  /** 准入条件列表（已 parse） */
  requiredCertifications: string[];
  /** 激活业务域列表（已 parse，如 ["core", "invest"]） */
  activeDomains: string[];
  /** 岗位系统提示词（4 层 prompt 拼接的最外层） */
  systemPrompt: string;
  icon: string | null;
  color: string | null;
  source: BusinessRoleSource;
  sortOrder: number;
  isEnabled: boolean;
  createdAt: number;
  updatedAt: number;
}

/**
 * 创建/更新业务岗位的输入。
 * 严格对齐后端 `SaveBusinessRoleInput`
 * （`src-tauri/src/commands/business_role.rs`）。
 *
 * 语义为 upsert：前端创建时需自行生成 uuid 作为 id；
 * 更新时传入已有 id。后端创建时 is_enabled 默认为 true，
 * 目前未提供通过 save 命令修改 is_enabled 的入口。
 */
export interface SaveBusinessRoleInput {
  /** 必填，前端创建时用 crypto.randomUUID() 生成 */
  id: string;
  name: string;
  description?: string | null;
  responsibilities?: string[];
  /**
   * 决策权限边界，后端以 JSON 字符串存储。
   * 前端 store 层负责将对象 JSON.stringify 后传入；
   * 也可直接传字符串。
   */
  decisionAuthority?: string | null;
  reportsTo?: string | null;
  managedExpertIds?: string[];
  requiredCertifications?: string[];
  activeDomains?: string[];
  /** 必填，岗位系统提示词（4 层 prompt 拼接的最外层） */
  systemPrompt: string;
  icon?: string | null;
  color?: string | null;
  source?: BusinessRoleSource;
  sortOrder?: number;
}
