// SPDX-License-Identifier: AGPL-3.0-only

import { BUILTIN_PAGE_PATH } from "@/lib/pageRegistry";
import type { PageKey } from "@/types";
import { useLocation, useNavigate } from "react-router-dom";

/** 单一路径来源：直接复用 pageRegistry 的权威映射，禁止本地散写。 */
const pageKeyToPath = BUILTIN_PAGE_PATH as Record<PageKey, string>;

const pathToPageKey = (path: string): PageKey => {
  if (path === "/" || path === "") {
    return "dashboard";
  }
  const key = path.slice(1);
  if (key in pageKeyToPath) {
    return key as PageKey;
  }
  return "chat";
};

export function useActivePage(): PageKey {
  const location = useLocation();
  return pathToPageKey(location.pathname);
}

export function usePageNavigation() {
  const navigate = useNavigate();

  const navigateTo = (page: PageKey) => {
    navigate(pageKeyToPath[page]);
  };

  const isActive = (page: PageKey): boolean => {
    return pageKeyToPath[page] === window.location.pathname;
  };

  return { navigateTo, isActive };
}

export { pageKeyToPath, pathToPageKey };
