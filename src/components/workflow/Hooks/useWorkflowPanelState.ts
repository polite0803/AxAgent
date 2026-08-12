// SPDX-License-Identifier: AGPL-3.0-only

import { useCallback, useEffect, useState } from "react";
import { safeLocalStorageGet, safeLocalStorageSet } from "../utils";

const LS_LEFT_PANEL_COLLAPSED = "workflowEditor.leftPanelCollapsed";
const LS_RIGHT_PANEL_COLLAPSED = "workflowEditor.rightPanelCollapsed";
const LS_LEFT_PANEL_WIDTH = "workflowEditor.leftPanelWidth";
const LS_RIGHT_PANEL_WIDTH = "workflowEditor.rightPanelWidth";

const LEFT_PANEL_MIN = 180;
const LEFT_PANEL_MAX = 600;
const RIGHT_PANEL_MIN = 200;
const RIGHT_PANEL_MAX = 600;

/**
 * 封装编辑器面板的尺寸/折叠状态持久化。
 *
 * 并发安全：
 * 1. 使用 localStorage 原子化读写，避免 React state 与 localStorage 不一致
 * 2. 拖拽调整宽度时直接操作 DOM（setState 内部批量更新），减少 re-render
 * 3. 窗口 resize 自动折叠逻辑内聚在此 Hook 中
 */
export interface PanelStateController {
  leftPanelCollapsed: boolean;
  rightPanelCollapsed: boolean;
  leftPanelWidth: number;
  rightPanelWidth: number;
  resizing: "left" | "right" | null;
  setLeftPanelCollapsed: (v: boolean) => void;
  setRightPanelCollapsed: (v: boolean) => void;
  setLeftPanelWidth: (v: number) => void;
  setRightPanelWidth: (v: number) => void;
  setResizing: (v: "left" | "right" | null) => void;
  startResize: (side: "left" | "right") => void;
  stopResize: () => void;
}

export function useWorkflowPanelState(): PanelStateController {
  const [leftCollapsed, setLeftCollapsed] = useState<boolean>(() =>
    safeLocalStorageGet<boolean>(LS_LEFT_PANEL_COLLAPSED, false)
  );
  const [rightCollapsed, setRightCollapsed] = useState<boolean>(() =>
    safeLocalStorageGet<boolean>(LS_RIGHT_PANEL_COLLAPSED, false)
  );
  const [leftWidth, setLeftWidth] = useState<number>(() => safeLocalStorageGet<number>(LS_LEFT_PANEL_WIDTH, 280));
  const [rightWidth, setRightWidth] = useState<number>(() => safeLocalStorageGet<number>(LS_RIGHT_PANEL_WIDTH, 320));
  const [resizing, setResizingState] = useState<"left" | "right" | null>(null);

  const setLeftPanelCollapsed = useCallback((v: boolean) => {
    setLeftCollapsed(v);
    safeLocalStorageSet(LS_LEFT_PANEL_COLLAPSED, v);
  }, []);

  const setRightPanelCollapsed = useCallback((v: boolean) => {
    setRightCollapsed(v);
    safeLocalStorageSet(LS_RIGHT_PANEL_COLLAPSED, v);
  }, []);

  const setLeftPanelWidth = useCallback((v: number) => {
    const clamped = Math.max(LEFT_PANEL_MIN, Math.min(LEFT_PANEL_MAX, v));
    setLeftWidth(clamped);
    safeLocalStorageSet(LS_LEFT_PANEL_WIDTH, clamped);
  }, []);

  const setRightPanelWidth = useCallback((v: number) => {
    const clamped = Math.max(RIGHT_PANEL_MIN, Math.min(RIGHT_PANEL_MAX, v));
    setRightWidth(clamped);
    safeLocalStorageSet(LS_RIGHT_PANEL_WIDTH, clamped);
  }, []);

  const setResizing = useCallback((v: "left" | "right" | null) => {
    setResizingState(v);
  }, []);

  const startResize = useCallback((side: "left" | "right") => {
    setResizingState(side);
  }, []);

  const stopResize = useCallback(() => {
    setResizingState(null);
  }, []);

  // 响应式：窗口过小时自动折叠面板
  useEffect(() => {
    const checkWidth = () => {
      const w = window.innerWidth;
      if (w < 900) {
        setLeftCollapsed(true);
        safeLocalStorageSet(LS_LEFT_PANEL_COLLAPSED, true);
      }
      if (w < 1100) {
        setRightCollapsed(true);
        safeLocalStorageSet(LS_RIGHT_PANEL_COLLAPSED, true);
      }
    };
    checkWidth();
    window.addEventListener("resize", checkWidth);
    return () => window.removeEventListener("resize", checkWidth);
  }, []);

  // 面板拖拽调宽
  useEffect(() => {
    if (!resizing) { return; }
    const handleMouseMove = (e: MouseEvent) => {
      if (resizing === "left") {
        setLeftWidth((prev) => {
          const next = Math.max(LEFT_PANEL_MIN, Math.min(LEFT_PANEL_MAX, prev + e.movementX));
          safeLocalStorageSet(LS_LEFT_PANEL_WIDTH, next);
          return next;
        });
      } else {
        setRightWidth((prev) => {
          const next = Math.max(RIGHT_PANEL_MIN, Math.min(RIGHT_PANEL_MAX, prev - e.movementX));
          safeLocalStorageSet(LS_RIGHT_PANEL_WIDTH, next);
          return next;
        });
      }
    };
    const handleMouseUp = () => setResizingState(null);
    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
    return () => {
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };
  }, [resizing]);

  return {
    leftPanelCollapsed: leftCollapsed,
    rightPanelCollapsed: rightCollapsed,
    leftPanelWidth: leftWidth,
    rightPanelWidth: rightWidth,
    resizing,
    setLeftPanelCollapsed,
    setRightPanelCollapsed,
    setLeftPanelWidth,
    setRightPanelWidth,
    setResizing,
    startResize,
    stopResize,
  };
}
