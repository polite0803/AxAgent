// SPDX-License-Identifier: AGPL-3.0-only

/**
 * OfficeGame — React 包装的 Phaser 像素办公室。
 *
 * 职责：
 * - 创建 Phaser.Game 实例（绑定到容器 div）
 * - 监听 officeStore 的成员变化，diff 同步到 Scene（保留动画状态）
 * - 暴露 onAgentClick 事件给上层
 * - 卸载时销毁 Phaser.Game
 * - 画布尺寸跟随容器（ResizeObserver），不再固定 800×500
 *
 * 注意：Phaser.Game 是重型资源，必须严格保证「挂载时创建一次，
 * 卸载时销毁一次」。React StrictMode 会导致 useEffect 跑两次，
 * 用 ref 守卫避免重复创建。
 */

import Phaser from "phaser";
import { useEffect, useRef, useState } from "react";
import { OfficeScene, type SceneMember } from "./OfficeScene";

export interface OfficeGameProps {
  /** 场景模板 slug（可选，默认为 default_office） */
  sceneTemplateSlug?: string;
  /** 当前要渲染的成员列表 */
  members: SceneMember[];
  /** 精灵点击回调 */
  onAgentClick?: (agentSlug: string, memberId: string) => void;
  /** 房间 ID → 展示名（i18n） */
  roomLabels?: Record<string, string>;
  /** 画布最小高度（响应式高度不足时的下限） */
  minHeight?: number;
}

export function OfficeGame({
  sceneTemplateSlug,
  members,
  onAgentClick,
  roomLabels,
  minHeight = 320,
}: OfficeGameProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const gameRef = useRef<Phaser.Game | null>(null);
  /** Scene 实例引用，供成员同步使用 */
  const sceneRef = useRef<OfficeScene | null>(null);
  /** 容器测量尺寸 */
  const [size, setSize] = useState({ w: 800, h: 500 });

  /** 用 ref 持有最新的回调，避免 game 重建 */
  const callbackRef = useRef(onAgentClick);
  callbackRef.current = onAgentClick;

  /** 用 ref 持有最新的成员列表，用于 diff 同步 */
  const membersRef = useRef(members);
  membersRef.current = members;

  // ── 容器尺寸测量（响应式） ──
  useEffect(() => {
    const el = containerRef.current;
    if (!el) {
      return;
    }
    const ro = new ResizeObserver((entries) => {
      const r = entries[0]?.contentRect;
      if (!r) {
        return;
      }
      const w = Math.max(320, Math.floor(r.width));
      const h = Math.max(minHeight, Math.floor(r.height));
      setSize((prev) => (prev.w === w && prev.h === h ? prev : { w, h }));
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, [minHeight]);

  // ── 创建 Phaser.Game（仅一次） ──
  useEffect(() => {
    if (!containerRef.current) {
      return;
    }
    if (gameRef.current) {
      // StrictMode 双跑守卫
      return;
    }

    try {
      const scene = new OfficeScene();
      // 在 game 启动前注入初始数据（create() 会读取这些字段）
      scene.setOptions({
        sceneTemplateSlug,
        members: membersRef.current,
        roomLabels,
        onAgentClick: (slug: string, memberId: string) => {
          callbackRef.current?.(slug, memberId);
        },
      });
      sceneRef.current = scene;

      const game = new Phaser.Game({
        type: Phaser.AUTO,
        parent: containerRef.current,
        width: size.w,
        height: size.h,
        backgroundColor: "#1a1410",
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
    } catch (err) {
      // Phaser 初始化失败（无 WebGL/Canvas 等环境异常）不应阻断 React 树
      console.error("[OfficeGame] Phaser init failed:", err);
      sceneRef.current = null;
      gameRef.current = null;
    }

    return () => {
      try {
        gameRef.current?.destroy(true);
      } catch (err) {
        console.warn("[OfficeGame] Phaser destroy failed:", err);
      }
      gameRef.current = null;
      sceneRef.current = null;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // ── 画布尺寸变化时调整 ──
  useEffect(() => {
    if (gameRef.current) {
      gameRef.current.scale.resize(size.w, size.h);
    }
  }, [size.w, size.h]);

  // ── 成员变化 diff 同步到 Scene（保留已有精灵动画状态） ──
  useEffect(() => {
    const scene = sceneRef.current;
    if (!scene || !scene.sys.isActive()) {
      return;
    }
    try {
      scene.syncMembers(membersRef.current);
    } catch (err) {
      console.warn("[OfficeGame] member sync failed:", err);
    }
  }, [members]);

  return (
    <div
      ref={containerRef}
      style={{
        width: "100%",
        height: "100%",
        minHeight: `${minHeight}px`,
        display: "flex",
        justifyContent: "center",
        alignItems: "center",
        background: "#1a1410",
        borderRadius: 8,
        overflow: "hidden",
      }}
    />
  );
}
