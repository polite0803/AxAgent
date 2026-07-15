// SPDX-License-Identifier: AGPL-3.0-only

/**
 * Theme-aware message toast proxy.
 *
 * antd v5/v6 的静态 `message.success()` 等 API 无法感知动态主题（运行时切换 dark/light），
 * 且 v6 已将其标记为 deprecated。本模块从 AppInner 注入的 `App.useApp().message` 实例代理
 * 所有 message 调用，让 toast 弹窗能正确消费 dynamic theme context，并消除弃用告警。
 *
 * 用法：`import { message } from "@/lib/toast"`
 * 完全兼容 `import { message } from "antd"` 的调用方式：
 *  - success / error / info / warning / loading 的位置参数与 ArgsProps 配置对象两种形态
 *  - open(config) / destroy(key)
 *  - useMessage() hook（直接转发 antd 的独立 hook，返回 [实例, contextHolder]）
 *
 * 注意：antd v6 中 `useMessage` 是独立 hook，不在 MessageInstance 上，故此处单独转发。
 */

import { message as staticMessage } from "antd";
import type { ArgsProps, MessageInstance, MessageType } from "antd/es/message/interface";
import type { Key, ReactNode } from "react";

let _appInstance: MessageInstance | null = null;

/** 由 AppInner 在挂载后注入 App.useApp().message 实例 */
export function setMessageInstance(instance: MessageInstance) {
  _appInstance = instance;
}

function getMessage(): MessageInstance {
  // AppInner 在首次渲染时注入实例，此后所有 message 调用（含事件回调 / store action 中触发）都能拿到
  if (!_appInstance) {
    throw new Error(
      "message instance not initialized — setMessageInstance() must be called from AppInner before any message calls",
    );
  }
  return _appInstance;
}

// 位置参数 / 配置对象双重载统一转发。
// 运行期 antd 自身会按首个参数是否为 ArgsProps 自动判定 content / config 模式，
// 故这里始终以位置参数形态转发即可。
function call(
  method: "success" | "error" | "info" | "warning" | "loading",
  content: ReactNode | ArgsProps,
  duration?: number | (() => void),
  onClose?: () => void,
): MessageType {
  const fn = getMessage()[method] as unknown as (
    content: ReactNode | ArgsProps,
    duration?: number | (() => void),
    onClose?: () => void,
  ) => MessageType;
  return fn(content, duration, onClose);
}

export const message = {
  success: (content: ReactNode | ArgsProps, duration?: number | (() => void), onClose?: () => void) =>
    call("success", content, duration, onClose),
  error: (content: ReactNode | ArgsProps, duration?: number | (() => void), onClose?: () => void) =>
    call("error", content, duration, onClose),
  info: (content: ReactNode | ArgsProps, duration?: number | (() => void), onClose?: () => void) =>
    call("info", content, duration, onClose),
  warning: (content: ReactNode | ArgsProps, duration?: number | (() => void), onClose?: () => void) =>
    call("warning", content, duration, onClose),
  loading: (content: ReactNode | ArgsProps, duration?: number | (() => void), onClose?: () => void) =>
    call("loading", content, duration, onClose),
  open: (config: ArgsProps) => getMessage().open(config),
  destroy: (key?: Key) => getMessage().destroy(key),
  // antd v6 的独立 hook（返回 [实例, contextHolder]），直接转发，不触发静态 message 弃用告警
  useMessage: staticMessage.useMessage,
};
