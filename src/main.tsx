import React from "react";
import ReactDOM from "react-dom/client";
import { AppRoot } from "./App";
import "./index.css";
import { logIpcError } from "@/lib/invoke";
import { initStoreRegistry } from "./lib/storeRegistry";
import { ensureHotReloadRegistered } from "./stores/feature/skillExtensionStore";

// Native context menu prevention is handled by GlobalCopyMenu component.
// It prevents the native menu while providing a custom Copy menu when text is selected.

// ── 初始化 Store 注册表（P0）──
initStoreRegistry().catch(logIpcError("Store registry init failed"));

// ── 初始化 Skill 热重载监听（P1）──
ensureHotReloadRegistered();

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <AppRoot />
  </React.StrictMode>,
);
