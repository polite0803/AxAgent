import i18n from "@/i18n";
import { isTauri } from "@/lib/invoke";
import { invoke } from "@/lib/invoke";
import { SHORTCUT_ACTION_LABEL_KEYS, type ShortcutAction } from "@/lib/shortcuts";
import { useSettingsStore } from "@/stores";
import type { GatewayStatus } from "@/types";
import { getAllWindows, getCurrentWindow } from "@tauri-apps/api/window";
import { message } from "antd";

function notifyShortcutTriggered(action: ShortcutAction) {
  const settings = useSettingsStore.getState().settings;
  if (!settings.shortcut_trigger_toast_enabled) { return; }
  const actionLabel = i18n.t(SHORTCUT_ACTION_LABEL_KEYS[action]);
  const text = i18n.t("settings.shortcutTriggeredMessage", { action: actionLabel });
  message.info(text);
}

function dispatchWindowEvent(name: string) {
  window.dispatchEvent(new CustomEvent(name));
}

function dispatchChatScopedEvent(name: string) {
  const isOnChat = window.location.pathname === "/";
  if (!isOnChat) {
    window.location.href = "/";
    window.setTimeout(() => {
      dispatchWindowEvent(name);
    }, 80);
  } else {
    dispatchWindowEvent(name);
  }
}

async function toggleCurrentWindow() {
  if (!isTauri()) { return; }
  const win = getCurrentWindow();
  const visible = await win.isVisible();
  if (visible) {
    await win.hide();
    return;
  }
  await win.show();
  await win.setFocus();
}

async function toggleAllWindows() {
  if (!isTauri()) { return; }
  const windows = await getAllWindows();
  if (windows.length === 0) { return; }
  const visibility = await Promise.all(windows.map((win) => win.isVisible()));
  const shouldHide = visibility.some(Boolean);
  if (shouldHide) {
    await Promise.all(windows.map((win) => win.hide()));
    return;
  }
  await Promise.all(windows.map((win) => win.show()));
  await windows[0].setFocus();
}

async function closeCurrentWindow() {
  if (!isTauri()) { return; }
  await getCurrentWindow().close();
}

async function toggleGatewayPage() {
  const status = await invoke<GatewayStatus>("get_gateway_status");
  if (status.is_running) {
    await invoke("stop_gateway");
  } else {
    await invoke("start_gateway");
  }
}

export async function executeShortcutAction(action: ShortcutAction): Promise<void> {
  switch (action) {
    case "toggleCurrentWindow":
      notifyShortcutTriggered(action);
      await toggleCurrentWindow();
      return;
    case "toggleAllWindows":
      notifyShortcutTriggered(action);
      await toggleAllWindows();
      return;
    case "closeWindow":
      notifyShortcutTriggered(action);
      await closeCurrentWindow();
      return;
    case "newConversation":
      notifyShortcutTriggered(action);
      dispatchChatScopedEvent("axagent:new-conversation");
      return;
    case "openSettings":
      notifyShortcutTriggered(action);
      if (window.location.pathname === "/settings" || window.location.pathname.startsWith("/settings/")) {
        window.location.href = "/";
      } else {
        window.location.href = "/settings";
      }
      return;
    case "toggleModelSelector":
      notifyShortcutTriggered(action);
      dispatchChatScopedEvent("axagent:toggle-model-selector");
      return;
    case "fillLastMessage":
      notifyShortcutTriggered(action);
      dispatchChatScopedEvent("axagent:fill-last-message");
      return;
    case "clearContext":
      notifyShortcutTriggered(action);
      dispatchChatScopedEvent("axagent:clear-context");
      return;
    case "clearConversationMessages":
      notifyShortcutTriggered(action);
      dispatchChatScopedEvent("axagent:clear-conversation-messages");
      return;
    case "toggleGateway":
      notifyShortcutTriggered(action);
      await toggleGatewayPage();
      return;
    case "toggleMode":
      notifyShortcutTriggered(action);
      dispatchChatScopedEvent("axagent:toggle-mode");
      return;
    case "showQuickBar":
      notifyShortcutTriggered(action);
      await invoke("show_quickbar");
      return;
  }
}
