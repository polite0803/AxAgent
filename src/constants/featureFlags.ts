// SPDX-License-Identifier: AGPL-3.0-only

/**
 * 功能开关配置
 * 每个维度独立 feature flag，关闭后回退到旧行为
 */
export const FEATURE_FLAGS = {
  /** Phase 1: Agent-in-the-loop — 全局 Agent Panel + 页面上下文注入 */
  AGENT_IN_THE_LOOP: true,

  /** Phase 2: 动态 UI 构建引擎 */
  DYNAMIC_UI: false,

  /** Phase 3: 自我进化前端控制面 */
  SELF_EVOLUTION_UI: false,

  /** Phase 4: 自然语言驱动动态业务扩展 */
  NL_EXTENSION: false,
} as const;

export type FeatureFlagKey = keyof typeof FEATURE_FLAGS;

/** 检查指定 feature flag 是否启用 */
export function isFeatureEnabled(flag: FeatureFlagKey): boolean {
  return FEATURE_FLAGS[flag] === true;
}
