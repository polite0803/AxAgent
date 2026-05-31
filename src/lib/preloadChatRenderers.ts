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
    import("@/pages/KnowledgeHubPage").catch(() => {});
    import("@/pages/GatewayLinkPage").catch(() => {});
  };

  if (typeof win.requestIdleCallback === "function") {
    win.requestIdleCallback(doPreload, { timeout: 5000 });
  } else {
    setTimeout(doPreload, 2000);
  }
}
