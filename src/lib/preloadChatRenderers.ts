import { preloadExtendedLanguageIcons } from 'markstream-react';

let preloadPromise: Promise<void> | null = null;

export function preloadChatRenderers(): Promise<void> {
  if (preloadPromise) return preloadPromise;

  preloadPromise = (async () => {
    try {
      const [streamMonacoModule] = await Promise.all([
        import('stream-monaco'),
      ]);

      preloadExtendedLanguageIcons();

      void streamMonacoModule;
    } catch (e) {
      console.warn('Failed to preload chat renderers:', e);
    }
  })();

  return preloadPromise;
}
