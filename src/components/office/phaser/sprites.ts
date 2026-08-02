// SPDX-License-Identifier: AGPL-3.0-only

/**
 * Phaser agent 精灵 — 像素小人。
 *
 * 像素基础单位 PX=3，角色总尺寸约 21×48。
 *
 * 部件层级（从下到上）：
 *   ┌─────────────────────────┐
 *   │ ▓▓▓▓▓▓▓  ← 头发(造型)    │
 *   │ ████ ██  ← 脸(眼+嘴)     │
 *   │ ░░░░░░░  ← 衣领          │
 *   │█░░  ░░█  ← 手臂+身体     │
 *   │█░░  ░░█  ← (衣纹/领带)   │
 *   │ ░░  ░░  ← 腿            │
 *   │ ▀▀  ▀▀  ← 脚            │
 *   └─────────────────────────┘
 *     ○ ← 脚下阴影椭圆
 *     ◎ ← 状态光环(busy/error 时显示)
 *
 * 每个 agent 的衣服色 / 头发色 / 发型由 slug 哈希决定，稳定不变。
 */

import Phaser from "phaser";

// ── 类型 ──────────────────────────────────────────────

export type SpriteAnimation = "idle" | "walking" | "typing" | "celebrating";

export const STATUS_COLORS: Record<string, number> = {
  idle: 0x52c41a,
  busy: 0x1677ff,
  paused: 0xfaad14,
  error: 0xff4d4f,
  offline: 0x8c8c8c,
};

export interface AgentSprite {
  container: Phaser.GameObjects.Container;
  // 动画部件
  legL: Phaser.GameObjects.Rectangle;
  legR: Phaser.GameObjects.Rectangle;
  footL: Phaser.GameObjects.Rectangle;
  footR: Phaser.GameObjects.Rectangle;
  armL: Phaser.GameObjects.Rectangle;
  armR: Phaser.GameObjects.Rectangle;
  body: Phaser.GameObjects.Rectangle;
  // 静态部件（动画时整体微调）
  head: Phaser.GameObjects.Rectangle;
  hairTop: Phaser.GameObjects.Rectangle;
  hairSideL: Phaser.GameObjects.Rectangle;
  hairSideR: Phaser.GameObjects.Rectangle;
  eyeL: Phaser.GameObjects.Rectangle;
  eyeR: Phaser.GameObjects.Rectangle;
  eyeGleamL: Phaser.GameObjects.Rectangle;
  eyeGleamR: Phaser.GameObjects.Rectangle;
  mouth: Phaser.GameObjects.Rectangle;
  collar: Phaser.GameObjects.Rectangle;
  tie: Phaser.GameObjects.Rectangle;
  belt: Phaser.GameObjects.Rectangle;
  // 特效
  statusDot: Phaser.GameObjects.Rectangle;
  statusRing: Phaser.GameObjects.Arc;
  shadow: Phaser.GameObjects.Ellipse;
  label: Phaser.GameObjects.Text;
  // 状态
  animation: SpriteAnimation;
  facing: "left" | "right";
  roomId: string;
  memberId: string;
  agentSlug: string;
  /** 当前站立/行走落点 Y（动画整体位移的基准，创建/移动后更新） */
  baseY: number;
  targetX?: number;
  targetY?: number;
  walkPhase: number;
  jumpPhase: number;
  armPhase: number;
  // 染色缓存
  outfitColor: number;
  hairColor: number;
}

// ── 像素常量 ──────────────────────────────────────────

const PX = 3;
const SPRITE_W = 21;
const SPRITE_H = 48;

// ── 颜色工具 ──────────────────────────────────────────

function hashStr(s: string): number {
  let h = 0;
  for (let i = 0; i < s.length; i++) {
    h = (h * 31 + s.charCodeAt(i)) | 0;
  }
  return Math.abs(h);
}

function hslToHex(h: number, s: number, l: number): number {
  const c = (1 - Math.abs(2 * l - 1)) * s;
  const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
  const m = l - c / 2;
  let r = 0, g = 0, b = 0;
  if (h < 60) {
    r = c;
    g = x;
  } else if (h < 120) {
    r = x;
    g = c;
  } else if (h < 180) {
    g = c;
    b = x;
  } else if (h < 240) {
    g = x;
    b = c;
  } else if (h < 300) {
    r = x;
    b = c;
  } else {
    r = c;
    b = x;
  }
  return (Math.round((r + m) * 255) << 16) | (Math.round((g + m) * 255) << 8) | Math.round((b + m) * 255);
}

function pickOutfitColor(slug: string): number {
  return hslToHex(hashStr(slug) % 360, 0.55, 0.5);
}

function pickHairColor(slug: string): number {
  return hslToHex(hashStr(slug + ":h") % 360, 0.45, 0.3);
}

function pickSkinTone(slug: string): number {
  const tones = [0xf5d0a9, 0xe8b88a, 0xd4a373, 0xc68b6c, 0x8d5524];
  return tones[hashStr(slug + ":s") % tones.length];
}

// ── 创建精灵 ──────────────────────────────────────────

export function createAgentSprite(
  scene: Phaser.Scene,
  x: number,
  y: number,
  agentSlug: string,
  memberId: string,
  status: string = "idle",
): AgentSprite {
  const statusColor = STATUS_COLORS[status] ?? STATUS_COLORS.idle;
  const outfit = pickOutfitColor(agentSlug);
  const hair = pickHairColor(agentSlug);
  const skin = pickSkinTone(agentSlug);
  const outline = 0x1a1a2e;
  const pantsColor = 0x2d2d44;
  const shoeColor = 0x1a1a2e;

  // ── 阴影（最底层）──
  const shadow = scene.add.ellipse(0, 2, 24, 6, 0x000000, 0.25);

  // ── 脚 ──
  const footL = scene.add.rectangle(-PX * 1.5, -PX * 0.5, PX * 1.2, PX * 0.6, shoeColor, 1);
  footL.setStrokeStyle(1, outline, 0.8);
  const footR = scene.add.rectangle(PX * 1.5, -PX * 0.5, PX * 1.2, PX * 0.6, shoeColor, 1);
  footR.setStrokeStyle(1, outline, 0.8);

  // ── 腿 ──
  const legL = scene.add.rectangle(-PX * 1.5, -PX * 3.5, PX * 1.2, PX * 3, pantsColor, 1);
  legL.setStrokeStyle(1, outline, 0.7);
  const legR = scene.add.rectangle(PX * 1.5, -PX * 3.5, PX * 1.2, PX * 3, pantsColor, 1);
  legR.setStrokeStyle(1, outline, 0.7);

  // ── 鞋面装饰线 ──
  const shoeLineL = scene.add.rectangle(-PX * 1.5, -PX * 0.5, PX * 0.8, PX * 0.2, 0xffffff, 0.3);
  const shoeLineR = scene.add.rectangle(PX * 1.5, -PX * 0.5, PX * 0.8, PX * 0.2, 0xffffff, 0.3);

  // ── 身体（衣服） ──
  const body = scene.add.rectangle(0, -PX * 8.5, PX * 4, PX * 5, outfit, 1);
  body.setStrokeStyle(1, outline, 0.9);

  // ── 腰带 ──
  const belt = scene.add.rectangle(0, -PX * 6.2, PX * 4, PX * 0.5, 0x1a1a2e, 1);

  // ── 领带/条纹（根据 hash 决定） ──
  const hasTie = hashStr(agentSlug + ":tie") % 3 === 0;
  const tie = hasTie
    ? scene.add.rectangle(0, -PX * 8.5, PX * 0.8, PX * 4, hslToHex((hashStr(agentSlug) + 180) % 360, 0.6, 0.4), 1)
    : scene.add.rectangle(0, -PX * 10, PX * 3, PX * 0.4, 0xffffff, 0.25);

  // ── 衣领 ──
  const collar = scene.add.rectangle(0, -PX * 10.5, PX * 3, PX * 0.6, 0xffffff, 0.4);

  // ── 手臂 ──
  const armL = scene.add.rectangle(-PX * 2.8, -PX * 8.5, PX * 1, PX * 4, outfit, 1);
  armL.setStrokeStyle(1, outline, 0.8);
  const armR = scene.add.rectangle(PX * 2.8, -PX * 8.5, PX * 1, PX * 4, outfit, 1);
  armR.setStrokeStyle(1, outline, 0.8);

  // ── 手（手臂末端肤色小方块） ──
  const handL = scene.add.rectangle(-PX * 2.8, -PX * 6.3, PX * 1, PX * 0.8, skin, 1);
  handL.setStrokeStyle(1, outline, 0.6);
  const handR = scene.add.rectangle(PX * 2.8, -PX * 6.3, PX * 1, PX * 0.8, skin, 1);
  handR.setStrokeStyle(1, outline, 0.6);

  // ── 脸 ──
  const head = scene.add.rectangle(0, -PX * 12.5, PX * 3.5, PX * 3, skin, 1);
  head.setStrokeStyle(1, outline, 0.9);

  // ── 眼睛 ──
  const eyeL = scene.add.rectangle(-PX * 0.8, -PX * 12.5, PX * 0.5, PX * 0.6, outline, 1);
  const eyeR = scene.add.rectangle(PX * 0.8, -PX * 12.5, PX * 0.5, PX * 0.6, outline, 1);
  // 眼睛高光
  const eyeGleamL = scene.add.rectangle(-PX * 0.7, -PX * 12.7, PX * 0.2, PX * 0.2, 0xffffff, 0.9);
  const eyeGleamR = scene.add.rectangle(PX * 0.9, -PX * 12.7, PX * 0.2, PX * 0.2, 0xffffff, 0.9);

  // ── 嘴 ──
  const mouth = scene.add.rectangle(0, -PX * 11.3, PX * 0.8, PX * 0.2, 0x8b4513, 0.8);

  // ── 头发 ──
  // 顶部
  const hairTop = scene.add.rectangle(0, -PX * 14.5, PX * 4, PX * 1.5, hair, 1);
  hairTop.setStrokeStyle(1, outline, 0.7);
  // 左刘海
  const hairSideL = scene.add.rectangle(-PX * 2, -PX * 13.5, PX * 0.6, PX * 1.2, hair, 1);
  // 右刘海
  const hairSideR = scene.add.rectangle(PX * 2, -PX * 13.5, PX * 0.6, PX * 1.2, hair, 1);

  // ── 状态光环（头顶光晕）──
  const statusRing = scene.add.circle(0, -PX * 14.5, 10, statusColor, 0.15);
  statusRing.setStrokeStyle(2, statusColor, 0.4);
  statusRing.setVisible(status === "busy" || status === "error");

  // ── 状态点（头顶左上）──
  const statusDot = scene.add.rectangle(-PX * 2.5, -PX * 15, PX * 0.8, PX * 0.8, statusColor, 1);
  statusDot.setStrokeStyle(1, 0xffffff, 0.9);

  // ── 标签 ──
  const label = scene.add.text(0, PX * 2, agentSlug, {
    fontFamily: "monospace",
    fontSize: "10px",
    color: "#1a1a2e",
    backgroundColor: "rgba(255,255,255,0.9)",
    padding: { x: 4, y: 2 },
    resolution: 2,
  }).setOrigin(0.5);

  // ── 组装 ──
  const allParts = [
    shadow,
    footL,
    footR,
    legL,
    legR,
    shoeLineL,
    shoeLineR,
    body,
    belt,
    tie,
    collar,
    armL,
    armR,
    handL,
    handR,
    head,
    eyeL,
    eyeR,
    eyeGleamL,
    eyeGleamR,
    mouth,
    hairTop,
    hairSideL,
    hairSideR,
    statusRing,
    statusDot,
    label,
  ];
  const container = scene.add.container(x, y, allParts);

  return {
    container,
    legL,
    legR,
    footL,
    footR,
    armL,
    armR,
    body,
    head,
    hairTop,
    hairSideL,
    hairSideR,
    eyeL,
    eyeR,
    eyeGleamL,
    eyeGleamR,
    mouth,
    collar,
    tie,
    belt,
    statusDot,
    statusRing,
    shadow,
    label,
    animation: "idle",
    facing: "right",
    roomId: "",
    memberId,
    agentSlug,
    baseY: y,
    walkPhase: 0,
    jumpPhase: 0,
    armPhase: 0,
    outfitColor: outfit,
    hairColor: hair,
  };
}

// ── 状态更新 ──────────────────────────────────────────

export function updateSpriteStatus(sprite: AgentSprite, status: string): void {
  const color = STATUS_COLORS[status] ?? STATUS_COLORS.idle;
  sprite.statusDot.setFillStyle(color);
  sprite.statusRing.setFillStyle(color, 0.15);
  sprite.statusRing.setStrokeStyle(2, color, 0.4);
  // busy / error 时显示头顶光环
  const showRing = status === "busy" || status === "error";
  sprite.statusRing.setVisible(showRing);
}

export function setSpriteAnimation(sprite: AgentSprite, anim: SpriteAnimation): void {
  sprite.animation = anim;
  if (anim === "walking") {
    sprite.walkPhase = 0;
    sprite.armPhase = 0;
  }
  if (anim === "celebrating") {
    sprite.jumpPhase = 0;
  }
}

// ── 动画驱动 ──────────────────────────────────────────

export function tickSpriteAnimation(sprite: AgentSprite, time: number, delta: number): void {
  const dt = delta / 1000;

  switch (sprite.animation) {
    case "idle": {
      // 轻微呼吸：以 baseY 为基准整体上下 1px（此前 (y-y) 恒 0 为死代码）
      const bob = Math.sin(time / 700) * 0.8;
      sprite.container.y = sprite.baseY + bob;
      // 复位手臂和腿
      sprite.armL.y = -PX * 8.5;
      sprite.armR.y = -PX * 8.5;
      sprite.legL.y = -PX * 3.5;
      sprite.legR.y = -PX * 3.5;
      sprite.footL.y = -PX * 0.5;
      sprite.footR.y = -PX * 0.5;
      // 阴影呼吸
      const shadowScale = 1 + Math.sin(time / 700) * 0.05;
      sprite.shadow.setScale(shadowScale, 1);
      // 状态光环呼吸
      if (sprite.statusRing.visible) {
        const ringScale = 1 + Math.sin(time / 400) * 0.1;
        sprite.statusRing.setScale(ringScale);
        sprite.statusRing.setAlpha(0.3 + Math.sin(time / 400) * 0.15);
      }
      break;
    }
    case "walking": {
      sprite.walkPhase += dt * 9;
      sprite.armPhase += dt * 9;
      // 左右腿交替上下
      const legLift = Math.sin(sprite.walkPhase);
      sprite.legL.y = -PX * 3.5 - Math.max(0, legLift) * 2;
      sprite.legR.y = -PX * 3.5 - Math.max(0, -legLift) * 2;
      sprite.footL.y = -PX * 0.5 - Math.max(0, legLift) * 2;
      sprite.footR.y = -PX * 0.5 - Math.max(0, -legLift) * 2;
      // 手臂前后摆动（与腿反向）
      const armSwing = Math.sin(sprite.armPhase);
      sprite.armL.x = -PX * 2.8 + armSwing * 1.5;
      sprite.armR.x = PX * 2.8 - armSwing * 1.5;
      sprite.armL.y = -PX * 8.5 - Math.max(0, -armSwing) * 1;
      sprite.armR.y = -PX * 8.5 - Math.max(0, armSwing) * 1;
      // 整体上下弹动（以 baseY 为基准，修复死代码）
      const bob = Math.abs(Math.sin(sprite.walkPhase * 2)) * 1.5;
      sprite.container.y = sprite.baseY - bob;
      // 头部微微左右晃
      const headSway = Math.sin(sprite.walkPhase) * 0.5;
      sprite.head.x = headSway;
      sprite.hairTop.x = headSway;
      sprite.eyeL.x = -PX * 0.8 + headSway;
      sprite.eyeR.x = PX * 0.8 + headSway;
      sprite.eyeGleamL.x = -PX * 0.7 + headSway;
      sprite.eyeGleamR.x = PX * 0.9 + headSway;
      sprite.mouth.x = headSway;
      sprite.hairSideL.x = -PX * 2 + headSway;
      sprite.hairSideR.x = PX * 2 + headSway;
      break;
    }
    case "typing": {
      // 头部快速小幅度震动
      const shake = Math.sin(time / 40) * 0.5;
      sprite.head.y = -PX * 12.5 + shake;
      sprite.eyeL.y = -PX * 12.5 + shake;
      sprite.eyeR.y = -PX * 12.5 + shake;
      sprite.eyeGleamL.y = -PX * 12.7 + shake;
      sprite.eyeGleamR.y = -PX * 12.7 + shake;
      sprite.mouth.y = -PX * 11.3 + shake;
      sprite.hairTop.y = -PX * 14.5 + shake;
      sprite.hairSideL.y = -PX * 13.5 + shake;
      sprite.hairSideR.y = -PX * 13.5 + shake;
      // 手臂快速上下（打字动作）
      const armShake = Math.sin(time / 60) * 1;
      sprite.armR.y = -PX * 8.5 + armShake;
      // 屏幕光闪烁感（通过状态点闪烁）
      const flicker = 0.7 + Math.sin(time / 80) * 0.3;
      sprite.statusDot.setAlpha(flicker);
      // 光环
      if (sprite.statusRing.visible) {
        const ringScale = 1 + Math.sin(time / 300) * 0.08;
        sprite.statusRing.setScale(ringScale);
      }
      break;
    }
    case "celebrating": {
      sprite.jumpPhase += dt * 6;
      // 整体跳跃（以 baseY 为基准，修复死代码）
      const jump = Math.abs(Math.sin(sprite.jumpPhase)) * -18;
      sprite.container.y = sprite.baseY + jump;
      // 跳跃时手臂上举
      const armRaise = Math.abs(Math.sin(sprite.jumpPhase)) * 3;
      sprite.armL.y = -PX * 8.5 - armRaise;
      sprite.armR.y = -PX * 8.5 - armRaise;
      sprite.armL.x = -PX * 2.8 - armRaise * 0.5;
      sprite.armR.x = PX * 2.8 + armRaise * 0.5;
      // 阴影缩小（跳起来时）
      const shadowScale = 1 - Math.abs(Math.sin(sprite.jumpPhase)) * 0.3;
      sprite.shadow.setScale(shadowScale, 1);
      // 光环放大
      if (sprite.statusRing.visible) {
        const ringScale = 1 + Math.abs(Math.sin(sprite.jumpPhase)) * 0.3;
        sprite.statusRing.setScale(ringScale);
        sprite.statusRing.setAlpha(0.4 + Math.sin(sprite.jumpPhase) * 0.2);
      }
      break;
    }
  }
}

// ── 朝向 ──────────────────────────────────────────────

export function setSpriteFacing(sprite: AgentSprite, facing: "left" | "right"): void {
  if (sprite.facing === facing) { return; }
  sprite.facing = facing;
  const sx = facing === "left" ? -1 : 1;
  sprite.container.setScale(sx, 1);
  // 标签和状态点不翻转
  sprite.label.setScale(sx, 1);
  sprite.statusDot.setScale(sx, 1);
  sprite.statusRing.setScale(sx, 1);
  sprite.shadow.setScale(sx, 1);
}

// ── 销毁 ──────────────────────────────────────────────

export function destroySprite(sprite: AgentSprite): void {
  sprite.container.destroy(true);
}

// ── 尺寸常量 ──────────────────────────────────────────

export const SPRITE_TOTAL_W = SPRITE_W;
export const SPRITE_TOTAL_H = SPRITE_H;
