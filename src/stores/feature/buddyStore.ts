import { create } from "zustand";

// ── 类型定义 ──────────────────────────────────────────────

export interface BuddyInfo {
  speciesId: string;
  name: string;
  emoji: string;
  level: number;
  xp: number;
  rarity: "common" | "uncommon" | "rare" | "epic" | "legendary";
  attributes: BuddyAttributes;
}

export interface BuddyAttributes {
  debugging: number; // 调试
  patience: number; // 耐心
  chaos: number; // 混乱
  wisdom: number; // 智慧
  snark: number; // 毒舌
}

export interface BuddyMessage {
  text: string;
  mood: BuddyMood;
  timestamp: number;
}

export type BuddyMood =
  | "happy"
  | "proud"
  | "curious"
  | "snarky"
  | "concerned"
  | "excited";

export interface BuddyState {
  // 当前激活的 Buddy
  activeBuddy: BuddyInfo | null;
  // 是否显示 Buddy 面板
  showPanel: boolean;
  // Buddy 消息历史
  messages: BuddyMessage[];
  // 是否可见（全局开关）
  visible: boolean;
  // 拖动位置（null 表示默认右下角）
  position: { x: number; y: number } | null;

  // Actions
  summonBuddy: (speciesId?: string) => void;
  dismissBuddy: () => void;
  togglePanel: () => void;
  addMessage: (msg: BuddyMessage) => void;
  grantXp: (amount: number) => void;
  setVisible: (v: boolean) => void;
  setPosition: (x: number, y: number) => void;
}

// ── 物种数据 ──────────────────────────────────────────────

interface SpeciesTemplate {
  speciesId: string;
  name: string;
  emoji: string;
  rarity: BuddyInfo["rarity"];
  attributes: BuddyAttributes;
}

const SPECIES: SpeciesTemplate[] = [
  {
    speciesId: "duck",
    name: "鸭子",
    emoji: "🦆",
    rarity: "common",
    attributes: { debugging: 4, patience: 3, chaos: 2, wisdom: 3, snark: 2 },
  },
  {
    speciesId: "cat",
    name: "猫咪",
    emoji: "🐱",
    rarity: "common",
    attributes: { debugging: 3, patience: 2, chaos: 4, wisdom: 3, snark: 4 },
  },
  {
    speciesId: "owl",
    name: "猫头鹰",
    emoji: "🦉",
    rarity: "uncommon",
    attributes: { debugging: 3, patience: 4, chaos: 1, wisdom: 5, snark: 2 },
  },
  {
    speciesId: "fox",
    name: "狐狸",
    emoji: "🦊",
    rarity: "uncommon",
    attributes: { debugging: 4, patience: 2, chaos: 3, wisdom: 4, snark: 3 },
  },
  {
    speciesId: "dragon",
    name: "小龙",
    emoji: "🐉",
    rarity: "rare",
    attributes: { debugging: 5, patience: 3, chaos: 4, wisdom: 4, snark: 3 },
  },
  {
    speciesId: "unicorn",
    name: "独角兽",
    emoji: "🦄",
    rarity: "epic",
    attributes: { debugging: 4, patience: 4, chaos: 3, wisdom: 5, snark: 2 },
  },
];

// ── 工具函数 ──────────────────────────────────────────────

/** 根据物种 ID 查找模板，找不到则随机选一个 */
function resolveSpecies(speciesId?: string): SpeciesTemplate {
  if (speciesId) {
    const found = SPECIES.find((s) => s.speciesId === speciesId);
    if (found) { return found; }
  }
  return SPECIES[Math.floor(Math.random() * SPECIES.length)];
}

/** 根据等级计算升级所需经验 */
function xpForNextLevel(level: number): number {
  return 100 + level * 50;
}

/** 创建一只新的 Buddy */
function createBuddy(speciesId?: string): BuddyInfo {
  const template = resolveSpecies(speciesId);
  return {
    speciesId: template.speciesId,
    name: template.name,
    emoji: template.emoji,
    level: 1,
    xp: 0,
    rarity: template.rarity,
    attributes: { ...template.attributes },
  };
}

// ── Store ─────────────────────────────────────────────────

export const useBuddyStore = create<BuddyState>((set) => ({
  activeBuddy: null,
  showPanel: false,
  messages: [],
  visible: false,
  position: null,

  summonBuddy: (speciesId) => {
    const buddy = createBuddy(speciesId);
    set({ activeBuddy: buddy, showPanel: true, visible: true });
  },

  dismissBuddy: () => {
    set({ activeBuddy: null, showPanel: false, messages: [] });
  },

  togglePanel: () => {
    set((s) => ({ showPanel: !s.showPanel }));
  },

  addMessage: (msg) => {
    set((s) => {
      const messages = [...s.messages, msg];
      if (messages.length > 50) {
        return { messages: messages.slice(-50) };
      }
      return { messages };
    });
  },

  grantXp: (amount) => {
    set((s) => {
      if (!s.activeBuddy) { return {}; }
      let { level, xp } = s.activeBuddy;
      xp += amount;

      while (xp >= xpForNextLevel(level)) {
        xp -= xpForNextLevel(level);
        level += 1;
      }

      return {
        activeBuddy: { ...s.activeBuddy, level, xp },
      };
    });
  },

  setVisible: (v) => set({ visible: v }),

  setPosition: (x, y) => set({ position: { x, y } }),
}));
