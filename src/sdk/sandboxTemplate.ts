/**
 * Skill 沙箱 HTML 模板生成器
 *
 * 为每个 Skill 生成一个自包含的 HTML 页面，在 sandbox iframe 中运行。
 * 该页面包含：
 * 1. RPC 通信层（postMessage）— 双向：skill→host 和 host→skill
 * 2. ctx 对象（api, ui, store）
 * 3. Skill 组件代码占位
 * 4. 全局错误上报
 */

import type { SkillPermissions } from "./types";

/** RPC 调用默认超时（毫秒） */
export const DEFAULT_RPC_TIMEOUT_MS = 15000;

export interface SandboxTemplateOptions {
  skillName: string;
  skillId: string;
  props: Record<string, unknown>;
  permissions: SkillPermissions;
  htmlContent: string;
  entryScript?: string;
  devMode?: boolean;
  /** RPC 调用超时，默认 15000ms */
  rpcTimeoutMs?: number;
}

/**
 * 生成 Skill 沙箱 HTML 完整页面
 */
export function generateSandboxHtml(options: SandboxTemplateOptions): string {
  const { skillName, htmlContent, rpcTimeoutMs = DEFAULT_RPC_TIMEOUT_MS } = options;

  const runtimeScript = generateRuntimeScript(rpcTimeoutMs);

  return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Skill: ${skillName}</title>
  <style>
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
    // 删除危险 API
    delete window.fetch;
    delete window.XMLHttpRequest;
    // 禁止顶层导航（sandbox 属性已限制，此处作为额外防护）
    window.addEventListener("beforeunload", function(e) {
      e.preventDefault();
      e.returnValue = "";
    });
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
 * 生成 Skill 沙箱运行时脚本。
 *
 * 提供：
 * - ctx 对象（api, ui, store）通过 postMessage RPC 实现
 * - 双向 RPC：skill→host 和 host→skill（callSkillMethod）
 * - 宿主消息监听（host:event, host:lifecycle, rpc:request）
 * - 全局错误上报
 */
function generateRuntimeScript(rpcTimeoutMs: number): string {
  return `
// ── AxAgent Skill Sandbox Runtime ──────────────────────────────────
(function() {
  "use strict";

  var SKILL_NAME = ${JSON.stringify("__SKILL_NAME__")};
  var SKILL_ID = ${JSON.stringify("__SKILL_ID__")};
  var INITIAL_PROPS = __INITIAL_PROPS__;
  var PERMISSIONS = __PERMISSIONS__;
  var RPC_TIMEOUT_MS = ${rpcTimeoutMs};

  // ── 注册的 RPC 方法（供宿主通过 callSkillMethod 调用） ──
  var registeredMethods = {};
  var themeChangeCallbacks = [];

  // ── RPC 基础设施 ──

  var pendingCalls = new Map();
  var callIdCounter = 0;

  function callHost(method, args) {
    return new Promise(function(resolve, reject) {
      var callId = "skill_" + (++callIdCounter) + "_" + Date.now();
      var timer = setTimeout(function() {
        pendingCalls.delete(callId);
        reject(new Error('RPC call "' + method + '" timed out'));
      }, RPC_TIMEOUT_MS);

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
      pendingCalls.set(callId, { resolve: resolve, reject: reject, timer: timer });

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

  function sendResponse(callId, result, error) {
    try {
      window.parent.postMessage({
        type: "rpc:response",
        callId: callId,
        result: result,
        error: error
      }, "*");
    } catch(e) {}
  }

  // ── ctx 对象 ──

  window.ctx = Object.freeze({
    get skillName() { return SKILL_NAME; },
    get skillId()   { return SKILL_ID; },
    get props()     { return INITIAL_PROPS; },

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
      },
      onThemeChange: function(callback) {
        if (typeof callback === "function") {
          themeChangeCallbacks.push(callback);
        }
      }
    }),

    store: Object.freeze({
      read: function(storeName, selector) {
        return callHost("store.read", { storeName: storeName, selector: selector });
      },
      write: function(storeName, value, selector) {
        return callHost("store.write", { storeName: storeName, value: value, selector: selector });
      }
    })
  });

  // ── 供 Skill 注册 RPC 方法的 API ──
  window.registerSkillMethod = function(name, fn) {
    if (typeof fn !== "function") {
      throw new Error("registerSkillMethod: second argument must be a function");
    }
    registeredMethods[name] = fn;
  };

  // ── 宿主消息监听 ──

  window.addEventListener("message", function(event) {
    var msg = event.data;
    if (!msg || typeof msg.type !== "string") { return; }

    switch (msg.type) {
      case "host:event":
        if (window.dispatchEvent) {
          window.dispatchEvent(new CustomEvent("ax:" + msg.event, { detail: msg.payload }));
        }
        if (msg.event === "theme-change" && msg.payload && msg.payload.theme) {
          document.documentElement.setAttribute("data-theme", msg.payload.theme);
          for (var ti = 0; ti < themeChangeCallbacks.length; ti++) {
            try { themeChangeCallbacks[ti](msg.payload.theme); } catch(e) {}
          }
        }
        break;

      case "host:lifecycle":
        if (msg.phase === "mount" && typeof window.onSkillMount === "function") {
          window.onSkillMount(window.ctx, msg.props || {});
        } else if (msg.phase === "unmount" && typeof window.onSkillUnmount === "function") {
          window.onSkillUnmount();
        }
        break;

      case "rpc:request":
        // 处理宿主对 skill 的 RPC 调用（callSkillMethod）
        var method = registeredMethods[msg.method];
        if (!method) {
          sendResponse(msg.callId, undefined, "Unknown method: " + msg.method);
          return;
        }
        try {
          var result = method(msg.args || {});
          if (result && typeof result.then === "function") {
            result.then(
              function(v) { sendResponse(msg.callId, v); },
              function(e) { sendResponse(msg.callId, undefined, String(e)); }
            );
          } else {
            sendResponse(msg.callId, result);
          }
        } catch (e) {
          sendResponse(msg.callId, undefined, String(e));
        }
        break;
    }
  });

  // ── 全局错误上报 ──
  window.addEventListener("error", function(event) {
    try {
      window.parent.postMessage({
        type: "skill:error",
        error: event.message || "Unhandled error",
        source: event.filename,
        line: event.lineno,
        col: event.colno
      }, "*");
    } catch(e) {}
  });

  window.addEventListener("unhandledrejection", function(event) {
    try {
      window.parent.postMessage({
        type: "skill:error",
        error: "Unhandled rejection: " + String(event.reason)
      }, "*");
    } catch(e) {}
  });

  // ── 向宿主报告就绪 ──
  try {
    window.parent.postMessage({ type: "skill:ready" }, "*");
  } catch(e) {}

  callHost("ui.getTheme").then(function(theme) {
    if (theme) {
      document.documentElement.setAttribute("data-theme", theme);
    }
  });

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
