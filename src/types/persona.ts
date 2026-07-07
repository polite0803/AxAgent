// SPDX-License-Identifier: AGPL-3.0-only

/** 人格列表项 */
export interface PersonalityInfo {
  name: string;
  description?: string;
  is_active: boolean;
}

/** 人格完整详情 */
export interface Personality {
  name: string;
  description?: string;
  /** SOUL.md 内容 */
  content?: string;
  /** 对应 IDENTITY.md 的 identity 部分 */
  identity: string;
  /** 对应 USER.md 的 user 部分 */
  user: string;
}
