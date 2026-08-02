// SPDX-License-Identifier: AGPL-3.0-only

/**
 * Phaser 像素办公室场景模板。
 *
 * 不依赖外部图片资源 — 房间布局/家具/装饰全部用 Phaser Graphics 现场绘制。
 *
 * 坐标系：左上角原点，单位为像素。默认画布 800×500。
 *
 * 下游扩展：通过 `registerSceneTemplate()` 追加自定义模板。
 * AxAgent 自身只提供"基础能力"（默认模板 + 通用家具 + 通用装饰）。
 */

// ── 房间 ──────────────────────────────────────────────

export interface RoomRect {
  id: string;
  nameKey: string;
  x: number;
  y: number;
  width: number;
  height: number;
  color: number;
}

// ── 家具 ──────────────────────────────────────────────

export type FurnitureKind =
  | "desk"
  | "chair"
  | "plant"
  | "whiteboard"
  | "shelf"
  | "sofa"
  | "rug"
  | "lamp"
  | "painting"
  | "window"
  | "door";

export interface RoomFurniture {
  kind: FurnitureKind;
  x: number;
  y: number;
  /** 可选宽度/高度（部分家具有自定义尺寸） */
  w?: number;
  h?: number;
  /** 可选颜色覆盖 */
  color?: number;
}

// ── 装饰 ──────────────────────────────────────────────

export interface RoomDecoration {
  kind: "window" | "painting" | "lamp" | "rug" | "door";
  x: number;
  y: number;
  w?: number;
  h?: number;
  color?: number;
}

// ── 模板 ──────────────────────────────────────────────

export interface OfficeSceneTemplate {
  slug: string;
  displayNameKey: string;
  canvasWidth: number;
  canvasHeight: number;
  rooms: RoomRect[];
  defaultRoomId: string;
  /** 家具（key = roomId） */
  furniture?: Record<string, RoomFurniture[]>;
  /** 墙面装饰：窗户/画框/灯（key = roomId） */
  decorations?: Record<string, RoomDecoration[]>;
}

/** 默认办公室模板 — 4 房间布局 */
export const DEFAULT_OFFICE_TEMPLATE: OfficeSceneTemplate = {
  slug: "default_office",
  displayNameKey: "default_office",
  canvasWidth: 800,
  canvasHeight: 500,
  defaultRoomId: "workspace",
  rooms: [
    { id: "workspace", nameKey: "workspace", x: 40, y: 60, width: 360, height: 220, color: 0x1677ff },
    { id: "meeting", nameKey: "meeting", x: 440, y: 60, width: 320, height: 180, color: 0x52c41a },
    { id: "lounge", nameKey: "lounge", x: 440, y: 280, width: 320, height: 160, color: 0xfa8c16 },
    { id: "manager", nameKey: "manager", x: 40, y: 320, width: 360, height: 120, color: 0x722ed1 },
  ],
  furniture: {
    workspace: [
      { kind: "desk", x: 80, y: 130 },
      { kind: "chair", x: 80, y: 165 },
      { kind: "desk", x: 220, y: 130 },
      { kind: "chair", x: 220, y: 165 },
      { kind: "shelf", x: 310, y: 100 },
      { kind: "plant", x: 50, y: 80 },
      { kind: "plant", x: 330, y: 220 },
    ],
    meeting: [
      { kind: "whiteboard", x: 160, y: 80 },
      { kind: "plant", x: 50, y: 80 },
      { kind: "plant", x: 280, y: 80 },
      { kind: "plant", x: 50, y: 150 },
      { kind: "plant", x: 280, y: 150 },
      { kind: "rug", x: 160, y: 120, w: 200, h: 80, color: 0x52c41a },
    ],
    lounge: [
      { kind: "sofa", x: 110, y: 80 },
      { kind: "sofa", x: 240, y: 80 },
      { kind: "plant", x: 50, y: 130 },
      { kind: "plant", x: 280, y: 130 },
      { kind: "rug", x: 160, y: 110, w: 180, h: 60, color: 0xfa8c16 },
    ],
    manager: [
      { kind: "desk", x: 100, y: 70 },
      { kind: "chair", x: 100, y: 100 },
      { kind: "shelf", x: 240, y: 60 },
      { kind: "plant", x: 310, y: 100 },
    ],
  },
  decorations: {
    workspace: [
      { kind: "window", x: 320, y: 5, w: 60, h: 40 },
      { kind: "painting", x: 160, y: 5, w: 40, h: 30 },
      { kind: "lamp", x: 20, y: 5 },
    ],
    meeting: [
      { kind: "window", x: 120, y: 5, w: 80, h: 35 },
      { kind: "painting", x: 240, y: 5, w: 40, h: 25 },
      { kind: "lamp", x: 10, y: 5 },
    ],
    lounge: [
      { kind: "window", x: 120, y: 5, w: 80, h: 30 },
      { kind: "painting", x: 240, y: 5, w: 50, h: 25 },
      { kind: "lamp", x: 10, y: 5 },
    ],
    manager: [
      { kind: "window", x: 280, y: 5, w: 50, h: 25 },
      { kind: "painting", x: 140, y: 5, w: 40, h: 20 },
      { kind: "lamp", x: 10, y: 5 },
    ],
  },
};

/** 创业 LOFT 模板 — 开放空间 3 房间布局（验证 registerSceneTemplate 扩展点） */
export const STARTUP_LOFT_TEMPLATE: OfficeSceneTemplate = {
  slug: "startup_loft",
  displayNameKey: "startup_loft",
  canvasWidth: 800,
  canvasHeight: 500,
  defaultRoomId: "studio",
  rooms: [
    { id: "studio", nameKey: "studio", x: 40, y: 60, width: 460, height: 240, color: 0x0ea5e9 },
    { id: "warroom", nameKey: "warroom", x: 540, y: 60, width: 220, height: 240, color: 0xf59e0b },
    { id: "corner", nameKey: "corner", x: 40, y: 340, width: 720, height: 110, color: 0x8b5cf6 },
  ],
  furniture: {
    studio: [
      { kind: "desk", x: 90, y: 140 },
      { kind: "chair", x: 90, y: 175 },
      { kind: "desk", x: 220, y: 140 },
      { kind: "chair", x: 220, y: 175 },
      { kind: "desk", x: 350, y: 140 },
      { kind: "chair", x: 350, y: 175 },
      { kind: "shelf", x: 420, y: 90 },
      { kind: "plant", x: 60, y: 90 },
      { kind: "plant", x: 430, y: 230 },
    ],
    warroom: [
      { kind: "whiteboard", x: 110, y: 80 },
      { kind: "rug", x: 110, y: 140, w: 160, h: 80, color: 0xf59e0b },
      { kind: "plant", x: 40, y: 100 },
      { kind: "plant", x: 180, y: 200 },
    ],
    corner: [
      { kind: "sofa", x: 200, y: 55 },
      { kind: "plant", x: 60, y: 80 },
      { kind: "plant", x: 660, y: 80 },
      { kind: "rug", x: 360, y: 65, w: 260, h: 60, color: 0x8b5cf6 },
    ],
  },
  decorations: {
    studio: [
      { kind: "window", x: 300, y: 5, w: 70, h: 40 },
      { kind: "painting", x: 180, y: 5, w: 40, h: 30 },
      { kind: "lamp", x: 20, y: 5 },
      { kind: "lamp", x: 420, y: 5 },
    ],
    warroom: [
      { kind: "window", x: 70, y: 5, w: 60, h: 30 },
      { kind: "painting", x: 160, y: 5, w: 35, h: 25 },
      { kind: "lamp", x: 10, y: 5 },
    ],
    corner: [
      { kind: "window", x: 560, y: 5, w: 80, h: 28 },
      { kind: "painting", x: 380, y: 5, w: 50, h: 25 },
      { kind: "lamp", x: 20, y: 5 },
      { kind: "lamp", x: 680, y: 5 },
    ],
  },
};

// ── 金融投资行业 — 交易室 + 分析师区 + 风控室 + 会议室 ──

export const FINANCE_INVEST_TEMPLATE: OfficeSceneTemplate = {
  slug: "finance_invest",
  displayNameKey: "finance_invest",
  canvasWidth: 800,
  canvasHeight: 500,
  defaultRoomId: "trading",
  rooms: [
    { id: "trading", nameKey: "trading", x: 40, y: 60, width: 440, height: 220, color: 0x1a237e },
    { id: "analysis", nameKey: "analysis", x: 500, y: 60, width: 260, height: 140, color: 0x283593 },
    { id: "risk", nameKey: "risk", x: 500, y: 220, width: 260, height: 120, color: 0x3949ab },
    { id: "meeting", nameKey: "meeting", x: 40, y: 320, width: 300, height: 140, color: 0x5c6bc0 },
  ],
  furniture: {
    trading: [
      { kind: "desk", x: 80, y: 140 },
      { kind: "chair", x: 80, y: 175 },
      { kind: "desk", x: 180, y: 140 },
      { kind: "chair", x: 180, y: 175 },
      { kind: "desk", x: 280, y: 140 },
      { kind: "chair", x: 280, y: 175 },
      { kind: "desk", x: 380, y: 140 },
      { kind: "chair", x: 380, y: 175 },
      { kind: "shelf", x: 430, y: 90 },
      { kind: "plant", x: 50, y: 80 },
      { kind: "plant", x: 430, y: 240 },
    ],
    analysis: [
      { kind: "desk", x: 100, y: 100 },
      { kind: "chair", x: 100, y: 135 },
      { kind: "desk", x: 200, y: 100 },
      { kind: "chair", x: 200, y: 135 },
      { kind: "shelf", x: 240, y: 70 },
    ],
    risk: [
      { kind: "desk", x: 80, y: 70 },
      { kind: "chair", x: 80, y: 105 },
      { kind: "plant", x: 200, y: 70 },
      { kind: "plant", x: 240, y: 100 },
    ],
    meeting: [
      { kind: "whiteboard", x: 150, y: 80 },
      { kind: "plant", x: 50, y: 80 },
      { kind: "plant", x: 250, y: 80 },
      { kind: "rug", x: 150, y: 120, w: 160, h: 60, color: 0x5c6bc0 },
    ],
  },
  decorations: {
    trading: [
      { kind: "window", x: 350, y: 5, w: 80, h: 40 },
      { kind: "painting", x: 200, y: 5, w: 50, h: 30 },
      { kind: "lamp", x: 30, y: 5 },
      { kind: "lamp", x: 400, y: 5 },
    ],
    analysis: [
      { kind: "window", x: 180, y: 5, w: 60, h: 30 },
      { kind: "lamp", x: 10, y: 5 },
    ],
    risk: [
      { kind: "window", x: 150, y: 5, w: 60, h: 25 },
      { kind: "lamp", x: 10, y: 5 },
    ],
    meeting: [
      { kind: "window", x: 100, y: 5, w: 80, h: 35 },
      { kind: "painting", x: 220, y: 5, w: 40, h: 25 },
      { kind: "lamp", x: 10, y: 5 },
    ],
  },
};

// ── 软件开发行业 — 开发工位 + 代码审查区 + 测试实验室 + 会议室 ──

export const SOFTWARE_DEV_TEMPLATE: OfficeSceneTemplate = {
  slug: "software_dev",
  displayNameKey: "software_dev",
  canvasWidth: 800,
  canvasHeight: 500,
  defaultRoomId: "dev",
  rooms: [
    { id: "dev", nameKey: "dev", x: 40, y: 60, width: 380, height: 240, color: 0x00695c },
    { id: "review", nameKey: "review", x: 440, y: 60, width: 180, height: 140, color: 0x00897b },
    { id: "lab", nameKey: "lab", x: 640, y: 60, width: 140, height: 140, color: 0x26a69a },
    { id: "meeting", nameKey: "meeting", x: 440, y: 220, width: 340, height: 160, color: 0x80cbc4 },
    { id: "lounge", nameKey: "lounge", x: 40, y: 320, width: 380, height: 140, color: 0xb2dfdb },
  ],
  furniture: {
    dev: [
      { kind: "desk", x: 70, y: 120 },
      { kind: "chair", x: 70, y: 155 },
      { kind: "desk", x: 170, y: 120 },
      { kind: "chair", x: 170, y: 155 },
      { kind: "desk", x: 270, y: 120 },
      { kind: "chair", x: 270, y: 155 },
      { kind: "shelf", x: 340, y: 90 },
      { kind: "plant", x: 50, y: 80 },
      { kind: "plant", x: 350, y: 260 },
    ],
    review: [
      { kind: "desk", x: 80, y: 80 },
      { kind: "chair", x: 80, y: 115 },
      { kind: "whiteboard", x: 80, y: 50 },
    ],
    lab: [
      { kind: "desk", x: 70, y: 80 },
      { kind: "chair", x: 70, y: 115 },
      { kind: "plant", x: 110, y: 50 },
    ],
    meeting: [
      { kind: "whiteboard", x: 170, y: 80 },
      { kind: "plant", x: 50, y: 80 },
      { kind: "plant", x: 290, y: 80 },
      { kind: "rug", x: 170, y: 120, w: 180, h: 60, color: 0x80cbc4 },
    ],
    lounge: [
      { kind: "sofa", x: 120, y: 70 },
      { kind: "sofa", x: 250, y: 70 },
      { kind: "plant", x: 60, y: 90 },
      { kind: "plant", x: 320, y: 90 },
    ],
  },
  decorations: {
    dev: [
      { kind: "window", x: 280, y: 5, w: 70, h: 40 },
      { kind: "painting", x: 150, y: 5, w: 40, h: 30 },
      { kind: "lamp", x: 20, y: 5 },
      { kind: "lamp", x: 350, y: 5 },
    ],
    review: [
      { kind: "window", x: 80, y: 5, w: 60, h: 30 },
      { kind: "lamp", x: 10, y: 5 },
    ],
    lab: [
      { kind: "window", x: 60, y: 5, w: 50, h: 25 },
      { kind: "lamp", x: 10, y: 5 },
    ],
    meeting: [
      { kind: "window", x: 150, y: 5, w: 80, h: 35 },
      { kind: "painting", x: 260, y: 5, w: 40, h: 25 },
      { kind: "lamp", x: 10, y: 5 },
    ],
    lounge: [
      { kind: "window", x: 100, y: 5, w: 80, h: 30 },
      { kind: "painting", x: 250, y: 5, w: 50, h: 25 },
      { kind: "lamp", x: 20, y: 5 },
    ],
  },
};

// ── 会计财务行业 — 财务工位 + 档案室 + 审计区 + 会议室 ──

export const ACCOUNTING_TEMPLATE: OfficeSceneTemplate = {
  slug: "accounting",
  displayNameKey: "accounting",
  canvasWidth: 800,
  canvasHeight: 500,
  defaultRoomId: "finance",
  rooms: [
    { id: "finance", nameKey: "finance", x: 40, y: 60, width: 360, height: 220, color: 0x4527a0 },
    { id: "archive", nameKey: "archive", x: 420, y: 60, width: 180, height: 220, color: 0x5e35b1 },
    { id: "audit", nameKey: "audit", x: 620, y: 60, width: 140, height: 220, color: 0x7e57c2 },
    { id: "meeting", nameKey: "meeting", x: 40, y: 320, width: 300, height: 140, color: 0x9575cd },
    { id: "manager", nameKey: "manager", x: 360, y: 320, width: 400, height: 140, color: 0xb39ddb },
  ],
  furniture: {
    finance: [
      { kind: "desk", x: 70, y: 130 },
      { kind: "chair", x: 70, y: 165 },
      { kind: "desk", x: 170, y: 130 },
      { kind: "chair", x: 170, y: 165 },
      { kind: "desk", x: 270, y: 130 },
      { kind: "chair", x: 270, y: 165 },
      { kind: "plant", x: 50, y: 80 },
      { kind: "plant", x: 330, y: 220 },
    ],
    archive: [
      { kind: "shelf", x: 50, y: 100 },
      { kind: "shelf", x: 120, y: 100 },
      { kind: "shelf", x: 50, y: 180 },
      { kind: "shelf", x: 120, y: 180 },
      { kind: "plant", x: 80, y: 60 },
    ],
    audit: [
      { kind: "desk", x: 70, y: 100 },
      { kind: "chair", x: 70, y: 135 },
      { kind: "plant", x: 40, y: 60 },
    ],
    meeting: [
      { kind: "whiteboard", x: 140, y: 80 },
      { kind: "plant", x: 40, y: 80 },
      { kind: "plant", x: 240, y: 80 },
      { kind: "rug", x: 140, y: 120, w: 160, h: 60, color: 0x9575cd },
    ],
    manager: [
      { kind: "desk", x: 100, y: 70 },
      { kind: "chair", x: 100, y: 105 },
      { kind: "shelf", x: 280, y: 70 },
      { kind: "plant", x: 350, y: 100 },
      { kind: "sofa", x: 450, y: 80 },
    ],
  },
  decorations: {
    finance: [
      { kind: "window", x: 280, y: 5, w: 60, h: 35 },
      { kind: "painting", x: 140, y: 5, w: 40, h: 25 },
      { kind: "lamp", x: 20, y: 5 },
      { kind: "lamp", x: 320, y: 5 },
    ],
    archive: [
      { kind: "window", x: 90, y: 5, w: 60, h: 35 },
      { kind: "lamp", x: 10, y: 5 },
    ],
    audit: [
      { kind: "window", x: 60, y: 5, w: 50, h: 30 },
      { kind: "lamp", x: 10, y: 5 },
    ],
    meeting: [
      { kind: "window", x: 100, y: 5, w: 80, h: 35 },
      { kind: "painting", x: 220, y: 5, w: 40, h: 25 },
      { kind: "lamp", x: 10, y: 5 },
    ],
    manager: [
      { kind: "window", x: 350, y: 5, w: 80, h: 35 },
      { kind: "painting", x: 500, y: 5, w: 50, h: 30 },
      { kind: "lamp", x: 20, y: 5 },
      { kind: "lamp", x: 650, y: 5 },
    ],
  },
};

// ── AI 科技研究行业 — 研究工位 + GPU 计算区 + 论文区 + 会议室 ──

export const AI_RESEARCH_TEMPLATE: OfficeSceneTemplate = {
  slug: "ai_research",
  displayNameKey: "ai_research",
  canvasWidth: 800,
  canvasHeight: 500,
  defaultRoomId: "research",
  rooms: [
    { id: "research", nameKey: "research", x: 40, y: 60, width: 380, height: 220, color: 0x004d40 },
    { id: "compute", nameKey: "compute", x: 440, y: 60, width: 180, height: 140, color: 0x00695c },
    { id: "paper", nameKey: "paper", x: 640, y: 60, width: 140, height: 140, color: 0x00796b },
    { id: "meeting", nameKey: "meeting", x: 440, y: 220, width: 340, height: 160, color: 0x26a69a },
    { id: "lounge", nameKey: "lounge", x: 40, y: 320, width: 380, height: 140, color: 0x4db6ac },
  ],
  furniture: {
    research: [
      { kind: "desk", x: 70, y: 130 },
      { kind: "chair", x: 70, y: 165 },
      { kind: "desk", x: 170, y: 130 },
      { kind: "chair", x: 170, y: 165 },
      { kind: "desk", x: 270, y: 130 },
      { kind: "chair", x: 270, y: 165 },
      { kind: "shelf", x: 350, y: 90 },
      { kind: "plant", x: 50, y: 80 },
    ],
    compute: [
      { kind: "shelf", x: 50, y: 70 },
      { kind: "shelf", x: 120, y: 70 },
      { kind: "shelf", x: 50, y: 120 },
      { kind: "shelf", x: 120, y: 120 },
    ],
    paper: [
      { kind: "desk", x: 60, y: 80 },
      { kind: "chair", x: 60, y: 115 },
      { kind: "plant", x: 100, y: 50 },
    ],
    meeting: [
      { kind: "whiteboard", x: 170, y: 80 },
      { kind: "plant", x: 50, y: 80 },
      { kind: "plant", x: 290, y: 80 },
      { kind: "rug", x: 170, y: 120, w: 180, h: 60, color: 0x26a69a },
    ],
    lounge: [
      { kind: "sofa", x: 120, y: 70 },
      { kind: "sofa", x: 250, y: 70 },
      { kind: "plant", x: 60, y: 90 },
      { kind: "plant", x: 320, y: 90 },
    ],
  },
  decorations: {
    research: [
      { kind: "window", x: 280, y: 5, w: 70, h: 40 },
      { kind: "painting", x: 150, y: 5, w: 40, h: 30 },
      { kind: "lamp", x: 20, y: 5 },
      { kind: "lamp", x: 350, y: 5 },
    ],
    compute: [
      { kind: "window", x: 80, y: 5, w: 60, h: 30 },
      { kind: "lamp", x: 10, y: 5 },
    ],
    paper: [
      { kind: "window", x: 60, y: 5, w: 50, h: 25 },
      { kind: "lamp", x: 10, y: 5 },
    ],
    meeting: [
      { kind: "window", x: 150, y: 5, w: 80, h: 35 },
      { kind: "painting", x: 260, y: 5, w: 40, h: 25 },
      { kind: "lamp", x: 10, y: 5 },
    ],
    lounge: [
      { kind: "window", x: 100, y: 5, w: 80, h: 30 },
      { kind: "painting", x: 250, y: 5, w: 50, h: 25 },
      { kind: "lamp", x: 20, y: 5 },
    ],
  },
};

// ── 内容媒体行业 — 创作工位 + 拍摄区 + 编辑室 + 发布区 ──

export const CONTENT_MEDIA_TEMPLATE: OfficeSceneTemplate = {
  slug: "content_media",
  displayNameKey: "content_media",
  canvasWidth: 800,
  canvasHeight: 500,
  defaultRoomId: "creation",
  rooms: [
    { id: "creation", nameKey: "creation", x: 40, y: 60, width: 360, height: 220, color: 0x6a1b9a },
    { id: "shooting", nameKey: "shooting", x: 420, y: 60, width: 180, height: 140, color: 0x8e24aa },
    { id: "editing", nameKey: "editing", x: 620, y: 60, width: 140, height: 140, color: 0xab47bc },
    { id: "publishing", nameKey: "publishing", x: 420, y: 220, width: 340, height: 160, color: 0xce93d8 },
    { id: "lounge", nameKey: "lounge", x: 40, y: 320, width: 360, height: 140, color: 0xe1bee7 },
  ],
  furniture: {
    creation: [
      { kind: "desk", x: 70, y: 130 },
      { kind: "chair", x: 70, y: 165 },
      { kind: "desk", x: 170, y: 130 },
      { kind: "chair", x: 170, y: 165 },
      { kind: "desk", x: 270, y: 130 },
      { kind: "chair", x: 270, y: 165 },
      { kind: "shelf", x: 340, y: 90 },
      { kind: "plant", x: 50, y: 80 },
    ],
    shooting: [
      { kind: "plant", x: 40, y: 50 },
      { kind: "plant", x: 140, y: 50 },
      { kind: "rug", x: 90, y: 100, w: 120, h: 70, color: 0x8e24aa },
    ],
    editing: [
      { kind: "desk", x: 60, y: 80 },
      { kind: "chair", x: 60, y: 115 },
    ],
    publishing: [
      { kind: "desk", x: 80, y: 80 },
      { kind: "chair", x: 80, y: 115 },
      { kind: "desk", x: 200, y: 80 },
      { kind: "chair", x: 200, y: 115 },
      { kind: "shelf", x: 280, y: 70 },
      { kind: "whiteboard", x: 150, y: 130 },
    ],
    lounge: [
      { kind: "sofa", x: 110, y: 70 },
      { kind: "sofa", x: 240, y: 70 },
      { kind: "plant", x: 60, y: 90 },
      { kind: "plant", x: 310, y: 90 },
    ],
  },
  decorations: {
    creation: [
      { kind: "window", x: 260, y: 5, w: 70, h: 40 },
      { kind: "painting", x: 140, y: 5, w: 40, h: 30 },
      { kind: "lamp", x: 20, y: 5 },
      { kind: "lamp", x: 320, y: 5 },
    ],
    shooting: [
      { kind: "window", x: 70, y: 5, w: 60, h: 30 },
      { kind: "lamp", x: 10, y: 5 },
    ],
    editing: [
      { kind: "window", x: 60, y: 5, w: 50, h: 25 },
      { kind: "lamp", x: 10, y: 5 },
    ],
    publishing: [
      { kind: "window", x: 150, y: 5, w: 80, h: 35 },
      { kind: "painting", x: 250, y: 5, w: 40, h: 25 },
      { kind: "lamp", x: 10, y: 5 },
    ],
    lounge: [
      { kind: "window", x: 100, y: 5, w: 70, h: 30 },
      { kind: "painting", x: 230, y: 5, w: 50, h: 25 },
      { kind: "lamp", x: 20, y: 5 },
    ],
  },
};

// ── 品牌电商行业 — 运营工位 + 选品室 + 客服中心 + 仓储区 ──

export const ECOMMERCE_TEMPLATE: OfficeSceneTemplate = {
  slug: "ecommerce",
  displayNameKey: "ecommerce",
  canvasWidth: 800,
  canvasHeight: 500,
  defaultRoomId: "operations",
  rooms: [
    { id: "operations", nameKey: "operations", x: 40, y: 60, width: 360, height: 220, color: 0xe65100 },
    { id: "showroom", nameKey: "showroom", x: 420, y: 60, width: 180, height: 140, color: 0xef6c00 },
    { id: "service", nameKey: "service", x: 620, y: 60, width: 140, height: 140, color: 0xf57c00 },
    { id: "warehouse", nameKey: "warehouse", x: 420, y: 220, width: 340, height: 160, color: 0xfb8c00 },
    { id: "lounge", nameKey: "lounge", x: 40, y: 320, width: 360, height: 140, color: 0xfcc02f },
  ],
  furniture: {
    operations: [
      { kind: "desk", x: 70, y: 130 },
      { kind: "chair", x: 70, y: 165 },
      { kind: "desk", x: 170, y: 130 },
      { kind: "chair", x: 170, y: 165 },
      { kind: "desk", x: 270, y: 130 },
      { kind: "chair", x: 270, y: 165 },
      { kind: "shelf", x: 340, y: 90 },
      { kind: "plant", x: 50, y: 80 },
    ],
    showroom: [
      { kind: "shelf", x: 50, y: 70 },
      { kind: "shelf", x: 120, y: 70 },
      { kind: "shelf", x: 50, y: 120 },
      { kind: "shelf", x: 120, y: 120 },
      { kind: "plant", x: 80, y: 40 },
    ],
    service: [
      { kind: "desk", x: 60, y: 80 },
      { kind: "chair", x: 60, y: 115 },
    ],
    warehouse: [
      { kind: "shelf", x: 60, y: 70 },
      { kind: "shelf", x: 140, y: 70 },
      { kind: "shelf", x: 220, y: 70 },
      { kind: "shelf", x: 60, y: 120 },
      { kind: "shelf", x: 140, y: 120 },
      { kind: "shelf", x: 220, y: 120 },
      { kind: "plant", x: 280, y: 50 },
    ],
    lounge: [
      { kind: "sofa", x: 110, y: 70 },
      { kind: "sofa", x: 240, y: 70 },
      { kind: "plant", x: 60, y: 90 },
      { kind: "plant", x: 310, y: 90 },
    ],
  },
  decorations: {
    operations: [
      { kind: "window", x: 260, y: 5, w: 70, h: 40 },
      { kind: "painting", x: 140, y: 5, w: 40, h: 30 },
      { kind: "lamp", x: 20, y: 5 },
      { kind: "lamp", x: 320, y: 5 },
    ],
    showroom: [
      { kind: "window", x: 80, y: 5, w: 60, h: 30 },
      { kind: "lamp", x: 10, y: 5 },
    ],
    service: [
      { kind: "window", x: 60, y: 5, w: 50, h: 25 },
      { kind: "lamp", x: 10, y: 5 },
    ],
    warehouse: [
      { kind: "window", x: 150, y: 5, w: 80, h: 35 },
      { kind: "lamp", x: 10, y: 5 },
      { kind: "lamp", x: 300, y: 5 },
    ],
    lounge: [
      { kind: "window", x: 100, y: 5, w: 70, h: 30 },
      { kind: "painting", x: 230, y: 5, w: 50, h: 25 },
      { kind: "lamp", x: 20, y: 5 },
    ],
  },
};

// ── 教育培训行业 — 教室 + 备课区 + 学生服务区 + 会议室 ──

export const EDUCATION_TEMPLATE: OfficeSceneTemplate = {
  slug: "education",
  displayNameKey: "education",
  canvasWidth: 800,
  canvasHeight: 500,
  defaultRoomId: "classroom",
  rooms: [
    { id: "classroom", nameKey: "classroom", x: 40, y: 60, width: 460, height: 240, color: 0x1b5e20 },
    { id: "prepare", nameKey: "prepare", x: 520, y: 60, width: 160, height: 140, color: 0x2e7d32 },
    { id: "service", nameKey: "service", x: 520, y: 220, width: 260, height: 140, color: 0x388e3c },
    { id: "meeting", nameKey: "meeting", x: 40, y: 320, width: 300, height: 140, color: 0x43a047 },
  ],
  furniture: {
    classroom: [
      { kind: "whiteboard", x: 230, y: 80 },
      { kind: "desk", x: 80, y: 180 },
      { kind: "chair", x: 80, y: 200 },
      { kind: "desk", x: 180, y: 180 },
      { kind: "chair", x: 180, y: 200 },
      { kind: "desk", x: 280, y: 180 },
      { kind: "chair", x: 280, y: 200 },
      { kind: "desk", x: 380, y: 180 },
      { kind: "chair", x: 380, y: 200 },
      { kind: "plant", x: 50, y: 80 },
      { kind: "plant", x: 410, y: 80 },
      { kind: "rug", x: 230, y: 130, w: 260, h: 40, color: 0x43a047 },
    ],
    prepare: [
      { kind: "desk", x: 70, y: 80 },
      { kind: "chair", x: 70, y: 115 },
      { kind: "shelf", x: 100, y: 60 },
      { kind: "plant", x: 40, y: 60 },
    ],
    service: [
      { kind: "desk", x: 80, y: 80 },
      { kind: "chair", x: 80, y: 115 },
      { kind: "desk", x: 180, y: 80 },
      { kind: "chair", x: 180, y: 115 },
      { kind: "plant", x: 240, y: 60 },
    ],
    meeting: [
      { kind: "whiteboard", x: 150, y: 80 },
      { kind: "plant", x: 50, y: 80 },
      { kind: "plant", x: 250, y: 80 },
      { kind: "rug", x: 150, y: 120, w: 160, h: 60, color: 0x43a047 },
    ],
  },
  decorations: {
    classroom: [
      { kind: "window", x: 350, y: 5, w: 80, h: 40 },
      { kind: "painting", x: 200, y: 5, w: 50, h: 30 },
      { kind: "lamp", x: 30, y: 5 },
      { kind: "lamp", x: 430, y: 5 },
    ],
    prepare: [
      { kind: "window", x: 70, y: 5, w: 50, h: 30 },
      { kind: "lamp", x: 10, y: 5 },
    ],
    service: [
      { kind: "window", x: 150, y: 5, w: 60, h: 30 },
      { kind: "lamp", x: 10, y: 5 },
    ],
    meeting: [
      { kind: "window", x: 100, y: 5, w: 80, h: 35 },
      { kind: "painting", x: 220, y: 5, w: 40, h: 25 },
      { kind: "lamp", x: 10, y: 5 },
    ],
  },
};

// ── 行业咨询行业 — 顾问工位 + 资料室 + 会议室 + 接待区 ──

export const INDUSTRY_CONSULTING_TEMPLATE: OfficeSceneTemplate = {
  slug: "industry_consulting",
  displayNameKey: "industry_consulting",
  canvasWidth: 800,
  canvasHeight: 500,
  defaultRoomId: "consulting",
  rooms: [
    { id: "consulting", nameKey: "consulting", x: 40, y: 60, width: 360, height: 220, color: 0x263238 },
    { id: "library", nameKey: "library", x: 420, y: 60, width: 180, height: 220, color: 0x37474f },
    { id: "meeting", nameKey: "meeting", x: 620, y: 60, width: 140, height: 220, color: 0x455a64 },
    { id: "reception", nameKey: "reception", x: 40, y: 320, width: 720, height: 140, color: 0x546e7a },
  ],
  furniture: {
    consulting: [
      { kind: "desk", x: 70, y: 130 },
      { kind: "chair", x: 70, y: 165 },
      { kind: "desk", x: 170, y: 130 },
      { kind: "chair", x: 170, y: 165 },
      { kind: "desk", x: 270, y: 130 },
      { kind: "chair", x: 270, y: 165 },
      { kind: "shelf", x: 340, y: 90 },
      { kind: "plant", x: 50, y: 80 },
    ],
    library: [
      { kind: "shelf", x: 50, y: 100 },
      { kind: "shelf", x: 120, y: 100 },
      { kind: "shelf", x: 50, y: 180 },
      { kind: "shelf", x: 120, y: 180 },
      { kind: "plant", x: 80, y: 60 },
    ],
    meeting: [
      { kind: "whiteboard", x: 70, y: 80 },
      { kind: "plant", x: 40, y: 120 },
      { kind: "plant", x: 110, y: 120 },
      { kind: "rug", x: 70, y: 150, w: 100, h: 60, color: 0x455a64 },
    ],
    reception: [
      { kind: "sofa", x: 100, y: 70 },
      { kind: "sofa", x: 230, y: 70 },
      { kind: "desk", x: 450, y: 80 },
      { kind: "chair", x: 450, y: 115 },
      { kind: "plant", x: 50, y: 90 },
      { kind: "plant", x: 670, y: 90 },
    ],
  },
  decorations: {
    consulting: [
      { kind: "window", x: 260, y: 5, w: 70, h: 40 },
      { kind: "painting", x: 140, y: 5, w: 40, h: 30 },
      { kind: "lamp", x: 20, y: 5 },
      { kind: "lamp", x: 320, y: 5 },
    ],
    library: [
      { kind: "window", x: 80, y: 5, w: 60, h: 35 },
      { kind: "lamp", x: 10, y: 5 },
    ],
    meeting: [
      { kind: "window", x: 60, y: 5, w: 50, h: 35 },
      { kind: "lamp", x: 10, y: 5 },
    ],
    reception: [
      { kind: "window", x: 200, y: 5, w: 100, h: 35 },
      { kind: "painting", x: 400, y: 5, w: 50, h: 30 },
      { kind: "lamp", x: 20, y: 5 },
      { kind: "lamp", x: 680, y: 5 },
    ],
  },
};

// ── 销售增长行业 — 销售工位 + 营销区 + 客户中心 + 会议室 ──

export const SALES_GROWTH_TEMPLATE: OfficeSceneTemplate = {
  slug: "sales_growth",
  displayNameKey: "sales_growth",
  canvasWidth: 800,
  canvasHeight: 500,
  defaultRoomId: "sales",
  rooms: [
    { id: "sales", nameKey: "sales", x: 40, y: 60, width: 360, height: 220, color: 0x880e4f },
    { id: "marketing", nameKey: "marketing", x: 420, y: 60, width: 180, height: 140, color: 0xad1457 },
    { id: "service", nameKey: "service", x: 620, y: 60, width: 140, height: 140, color: 0xc2185b },
    { id: "meeting", nameKey: "meeting", x: 420, y: 220, width: 340, height: 160, color: 0xd81b60 },
    { id: "lounge", nameKey: "lounge", x: 40, y: 320, width: 360, height: 140, color: 0xec407a },
  ],
  furniture: {
    sales: [
      { kind: "desk", x: 70, y: 130 },
      { kind: "chair", x: 70, y: 165 },
      { kind: "desk", x: 170, y: 130 },
      { kind: "chair", x: 170, y: 165 },
      { kind: "desk", x: 270, y: 130 },
      { kind: "chair", x: 270, y: 165 },
      { kind: "plant", x: 50, y: 80 },
      { kind: "plant", x: 330, y: 220 },
    ],
    marketing: [
      { kind: "desk", x: 70, y: 80 },
      { kind: "chair", x: 70, y: 115 },
      { kind: "whiteboard", x: 90, y: 50 },
      { kind: "plant", x: 40, y: 50 },
    ],
    service: [
      { kind: "desk", x: 60, y: 80 },
      { kind: "chair", x: 60, y: 115 },
    ],
    meeting: [
      { kind: "whiteboard", x: 170, y: 80 },
      { kind: "plant", x: 50, y: 80 },
      { kind: "plant", x: 290, y: 80 },
      { kind: "rug", x: 170, y: 120, w: 180, h: 60, color: 0xd81b60 },
    ],
    lounge: [
      { kind: "sofa", x: 110, y: 70 },
      { kind: "sofa", x: 240, y: 70 },
      { kind: "plant", x: 60, y: 90 },
      { kind: "plant", x: 310, y: 90 },
    ],
  },
  decorations: {
    sales: [
      { kind: "window", x: 260, y: 5, w: 70, h: 40 },
      { kind: "painting", x: 140, y: 5, w: 40, h: 30 },
      { kind: "lamp", x: 20, y: 5 },
      { kind: "lamp", x: 320, y: 5 },
    ],
    marketing: [
      { kind: "window", x: 80, y: 5, w: 60, h: 30 },
      { kind: "lamp", x: 10, y: 5 },
    ],
    service: [
      { kind: "window", x: 60, y: 5, w: 50, h: 25 },
      { kind: "lamp", x: 10, y: 5 },
    ],
    meeting: [
      { kind: "window", x: 150, y: 5, w: 80, h: 35 },
      { kind: "painting", x: 260, y: 5, w: 40, h: 25 },
      { kind: "lamp", x: 10, y: 5 },
    ],
    lounge: [
      { kind: "window", x: 100, y: 5, w: 70, h: 30 },
      { kind: "painting", x: 230, y: 5, w: 50, h: 25 },
      { kind: "lamp", x: 20, y: 5 },
    ],
  },
};

export const SCENE_TEMPLATES: OfficeSceneTemplate[] = [
  DEFAULT_OFFICE_TEMPLATE,
  STARTUP_LOFT_TEMPLATE,
  FINANCE_INVEST_TEMPLATE,
  SOFTWARE_DEV_TEMPLATE,
  ACCOUNTING_TEMPLATE,
  AI_RESEARCH_TEMPLATE,
  CONTENT_MEDIA_TEMPLATE,
  ECOMMERCE_TEMPLATE,
  EDUCATION_TEMPLATE,
  INDUSTRY_CONSULTING_TEMPLATE,
  SALES_GROWTH_TEMPLATE,
];

export function registerSceneTemplate(template: OfficeSceneTemplate): void {
  if (!SCENE_TEMPLATES.some((t) => t.slug === template.slug)) {
    SCENE_TEMPLATES.push(template);
  }
}

export function resolveSceneTemplate(slug?: string): OfficeSceneTemplate {
  if (!slug) { return DEFAULT_OFFICE_TEMPLATE; }
  return SCENE_TEMPLATES.find((t) => t.slug === slug) ?? DEFAULT_OFFICE_TEMPLATE;
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
  const cols = Math.min(4, memberCount);
  const col = index % cols;
  const row = Math.floor(index / cols);
  const cellW = room.width / (cols + 1);
  const cellH = 50;
  return { x: room.x + cellW * (col + 1), y: room.y + 40 + row * cellH };
}
