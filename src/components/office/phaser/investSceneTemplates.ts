// SPDX-License-Identifier: AGPL-3.0-only

/**
 * AxInvest 投研办公室场景模板扩展。
 *
 * 本模块是 AxInvest fork 对上游 AxAgent `sceneTemplates.ts` 的扩展点：
 * 通过调用 `registerSceneTemplate` 将投研办公室模板注入到上游
 * 维护的 `SCENE_TEMPLATES` 数组中，避免直接修改上游文件，便于
 * 后续合并上游时保持零冲突。
 *
 * 调用时机：在 `src/main.tsx` 的 `queueMicrotask` 初始化阶段调用
 * `registerInvestSceneTemplates()`，与 Store 注册表 / DynamicUI
 * 注册等同级，确保后续渲染 OfficeTab / CreateFleetModal 时
 * `SCENE_TEMPLATES` 已包含投研模板。
 */

import { type OfficeSceneTemplate, registerSceneTemplate } from "./sceneTemplates";

/**
 * 股票投资办公室模板 — 6 房间布局（适合 AxInvest 投研团队）。
 *
 * 房间按典型投研业务流划分：研究 → 数据 → 会议（上排），
 * 策略 → 交易 → 风控（下排），覆盖从投研到风控的完整链路。
 * 画布 800×500，2 行 3 列规整布局，单房间 240×210。
 */
export const INVESTMENT_OFFICE_TEMPLATE: OfficeSceneTemplate = {
  slug: "investment_office",
  displayNameKey: "investment_office",
  canvasWidth: 800,
  canvasHeight: 500,
  // 交易室作为默认房间，符合投资团队以交易为核心的工作流
  defaultRoomId: "trading",
  rooms: [
    {
      // 投研室 — 行业研究、基本面分析
      id: "research",
      nameKey: "research",
      x: 20,
      y: 20,
      width: 240,
      height: 210,
      color: 0x1677ff,
    },
    {
      // 数据室 — 行情数据、量化数据接入
      id: "data_room",
      nameKey: "data_room",
      x: 280,
      y: 20,
      width: 240,
      height: 210,
      color: 0x13c2c2,
    },
    {
      // 会议室 — 晨会、投研会议
      id: "meeting",
      nameKey: "meeting",
      x: 540,
      y: 20,
      width: 240,
      height: 210,
      color: 0x722ed1,
    },
    {
      // 策略室 — 策略研发、回测
      id: "strategy",
      nameKey: "strategy",
      x: 20,
      y: 250,
      width: 240,
      height: 210,
      color: 0xeb2f96,
    },
    {
      // 交易室 — 下单、实时盯盘（默认房间）
      id: "trading",
      nameKey: "trading",
      x: 280,
      y: 250,
      width: 240,
      height: 210,
      color: 0xf5222d,
    },
    {
      // 风控室 — 风险监控、合规
      id: "risk",
      nameKey: "risk",
      x: 540,
      y: 250,
      width: 240,
      height: 210,
      color: 0xfa8c16,
    },
  ],
};

/**
 * 注册 AxInvest 投研场景模板到上游 SCENE_TEMPLATES 数组。
 *
 * 幂等：重复调用不会产生重复条目（由 registerSceneTemplate 保证）。
 * 应在前端初始化阶段调用一次。
 */
export function registerInvestSceneTemplates(): void {
  registerSceneTemplate(INVESTMENT_OFFICE_TEMPLATE);
}
