import type { PageKey } from "@/types";
import { useLocation } from "react-router-dom";

const pageKeyToPath: Record<PageKey, string> = {
  chat: "/",
  knowledge: "/knowledge",
  memory: "/memory",
  link: "/link",
  gateway: "/gateway",
  files: "/files",
  settings: "/settings",
};

const pathToPageKey = (path: string): PageKey => {
  if (path === "/" || path === "") {
    return "chat";
  }
  const key = path.slice(1) as PageKey;
  if (key in pageKeyToPath) {
    return key;
  }
  return "chat";
};

export function useActivePage(): PageKey {
  const location = useLocation();
  return pathToPageKey(location.pathname);
}
