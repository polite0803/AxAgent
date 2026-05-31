import { preloadExtendedLanguageIcons } from "markstream-react";

let preloadPromise: Promise<void> | null = null;

export function preloadChatRenderers(): Promise<void> {
  if (preloadPromise) {
    return preloadPromise;
  }

  preloadPromise = (async () => {
    try {
      const [streamMonacoModule] = await Promise.all([import("stream-monaco")]);

      preloadExtendedLanguageIcons();

      void streamMonacoModule;
    } catch (e) {
      console.warn("Failed to preload chat renderers:", e);
    }
  })();

  return preloadPromise;
}

let pagesPreloaded = false;

export function preloadCommonPages(): void {
  if (pagesPreloaded) {
    return;
  }
  pagesPreloaded = true;

  const win = window as Window & {
    requestIdleCallback?: (cb: () => void, opts?: { timeout: number }) => number;
  };

  const doPreload = () => {
    Promise.all([
      import("@/pages/KnowledgeHubPage"),
      import("@/pages/GatewayLinkPage"),
      import("@/pages/SettingsPage"),
      import("@/pages/TerminalPage"),
      import("@/pages/FilesPage"),
      import("@/pages/WorkflowPage"),
    ]).catch(() => {});
  };

  if (typeof win.requestIdleCallback === "function") {
    win.requestIdleCallback(doPreload, { timeout: 3000 });
  } else {
    setTimeout(doPreload, 1000);
  }
}
