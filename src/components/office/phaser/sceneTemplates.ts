// SPDX-License-Identifier: AGPL-3.0-only

/**
 * Phaser 像素办公室场景模板。
 *
 * 不依赖外部图片资源 — 房间布局通过坐标定义，墙体/家具在 OfficeScene
 * 中用 Phaser Graphics 现场绘制。
 *
 * 坐标系：左上角原点，单位为像素。默认画布 800×500。
 */

export interface RoomRect {
  /** 房间 ID（与 FleetMember.roomId 对应） */
  id: string;
  /** 房间名称的 i18n key 后缀（完整 key 为 `office.room.${suffix}`） */
  nameKey: string;
  x: number;
  y: number;
  width: number;
  height: number;
  /** 房间主题色（墙体描边），16 进制数字 */
  color: number;
}

export interface OfficeSceneTemplate {
  slug: string;
  /** 显示名称 i18n key 后缀（完整 key 为 `office.scene.${suffix}`） */
  displayNameKey: string;
  /** 画布宽度（像素） */
  canvasWidth: number;
  /** 画布高度（像素） */
  canvasHeight: number;
  /** 房间列表 */
  rooms: RoomRect[];
  /** 默认房间 ID（新加入成员的初始位置） */
  defaultRoomId: string;
}

/** 默认办公室模板 — 4 房间布局（适合 5-8 人 AI 团队） */
export const DEFAULT_OFFICE_TEMPLATE: OfficeSceneTemplate = {
  slug: "default_office",
  displayNameKey: "default_office",
  canvasWidth: 800,
  canvasHeight: 500,
  defaultRoomId: "workspace",
  rooms: [
    {
      id: "workspace",
      nameKey: "workspace",
      x: 40,
      y: 60,
      width: 360,
      height: 220,
      color: 0x1677ff,
    },
    {
      id: "meeting",
      nameKey: "meeting",
      x: 440,
      y: 60,
      width: 320,
      height: 180,
      color: 0x52c41a,
    },
    {
      id: "lounge",
      nameKey: "lounge",
      x: 440,
      y: 280,
      width: 320,
      height: 160,
      color: 0xfa8c16,
    },
    {
      id: "manager",
      nameKey: "manager",
      x: 40,
      y: 320,
      width: 360,
      height: 120,
      color: 0x722ed1,
    },
  ],
};

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
 * 全部内置场景模板（按 slug 索引）。
 *
 * AxAgent 默认提供 DEFAULT_OFFICE_TEMPLATE；AxInvest fork 追加
 * INVESTMENT_OFFICE_TEMPLATE 用于投研团队场景。下游业务方可
 * 通过 registerSceneTemplate 向此数组追加自定义模板来扩展布局。
 *
 * 同时通过 `BUILTIN_OFFICE_TEMPLATES` 暴露给本地 AxInvest 组件
 * （如 CreateFleetModal）使用。
 */
export const BUILTIN_OFFICE_TEMPLATES: OfficeSceneTemplate[] = [
  DEFAULT_OFFICE_TEMPLATE,
  INVESTMENT_OFFICE_TEMPLATE,
];

/** 上游兼容别名 — 与 BUILTIN_OFFICE_TEMPLATES 同源 */
export const SCENE_TEMPLATES: OfficeSceneTemplate[] = BUILTIN_OFFICE_TEMPLATES;

/**
 * 注册下游自定义场景模板（可选）。
 *
 * 下游业务方在初始化阶段调用此函数即可追加模板，
 * AxAgent 自身不调用 —— 只提供扩展点。
 */
export function registerSceneTemplate(template: OfficeSceneTemplate): void {
  if (!SCENE_TEMPLATES.some((t) => t.slug === template.slug)) {
    SCENE_TEMPLATES.push(template);
  }
}

/** slug → 模板查找表（用于快速查找） */
const OFFICE_TEMPLATE_MAP: Record<string, OfficeSceneTemplate> = {
  [DEFAULT_OFFICE_TEMPLATE.slug]: DEFAULT_OFFICE_TEMPLATE,
  [INVESTMENT_OFFICE_TEMPLATE.slug]: INVESTMENT_OFFICE_TEMPLATE,
};

/** 按场景 slug 查找模板（前端 fallback 到 default） */
export function resolveSceneTemplate(slug?: string): OfficeSceneTemplate {
  if (!slug) {
    return DEFAULT_OFFICE_TEMPLATE;
  }
  // 优先走静态 map（O(1)），未命中再走动态数组（包含 registerSceneTemplate 注册的）
  if (OFFICE_TEMPLATE_MAP[slug]) {
    return OFFICE_TEMPLATE_MAP[slug];
  }
  const found = SCENE_TEMPLATES.find((t) => t.slug === slug);
  return found ?? DEFAULT_OFFICE_TEMPLATE;
}

/** 给定场景模板 slug，返回其所有房间的 id 列表（供 UI 选择器使用） */
export function getBuiltinTemplateRooms(slug?: string): string[] {
  return resolveSceneTemplate(slug).rooms.map((r) => r.id);
}

/** 给定房间与成员数，计算房间内均匀分布的初始坐标 */
export function distributeMembersInRoom(
  room: RoomRect,
  memberCount: number,
  index: number,
): { x: number; y: number } {
  if (memberCount <= 0) {
    return { x: room.x + room.width / 2, y: room.y + room.height / 2 };
  }
  // 简单网格布局：每行最多 4 个，行间距 50px
  const cols = Math.min(4, memberCount);
  const col = index % cols;
  const row = Math.floor(index / cols);
  const cellW = room.width / (cols + 1);
  const cellH = 50;
  return {
    x: room.x + cellW * (col + 1),
    y: room.y + 40 + row * cellH,
  };
}
