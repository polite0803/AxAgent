// SPDX-License-Identifier: AGPL-3.0-only

// Wiki 编辑器保存行为共享 hook：Ctrl+S / Cmd+S 立即保存 + 空闲自动保存。
// F5 去重：原 WikiEditorPage / WikiDetailPanel 各持一份行为相同的实现，统一收编至此。
import { useEffect, useRef } from "react";

interface UseWikiAutoSaveOptions {
  content: string;
  title: string;
  /** 是否允许自动保存（调用方合并 hasChanges / saving / 防重入标志） */
  autoSaveEnabled: boolean;
  handleSave: () => void | Promise<void>;
  /** 空闲触发间隔 ms，默认 3000 */
  delayMs?: number;
}

export function useWikiAutoSave({
  content,
  title,
  autoSaveEnabled,
  handleSave,
  delayMs = 3000,
}: UseWikiAutoSaveOptions): void {
  // refs 保持最新值，避免 effect 因回调/状态引用变化频繁重挂全局监听
  const handleSaveRef = useRef(handleSave);
  handleSaveRef.current = handleSave;
  const autoSaveEnabledRef = useRef(autoSaveEnabled);
  autoSaveEnabledRef.current = autoSaveEnabled;
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Ctrl+S / Cmd+S 立即保存
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === "s") {
        e.preventDefault();
        void handleSaveRef.current();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  // 自动保存（空闲 delayMs 触发）
  useEffect(() => {
    if (!autoSaveEnabledRef.current) {
      return;
    }
    if (timerRef.current) {
      clearTimeout(timerRef.current);
    }
    timerRef.current = setTimeout(() => {
      void handleSaveRef.current();
    }, delayMs);
    return () => {
      if (timerRef.current) {
        clearTimeout(timerRef.current);
      }
    };
  }, [content, title, delayMs]);
}
