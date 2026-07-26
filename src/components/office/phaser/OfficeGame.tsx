// SPDX-License-Identifier: AGPL-3.0-only

/**
 * OfficeGame — React 包装的 Phaser 像素办公室。
 *
 * 职责：
 * - 创建 Phaser.Game 实例（绑定到容器 div）
 * - 监听 officeStore 的成员变化，同步到 Scene
 * - 暴露 onAgentClick 事件给上层
 * - 卸载时销毁 Phaser.Game
 *
 * 注意：Phaser.Game 是重型资源，必须严格保证「挂载时创建一次，
 * 卸载时销毁一次」。React StrictMode 会导致 useEffect 跑两次，
 * 用 ref 守卫避免重复创建。
 */

import Phaser from "phaser";
import { useEffect, useRef } from "react";
import { OfficeScene, type SceneMember } from "./OfficeScene";

export interface OfficeGameProps {
  /** 场景模板 slug（可选，默认为 default_office） */
  sceneTemplateSlug?: string;
  /** 当前要渲染的成员列表 */
  members: SceneMember[];
  /** 精灵点击回调 */
  onAgentClick?: (agentSlug: string, memberId: string) => void;
  /** 画布容器宽度 */
  width?: number;
  /** 画布容器高度 */
  height?: number;
}

export function OfficeGame({
  sceneTemplateSlug,
  members,
  onAgentClick,
  width = 800,
  height = 500,
}: OfficeGameProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const gameRef = useRef<Phaser.Game | null>(null);
  /** Scene 实例引用，供成员同步使用 */
  const sceneRef = useRef<OfficeScene | null>(null);

  /** 用 ref 持有最新的回调，避免 game 重建 */
  const callbackRef = useRef(onAgentClick);
  callbackRef.current = onAgentClick;

  /** 用 ref 持有最新的成员列表，用于 diff 同步 */
  const membersRef = useRef(members);
  membersRef.current = members;

  // ── 创建 Phaser.Game（仅一次） ──
  useEffect(() => {
    if (!containerRef.current) {
      return;
    }
    if (gameRef.current) {
      // StrictMode 双跑守卫
      return;
    }

    const scene = new OfficeScene();
    // 在 game 启动前注入初始数据（create() 会读取这些字段）
    scene.setOptions({
      sceneTemplateSlug,
      members: membersRef.current,
      onAgentClick: (slug: string, memberId: string) => {
        callbackRef.current?.(slug, memberId);
      },
    });
    sceneRef.current = scene;

    const game = new Phaser.Game({
      type: Phaser.AUTO,
      parent: containerRef.current,
      width,
      height,
      backgroundColor: "#f5f5f5",
      render: {
        antialias: false, // 像素风
        pixelArt: true,
      },
      // scene 在 game 创建时自动启动并触发 create()
      scene: [scene],
      input: {
        activePointers: 3,
      },
      scale: {
        mode: Phaser.Scale.FIT,
        autoCenter: Phaser.Scale.CENTER_BOTH,
      },
    });
    gameRef.current = game;

    return () => {
      game.destroy(true);
      gameRef.current = null;
      sceneRef.current = null;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // ── 画布尺寸变化时调整 ──
  useEffect(() => {
    if (gameRef.current) {
      gameRef.current.scale.resize(width, height);
    }
  }, [width, height]);

  // ── 成员变化同步到 Scene ──
  useEffect(() => {
    const scene = sceneRef.current;
    if (!scene || !scene.sys.isActive()) {
      return;
    }
    const current = membersRef.current;
    const sceneMembers = new Map<string, SceneMember>();
    for (const m of current) {
      sceneMembers.set(m.memberId, m);
    }

    // 用 Registry 里的初始成员列表作为对比基线（create 后清空）
    // 简化处理：调用 scene 暴露的同步方法
    // 这里用 clearAll + 重建，避免复杂的 diff；成员数通常 <20，性能可接受
    scene.clearAll();
    for (const m of current) {
      scene.addMemberSprite(m);
    }
  }, [members]);

  return (
    <div
      ref={containerRef}
      style={{
        width: "100%",
        height: `${height}px`,
        display: "flex",
        justifyContent: "center",
        alignItems: "center",
        background: "#f5f5f5",
        borderRadius: 8,
        overflow: "hidden",
      }}
    />
  );
}
