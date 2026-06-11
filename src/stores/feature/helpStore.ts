// SPDX-License-Identifier: AGPL-3.0-only

// 帮助面板全局状态
import { create } from "zustand";

interface HelpStore {
  open: boolean;
  activeSection: string | null;
  toggle: () => void;
  openSection: (section: string) => void;
  close: () => void;
}

export const useHelpStore = create<HelpStore>((set) => ({
  open: false,
  activeSection: null,
  toggle: () => set((s) => ({ open: !s.open })),
  openSection: (section) => set({ open: true, activeSection: section }),
  close: () => set({ open: false, activeSection: null }),
}));
