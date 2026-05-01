import type { PageKey } from "@/types";
import { useLocation, useNavigate } from "react-router-dom";

const pageKeyToPath: Record<PageKey, string> = {
  chat: "/",
  skills: "/skills",
  marketplace: "/marketplace",
  prompts: "/prompts",
  knowledge: "/knowledge",
  memory: "/memory",
  link: "/link",
  gateway: "/gateway",
  files: "/files",
  wiki: "/wiki",
  settings: "/settings",
};

const pathToPageKey = (path: string): PageKey => {
  if (path === "/" || path === "") { return "chat"; }
  const key = path.slice(1) as PageKey;
  if (key in pageKeyToPath) { return key; }
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
