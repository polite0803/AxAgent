// SPDX-License-Identifier: AGPL-3.0-only

import { ensureMonacoWorkers } from "stream-monaco";

declare global {
  interface Window {
    monaco?: typeof import("monaco-editor");
  }
}

let monacoPromise: Promise<typeof import("monaco-editor")> | null = null;

/**
 * 动态加载 monaco-editor（模块级缓存，只加载一次，可并发等待）。
 *
 * 背景：MonacoEditor / DiffViewer / WikiEditorPage 均依赖 monaco 实例，
 * 但旧实现直接读全局 window.monaco，而没有任何模块负责赋值 ——
 * 依赖 fire-and-forget 的预加载存在竞态，window.monaco 未就绪时编辑器挂载即崩溃。
 *
 * 本模块统一承担加载职责：
 * 1. 初始化 Monaco worker 环境（stream-monaco 打包的 worker bundle）；
 * 2. 动态 import monaco-editor，并挂到 window.monaco 兼容既有全局读取；
 * 3. 返回 Promise，调用方等待就绪后再创建编辑器，杜绝竞态。
 */
export function loadMonaco(): Promise<typeof import("monaco-editor")> {
  if (!monacoPromise) {
    monacoPromise = (async () => {
      try {
        ensureMonacoWorkers();
      } catch {
        // worker 环境不可用时 Monaco 退回主线程渲染，不影响编辑器功能
      }
      const monaco = await import("monaco-editor");
      window.monaco = window.monaco ?? monaco;
      return window.monaco;
    })();
  }
  return monacoPromise;
}
