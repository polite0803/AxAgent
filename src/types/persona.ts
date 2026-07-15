// SPDX-License-Identifier: AGPL-3.0-only

/** 人格列表项 */
export interface PersonalityInfo {
  name: string;
  version: string;
  description?: string;
  is_active: boolean;
}

/** 人格完整详情 */
export interface Personality {
  name: string;
  version: string;
  description?: string;
  /** SOUL.md 内容 */
  content?: string;
  /** 对应 IDENTITY.md 的 identity 部分 */
  identity: string;
  /** 对应 USER.md 的 user 部分 */
  user: string;
  /** 创建时间（UTC ISO 8601 字符串） */
  created_at: string;
}

/** Persona 自动学习结果摘要 */
export interface AutoLearnResult {
  /** 是否成功学习（样本数过少时为 false） */
  learned: boolean;
  /** 人类可读的风格摘要 */
  style_summary: string;
  /** 实际更新过的字段名列表 */
  updated_fields: string[];
  /** 收集到的代码样本数 */
  code_sample_count: number;
  /** 收集到的消息样本数 */
  message_sample_count: number;
  /** 风格置信度（0.0-1.0） */
  confidence: number;
  /** 回写到的 Persona 名称 */
  persona_name: string;
}
