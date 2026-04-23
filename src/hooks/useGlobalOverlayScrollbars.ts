import { useEffect, useRef } from 'react';
import { OverlayScrollbars } from 'overlayscrollbars';

/**
 * Selectors for elements that should receive custom overlay scrollbars.
 *
 * `.overflow-y-auto` — Tailwind utility; covers sidebar, settings panels, etc.
 * `[data-os-scrollbar]` — explicit opt-in for containers using inline styles.
 *
 * NOTE: antd Bubble.List is excluded — it uses `flex-direction: column-reverse`
 * which inverts the scroll coordinate system.  OverlayScrollbars cannot handle
 * reversed scroll containers, so the chat area uses a separate lightweight
 * scroll indicator (`ChatScrollIndicator`) instead.
 */
const SCROLLABLE_SELECTORS = [
  '.overflow-y-auto',
  '[data-os-scrollbar]',
];

const OS_OPTIONS: Parameters<typeof OverlayScrollbars>[1] = {
  scrollbars: {
    theme: 'os-theme-axagent',
    autoHide: 'scroll',
    autoHideDelay: 600,
    autoHideSuspend: true,
    clickScroll: true,
  },
  overflow: {
    x: 'hidden',
  },
};

/**
 * Global hook that automatically finds scrollable containers and initialises
 * OverlayScrollbars on them.  Uses a MutationObserver to handle elements
 * that mount later (e.g. route changes, lazy components).
 *
 * The `elements.viewport` option is passed so that OverlayScrollbars re-uses
 * each existing scrollable element as the viewport, minimising DOM
 * restructuring.
 */
export function useGlobalOverlayScrollbars() {
  const instancesRef = useRef(new Map<Element, ReturnType<typeof OverlayScrollbars>>());

  useEffect(() => {
    const instances = instancesRef.current;

    function initElement(el: HTMLElement) {
      if (instances.has(el)) return;
      if (OverlayScrollbars.valid(el)) return;

      try {
        const inst = OverlayScrollbars(
          { target: el, elements: { viewport: el } },
          OS_OPTIONS,
        );
        instances.set(el, inst);
      } catch {
        // Element may have been removed before init completed
      }
    }

    function scanAndInit() {
      const selector = SCROLLABLE_SELECTORS.join(',');
      document.querySelectorAll<HTMLElement>(selector).forEach(initElement);
    }

    function cleanup() {
      instances.forEach((inst, el) => {
        if (!document.contains(el)) {
          inst.destroy();
          instances.delete(el);
        }
      });
    }

    // Initial scan
    scanAndInit();

    // Watch DOM mutations (debounced)
    let rafId = 0;
    const observer = new MutationObserver(() => {
      cancelAnimationFrame(rafId);
      rafId = requestAnimationFrame(() => {
        scanAndInit();
        cleanup();
      });
    });

    observer.observe(document.body, { childList: true, subtree: true });

    return () => {
      observer.disconnect();
      cancelAnimationFrame(rafId);
      instances.forEach((inst) => inst.destroy());
      instances.clear();
    };
  }, []);
}
