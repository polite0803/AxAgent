// SPDX-License-Identifier: AGPL-3.0-only

/**
 * Phaser agent 精灵生成 — 纯 Graphics 绘制，无外部资源依赖。
 *
 * 每个 agent 用一个 Container 装载：
 *   - body（矩形 + 圆头，主色按状态变化）
 *   - label（agent_slug 文字）
 *   - statusDot（状态指示点）
 *
 * 动画通过 Container 属性驱动（在 OfficeScene.update 中根据 sprite.animation
 * 字段调整 y 偏移 / 颜色 / 旋转），不依赖 Phaser 动画系统。
 */

import Phaser from "phaser";

/** agent 精灵动画状态（与 types/office.ts AgentSpriteState.animation 对应） */
export type SpriteAnimation = "idle" | "walking" | "typing" | "celebrating";

/** agent 状态颜色映射（与 FleetMemberStatus 对齐） */
export const STATUS_COLORS: Record<string, number> = {
  idle: 0x52c41a, // 绿色：空闲
  busy: 0x1677ff, // 蓝色：忙碌
  paused: 0xfaad14, // 黄色：暂停
  error: 0xff4d4f, // 红色：错误
  offline: 0x8c8c8c, // 灰色：离线
};

const BODY_WIDTH = 24;
const BODY_HEIGHT = 32;
const HEAD_RADIUS = 10;

export interface AgentSprite {
  /** Phaser 容器（包含 body + label + statusDot） */
  container: Phaser.GameObjects.Container;
  /** 主体矩形（用于动画时改色） */
  body: Phaser.GameObjects.Rectangle;
  /** 头部圆形（用于动画时改色） */
  head: Phaser.GameObjects.Arc;
  /** 状态指示点（小圆点） */
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
 * 调用方负责把它加入 scene 的 children 列表（通过 scene.add.existing）。
 */
export function createAgentSprite(
  scene: Phaser.Scene,
  x: number,
  y: number,
  agentSlug: string,
  memberId: string,
  status: string = "idle",
): AgentSprite {
  const color = STATUS_COLORS[status] ?? STATUS_COLORS.idle;

  // 主体（矩形）— 锚点居中
  const body = scene.add.rectangle(0, 4, BODY_WIDTH, BODY_HEIGHT, color, 1);
  body.setStrokeStyle(1, 0x000000, 0.4);

  // 头部（圆形）
  const head = scene.add.circle(0, -BODY_HEIGHT / 2 - HEAD_RADIUS + 4, HEAD_RADIUS, color, 1);
  head.setStrokeStyle(1, 0x000000, 0.4);

  // 状态点（小圆，位于头部右上方）
  const statusDot = scene.add.circle(HEAD_RADIUS - 2, -BODY_HEIGHT / 2 - HEAD_RADIUS, 3, color, 1);
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
    walkPhase: 0,
    jumpPhase: 0,
  };
}

/** 更新精灵颜色（状态变化时调用） */
export function updateSpriteStatus(sprite: AgentSprite, status: string): void {
  const color = STATUS_COLORS[status] ?? STATUS_COLORS.idle;
  sprite.body.setFillStyle(color);
  sprite.head.setFillStyle(color);
  sprite.statusDot.setFillStyle(color);
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
