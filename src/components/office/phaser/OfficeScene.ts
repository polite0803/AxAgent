// SPDX-License-Identifier: AGPL-3.0-only

/**
 * Phaser 像素办公室场景。
 *
 * 职责：
 * - 按场景模板绘制房间墙体与名称
 * - 创建/管理 agent 精灵（按 FleetMember 列表）
 * - 驱动每帧动画（idle/walking/typing/celebrating）
 * - 暴露同步接口（addMember / removeMember / updateStatus / moveToRoom）
 *   供 React 层在 store 变化时调用
 * - 处理精灵点击事件 → 通过 EventEmitter 上抛到 React
 */

import type { FleetMember, FleetMemberStatus } from "@/types";
import Phaser from "phaser";
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
  type SpriteAnimation,
  tickSpriteAnimation,
  updateSpriteStatus,
} from "./sprites";

/** Scene 内部使用的成员视图（剥离了不必要的字段） */
export interface SceneMember {
  memberId: string;
  agentSlug: string;
  displayName: string;
  roomId: string;
  status: FleetMemberStatus;
}

export interface OfficeSceneOptions {
  /** 场景模板 slug（解析为内置模板） */
  sceneTemplateSlug?: string;
  /** 初始成员列表 */
  members: SceneMember[];
  /** 精灵点击回调（上抛 agentSlug + memberId 给 React） */
  onAgentClick?: (agentSlug: string, memberId: string) => void;
}

/** Phaser 场景 key */
export const OFFICE_SCENE_KEY = "OfficeScene";

export class OfficeScene extends Phaser.Scene {
  private template!: OfficeSceneTemplate;
  /** memberId → sprite */
  private sprites = new Map<string, AgentSprite>();
  /** roomId → room */
  private rooms = new Map<string, RoomRect>();
  /** 成员点击回调 */
  private onAgentClick?: (agentSlug: string, memberId: string) => void;
  /** 初始成员（create 时读取） */
  private pendingMembers: SceneMember[] = [];

  constructor() {
    super({ key: OFFICE_SCENE_KEY });
  }

  /**
   * 由 React 层在创建 game 前调用，注入初始数据。
   *
   * Phaser 的 create() 在 game.start() 后异步执行，因此数据必须在
   * 构造后、start 前注入完毕。
   *
   * 不能用 Phaser 的 init(data) 机制，因为本 scene 已通过
   * `scene: [scene]` 配置在 game 创建时自动启动，无法在启动时
   * 传 data。改用显式 setter 是更可控的方式。
   */
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
    this.drawRooms();
    for (const m of this.pendingMembers) {
      this.addMemberSprite(m);
    }
    this.pendingMembers = [];
  }

  update(_time: number, delta: number): void {
    for (const sprite of this.sprites.values()) {
      tickSpriteAnimation(sprite, _time, delta);
    }
  }

  // ── 对外同步接口（React 层在 store 变化时调用） ──

  /** 添加一个成员精灵 */
  addMemberSprite(member: SceneMember): void {
    if (this.sprites.has(member.memberId)) {
      return;
    }
    const room = this.rooms.get(member.roomId) ?? this.rooms.get(this.template.defaultRoomId);
    if (!room) {
      return;
    }
    // 在房间内分布
    const sameRoomCount = this.countMembersInRoom(member.roomId);
    const pos = distributeMembersInRoom(room, sameRoomCount + 1, sameRoomCount);
    const sprite = createAgentSprite(
      this,
      pos.x,
      pos.y,
      member.agentSlug,
      member.memberId,
      member.status,
    );
    sprite.roomId = member.roomId;
    this.setupSpriteInteraction(sprite);
    this.sprites.set(member.memberId, sprite);

    // 状态映射到动画
    this.applyStatusAnimation(sprite, member.status);
  }

  /** 移除一个成员精灵 */
  removeMemberSprite(memberId: string): void {
    const sprite = this.sprites.get(memberId);
    if (!sprite) {
      return;
    }
    destroySprite(sprite);
    this.sprites.delete(memberId);
  }

  /** 更新成员状态（颜色 + 动画） */
  updateMemberStatus(memberId: string, status: FleetMemberStatus): void {
    const sprite = this.sprites.get(memberId);
    if (!sprite) {
      return;
    }
    updateSpriteStatus(sprite, status);
    this.applyStatusAnimation(sprite, status);
  }

  /** 移动成员到另一个房间（带行走动画） */
  moveMemberToRoom(memberId: string, roomId: string): void {
    const sprite = this.sprites.get(memberId);
    const targetRoom = this.rooms.get(roomId);
    if (!sprite || !targetRoom) {
      return;
    }
    if (sprite.roomId === roomId) {
      return;
    }
    sprite.roomId = roomId;
    const sameRoomCount = this.countMembersInRoom(roomId);
    const pos = distributeMembersInRoom(targetRoom, sameRoomCount + 1, sameRoomCount);
    sprite.targetX = pos.x;
    sprite.targetY = pos.y;
    setSpriteAnimation(sprite, "walking");
    // Phaser tween 驱动 container 移动
    this.tweens.add({
      targets: sprite.container,
      x: pos.x,
      y: pos.y,
      duration: 1500,
      ease: "Quad.easeInOut",
      onComplete: () => {
        // 行走结束回到 idle（除非状态已经是 busy）
        if (sprite.animation === "walking") {
          setSpriteAnimation(sprite, "idle");
        }
      },
    });
  }

  /** 高亮某个成员（用于 DM 点击） */
  highlightMember(memberId: string): void {
    const sprite = this.sprites.get(memberId);
    if (!sprite) {
      return;
    }
    // 短暂放大 + 颜色闪烁
    this.tweens.add({
      targets: sprite.container,
      scale: 1.2,
      duration: 200,
      yoyo: true,
      onComplete: () => {
        sprite.container.setScale(1);
      },
    });
  }

  /** 销毁所有精灵（用于 fleet 切换） */
  clearAll(): void {
    for (const sprite of this.sprites.values()) {
      destroySprite(sprite);
    }
    this.sprites.clear();
  }

  // ── 内部辅助 ──

  private drawRooms(): void {
    for (const room of this.template.rooms) {
      // 房间背景（半透明填充）
      this.add.rectangle(
        room.x + room.width / 2,
        room.y + room.height / 2,
        room.width,
        room.height,
        room.color,
        0.08,
      );
      // 房间墙体描边
      this.add.rectangle(
        room.x + room.width / 2,
        room.y + room.height / 2,
        room.width,
        room.height,
        undefined,
        0,
      ).setStrokeStyle(2, room.color, 0.6);
      // 房间名称（顶部居中）
      this.add.text(room.x + room.width / 2, room.y + 12, room.id, {
        fontFamily: "monospace",
        fontSize: "11px",
        color: `#${room.color.toString(16).padStart(6, "0")}`,
        backgroundColor: "rgba(255,255,255,0.9)",
        padding: { x: 6, y: 2 },
      }).setOrigin(0.5, 0);
    }
  }

  private countMembersInRoom(roomId: string): number {
    let count = 0;
    for (const s of this.sprites.values()) {
      if (s.roomId === roomId) {
        count++;
      }
    }
    return count;
  }

  private setupSpriteInteraction(sprite: AgentSprite): void {
    const objects = [sprite.body, sprite.head, sprite.label];
    for (const obj of objects) {
      obj.setInteractive({ useHandCursor: true });
      obj.on("pointerup", () => {
        this.onAgentClick?.(sprite.agentSlug, sprite.memberId);
      });
    }
  }

  private applyStatusAnimation(sprite: AgentSprite, status: FleetMemberStatus): void {
    const anim: SpriteAnimation = status === "busy"
      ? "typing"
      : status === "idle"
      ? "idle"
      : status === "error"
      ? "celebrating" // 错误时也用庆祝动画吸引注意（后续可换）
      : "idle";
    setSpriteAnimation(sprite, anim);
  }
}

// ── React 层辅助：把 FleetMember 转为 SceneMember ──

export function fleetMemberToSceneMember(m: FleetMember): SceneMember {
  return {
    memberId: m.id,
    agentSlug: m.agentSlug,
    displayName: m.displayName,
    roomId: m.roomId,
    status: m.status,
  };
}
