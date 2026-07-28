// SPDX-License-Identifier: AGPL-3.0-only

/**
 * Phaser 像素办公室场景。
 *
 * 视觉层次（从下到上）：
 *   1. 画布底色（深色木质地板）
 *   2. 房间阴影（右下偏移 → 立体感）
 *   3. 房间地板（棋盘格 + 木纹条纹）
 *   4. 房间墙体（顶墙有厚度感 + 左侧暗影）
 *   5. 地毯
 *   6. 家具（桌子/椅子/植物/白板/书架/沙发）
 *   7. 墙面装饰（窗户/画/吊灯）
 *   8. 房间标签（白底+主色边框）
 *   9. agent 精灵
 *
 * 动画：
 *   - 吊灯光晕呼吸
 *   - 屏幕闪烁（busy 状态 agent 附近）
 *   - 植物叶子微摆
 */

import type { FleetMember, FleetMemberStatus } from "@/types";
import Phaser from "phaser";
import { drawDecorationItem, drawFurnitureItem } from "./furniture";
import {
  distributeMembersInRoom,
  type OfficeSceneTemplate,
  resolveSceneTemplate,
  type RoomRect,
} from "./sceneTemplates";
import {
  type AgentSprite,
  createAgentSprite,
  destroySprite,
  setSpriteAnimation,
  setSpriteFacing,
  type SpriteAnimation,
  tickSpriteAnimation,
  updateSpriteStatus,
} from "./sprites";

// ── 类型 ──────────────────────────────────────────────

export interface SceneMember {
  memberId: string;
  agentSlug: string;
  displayName: string;
  roomId: string;
  status: FleetMemberStatus;
}

export interface OfficeSceneOptions {
  sceneTemplateSlug?: string;
  members: SceneMember[];
  onAgentClick?: (agentSlug: string, memberId: string) => void;
}

export const OFFICE_SCENE_KEY = "OfficeScene";

// ── 场景 ──────────────────────────────────────────────

const TILE = 16;

export class OfficeScene extends Phaser.Scene {
  private template!: OfficeSceneTemplate;
  private sprites = new Map<string, AgentSprite>();
  private rooms = new Map<string, RoomRect>();
  private onAgentClick?: (agentSlug: string, memberId: string) => void;
  private pendingMembers: SceneMember[] = [];
  /** 吊灯光晕引用（动画用） */
  private lampGlows: Phaser.GameObjects.Arc[] = [];

  constructor() {
    super({ key: OFFICE_SCENE_KEY });
  }

  setOptions(options: OfficeSceneOptions): void {
    this.template = resolveSceneTemplate(options.sceneTemplateSlug);
    this.onAgentClick = options.onAgentClick;
    this.rooms.clear();
    for (const r of this.template.rooms) {
      this.rooms.set(r.id, r);
    }
    this.pendingMembers = [...options.members];
  }

  create(): void {
    this.drawBackdrop();
    this.drawRooms();
    this.drawFurniture();
    this.drawDecorations();
    this.drawRoomLabels();
    for (const m of this.pendingMembers) {
      this.addMemberSprite(m);
    }
    this.pendingMembers = [];
  }

  update(time: number, delta: number): void {
    // 驱动 agent 动画
    for (const sprite of this.sprites.values()) {
      tickSpriteAnimation(sprite, time, delta);
    }
    // 吊灯光晕呼吸
    for (const glow of this.lampGlows) {
      const pulse = 0.06 + Math.sin(time / 800) * 0.03;
      glow.setAlpha(pulse);
    }
  }

  // ── 同步接口 ──

  addMemberSprite(member: SceneMember): void {
    if (this.sprites.has(member.memberId)) { return; }
    const room = this.rooms.get(member.roomId) ?? this.rooms.get(this.template.defaultRoomId);
    if (!room) { return; }
    const placed = this.findFreeSpot(room, this.countMembersInRoom(member.roomId));
    const sprite = createAgentSprite(this, placed.x, placed.y, member.agentSlug, member.memberId, member.status);
    sprite.roomId = member.roomId;
    this.setupSpriteInteraction(sprite);
    this.sprites.set(member.memberId, sprite);
    this.applyStatusAnimation(sprite, member.status);
  }

  removeMemberSprite(memberId: string): void {
    const sprite = this.sprites.get(memberId);
    if (!sprite) { return; }
    destroySprite(sprite);
    this.sprites.delete(memberId);
  }

  updateMemberStatus(memberId: string, status: FleetMemberStatus): void {
    const sprite = this.sprites.get(memberId);
    if (!sprite) { return; }
    updateSpriteStatus(sprite, status);
    this.applyStatusAnimation(sprite, status);
  }

  moveMemberToRoom(memberId: string, roomId: string): void {
    const sprite = this.sprites.get(memberId);
    const targetRoom = this.rooms.get(roomId);
    if (!sprite || !targetRoom) { return; }
    if (sprite.roomId === roomId) { return; }
    sprite.roomId = roomId;
    const sameRoomCount = this.countMembersInRoom(roomId);
    const pos = this.findFreeSpot(targetRoom, sameRoomCount);
    if (pos.x < sprite.container.x) { setSpriteFacing(sprite, "left"); }
    else if (pos.x > sprite.container.x) { setSpriteFacing(sprite, "right"); }
    sprite.targetX = pos.x;
    sprite.targetY = pos.y;
    setSpriteAnimation(sprite, "walking");
    this.tweens.add({
      targets: sprite.container,
      x: pos.x,
      y: pos.y,
      duration: 1500,
      ease: "Quad.easeInOut",
      onComplete: () => {
        if (sprite.animation === "walking") { setSpriteAnimation(sprite, "idle"); }
      },
    });
  }

  highlightMember(memberId: string): void {
    const sprite = this.sprites.get(memberId);
    if (!sprite) { return; }
    this.tweens.add({
      targets: sprite.container,
      scale: 1.2,
      duration: 200,
      yoyo: true,
      onComplete: () => {
        sprite.container.setScale(sprite.facing === "left" ? -1 : 1, 1);
      },
    });
  }

  clearAll(): void {
    for (const sprite of this.sprites.values()) {
      destroySprite(sprite);
    }
    this.sprites.clear();
  }

  // ── 绘制 ──

  /** 整张画布的深色木质背景 */
  private drawBackdrop(): void {
    const W = this.template.canvasWidth;
    const H = this.template.canvasHeight;
    // 底色
    this.add.rectangle(W / 2, H / 2, W, H, 0x1a1410, 1);
    // 大棋盘格（深木色）
    const bgTile = TILE * 2;
    for (let x = 0; x < W; x += bgTile) {
      for (let y = 0; y < H; y += bgTile) {
        const dark = ((x / bgTile + y / bgTile) % 2) === 0;
        this.add.rectangle(x + bgTile / 2, y + bgTile / 2, bgTile, bgTile, dark ? 0x2a1d12 : 0x1f150e, 1);
      }
    }
  }

  /** 绘制所有房间：阴影 + 地板 + 墙体 */
  private drawRooms(): void {
    for (const room of this.template.rooms) {
      this.drawOneRoom(room);
    }
  }

  private drawOneRoom(room: RoomRect): void {
    const { x, y, width: w, height: h, color } = room;

    // 1) 阴影偏移（右下 6px）
    this.add.rectangle(x + w / 2 + 6, y + h / 2 + 6, w, h, 0x000000, 0.3);

    // 2) 地板底色
    this.add.rectangle(x + w / 2, y + h / 2, w, h, this.lighten(color, 0.5), 1);

    // 3) 地板棋盘格
    for (let dx = 0; dx < w; dx += TILE) {
      for (let dy = 0; dy < h; dy += TILE) {
        const dark = ((dx / TILE + dy / TILE) % 2) === 0;
        const tileColor = dark ? this.lighten(color, 0.55) : this.lighten(color, 0.4);
        this.add.rectangle(x + dx + TILE / 2, y + dy + TILE / 2, TILE, TILE, tileColor, 1);
      }
    }

    // 4) 地板木纹（横向条纹）
    for (let dy = 0; dy < h; dy += 32) {
      this.add.rectangle(x + w / 2, y + dy + 16, w - 4, 1, this.darken(color, 0.6), 0.15);
    }

    // 5) 墙体立体感：顶墙有厚度
    const wallThick = 4;
    const wallColor = this.darken(color, 0.25);
    const wallShadow = this.darken(color, 0.5);
    // 顶墙外沿（深色）
    this.add.rectangle(x + w / 2, y + wallThick / 2, w, wallThick, wallShadow, 1);
    // 顶墙内沿（中色）
    this.add.rectangle(x + w / 2, y + wallThick + 1, w, 2, wallColor, 1);
    // 左墙
    this.add.rectangle(x + 1, y + h / 2, 2, h, wallShadow, 1);
    this.add.rectangle(x + 3, y + h / 2, 2, h, wallColor, 1);
    // 右墙高光
    this.add.rectangle(x + w - 1, y + h / 2, 2, h, this.lighten(color, 0.3), 0.5);
    // 底墙
    this.add.rectangle(x + w / 2, y + h - 1, w, 2, wallShadow, 1);

    // 6) 外深描边
    this.add.rectangle(x + w / 2, y + h / 2, w, h, undefined, 0)
      .setStrokeStyle(2, this.darken(color, 0.5), 0.9);
  }

  /** 绘制家具 */
  private drawFurniture(): void {
    for (const room of this.template.rooms) {
      const furniture = this.template.furniture?.[room.id] ?? [];
      // 按 kind 排序：rug 先画（在地板上），其余后画
      const sorted = [...furniture].sort((a, b) => {
        const order = (k: string) => k === "rug" ? 0 : 1;
        return order(a.kind) - order(b.kind);
      });
      for (const item of sorted) {
        drawFurnitureItem(this, room, item);
      }
    }
  }

  /** 绘制墙面装饰 */
  private drawDecorations(): void {
    for (const room of this.template.rooms) {
      const decos = this.template.decorations?.[room.id] ?? [];
      for (const deco of decos) {
        drawDecorationItem(this, room, deco);
        // 如果是吊灯，额外添加光晕引用用于动画
        if (deco.kind === "lamp") {
          const cx = room.x + deco.x;
          const cy = room.y + deco.y;
          const glow = this.add.circle(cx, cy + 4, 20, 0xfbbf24, 0.06);
          this.lampGlows.push(glow);
        }
      }
    }
  }

  /** 房间标签（最后画，确保在最上层） */
  private drawRoomLabels(): void {
    for (const room of this.template.rooms) {
      const { x, y, width: w, color } = room;
      // 标签背景（白底 + 主色边框）
      const labelW = w * 0.5;
      const labelH = 20;
      const labelX = x + w / 2;
      const labelY = y + 14;
      const labelBg = this.add.rectangle(labelX, labelY, labelW, labelH, 0xffffff, 0.95);
      labelBg.setStrokeStyle(1, this.darken(color, 0.2), 1);
      // 文字
      this.add.text(labelX, labelY, room.id, {
        fontFamily: "monospace",
        fontSize: "11px",
        color: `#${this.darken(color, 0.3).toString(16).padStart(6, "0")}`,
        fontStyle: "bold",
      }).setOrigin(0.5);
    }
  }

  // ── 空闲站位查找 ──

  private findFreeSpot(room: RoomRect, hintIndex: number): { x: number; y: number } {
    const furniture = this.template.furniture?.[room.id] ?? [];
    const base = distributeMembersInRoom(room, Math.max(hintIndex + 1, 1), hintIndex);
    const tooClose = (fx: number, fy: number) => {
      for (const item of furniture) {
        if (item.kind === "rug") { continue; }
        if (Math.hypot(fx - (room.x + item.x), fy - (room.y + item.y)) < 30) { return true; }
      }
      return false;
    };
    if (!tooClose(base.x, base.y)) { return base; }
    for (let y = 40; y < room.height - 20; y += 16) {
      for (let x = 20; x < room.width - 20; x += 16) {
        const px = room.x + x;
        const py = room.y + y;
        if (!tooClose(px, py)) { return { x: px, y: py }; }
      }
    }
    return base;
  }

  private countMembersInRoom(roomId: string): number {
    let count = 0;
    for (const s of this.sprites.values()) {
      if (s.roomId === roomId) { count++; }
    }
    return count;
  }

  // ── 精灵交互 ──

  private setupSpriteInteraction(sprite: AgentSprite): void {
    sprite.container.setSize(28, 48);
    sprite.container.setInteractive(
      new Phaser.Geom.Rectangle(-14, -48, 28, 48),
      Phaser.Geom.Rectangle.Contains,
    );
    sprite.container.on("pointerup", () => {
      this.onAgentClick?.(sprite.agentSlug, sprite.memberId);
    });
    sprite.container.on("pointerover", () => {
      this.tweens.add({
        targets: sprite.container,
        scaleX: sprite.facing === "left" ? -1.1 : 1.1,
        scaleY: 1.1,
        duration: 100,
      });
    });
    sprite.container.on("pointerout", () => {
      this.tweens.add({
        targets: sprite.container,
        scaleX: sprite.facing === "left" ? -1 : 1,
        scaleY: 1,
        duration: 100,
      });
    });
  }

  private applyStatusAnimation(sprite: AgentSprite, status: FleetMemberStatus): void {
    const anim: SpriteAnimation = status === "busy"
      ? "typing"
      : status === "idle"
      ? "idle"
      : status === "error"
      ? "celebrating"
      : "idle";
    setSpriteAnimation(sprite, anim);
  }

  // ── 颜色工具 ──

  private lighten(color: number, t: number): number {
    const r = (color >> 16) & 0xff;
    const g = (color >> 8) & 0xff;
    const b = color & 0xff;
    return (Math.round(r + (255 - r) * t) << 16) | (Math.round(g + (255 - g) * t) << 8) | Math.round(b + (255 - b) * t);
  }

  private darken(color: number, t: number): number {
    const r = (color >> 16) & 0xff;
    const g = (color >> 8) & 0xff;
    const b = color & 0xff;
    return (Math.round(r * (1 - t)) << 16) | (Math.round(g * (1 - t)) << 8) | Math.round(b * (1 - t));
  }
}

// ── React 层辅助 ──

export function fleetMemberToSceneMember(m: FleetMember): SceneMember {
  return {
    memberId: m.id,
    agentSlug: m.agentSlug,
    displayName: m.displayName,
    roomId: m.roomId,
    status: m.status,
  };
}
