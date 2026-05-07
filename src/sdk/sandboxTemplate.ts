/**
 * Skill 沙箱 HTML 模板生成器
 *
 * 为每个 Skill 生成一个自包含的 HTML 页面，在 sandbox iframe 中运行。
 * 该页面包含：
 * 1. RPC 通信层（postMessage）
 * 2. ctx 对象（api, ui, store）
 * 3. Skill 组件代码占位
 *
 * @module sdk/sandboxTemplate
 */

import type { SkillPermissionsV2 } from "./types";

export interface SandboxTemplateOptions {
  /** Skill 名称 */
  skillName: string;
  /** Skill ID */
  skillId: string;
  /** 注入的初始 props */
  props: Record<string, unknown>;
  /** 权限声明 */
  permissions: SkillPermissionsV2;
  /** Skill HTML 内容（用户编写的 UI） */
  htmlContent: string;
  /** Skill 的入口脚本路径（相对于 skill 目录） */
  entryScript?: string;
  /** 是否开发模式（生产模式会移除 console.log） */
  devMode?: boolean;
}

/**
 * 生成 Skill 沙箱 HTML 完整页面
 */
export function generateSandboxHtml(options: SandboxTemplateOptions): string {
  const {
    skillName,
    htmlContent,
  } = options;

  const runtimeScript = generateRuntimeScript();

  return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Skill: ${skillName}</title>
  <style>
    /* Skill 沙箱基础样式重置 */
    *, *::before, *::after {
      box-sizing: border-box;
      margin: 0;
      padding: 0;
    }
    html, body {
      width: 100%;
      height: 100%;
      overflow: auto;
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
      font-size: 14px;
      line-height: 1.5;
      color: inherit;
      background: transparent;
    }
    #app {
      width: 100%;
      min-height: 100%;
    }
    /* Skill 错误显示 */
    .ax-skill-error {
      padding: 24px;
      color: #e74c3c;
      text-align: center;
      font-family: monospace;
    }
    .ax-skill-loading {
      display: flex;
      align-items: center;
      justify-content: center;
      height: 100%;
      color: #999;
    }
    .ax-skill-loading::after {
      content: "";
      width: 24px;
      height: 24px;
      border: 2px solid #eee;
      border-top-color: #1677ff;
      border-radius: 50%;
      animation: ax-spin 0.8s linear infinite;
      margin-left: 12px;
    }
    @keyframes ax-spin {
      to { transform: rotate(360deg); }
    }
  </style>
  <script>
    // ── 阻止 Skill 沙箱内的恶意行为 ──
    // 删除危险 API
    delete window.fetch;
    delete window.XMLHttpRequest;
    // 禁止导航
    window.onbeforeunload = function() { return false; };
  </script>
</head>
<body>
  <div id="app">${htmlContent}</div>

  <script>
${runtimeScript}
  </script>
</body>
</html>`;
}

/**
 * 生成 Skill 沙箱运行时脚本
 *
 * 该脚本提供：
 * - ctx 对象（api, ui, store）通过 postMessage RPC 实现
 * - 宿主消息监听
 * - Skill 生命周期管理
 */
function generateRuntimeScript(): string {
  return `
// ── AxAgent Skill Sandbox Runtime v2 ──────────────────────────────────
(function() {
  "use strict";

  const SKILL_NAME = ${JSON.stringify("__SKILL_NAME__")};
  const SKILL_ID = ${JSON.stringify("__SKILL_ID__")};
  const INITIAL_PROPS = __INITIAL_PROPS__;
  const PERMISSIONS = __PERMISSIONS__;

  // ── RPC 基础设施 ──

  const pendingCalls = new Map();
  let callIdCounter = 0;

  /** 调用宿主方法 */
  function callHost(method, args) {
    return new Promise(function(resolve, reject) {
      const callId = "skill_" + (++callIdCounter) + "_" + Date.now();
      var timer = setTimeout(function() {
        pendingCalls.delete(callId);
        reject(new Error('RPC call "' + method + '" timed out'));
      }, 15000);

      pendingCalls.set(callId, { resolve: resolve, reject: reject, timer: timer });

      var responseHandler = function(event) {
        var msg = event.data;
        if (msg && msg.type === "rpc:response" && msg.callId === callId) {
          window.removeEventListener("message", responseHandler);
          var pending = pendingCalls.get(callId);
          if (!pending) { return; }
          clearTimeout(pending.timer);
          pendingCalls.delete(callId);
          if (msg.error) {
            pending.reject(new Error(msg.error));
          } else {
            pending.resolve(msg.result);
          }
        }
      };

      window.addEventListener("message", responseHandler);

      try {
        window.parent.postMessage({
          type: "rpc:request",
          callId: callId,
          method: method,
          args: args
        }, "*");
      } catch (e) {
        window.removeEventListener("message", responseHandler);
        clearTimeout(timer);
        pendingCalls.delete(callId);
        reject(e);
      }
    });
  }

  // ── ctx 对象 ──

  window.ctx = Object.freeze({
    get skillName() { return SKILL_NAME; },
    get skillId() { return SKILL_ID; },
    get props() { return INITIAL_PROPS; },

    api: Object.freeze({
      invoke: function(command, args) {
        return callHost("api.invoke", { command: command, args: args });
      },
      emit: function(event, payload) {
        return callHost("api.emit", { event: event, payload: payload });
      }
    }),

    ui: Object.freeze({
      navigate: function(path) {
        return callHost("ui.navigate", { path: path });
      },
      notify: function(message, type) {
        return callHost("ui.notify", { message: message, type: type || "info" });
      },
      getTheme: function() {
        return callHost("ui.getTheme");
      },
      getLocale: function() {
        return callHost("ui.getLocale");
      }
    }),

    store: Object.freeze({
      read: function(storeName, selector) {
        return callHost("store.read", { storeName: storeName, selector: selector });
      },
      write: function(storeName, value) {
        return callHost("store.write", { storeName: storeName, value: value });
      }
    })
  });

  // ── 宿主消息监听 ──

  window.addEventListener("message", function(event) {
    var msg = event.data;
    if (!msg || typeof msg.type !== "string") { return; }

    switch (msg.type) {
      case "host:event":
        if (window.dispatchEvent) {
          window.dispatchEvent(new CustomEvent("ax:" + msg.event, { detail: msg.payload }));
        }
        break;
      case "host:lifecycle":
        if (msg.phase === "mount" && typeof window.onSkillMount === "function") {
          window.onSkillMount(msg.props || {});
        } else if (msg.phase === "unmount" && typeof window.onSkillUnmount === "function") {
          window.onSkillUnmount();
        }
        break;
    }
  });

  // ── 向宿主报告就绪 ──

  try {
    window.parent.postMessage({ type: "skill:ready" }, "*");
  } catch(e) {}

  // ── 如果 Skill 定义了主入口函数，自动调用 ──

  if (typeof window.onSkillInit === "function") {
    try {
      window.onSkillInit(window.ctx);
    } catch(e) {
      try {
        window.parent.postMessage({ type: "skill:error", error: String(e) }, "*");
      } catch(e2) {}
    }
  }
})();
`;
}
