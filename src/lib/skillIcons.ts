import {
  Archive,
  BookOpen,
  Brain,
  Bug,
  ChartBar,
  Code,
  Cog,
  Database,
  ExternalLink,
  FileText,
  FolderOpen,
  Globe,
  LayoutDashboard,
  Library,
  Link2,
  MessageSquare,
  Network,
  Package,
  Play,
  Puzzle,
  Router,
  Search,
  Settings,
  Sparkles,
  Store,
  Wrench,
  type LucideIcon,
} from "lucide-react";

const ICON_MAP: Record<string, LucideIcon> = {
  MessageSquare,
  Sparkles,
  Store,
  FileText,
  BookOpen,
  Library,
  Brain,
  Link2,
  Router,
  FolderOpen,
  Settings,
  Puzzle,
  Package,
  Wrench,
  Search,
  Network,
  Play,
  Code,
  ChartBar,
  Database,
  Globe,
  Cog,
  Bug,
  Archive,
  LayoutDashboard,
  ExternalLink,
};

/** 将图标标识符（如 "lucide:FolderOpen"）解析为 React 组件 */
export function resolveIconComponent(iconStr: string): LucideIcon {
  const name = iconStr.startsWith("lucide:") ? iconStr.slice(7) : iconStr;
  const component = ICON_MAP[name];
  if (component) return component;
  return Puzzle;
}
