import type { PageKey, SettingsSection } from "@/types";
import { create } from "zustand";

interface UIState {
  activePage: PageKey;
  previousPage: PageKey;
  sidebarCollapsed: boolean;
  settingsSection: SettingsSection;
  selectedProviderId: string | null;
  workflowEditorOpen: boolean;
  setActivePage: (page: PageKey) => void;
  enterSettings: () => void;
  exitSettings: () => void;
  toggleSidebar: () => void;
  setSettingsSection: (section: SettingsSection) => void;
  setSelectedProviderId: (id: string | null) => void;
  openWorkflowEditor: () => void;
  closeWorkflowEditor: () => void;
}

export const useUIStore = create<UIState>((set, get) => ({
  activePage: "chat",
  previousPage: "chat",
  sidebarCollapsed: false,
  settingsSection: "general",
  selectedProviderId: null,
  workflowEditorOpen: false,
  setActivePage: (page) => set({ activePage: page }),
  enterSettings: () => {
    const current = get().activePage;
    if (current !== "settings") {
      set({ previousPage: current, activePage: "settings" });
    }
  },
  exitSettings: () => {
    const prev = get().previousPage;
    set({ activePage: prev });
  },
  toggleSidebar: () => set((s) => ({ sidebarCollapsed: !s.sidebarCollapsed })),
  setSettingsSection: (section) => set({ settingsSection: section }),
  setSelectedProviderId: (id) => set({ selectedProviderId: id }),
  openWorkflowEditor: () => {
    set({ settingsSection: "workflow", workflowEditorOpen: true });
    const current = get().activePage;
    if (current !== "settings") {
      set({ previousPage: current, activePage: "settings" });
    }
  },
  closeWorkflowEditor: () => set({ workflowEditorOpen: false }),
}));
