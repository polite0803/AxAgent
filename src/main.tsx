// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";
import ReactDOM from "react-dom/client";
import { AppRoot } from "./App";
import "./index.css";
import { logIpcError } from "@/lib/invoke";
import { initStoreRegistry } from "./lib/storeRegistry";

// Native context menu prevention is handled by GlobalCopyMenu component.
// It prevents the native menu while providing a custom Copy menu when text is selected.

// 延迟初始化：让 React 首帧先渲染，init 在后 async 执行。
// registerBuiltins / ensureHotReloadRegistered 改为动态 import，
// 避免同步拉入 Monaco Editor / 图表库 / Markdown 渲染器等重型依赖阻塞首帧。
queueMicrotask(() => {
  // ── 初始化 Store 注册表（P0）──
  initStoreRegistry().catch(logIpcError("Store registry init failed"));

  // ── 初始化 Skill 热重载监听（P1）── 动态 import 避免同步拉入 i18n 全量 locale
  import("./stores/feature/skillExtensionStore")
    .then(({ ensureHotReloadRegistered }) => ensureHotReloadRegistered())
    .catch(logIpcError("Skill hot reload init failed"));

  // ── 注册所有内置 DynamicUI 组件（P0 - Phase 2）──
  // 动态 import：registerBuiltins 顶层同步 import 了 26 个组件（含 Monaco/图表/Markdown），
  // 必须延迟到首帧后加载，否则严重阻塞首屏渲染。
  import("./lib/dynamicUI/registerBuiltins")
    .then(({ registerAllBuiltins }) => registerAllBuiltins())
    .catch(logIpcError("DynamicUI builtin registration failed"));
});

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <AppRoot />
  </React.StrictMode>,
);
