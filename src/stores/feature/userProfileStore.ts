import { invoke, isTauri } from "@/lib/invoke";
import { create } from "zustand";
import { persist } from "zustand/middleware";

export type AvatarType = "icon" | "emoji" | "url" | "file";

export type NamingConvention = "camel_case" | "snake_case" | "pascal_case" | "kebab_case";
export type IndentationStyle = "spaces" | "tabs";
export type CommentStyle = "minimal" | "documented" | "verbose";
export type ModuleOrgStyle = "by_feature" | "by_type" | "by_layer" | "flat";
export type DetailLevel = "concise" | "moderate" | "detailed";
export type Tone = "formal" | "neutral" | "casual";
export type SkillLevel = "beginner" | "intermediate" | "advanced" | "expert";

export interface CodingStylePreferences {
  namingConvention: NamingConvention;
  indentationStyle: IndentationStyle;
  indentationSize: number;
  commentStyle: CommentStyle;
  moduleOrgStyle: ModuleOrgStyle;
  preferredLanguages: string[];
  preferredFrameworks: string[];
  confidence: number;
}

export interface CommunicationPreferences {
  detailLevel: DetailLevel;
  tone: Tone;
  language: string;
  includeExplanations: boolean;
  showReasoning: boolean;
  confidence: number;
}

export interface WorkHabitPreferences {
  peakHours: { start: number; end: number };
  lowActivityHours: { start: number; end: number };
  preferredDays: string[];
  sessionLength: number;
  breakFrequency: number;
  multiTaskingLevel: number;
  confidence: number;
}

export interface DomainKnowledgeProfile {
  expertiseAreas: Array<{
    name: string;
    level: SkillLevel;
    yearsExperience: number;
  }>;
  interestTopics: string[];
  confidence: number;
}

export interface LearningStateProfile {
  totalInteractions: number;
  explicitSettings: string[];
  lastUpdated: string;
  stabilityScore: number;
}

export interface TrajectoryUserProfile {
  id: string;
  userId: string;
  createdAt: string;
  updatedAt: string;
  codingStyle: CodingStylePreferences;
  communication: CommunicationPreferences;
  workHabits: WorkHabitPreferences;
  domainKnowledge: DomainKnowledgeProfile;
  learningState: LearningStateProfile;
}

interface UserProfile {
  name: string;
  avatarType: AvatarType;
  avatarValue: string;
}

interface UserProfileState {
  profile: UserProfile;
  trajectoryProfile: TrajectoryUserProfile | null;
  isLoading: boolean;
  error: string | null;
  updateProfile: (partial: Partial<UserProfile>) => void;
  saveAvatarFile: (dataUri: string) => Promise<void>;
  loadTrajectoryProfile: () => Promise<void>;
  updateTrajectoryProfile: (updates: Partial<TrajectoryUserProfile>) => Promise<void>;
  updateCodingStyle: (codingStyle: Partial<CodingStylePreferences>) => Promise<void>;
  updateCommunicationPrefs: (commPrefs: Partial<CommunicationPreferences>) => Promise<void>;
  updateWorkHabits: (workHabits: Partial<WorkHabitPreferences>) => Promise<void>;
  clearTrajectoryData: () => Promise<void>;
}

const defaultTrajectoryProfile: TrajectoryUserProfile = {
  id: "",
  userId: "",
  createdAt: new Date().toISOString(),
  updatedAt: new Date().toISOString(),
  codingStyle: {
    namingConvention: "snake_case",
    indentationStyle: "spaces",
    indentationSize: 2,
    commentStyle: "documented",
    moduleOrgStyle: "by_feature",
    preferredLanguages: [],
    preferredFrameworks: [],
    confidence: 0,
  },
  communication: {
    detailLevel: "moderate",
    tone: "neutral",
    language: "en",
    includeExplanations: true,
    showReasoning: true,
    confidence: 0,
  },
  workHabits: {
    peakHours: { start: 9, end: 17 },
    lowActivityHours: { start: 0, end: 6 },
    preferredDays: ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday"],
    sessionLength: 25,
    breakFrequency: 5,
    multiTaskingLevel: 3,
    confidence: 0,
  },
  domainKnowledge: {
    expertiseAreas: [],
    interestTopics: [],
    confidence: 0,
  },
  learningState: {
    totalInteractions: 0,
    explicitSettings: [],
    lastUpdated: new Date().toISOString(),
    stabilityScore: 0,
  },
};

export const useUserProfileStore = create<UserProfileState>()(
  persist(
    (set, get) => ({
      profile: {
        name: "",
        avatarType: "icon",
        avatarValue: "",
      },
      trajectoryProfile: null,
      isLoading: false,
      error: null,
      updateProfile: (partial) => {
        set({ profile: { ...get().profile, ...partial } });
      },
      saveAvatarFile: async (dataUri: string) => {
        const match = dataUri.match(/^data:([^;]+);base64,(.+)$/s);
        if (!match) { throw new Error("Invalid data URI"); }
        const [, mimeType, data] = match;
        if (isTauri()) {
          const relativePath = await invoke<string>("save_avatar_file", {
            data,
            mimeType,
          });
          set({
            profile: {
              ...get().profile,
              avatarType: "file",
              avatarValue: relativePath,
            },
          });
        } else {
          set({
            profile: {
              ...get().profile,
              avatarType: "file",
              avatarValue: dataUri,
            },
          });
        }
      },
      loadTrajectoryProfile: async () => {
        if (!isTauri()) {
          set({ trajectoryProfile: defaultTrajectoryProfile, isLoading: false });
          return;
        }
        set({ isLoading: true, error: null });
        try {
          const profile = await invoke<TrajectoryUserProfile>("get_user_profile");
          set({ trajectoryProfile: profile, isLoading: false });
        } catch (err) {
          set({
            trajectoryProfile: defaultTrajectoryProfile,
            error: err instanceof Error ? err.message : "Failed to load profile",
            isLoading: false,
          });
        }
      },
      updateTrajectoryProfile: async (updates) => {
        if (!isTauri()) {
          set({ error: "Profile update only available in Tauri app" });
          return;
        }
        set({ isLoading: true, error: null });
        try {
          const updated = await invoke<TrajectoryUserProfile>("update_user_profile", { updates });
          set({ trajectoryProfile: updated, isLoading: false });
        } catch (err) {
          set({
            error: err instanceof Error ? err.message : "Failed to update profile",
            isLoading: false,
          });
        }
      },
      updateCodingStyle: async (codingStyle) => {
        const current = get().trajectoryProfile;
        if (!current) { return; }
        await get().updateTrajectoryProfile({
          codingStyle: { ...current.codingStyle, ...codingStyle },
        });
      },
      updateCommunicationPrefs: async (commPrefs) => {
        const current = get().trajectoryProfile;
        if (!current) { return; }
        await get().updateTrajectoryProfile({
          communication: { ...current.communication, ...commPrefs },
        });
      },
      updateWorkHabits: async (workHabits) => {
        const current = get().trajectoryProfile;
        if (!current) { return; }
        await get().updateTrajectoryProfile({
          workHabits: { ...current.workHabits, ...workHabits },
        });
      },
      clearTrajectoryData: async () => {
        if (!isTauri()) {
          set({ error: "Profile clear only available in Tauri app" });
          return;
        }
        set({ isLoading: true, error: null });
        try {
          await invoke<void>("clear_user_profile_data");
          set({ trajectoryProfile: defaultTrajectoryProfile, isLoading: false });
        } catch (err) {
          set({
            error: err instanceof Error ? err.message : "Failed to clear profile",
            isLoading: false,
          });
        }
      },
    }),
    {
      name: "axagent_user_profile",
      partialize: (state) => ({
        profile: state.profile,
      }),
    },
  ),
);
