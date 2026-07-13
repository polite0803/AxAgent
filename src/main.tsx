// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";
import ReactDOM from "react-dom/client";
import { AppRoot } from "./App";
import "./index.css";
import { logIpcError } from "@/lib/invoke";
import { registerAllBuiltins } from "./lib/dynamicUI/registerBuiltins";
import { initStoreRegistry } from "./lib/storeRegistry";
import { ensureHotReloadRegistered } from "./stores/feature/skillExtensionStore";

// Native context menu prevention is handled by GlobalCopyMenu component.
// It prevents the native menu while providing a custom Copy menu when text is selected.

// 延迟初始化：让 React 首帧先渲染，init 在后 async 执行。
// Dev 模式下顶层 import 已经是串行瓶颈，不应再追加同步阻塞。
queueMicrotask(() => {
  // ── 初始化 Store 注册表（P0）──
  initStoreRegistry().catch(logIpcError("Store registry init failed"));

  // ── 初始化 Skill 热重载监听（P1）──
  ensureHotReloadRegistered();

  // ── 注册所有内置 DynamicUI 组件（P0 - Phase 2）──
  registerAllBuiltins();
});

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <AppRoot />
  </React.StrictMode>,
);
