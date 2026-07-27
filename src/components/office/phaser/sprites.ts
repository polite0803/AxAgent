// SPDX-License-Identifier: AGPL-3.0-only

/**
 * Phaser agent 精灵生成 — 纯 Graphics 绘制，无外部资源依赖。
 *
 * 每个 agent 用一个 Container 装载：
 *   - body（矩形 + 圆头，主色按角色变化）
 *   - label（agent_slug 文字）
 *   - statusDot（状态指示点，颜色按状态变化）
 *
 * 双映射机制：
 *   - body/head 主体颜色由 ROLE_COLORS 决定（基于业务角色）
 *   - statusDot 状态点颜色由 STATUS_DOT_COLORS 决定（基于运行时状态）
 *   - 二者正交，主体色反映"是谁"，状态点反映"在干什么"
 *
 * 动画通过 Container 属性驱动（在 OfficeScene.update 中根据 sprite.animation
 * 字段调整 y 偏移 / 颜色 / 旋转），不依赖 Phaser 动画系统。
 */

import Phaser from "phaser";

/** agent 精灵动画状态（与 types/office.ts AgentSpriteState.animation 对应） */
export type SpriteAnimation = "idle" | "walking" | "typing" | "celebrating";

/**
 * 业务角色 → 主体颜色映射。
 *
 * 通过 agent_slug 或 role 关键词匹配业务角色，决定精灵主体的"皮肤色"。
 * 与 STATUS_DOT_COLORS 正交：角色决定"是谁"，状态决定"在干什么"。
 *
 * 颜色取自 Ant Design 6 色板，保持与前端主题一致。
 */
export const ROLE_COLORS: Record<string, number> = {
  // 投研类 — 蓝色系（沉稳分析）
  research: 0x1677ff,
  analyst: 0x1677ff,
  researcher: 0x1677ff,
  // 数据类 — 青色系（数据工程）
  data: 0x13c2c2,
  data_room: 0x13c2c2,
  // 策略类 — 洋红系（策略研发）
  strategy: 0xeb2f96,
  strategist: 0xeb2f96,
  // 交易类 — 红色系（执行交易）
  trading: 0xf5222d,
  trader: 0xf5222d,
  // 风控类 — 橙色系（风险监控）
  risk: 0xfa8c16,
  risk_manager: 0xfa8c16,
  // 会议/管理类 — 紫色系（决策）
  meeting: 0x722ed1,
  manager: 0x722ed1,
  ceo: 0x722ed1,
  // 默认（通用助手）— 绿色系
  default: 0x52c41a,
};

/**
 * 运行时状态 → 状态点颜色映射。
 *
 * 仅作用于 statusDot（小圆点），不影响主体颜色。
 * 与原 STATUS_COLORS 字段一致，保留旧名称以兼容现有引用。
 */
export const STATUS_COLORS: Record<string, number> = {
  idle: 0x52c41a, // 绿色：空闲
  busy: 0x1677ff, // 蓝色：忙碌
  paused: 0xfaad14, // 黄色：暂停
  error: 0xff4d4f, // 红色：错误
  offline: 0x8c8c8c, // 灰色：离线
};

/** 状态点颜色映射（与 STATUS_COLORS 同义，命名更准确） */
export const STATUS_DOT_COLORS: Record<string, number> = STATUS_COLORS;

const BODY_WIDTH = 24;
const BODY_HEIGHT = 32;
const HEAD_RADIUS = 10;

/**
 * 根据 agent_slug + role 推断业务角色颜色。
 *
 * 优先匹配 slug（业务标识更稳定），其次匹配 role 关键词。
 * 都不匹配时返回 default 绿色。
 */
export function resolveRoleColor(agentSlug?: string, role?: string): number {
  const slug = (agentSlug ?? "").toLowerCase();
  // 1. slug 直接命中
  if (slug && ROLE_COLORS[slug]) {
    return ROLE_COLORS[slug];
  }
  // 2. slug 包含角色关键词（如 "research_analyst" 含 "research"）
  if (slug) {
    for (const key of Object.keys(ROLE_COLORS)) {
      if (key !== "default" && slug.includes(key)) {
        return ROLE_COLORS[key];
      }
    }
  }
  // 3. role 关键词匹配（如 "投研员" / "trader"）
  const roleLower = (role ?? "").toLowerCase();
  if (roleLower) {
    for (const key of Object.keys(ROLE_COLORS)) {
      if (key !== "default" && roleLower.includes(key)) {
        return ROLE_COLORS[key];
      }
    }
    // 中文角色名匹配
    if (roleLower.includes("投研") || roleLower.includes("研究员")) { return ROLE_COLORS.research; }
    if (roleLower.includes("数据")) { return ROLE_COLORS.data; }
    if (roleLower.includes("策略")) { return ROLE_COLORS.strategy; }
    if (roleLower.includes("交易") || roleLower.includes("交易员")) { return ROLE_COLORS.trading; }
    if (roleLower.includes("风控")) { return ROLE_COLORS.risk; }
    if (roleLower.includes("经理") || roleLower.includes("管理")) { return ROLE_COLORS.manager; }
  }
  return ROLE_COLORS.default;
}

export interface AgentSprite {
  /** Phaser 容器（包含 body + label + statusDot） */
  container: Phaser.GameObjects.Container;
  /** 主体矩形（颜色由角色决定，状态变化时不改色） */
  body: Phaser.GameObjects.Rectangle;
  /** 头部圆形（颜色由角色决定，状态变化时不改色） */
  head: Phaser.GameObjects.Arc;
  /** 状态指示点（小圆点，颜色由运行时状态决定） */
  statusDot: Phaser.GameObjects.Arc;
  /** 文字标签（显示 agent_slug） */
  label: Phaser.GameObjects.Text;
  /** 当前动画状态 */
  animation: SpriteAnimation;
  /** 朝向 */
  facing: "left" | "right";
  /** 当前房间 ID */
  roomId: string;
  /** 关联的成员 ID */
  memberId: string;
  /** 关联的 agent slug */
  agentSlug: string;
  /** 角色描述（用于角色色推断，状态变化时不变） */
  role: string;
  /** 目标坐标（行走动画的目标点） */
  targetX?: number;
  targetY?: number;
  /** 走路动画相位（用于 sin 摆动） */
  walkPhase: number;
  /** 跳跃动画相位（celebrating 用） */
  jumpPhase: number;
}

/**
 * 创建一个 agent 精灵。
 *
 * @param scene Phaser 场景
 * @param x 初始 x 坐标
 * @param y 初始 y 坐标
 * @param agentSlug 业务标识（用于角色色推断 + 标签显示）
 * @param memberId 成员 ID
 * @param status 初始状态（仅作用于 statusDot）
 * @param role 角色描述（用于角色色推断，可选）
 *
 * 调用方负责把它加入 scene 的 children 列表（通过 scene.add.existing）。
 */
export function createAgentSprite(
  scene: Phaser.Scene,
  x: number,
  y: number,
  agentSlug: string,
  memberId: string,
  status: string = "idle",
  role: string = "",
): AgentSprite {
  // 主体颜色由角色决定（双映射：角色色 + 状态点色）
  const roleColor = resolveRoleColor(agentSlug, role);
  const dotColor = STATUS_DOT_COLORS[status] ?? STATUS_DOT_COLORS.idle;

  // 主体（矩形）— 锚点居中，颜色由角色决定
  const body = scene.add.rectangle(0, 4, BODY_WIDTH, BODY_HEIGHT, roleColor, 1);
  body.setStrokeStyle(1, 0x000000, 0.4);

  // 头部（圆形），颜色由角色决定
  const head = scene.add.circle(0, -BODY_HEIGHT / 2 - HEAD_RADIUS + 4, HEAD_RADIUS, roleColor, 1);
  head.setStrokeStyle(1, 0x000000, 0.4);

  // 状态点（小圆，位于头部右上方），颜色由运行时状态决定
  const statusDot = scene.add.circle(HEAD_RADIUS - 2, -BODY_HEIGHT / 2 - HEAD_RADIUS, 3, dotColor, 1);
  statusDot.setStrokeStyle(1, 0xffffff, 0.8);

  // 文字标签（显示在角色下方）
  const label = scene.add.text(0, BODY_HEIGHT / 2 + 8, agentSlug, {
    fontFamily: "monospace",
    fontSize: "10px",
    color: "#000000",
    backgroundColor: "rgba(255,255,255,0.75)",
    padding: { x: 4, y: 2 },
  }).setOrigin(0.5);

  // 容器组装
  const container = scene.add.container(x, y, [body, head, statusDot, label]);

  return {
    container,
    body,
    head,
    statusDot,
    label,
    animation: "idle",
    facing: "right",
    roomId: "",
    memberId,
    agentSlug,
    role,
    walkPhase: 0,
    jumpPhase: 0,
  };
}

/**
 * 更新精灵状态（仅作用于 statusDot，不影响主体角色色）。
 *
 * 双映射机制：主体色由角色决定（创建时固定），状态点色由状态决定（运行时变化）。
 */
export function updateSpriteStatus(sprite: AgentSprite, status: string): void {
  const dotColor = STATUS_DOT_COLORS[status] ?? STATUS_DOT_COLORS.idle;
  sprite.statusDot.setFillStyle(dotColor);
}

/** 更新精灵动画状态 */
export function setSpriteAnimation(sprite: AgentSprite, anim: SpriteAnimation): void {
  sprite.animation = anim;
  if (anim === "walking") {
    sprite.walkPhase = 0;
  }
  if (anim === "celebrating") {
    sprite.jumpPhase = 0;
  }
}

/**
 * 在 scene.update 中调用 — 驱动单帧动画。
 *
 * - idle: 轻微上下浮动（呼吸感）
 * - walking: 沿 walkPhase 横向摆动 + 朝向翻转
 * - typing: 头部快速小幅震动
 * - celebrating: 整体跳跃
 */
export function tickSpriteAnimation(sprite: AgentSprite, time: number, delta: number): void {
  const dt = delta / 1000;
  const baseY = sprite.container.y;

  switch (sprite.animation) {
    case "idle": {
      // 用 time 而非累加相位，避免暂停后跳变
      const bob = Math.sin(time / 600) * 1.5;
      sprite.container.y = sprite.container.y + (bob - (sprite.container.y - baseY)) * 0.1;
      break;
    }
    case "walking": {
      sprite.walkPhase += dt * 6;
      const sway = Math.sin(sprite.walkPhase) * 2;
      // 头部微微左右晃
      sprite.head.x = sway;
      break;
    }
    case "typing": {
      const shake = Math.sin(time / 50) * 0.8;
      sprite.head.y = -BODY_HEIGHT / 2 - HEAD_RADIUS + 4 + shake;
      break;
    }
    case "celebrating": {
      sprite.jumpPhase += dt * 4;
      const jump = Math.abs(Math.sin(sprite.jumpPhase)) * -12;
      sprite.container.y = sprite.container.y + (jump - (sprite.container.y - baseY)) * 0.3;
      break;
    }
  }
}

/** 销毁精灵（成员被移除时调用） */
export function destroySprite(sprite: AgentSprite): void {
  sprite.container.destroy(true);
}
