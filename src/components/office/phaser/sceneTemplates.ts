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

/** 按场景 slug 查找模板（前端 fallback 到 default） */
export function resolveSceneTemplate(slug?: string): OfficeSceneTemplate {
  // 当前只有一个内置模板；后续扩展可在 map 中追加
  if (slug && slug !== "default_office") {
    // 未知 slug 也降级到默认，避免渲染崩溃
    return DEFAULT_OFFICE_TEMPLATE;
  }
  return DEFAULT_OFFICE_TEMPLATE;
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
