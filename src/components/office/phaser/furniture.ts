// SPDX-License-Identifier: AGPL-3.0-only

/**
 * 像素家具绘制 — 纯 Graphics 绘制，无外部资源依赖。
 *
 * 每种家具由多个像素块组合，带描边/阴影/高光，看起来像真正的像素艺术品。
 *
 * 家具清单：
 *   - desk    带显示器+键盘+鼠标+便签的办公桌
 *   - chair   带靠背+扶手的办公椅
 *   - plant   带花盆+多层叶子+茎的绿植
 *   - whiteboard  带边框+板面文字+磁贴的白板
 *   - shelf   带框架+多层+多色书+装饰品的书架
 *   - sofa    带底座+靠背+扶手+靠垫+腿的沙发
 *   - rug     地毯（带花纹）
 *   - window  窗户（带窗外景色+阳光）
 *   - painting 墙画（带画框+抽象画内容）
 *   - lamp    吊灯（带灯泡+光晕+绳线）
 */

import Phaser from "phaser";
import type { RoomDecoration, RoomFurniture, RoomRect } from "./sceneTemplates";

// ── 颜色工具 ──────────────────────────────────────────

function lighten(color: number, t: number): number {
  const r = (color >> 16) & 0xff;
  const g = (color >> 8) & 0xff;
  const b = color & 0xff;
  return (Math.round(r + (255 - r) * t) << 16) | (Math.round(g + (255 - g) * t) << 8) | Math.round(b + (255 - b) * t);
}

function darken(color: number, t: number): number {
  const r = (color >> 16) & 0xff;
  const g = (color >> 8) & 0xff;
  const b = color & 0xff;
  return (Math.round(r * (1 - t)) << 16) | (Math.round(g * (1 - t)) << 8) | Math.round(b * (1 - t));
}

// ── 桌子 ──────────────────────────────────────────────

export function drawDesk(scene: Phaser.Scene, cx: number, cy: number): void {
  const woodLight = 0x8b5a2b;
  const woodDark = 0x6b4423;
  const woodLighter = lighten(woodLight, 0.2);
  const outline = 0x2a1a0a;

  // 阴影
  scene.add.rectangle(cx + 3, cy + 14, 56, 6, 0x000000, 0.2);

  // 桌腿（4 条）
  const legW = 3;
  const legH = 10;
  scene.add.rectangle(cx - 22, cy + 8, legW, legH, woodDark, 1).setStrokeStyle(1, outline, 0.8);
  scene.add.rectangle(cx + 22, cy + 8, legW, legH, woodDark, 1).setStrokeStyle(1, outline, 0.8);

  // 桌面
  const desk = scene.add.rectangle(cx, cy + 2, 56, 8, woodLight, 1);
  desk.setStrokeStyle(1, outline, 1);
  // 桌面高光（左上角亮线）
  scene.add.rectangle(cx, cy - 1, 54, 1, woodLighter, 0.8);
  // 桌面木纹
  scene.add.rectangle(cx - 10, cy + 3, 16, 0.5, woodDark, 0.4);
  scene.add.rectangle(cx + 8, cy + 4, 12, 0.5, woodDark, 0.3);

  // 显示器
  const monBezel = scene.add.rectangle(cx, cy - 12, 32, 20, 0x1a1a2e, 1);
  monBezel.setStrokeStyle(1, 0x000000, 1);
  // 屏幕
  scene.add.rectangle(cx, cy - 12, 26, 15, 0x0d1b2a, 1);
  // 屏幕内容（蓝色窗口 + 代码行）
  scene.add.rectangle(cx - 4, cy - 14, 8, 3, 0x1b9aaa, 0.8);
  scene.add.rectangle(cx + 6, cy - 14, 4, 3, 0xe63946, 0.6);
  scene.add.rectangle(cx - 8, cy - 10, 6, 1, 0x4ade80, 0.7);
  scene.add.rectangle(cx - 8, cy - 8, 10, 1, 0x60a5fa, 0.7);
  scene.add.rectangle(cx - 8, cy - 6, 4, 1, 0xfbbf24, 0.6);
  // 屏幕底座
  scene.add.rectangle(cx, cy - 2, 8, 3, 0x1a1a2e, 1);
  scene.add.rectangle(cx, cy, 14, 2, 0x2a2a3e, 1).setStrokeStyle(1, outline, 0.8);

  // 键盘
  const kb = scene.add.rectangle(cx - 2, cy + 5, 20, 5, 0x2a2a3e, 1);
  kb.setStrokeStyle(1, outline, 0.9);
  // 键盘按键纹理
  for (let i = 0; i < 5; i++) {
    scene.add.rectangle(cx - 10 + i * 4, cy + 4, 2, 1, 0x4a4a5e, 0.6);
    scene.add.rectangle(cx - 10 + i * 4, cy + 6, 2, 1, 0x4a4a5e, 0.6);
  }

  // 鼠标
  scene.add.rectangle(cx + 14, cy + 5, 4, 6, 0x2a2a3e, 1).setStrokeStyle(1, outline, 0.8);
  scene.add.rectangle(cx + 14, cy + 4, 1, 1, 0x4a4a5e, 0.6);

  // 便签（黄色小方块）
  scene.add.rectangle(cx - 22, cy - 2, 6, 6, 0xfbbf24, 0.9).setStrokeStyle(1, 0xd4a017, 0.6);
  scene.add.rectangle(cx - 23, cy - 3, 2, 1, 0xd4a017, 0.4);

  // 咖啡杯
  scene.add.rectangle(cx + 20, cy - 2, 5, 6, 0xf5f5f5, 1).setStrokeStyle(1, outline, 0.7);
  scene.add.rectangle(cx + 20, cy - 4, 3, 1, 0x6b4423, 0.8); // 咖啡
  scene.add.rectangle(cx + 23, cy - 1, 1, 3, 0xf5f5f5, 0.8); // 把手
}

// ── 椅子 ──────────────────────────────────────────────

export function drawChair(scene: Phaser.Scene, cx: number, cy: number): void {
  const frame = 0x2a2a3e;
  const seat = 0x3a3a4e;
  const outline = 0x1a1a2e;

  // 阴影
  scene.add.rectangle(cx + 2, cy + 10, 20, 4, 0x000000, 0.2);

  // 椅腿
  scene.add.rectangle(cx - 6, cy + 6, 2, 6, frame, 1).setStrokeStyle(1, outline, 0.7);
  scene.add.rectangle(cx + 6, cy + 6, 2, 6, frame, 1).setStrokeStyle(1, outline, 0.7);

  // 座面
  const seatRect = scene.add.rectangle(cx, cy + 2, 18, 8, seat, 1);
  seatRect.setStrokeStyle(1, outline, 0.9);
  scene.add.rectangle(cx, cy, 16, 1, lighten(seat, 0.2), 0.5); // 高光

  // 靠背
  const back = scene.add.rectangle(cx, cy - 10, 16, 12, frame, 1);
  back.setStrokeStyle(1, outline, 0.9);
  // 靠背纹理
  scene.add.rectangle(cx, cy - 12, 14, 1, lighten(frame, 0.15), 0.4);
  scene.add.rectangle(cx, cy - 8, 14, 1, lighten(frame, 0.1), 0.3);

  // 扶手
  scene.add.rectangle(cx - 10, cy - 2, 2, 6, frame, 1).setStrokeStyle(1, outline, 0.7);
  scene.add.rectangle(cx + 10, cy - 2, 2, 6, frame, 1).setStrokeStyle(1, outline, 0.7);
}

// ── 绿植 ──────────────────────────────────────────────

export function drawPlant(scene: Phaser.Scene, cx: number, cy: number): void {
  const potColor = 0x8b4513;
  const potRim = 0xa0562a;
  const leafDark = 0x1b6b2f;
  const leafMid = 0x2d8a3e;
  const leafLight = 0x3da44a;
  const leafBright = 0x5ec85e;
  const outline = 0x1a3a1a;

  // 阴影
  scene.add.ellipse(cx + 2, cy + 14, 16, 4, 0x000000, 0.2);

  // 花盆
  const pot = scene.add.rectangle(cx, cy + 8, 14, 10, potColor, 1);
  pot.setStrokeStyle(1, outline, 0.8);
  // 花盆上沿
  const rim = scene.add.rectangle(cx, cy + 4, 16, 3, potRim, 1);
  rim.setStrokeStyle(1, outline, 0.7);
  // 花盆纹理
  scene.add.rectangle(cx - 4, cy + 10, 1, 4, darken(potColor, 0.3), 0.5);
  scene.add.rectangle(cx + 3, cy + 9, 1, 5, darken(potColor, 0.3), 0.5);

  // 茎
  scene.add.rectangle(cx, cy, 1, 8, leafDark, 0.8);
  scene.add.rectangle(cx - 3, cy + 2, 1, 6, leafDark, 0.6);
  scene.add.rectangle(cx + 3, cy + 2, 1, 6, leafDark, 0.6);

  // 叶子（多层）
  // 底层（深色）
  const leaf1 = scene.add.rectangle(cx - 5, cy - 2, 8, 8, leafDark, 1);
  leaf1.setStrokeStyle(1, outline, 0.7);
  const leaf2 = scene.add.rectangle(cx + 5, cy - 2, 8, 8, leafDark, 1);
  leaf2.setStrokeStyle(1, outline, 0.7);
  // 中层
  const leaf3 = scene.add.rectangle(cx - 3, cy - 6, 8, 8, leafMid, 1);
  leaf3.setStrokeStyle(1, outline, 0.6);
  const leaf4 = scene.add.rectangle(cx + 3, cy - 6, 8, 8, leafMid, 1);
  leaf4.setStrokeStyle(1, outline, 0.6);
  // 上层
  const leaf5 = scene.add.rectangle(cx, cy - 9, 7, 7, leafLight, 1);
  leaf5.setStrokeStyle(1, outline, 0.5);
  // 顶部亮点
  scene.add.rectangle(cx - 1, cy - 11, 3, 3, leafBright, 0.8);
  // 叶子高光
  scene.add.rectangle(cx - 4, cy - 7, 2, 1, leafBright, 0.5);
  scene.add.rectangle(cx + 4, cy - 7, 2, 1, leafBright, 0.4);
}

// ── 白板 ──────────────────────────────────────────────

export function drawWhiteboard(scene: Phaser.Scene, cx: number, cy: number): void {
  const frameColor = 0x6b7280;
  const boardColor = 0xf8f9fa;
  const outline = 0x374151;

  // 阴影
  scene.add.rectangle(cx + 3, cy + 3, 72, 40, 0x000000, 0.2);

  // 边框
  const frame = scene.add.rectangle(cx, cy, 72, 40, frameColor, 1);
  frame.setStrokeStyle(1, outline, 1);
  // 板面
  scene.add.rectangle(cx, cy, 66, 34, boardColor, 1);

  // 板面内容（彩色文字/图表线条）
  // 标题
  scene.add.rectangle(cx - 24, cy - 12, 16, 2, 0x1b9aaa, 0.8);
  // 流程图框
  scene.add.rectangle(cx - 24, cy - 6, 10, 6, 0xe63946, 0.3).setStrokeStyle(1, 0xe63946, 0.6);
  scene.add.rectangle(cx - 8, cy - 6, 10, 6, 0x4ade80, 0.3).setStrokeStyle(1, 0x4ade80, 0.6);
  // 箭头线
  scene.add.rectangle(cx - 14, cy - 6, 4, 1, 0x374151, 0.6);
  // 文字行
  scene.add.rectangle(cx - 28, cy + 4, 24, 1, 0x60a5fa, 0.7);
  scene.add.rectangle(cx - 28, cy + 7, 20, 1, 0x60a5fa, 0.6);
  scene.add.rectangle(cx - 28, cy + 10, 22, 1, 0x60a5fa, 0.5);
  // 右侧图表
  scene.add.rectangle(cx + 16, cy + 4, 3, 6, 0xfbbf24, 0.7);
  scene.add.rectangle(cx + 22, cy + 2, 3, 8, 0x4ade80, 0.7);
  scene.add.rectangle(cx + 28, cy - 2, 3, 12, 0xe63946, 0.7);

  // 磁贴
  scene.add.rectangle(cx - 30, cy - 16, 4, 4, 0xe63946, 0.9).setStrokeStyle(1, outline, 0.5);
  scene.add.rectangle(cx + 26, cy - 16, 4, 4, 0x1b9aaa, 0.9).setStrokeStyle(1, outline, 0.5);

  // 底部笔槽
  scene.add.rectangle(cx, cy + 18, 66, 3, frameColor, 1).setStrokeStyle(1, outline, 0.8);
  // 笔
  scene.add.rectangle(cx - 20, cy + 18, 6, 2, 0x1b9aaa, 0.9);
  scene.add.rectangle(cx + 10, cy + 18, 6, 2, 0xe63946, 0.9);
}

// ── 书架 ──────────────────────────────────────────────

export function drawShelf(scene: Phaser.Scene, cx: number, cy: number): void {
  const woodColor = 0x6b4423;
  const woodDark = 0x4a2c12;
  const outline = 0x2a1a0a;

  // 阴影
  scene.add.rectangle(cx + 3, cy + 3, 64, 32, 0x000000, 0.2);

  // 框架
  const shelf = scene.add.rectangle(cx, cy, 64, 32, woodColor, 1);
  shelf.setStrokeStyle(1, outline, 1);
  // 背板（深色）
  scene.add.rectangle(cx, cy, 58, 26, woodDark, 1);
  // 隔板
  scene.add.rectangle(cx, cy - 6, 58, 2, woodColor, 1).setStrokeStyle(1, outline, 0.6);
  scene.add.rectangle(cx, cy + 8, 58, 2, woodColor, 1).setStrokeStyle(1, outline, 0.6);

  // 书（上层）— 不同色不同高度
  const bookColors = [
    0xe63946,
    0x1b9aaa,
    0x4ade80,
    0xfbbf24,
    0x722ed1,
    0xf97316,
    0x06b6d4,
    0xec4899,
  ];
  for (let i = 0; i < 8; i++) {
    const bx = cx - 26 + i * 7;
    const bh = 8 + (i % 3) * 2;
    const book = scene.add.rectangle(bx, cy - 10, 5, bh, bookColors[i % bookColors.length], 1);
    book.setStrokeStyle(1, outline, 0.6);
    // 书脊高光
    scene.add.rectangle(bx - 1, cy - 10, 1, bh - 2, lighten(bookColors[i % bookColors.length], 0.3), 0.5);
    // 书脊装饰线
    scene.add.rectangle(bx, cy - 13, 3, 0.5, 0xffd700, 0.4);
  }

  // 下层：装饰品 + 书
  for (let i = 0; i < 4; i++) {
    const bx = cx - 20 + i * 12;
    const bh = 6 + (i % 2) * 3;
    const book = scene.add.rectangle(bx, cy + 5, 5, bh, bookColors[(i + 3) % bookColors.length], 1);
    book.setStrokeStyle(1, outline, 0.5);
  }
  // 装饰品：小花瓶
  scene.add.rectangle(cx + 20, cy + 5, 4, 8, 0x60a5fa, 0.8).setStrokeStyle(1, outline, 0.5);
  scene.add.rectangle(cx + 20, cy + 1, 2, 2, 0xe63946, 0.7); // 花

  // 顶部装饰
  scene.add.rectangle(cx, cy - 15, 58, 1, lighten(woodColor, 0.2), 0.4);
}

// ── 沙发 ──────────────────────────────────────────────

export function drawSofa(scene: Phaser.Scene, cx: number, cy: number): void {
  const sofaColor = 0x8b3a3a;
  const sofaDark = 0x6b2a2a;
  const sofaLight = lighten(sofaColor, 0.15);
  const cushionColor = 0xc0a062;
  const cushionLight = lighten(cushionColor, 0.15);
  const outline = 0x3a1f1f;

  // 阴影
  scene.add.rectangle(cx + 3, cy + 12, 64, 6, 0x000000, 0.2);

  // 沙发腿
  scene.add.rectangle(cx - 26, cy + 8, 4, 6, 0x4a2c12, 1).setStrokeStyle(1, outline, 0.7);
  scene.add.rectangle(cx + 26, cy + 8, 4, 6, 0x4a2c12, 1).setStrokeStyle(1, outline, 0.7);

  // 底座
  const base = scene.add.rectangle(cx, cy + 2, 60, 12, sofaColor, 1);
  base.setStrokeStyle(1, outline, 1);
  scene.add.rectangle(cx, cy + 6, 58, 1, sofaLight, 0.3); // 高光

  // 靠背
  const back = scene.add.rectangle(cx, cy - 8, 60, 8, sofaDark, 1);
  back.setStrokeStyle(1, outline, 1);
  scene.add.rectangle(cx, cy - 10, 58, 1, sofaLight, 0.3);

  // 扶手
  const armL = scene.add.rectangle(cx - 28, cy - 2, 6, 16, sofaColor, 1);
  armL.setStrokeStyle(1, outline, 0.9);
  const armR = scene.add.rectangle(cx + 28, cy - 2, 6, 16, sofaColor, 1);
  armR.setStrokeStyle(1, outline, 0.9);
  // 扶手顶面高光
  scene.add.rectangle(cx - 28, cy - 9, 5, 1, sofaLight, 0.4);
  scene.add.rectangle(cx + 28, cy - 9, 5, 1, sofaLight, 0.4);

  // 靠垫
  const cushion1 = scene.add.rectangle(cx - 10, cy - 1, 16, 10, cushionColor, 1);
  cushion1.setStrokeStyle(1, outline, 0.8);
  scene.add.rectangle(cx - 10, cy - 4, 14, 1, cushionLight, 0.5);
  const cushion2 = scene.add.rectangle(cx + 10, cy - 1, 16, 10, cushionColor, 1);
  cushion2.setStrokeStyle(1, outline, 0.8);
  scene.add.rectangle(cx + 10, cy - 4, 14, 1, cushionLight, 0.5);
}

// ── 地毯 ──────────────────────────────────────────────

export function drawRug(scene: Phaser.Scene, cx: number, cy: number, w: number, h: number, color: number): void {
  const rugColor = color;
  const rugDark = darken(color, 0.25);
  const rugLight = lighten(color, 0.2);
  const outline = darken(color, 0.4);

  // 地毯主体（圆角矩形效果）
  const rug = scene.add.rectangle(cx, cy, w, h, rugColor, 0.8);
  rug.setStrokeStyle(2, outline, 0.4);

  // 边框纹
  scene.add.rectangle(cx, cy - h / 2 + 4, w - 8, 1, rugLight, 0.5);
  scene.add.rectangle(cx, cy + h / 2 - 4, w - 8, 1, rugLight, 0.5);
  scene.add.rectangle(cx - w / 2 + 4, cy, 1, h - 8, rugLight, 0.5);
  scene.add.rectangle(cx + w / 2 - 4, cy, 1, h - 8, rugLight, 0.5);

  // 中心花纹
  const cw = Math.min(w, h) * 0.3;
  scene.add.rectangle(cx, cy, cw, cw, rugDark, 0.4);
  scene.add.rectangle(cx, cy, cw * 0.5, cw * 0.5, rugLight, 0.3);

  // 四角装饰
  const cornerSize = 6;
  const cornerOffset = Math.min(w, h) * 0.25;
  scene.add.rectangle(cx - cornerOffset, cy - cornerOffset, cornerSize, cornerSize, rugDark, 0.3);
  scene.add.rectangle(cx + cornerOffset, cy - cornerOffset, cornerSize, cornerSize, rugDark, 0.3);
  scene.add.rectangle(cx - cornerOffset, cy + cornerOffset, cornerSize, cornerSize, rugDark, 0.3);
  scene.add.rectangle(cx + cornerOffset, cy + cornerOffset, cornerSize, cornerSize, rugDark, 0.3);
}

// ── 窗户 ──────────────────────────────────────────────

export function drawWindow(scene: Phaser.Scene, cx: number, cy: number, w: number, h: number): void {
  const frameColor = 0xd4c4a0;
  const frameDark = 0x8b7355;
  const skyTop = 0x4a90d9;
  const skyMid = 0x7ec0ee;
  const skyBottom = 0xa8d8f0;
  const cloudColor = 0xffffff;
  const outline = 0x4a3728;

  // 窗框外
  const outer = scene.add.rectangle(cx, cy, w + 6, h + 6, frameDark, 1);
  outer.setStrokeStyle(1, outline, 1);

  // 窗框
  const frame = scene.add.rectangle(cx, cy, w, h, frameColor, 1);
  frame.setStrokeStyle(1, outline, 0.8);

  // 天空渐变（三层模拟）
  scene.add.rectangle(cx, cy - h * 0.25, w - 4, h * 0.3, skyTop, 1);
  scene.add.rectangle(cx, cy, w - 4, h * 0.3, skyMid, 1);
  scene.add.rectangle(cx, cy + h * 0.25, w - 4, h * 0.3, skyBottom, 1);

  // 云朵
  scene.add.rectangle(cx - w * 0.2, cy - h * 0.15, 10, 4, cloudColor, 0.8);
  scene.add.rectangle(cx - w * 0.15, cy - h * 0.18, 6, 3, cloudColor, 0.7);
  scene.add.rectangle(cx + w * 0.2, cy - h * 0.05, 8, 3, cloudColor, 0.6);

  // 太阳
  scene.add.circle(cx + w * 0.25, cy - h * 0.2, 4, 0xfbbf24, 0.9);
  scene.add.circle(cx + w * 0.25, cy - h * 0.2, 6, 0xfbbf24, 0.2);

  // 窗户十字格
  scene.add.rectangle(cx, cy, 2, h - 4, frameColor, 1).setStrokeStyle(1, outline, 0.5);
  scene.add.rectangle(cx, cy, w - 4, 2, frameColor, 1).setStrokeStyle(1, outline, 0.5);

  // 窗台
  scene.add.rectangle(cx, cy + h / 2 + 2, w + 10, 3, frameDark, 1).setStrokeStyle(1, outline, 0.8);
}

// ── 墙画 ──────────────────────────────────────────────

export function drawPainting(scene: Phaser.Scene, cx: number, cy: number, w: number, h: number): void {
  const frameColor = 0xd4a017;
  const frameDark = 0x8b6914;
  const outline = 0x2a1a0a;

  // 阴影
  scene.add.rectangle(cx + 2, cy + 2, w + 4, h + 4, 0x000000, 0.2);

  // 画框
  const frame = scene.add.rectangle(cx, cy, w + 4, h + 4, frameColor, 1);
  frame.setStrokeStyle(1, outline, 1);
  scene.add.rectangle(cx, cy, w + 2, h + 2, frameDark, 1).setStrokeStyle(1, outline, 0.5);

  // 画布
  scene.add.rectangle(cx, cy, w, h, 0xf5f0e8, 1);

  // 抽象画内容（像素山水）
  // 天空
  scene.add.rectangle(cx, cy - h * 0.25, w - 2, h * 0.3, 0xa8d8f0, 0.8);
  // 山
  scene.add.rectangle(cx - w * 0.2, cy, w * 0.3, h * 0.3, 0x6b4423, 0.8);
  scene.add.rectangle(cx + w * 0.15, cy + h * 0.05, w * 0.25, h * 0.25, 0x4a2c12, 0.8);
  // 水面
  scene.add.rectangle(cx, cy + h * 0.3, w - 2, h * 0.15, 0x4a90d9, 0.7);
  // 小船
  scene.add.rectangle(cx - w * 0.15, cy + h * 0.3, 4, 1, 0x2a1a0a, 0.8);
}

// ── 吊灯 ──────────────────────────────────────────────

export function drawLamp(scene: Phaser.Scene, cx: number, cy: number): void {
  const cordColor = 0x2a2a2a;
  const shadeColor = 0xd4c4a0;
  const shadeDark = 0x8b7355;
  const bulbColor = 0xfbbf24;
  const outline = 0x2a1a0a;

  // 绳线
  scene.add.rectangle(cx, cy - 8, 1, 8, cordColor, 1);

  // 灯罩
  const shade = scene.add.rectangle(cx, cy, 16, 6, shadeColor, 1);
  shade.setStrokeStyle(1, outline, 0.8);
  scene.add.rectangle(cx, cy - 2, 14, 1, shadeDark, 0.6);
  // 灯罩底
  scene.add.rectangle(cx, cy + 3, 16, 2, shadeDark, 1).setStrokeStyle(1, outline, 0.7);

  // 灯泡（发光）
  scene.add.circle(cx, cy + 2, 3, bulbColor, 0.9);
  // 光晕
  scene.add.circle(cx, cy + 2, 8, bulbColor, 0.08);
  scene.add.circle(cx, cy + 2, 14, bulbColor, 0.04);
}

// ── 门 ──────────────────────────────────────────────

export function drawDoor(scene: Phaser.Scene, cx: number, cy: number): void {
  const frameColor = 0x6b4423;
  const doorColor = 0x8b5a2b;
  const doorDark = 0x5a3a1a;
  const outline = 0x2a1a0a;

  // 门框
  const frame = scene.add.rectangle(cx, cy, 24, 40, frameColor, 1);
  frame.setStrokeStyle(1, outline, 1);

  // 门
  const door = scene.add.rectangle(cx, cy, 20, 36, doorColor, 1);
  door.setStrokeStyle(1, outline, 0.9);
  // 门板纹理
  scene.add.rectangle(cx, cy - 8, 16, 1, doorDark, 0.5);
  scene.add.rectangle(cx, cy + 8, 16, 1, doorDark, 0.5);
  // 门把手
  scene.add.rectangle(cx + 7, cy, 2, 2, 0xd4a017, 1).setStrokeStyle(1, outline, 0.7);
}

// ── 统一入口 ──────────────────────────────────────────

export function drawFurnitureItem(scene: Phaser.Scene, room: RoomRect, item: RoomFurniture): void {
  const cx = room.x + item.x;
  const cy = room.y + item.y;

  switch (item.kind) {
    case "desk":
      drawDesk(scene, cx, cy);
      break;
    case "chair":
      drawChair(scene, cx, cy);
      break;
    case "plant":
      drawPlant(scene, cx, cy);
      break;
    case "whiteboard":
      drawWhiteboard(scene, cx, cy);
      break;
    case "shelf":
      drawShelf(scene, cx, cy);
      break;
    case "sofa":
      drawSofa(scene, cx, cy);
      break;
    case "rug":
      drawRug(scene, cx, cy, item.w ?? 100, item.h ?? 60, item.color ?? 0x8b3a3a);
      break;
    case "window":
      drawWindow(scene, cx, cy, item.w ?? 60, item.h ?? 40);
      break;
    case "painting":
      drawPainting(scene, cx, cy, item.w ?? 40, item.h ?? 30);
      break;
    case "lamp":
      drawLamp(scene, cx, cy);
      break;
    case "door":
      drawDoor(scene, cx, cy);
      break;
  }
}

export function drawDecorationItem(scene: Phaser.Scene, room: RoomRect, deco: RoomDecoration): void {
  const cx = room.x + deco.x;
  const cy = room.y + deco.y;

  switch (deco.kind) {
    case "window":
      drawWindow(scene, cx, cy, deco.w ?? 60, deco.h ?? 40);
      break;
    case "painting":
      drawPainting(scene, cx, cy, deco.w ?? 40, deco.h ?? 30);
      break;
    case "lamp":
      drawLamp(scene, cx, cy);
      break;
    case "rug":
      drawRug(scene, cx, cy, deco.w ?? 100, deco.h ?? 60, deco.color ?? 0x8b3a3a);
      break;
    case "door":
      drawDoor(scene, cx, cy);
      break;
  }
}
