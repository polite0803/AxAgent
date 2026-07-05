// SPDX-License-Identifier: AGPL-3.0-only

/**
 * Theme-aware message toast proxy.
 *
 * antd v5 的静态 `message.success()` 等 API 无法感知动态主题（运行时切换 dark/light）。
 * 本模块从 AppInner 注入的 `App.useApp().message` 实例代理所有 message 调用，
 * 让 toast 弹窗能正确消费 dynamic theme context。
 *
 * 用法：`import { message } from "@/lib/toast"`
 * 完全兼容 `import { message } from "antd"` 的调用方式。
 */

import type { ArgsProps } from "antd/es/message";
import type { MessageInstance } from "antd/es/message/interface";
import type { ReactNode } from "react";

let _appInstance: MessageInstance | null = null;

/** 由 AppInner 在挂载后注入 App.useApp().message 实例 */
export function setMessageInstance(instance: MessageInstance) {
  _appInstance = instance;
}

function getMessage(): MessageInstance {
  // AppInner 在首次渲染时注入实例，此后所有 message 调用（事件回调中触发）都能拿到
  if (!_appInstance) {
    throw new Error(
      "message instance not initialized — setMessageInstance() must be called from AppInner before any message calls",
    );
  }
  return _appInstance;
}

// ── 与 antd MessageInstance 完全兼容的导出 API ──

export const message = {
  success(content: ReactNode, duration?: number, onClose?: () => void) {
    return getMessage().success(content, duration, onClose);
  },
  error(content: ReactNode, duration?: number, onClose?: () => void) {
    return getMessage().error(content, duration, onClose);
  },
  info(content: ReactNode, duration?: number, onClose?: () => void) {
    return getMessage().info(content, duration, onClose);
  },
  warning(content: ReactNode, duration?: number, onClose?: () => void) {
    return getMessage().warning(content, duration, onClose);
  },
  loading(content: ReactNode, duration?: number, onClose?: () => void) {
    return getMessage().loading(content, duration, onClose);
  },
  open(config: ArgsProps) {
    return getMessage().open(config);
  },
  destroy(key?: React.Key) {
    return getMessage().destroy(key);
  },
};
