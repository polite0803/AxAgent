// SPDX-License-Identifier: AGPL-3.0-only

/**
 * Unified page registry — single source of truth for all built-in routes.
 *
 * New built-in pages should be added here; the home-page selector in
 * GeneralSettings and the ContentArea redirect will pick them up
 * automatically. Skill / dynamic pages are registered separately via
 * skillExtensionStore.pages and appended at render time.
 */

export interface RegistryPage {
  /** Path the route listens on, e.g. "/knowledge" */
  path: string;
  /** i18n label key used in nav / dropdown, e.g. "nav.knowledge" */
  labelKey: string;
}

/**
 * Ordered list of built-in pages that are valid home-page targets.
 * The order here controls the order in the settings drop-down.
 *
 * Pages excluded:
 * - "chat"     → maps to "/" which is the redirect itself (circular)
 * - "settings" → makes no sense as a landing page
 * - "devtools" → debug-only
 * - "link"     → transient connection flows
 * - "marketplace" → no route defined
 */
const BUILTIN_HOME_PAGES: RegistryPage[] = [
  { path: "/dashboard", labelKey: "nav.dashboard" },
  { path: "/knowledge", labelKey: "nav.knowledge" },
  { path: "/memory", labelKey: "nav.memory" },
  { path: "/gateway", labelKey: "nav.gateway" },
  { path: "/terminal", labelKey: "nav.terminal" },
  { path: "/files", labelKey: "nav.files" },
  { path: "/workflow", labelKey: "nav.workflow" },
  { path: "/dynamic-ui", labelKey: "nav.dynamicUI" },
  { path: "/wiki", labelKey: "nav.wiki" },
];

/**
 * Full key→path mapping for all built-in pages (including those not
 * exposed in the home-page selector — used by sidebar etc.).
 */
export const BUILTIN_PAGE_PATH: Record<string, string> = {
  chat: "/",
  dashboard: "/dashboard",
  knowledge: "/knowledge",
  memory: "/memory",
  link: "/link",
  gateway: "/gateway",
  settings: "/settings",
  workflow: "/workflow",
  files: "/files",
  terminal: "/terminal",
  "dynamic-ui": "/dynamic-ui",
  marketplace: "/marketplace",
  wiki: "/wiki",
};

export default BUILTIN_HOME_PAGES;
